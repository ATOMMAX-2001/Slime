use axum::{
    Router,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use bytes::Bytes;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use rayon::ThreadPoolBuilder;

use axum::{
    body::{Body, to_bytes},
    http::Request,
    routing::any,
};

use axum::extract::Path;
use minijinja::{AutoEscape, Environment, path_loader};
use std::net::SocketAddr;
use std::path::Path as OsPath;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::ReceiverStream;
use tower_http::services::ServeDir;

use std::collections::HashMap;

use crate::request::{SlimeFile, SlimeRequest};
use crate::response::SlimeResponse;

pub struct Route {
    pub path: String,
    pub method: String,
    pub handler: Arc<Py<PyAny>>,
}

impl Clone for Route {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            method: self.method.clone(),
            handler: self.handler.clone(),
        }
    }
}

impl Route {
    pub fn new(path: String, method: String, handler: Py<PyAny>) -> Self {
        Self {
            path,
            method,
            handler: Arc::new(handler),
        }
    }
}

pub enum PyResponse {
    Http(SlimeResponse),
    Stream(mpsc::Receiver<Result<Bytes, String>>),
}

pub struct PyRequest {
    pub handler: Arc<Py<PyAny>>,
    pub request: SlimeRequest,
    pub response: oneshot::Sender<PyResult<PyResponse>>,
}

pub struct SlimeServer {
    filename: String,
    is_dev: bool,
    routes: Vec<Route>,
    host: String,
    port: usize,
    worker_txs: Arc<Vec<mpsc::Sender<PyRequest>>>,
    request_counter: Arc<AtomicUsize>,
    secret_key: Arc<Vec<u8>>,
    template: Arc<Environment<'static>>,
}

impl SlimeServer {
    pub fn new(
        host: String,
        port: usize,
        worker_txs: Arc<Vec<mpsc::Sender<PyRequest>>>,
        secret_key: String,
        filename: String,
        is_dev: bool,
    ) -> Self {
        let env = SlimeServer::get_template_environment(&filename);
        Self {
            filename: filename,
            is_dev: is_dev,
            routes: Vec::with_capacity(5),
            host,
            port,
            worker_txs,
            request_counter: Arc::new(AtomicUsize::new(0)),
            secret_key: Arc::new(secret_key.as_bytes().to_vec()),
            template: Arc::new(env),
        }
    }
    fn get_template_environment(filename: &String) -> Environment<'static> {
        let mut env = Environment::new();

        if let Some(file_path) = OsPath::new(&filename).parent() {
            env.set_loader(path_loader(file_path.join("templates")));
            env.set_auto_escape_callback(|name| {
                if name.ends_with(".html") {
                    AutoEscape::Html
                } else {
                    AutoEscape::None
                }
            });
        } else {
            println!("ERROR: Cant able to load template invalid path");
        };
        return env;
    }
    pub fn load_routes(&mut self, routes: &Bound<PyDict>) -> PyResult<()> {
        let mut routes_collection = Vec::with_capacity(5);
        for (key, value) in routes {
            let path: String = key.getattr("path")?.extract()?;
            let method: String = key.getattr("method")?.extract()?;
            let handler = value.unbind();
            routes_collection.push(Route::new(path, method, handler));
        }
        self.routes = routes_collection;
        Ok(())
    }

    fn set_server_routes(&self) -> Router {
        let mut server_router = Router::new();
        let static_dir = OsPath::new(&self.filename).parent().unwrap().join("static");
        let static_service = ServeDir::new(static_dir);
        server_router = server_router.nest_service("/static", static_service);
        for route in &self.routes {
            let route = route.clone();
            let path = route.path;
            let method = route.method;
            let handler = route.handler.clone();
            let worker_txs = self.worker_txs.clone();
            let request_counter = self.request_counter.clone();
            let worker_count = worker_txs.len();
            let secret_key = self.secret_key.clone();
            let mut template_engine = self.template.clone();
            let is_dev = self.is_dev;
            let filename = self.filename.to_owned();
            server_router = server_router.route(
                &path,
                any(
                    move |Path(params): Path<HashMap<String, String>>, request: Request<Body>| {
                        let handler = handler.clone();
                        let worker_txs = worker_txs.clone();
                        if is_dev {
                            template_engine =
                                Arc::new(SlimeServer::get_template_environment(&filename));
                        }
                        async move {
                            if request.method().as_str() != method {
                                return StatusCode::METHOD_NOT_ALLOWED.into_response();
                            }
                            let idx = request_counter.fetch_add(1, Ordering::Relaxed);
                            let worker_tx = &worker_txs[idx % worker_count];

                            let (resp_tx, resp_rx) = oneshot::channel();
                            let (parts, raw_body) = request.into_parts();
                            let content_type = parts
                                .headers
                                .get("content-type")
                                .and_then(|value| value.to_str().ok())
                                .unwrap_or("");

                            let body = match to_bytes(raw_body, 1024 * 1024 * 10).await {
                                Ok(bod) => bod,
                                Err(_) => return StatusCode::BAD_REQUEST.into_response(),
                            };
                            let mut json_body: Option<serde_json::Value> = None;
                            let mut form_body: Option<HashMap<String, String>> = None;
                            let mut file_body: Option<Vec<SlimeFile>> = None;
                            if content_type != "" {
                                if content_type.starts_with("application/json") {
                                    json_body =
                                        serde_json::from_slice::<serde_json::Value>(&body).ok();
                                } else if content_type
                                    .starts_with("application/x-www-form-urlencoded")
                                {
                                    form_body = serde_urlencoded::from_bytes::<
                                        HashMap<String, String>,
                                    >(&body)
                                    .ok();
                                } else if content_type.starts_with("multipart/form-data") {
                                    if let Ok(boundary) = multer::parse_boundary(content_type) {
                                        let body_clone = body.clone();
                                        let stream = futures_util::stream::once(async move {
                                            Ok::<_, std::io::Error>(body_clone)
                                        });

                                        let mut multipart =
                                            multer::Multipart::new(stream, boundary);

                                        let mut text_fields = HashMap::new();
                                        let mut files = Vec::with_capacity(2);
                                        while let Some(mut field) =
                                            multipart.next_field().await.unwrap_or(None)
                                        {
                                            let name = field.name().map(|s| s.to_string());

                                            if field.file_name().is_none() {
                                                if let (Some(name), Ok(text)) =
                                                    (name, field.text().await)
                                                {
                                                    text_fields.insert(name, text);
                                                }
                                            } else {
                                                // file uploads

                                                let content_type = field
                                                    .content_type()
                                                    .map(|value| value.to_string());
                                                let filename =
                                                    format!("slime_file_{}", uuid::Uuid::new_v4());
                                                let temp_path =
                                                    std::env::temp_dir().join(&filename);
                                                if let Ok(mut file) =
                                                    tokio::fs::File::create(&temp_path).await
                                                {
                                                    let mut size = 0;
                                                    while let Ok(Some(chunk)) = field.chunk().await
                                                    {
                                                        size += chunk.len();
                                                        if let Err(err) =
                                                            tokio::io::AsyncWriteExt::write_all(
                                                                &mut file, &chunk,
                                                            )
                                                            .await
                                                        {
                                                            pyo3::exceptions::PyException::new_err(
                                                                err.to_string(),
                                                            );
                                                        }
                                                    }
                                                    let file_content_type = content_type
                                                        .unwrap_or("UNKNOWN".to_string());
                                                    let extension: String = file_content_type
                                                        .split("/")
                                                        .last()
                                                        .unwrap_or("UNKNOWN")
                                                        .to_string();
                                                    files.push(SlimeFile {
                                                        filename: filename,
                                                        content_type: file_content_type,
                                                        temp_path: temp_path,
                                                        extension: extension,
                                                        size: size,
                                                    });
                                                } else {
                                                    pyo3::exceptions::PyException::new_err(
                                                        "Failed to create file",
                                                    );
                                                }
                                            }
                                        }
                                        file_body = Some(files);
                                        form_body = Some(text_fields);
                                    }
                                }
                            }

                            let query_params: HashMap<String, String> =
                                serde_urlencoded::from_str(parts.uri.query().unwrap_or(""))
                                    .unwrap_or_default();
                            let slime_request = SlimeRequest {
                                uri: parts.uri,
                                method: parts.method,
                                header: Arc::new(parts.headers),
                                body: body,
                                secret: secret_key,
                                template: template_engine,
                                query: query_params,
                                json_body: json_body,
                                form: form_body,
                                files: file_body,
                                params: params,
                            };
                            if worker_tx
                                .send(PyRequest {
                                    handler,
                                    request: slime_request,
                                    response: resp_tx,
                                })
                                .await
                                .is_err()
                            {
                                return (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    "Worker down".to_string(),
                                )
                                    .into_response();
                            }

                            // to client side response
                            match resp_rx.await {
                                Ok(Ok(PyResponse::Http(result))) => {
                                    return result._into_response();
                                }
                                Ok(Ok(PyResponse::Stream(result))) => {
                                    let stream = ReceiverStream::new(result);
                                    let body = Body::from_stream(stream);
                                    return Response::builder()
                                        .header("content-type", "text/plain")
                                        .body(body)
                                        .unwrap();
                                }
                                Ok(Err(err)) => {
                                    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
                                        .into_response()
                                }
                                Err(err) => {
                                    dbg!(&err);
                                    (
                                        StatusCode::INTERNAL_SERVER_ERROR,
                                        "Worker dropped".to_string(),
                                    )
                                        .into_response()
                                }
                            }
                        }
                    },
                ),
            );
        }
        return server_router;
    }

    pub async fn server_run(self) -> PyResult<()> {
        let address: SocketAddr = format!("{}:{}", self.host, self.port).parse()?;
        let server_router = self.set_server_routes();
        let listener = TcpListener::bind(address).await.unwrap();

        println!("Slime server is running at {}", address);
        let _ = axum::serve(listener, server_router)
            .with_graceful_shutdown(shutdown_signal())
            .await;
        Ok(())
    }
}

async fn shutdown_signal() {
    signal::ctrl_c().await.expect("Failed to listen for ctrl_c");
}

pub fn spawn_python_workers(
    worker_count: usize,
    runtime: &tokio::runtime::Handle,
) -> Arc<Vec<mpsc::Sender<PyRequest>>> {
    let mut worker_txs = Vec::with_capacity(worker_count);
    let pool = ThreadPoolBuilder::new()
        .num_threads(worker_count)
        .build()
        .unwrap();
    for _ in 0..worker_count {
        let (tx, mut rx) = mpsc::channel::<PyRequest>(1024);
        worker_txs.push(tx.clone());
        let runtime_handler = runtime.clone();
        pool.spawn(move || {
            Python::attach(|py| {
                while let Some(req) = py.detach(|| rx.blocking_recv()) {
                    match Py::new(py, SlimeResponse::new(py)) {
                        Ok(response_py) => {
                            match req
                                .handler
                                .call1(py, (req.request, response_py.clone_ref(py)))
                            {
                                Ok(_) => {
                                    let result = response_py.borrow(py).clone_obj(py);
                                    if result.is_stream {
                                        // todo streaming data
                                    } else {
                                        let _ = req.response.send(Ok(PyResponse::Http(result)));
                                    }
                                }
                                Err(err) => {
                                    let _ = req.response.send(Err(err));
                                }
                            }
                        }
                        Err(err) => {
                            let _ = req.response.send(Err(err));
                        }
                    }
                }
            });
        });
    }
    return Arc::new(worker_txs);
}
