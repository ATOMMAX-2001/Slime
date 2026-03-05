use axum::{
    Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::any,
};

use bytes::Bytes;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use rayon::ThreadPoolBuilder;

use axum::extract::Path;
use minijinja::{AutoEscape, Environment, path_loader};
use std::path::Path as OsPath;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::{io, net::SocketAddr};
use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::ReceiverStream;
use tower_http::services::ServeDir;

use crate::constant::SERVER;
use crate::request::{SlimeFile, SlimeRequest};
use crate::response::{SlimeResponse, SlimeStreamResponse};
use std::collections::HashMap;

pub struct Route {
    pub path: String,
    pub method: String,
    pub stream: Option<String>,
    pub handler: Arc<Vec<Py<PyAny>>>,
}

impl Clone for Route {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            method: self.method.clone(),
            stream: self.stream.to_owned(),
            handler: self.handler.clone(),
        }
    }
}

impl Route {
    pub fn new(
        path: String,
        method: String,
        stream: Option<String>,
        handler: Vec<Py<PyAny>>,
    ) -> Self {
        Self {
            path,
            method,
            stream,
            handler: Arc::new(handler),
        }
    }
}

pub enum PyRequestWorker {
    Http(PyRequest),
    Stream(PyRequestStream),
}

pub struct PyRequestStream {
    pub handler: Arc<Vec<Py<PyAny>>>,
    pub request: SlimeRequest,
    pub response: SlimeStreamResponse,
}

pub struct PyRequest {
    pub handler: Arc<Vec<Py<PyAny>>>,
    pub request: SlimeRequest,
    pub response: oneshot::Sender<PyResult<SlimeResponse>>,
}

pub struct SlimeServer {
    filename: String,
    is_dev: bool,
    routes: Vec<Route>,
    host: String,
    port: usize,
    worker_txs: Arc<Vec<mpsc::Sender<PyRequestWorker>>>,
    request_counter: Arc<AtomicUsize>,
    secret_key: Arc<Vec<u8>>,
    template: Arc<Environment<'static>>,
    tokio_handler: tokio::runtime::Handle,
}

impl SlimeServer {
    pub fn new(
        host: String,
        port: usize,
        worker_txs: Arc<Vec<mpsc::Sender<PyRequestWorker>>>,
        secret_key: String,
        filename: String,
        is_dev: bool,
        tokio_runtime_handler: tokio::runtime::Handle,
    ) -> SlimeServer {
        let env = SlimeServer::get_template_environment(&filename);
        SlimeServer {
            filename: filename,
            is_dev: is_dev,
            routes: Vec::with_capacity(5),
            host,
            port,
            worker_txs,
            request_counter: Arc::new(AtomicUsize::new(0)),
            secret_key: Arc::new(secret_key.as_bytes().to_vec()),
            template: Arc::new(env),
            tokio_handler: tokio_runtime_handler,
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
            let stream: Option<String> = key.getattr("stream")?.extract()?;
            let handler = value.cast::<PyDict>().unwrap();
            let mut handlers: Vec<Py<PyAny>> = Vec::with_capacity(3);
            if let Ok(Some(before_handler)) = handler.get_item("before") {
                if !before_handler.is_none() {
                    handlers.push(before_handler.unbind());
                }
            }
            if let Ok(Some(request_handler)) = handler.get_item("handler") {
                if !request_handler.is_none() {
                    handlers.push(request_handler.unbind());
                }
            }
            if let Ok(Some(after_handler)) = handler.get_item("after") {
                if !after_handler.is_none() {
                    handlers.push(after_handler.unbind());
                }
            }
            routes_collection.push(Route::new(path, method, stream, handlers));
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
            let stream_content = route.stream;
            let handler = route.handler.clone();
            let worker_txs = self.worker_txs.clone();
            let request_counter = self.request_counter.clone();
            let worker_count = worker_txs.len();
            let secret_key = self.secret_key.clone();
            let mut template_engine = self.template.clone();
            let is_dev = self.is_dev;
            let filename = self.filename.to_owned();
            let tokio_runtime = self.tokio_handler.clone();
            server_router = server_router.route(
                &path.to_owned(),
                any(
                    move |ConnectInfo(client): ConnectInfo<SocketAddr>,
                          Path(params): Path<HashMap<String, String>>,
                          request: Request<Body>| {
                        if is_dev {
                            println!(
                                "INFO: {} => {} {}",
                                &method,
                                &path,
                                &client.ip().to_string()
                            );
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
                            let content_type = &parts
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
                            if content_type != &"" {
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
                                client: client.ip(),
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

                            // send request to python workers
                            if stream_content.is_some() {
                                let stream_content_type = stream_content.unwrap();
                                let (stream_tx, stream_rx) =
                                    mpsc::channel::<Result<Bytes, io::Error>>(100);
                                let (started_tx, mut started_rx) =
                                    mpsc::channel::<HashMap<String, String>>(1);
                                let new_slime_stream_resonse = SlimeStreamResponse::new(
                                    stream_content_type.to_owned(),
                                    stream_tx,
                                    tokio_runtime.clone(),
                                    started_tx,
                                );
                                if let Err(err) = worker_tx
                                    .send(PyRequestWorker::Stream(PyRequestStream {
                                        handler,
                                        request: slime_request,
                                        response: new_slime_stream_resonse,
                                    }))
                                    .await
                                {
                                    return (
                                        StatusCode::INTERNAL_SERVER_ERROR,
                                        format!(
                                            "Worker cant able to handle the request (reason) => {}",
                                            err.to_string()
                                        ),
                                    )
                                        .into_response();
                                }
                                if let Some(headers) = started_rx.recv().await {
                                    let mut new_response = Response::builder()
                                        .header("content-type", stream_content_type)
                                        .header("Server", SERVER);
                                    for (key, value) in headers {
                                        new_response = new_response.header(key, value);
                                    }
                                    let stream = ReceiverStream::new(stream_rx);
                                    let body = Body::from_stream(stream);
                                    return new_response.body(body).unwrap();
                                }
                            } else {
                                if let Err(err) = worker_tx
                                    .send(PyRequestWorker::Http(PyRequest {
                                        handler,
                                        request: slime_request,
                                        response: resp_tx,
                                    }))
                                    .await
                                {
                                    return (
                                        StatusCode::INTERNAL_SERVER_ERROR,
                                        format!("Worker down cant able to handle the request (reason) => {}",err.to_string()),
                                    )
                                        .into_response();
                                }
                            }

                            // to client side response
                            match resp_rx.await {
                                Ok(Ok(result)) => {
                                    return result._into_response();
                                }
                                Ok(Err(err)) => {
                                    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
                                        .into_response()
                                }
                                Err(_) => (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    "Worker cant able to handle the response".to_string(),
                                )
                                    .into_response(),
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
        let _ = axum::serve(
            listener,
            server_router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal())
        .await;
        Ok(())
    }
}

async fn shutdown_signal() {
    signal::ctrl_c().await.expect("Failed to listen for ctrl_c");
}

pub fn spawn_python_workers(worker_count: usize) -> Arc<Vec<mpsc::Sender<PyRequestWorker>>> {
    let mut worker_txs = Vec::with_capacity(worker_count);
    let pool = ThreadPoolBuilder::new()
        .num_threads(worker_count)
        .build()
        .unwrap();
    for _ in 0..worker_count {
        let (tx, rx) = mpsc::channel::<PyRequestWorker>(1024 * 1024 * 10);
        worker_txs.push(tx.clone());
        pool.spawn(move || handle_python_call(rx));
    }
    return Arc::new(worker_txs);
}

#[inline]
fn handle_python_call(mut rx: mpsc::Receiver<PyRequestWorker>) {
    Python::attach(|py| {
        while let Some(req_worker) = py.detach(|| rx.blocking_recv()) {
            if let PyRequestWorker::Http(req) = req_worker {
                match (
                    Py::new(py, SlimeResponse::new(py)),
                    Py::new(py, req.request),
                ) {
                    (Ok(response_py), Ok(request_py)) => {
                        let mut is_error: Option<PyErr> = None;
                        for handler_method in 0..req.handler.len() {
                            if let Err(err) =
                                req.handler[handler_method].call1(py, (&request_py, &response_py))
                            {
                                let path = request_py.getattr(py, "path").unwrap().to_string();
                                let method = request_py.getattr(py, "method").unwrap().to_string();
                                println!(
                                    "ERROR @ path: [{}] for method [{}]: {}",
                                    path, method, err
                                );
                                is_error = Some(err);
                                break;
                            }
                        }
                        if is_error.is_some() {
                            let _ = &req.response.send(Err(is_error.unwrap()));
                        } else {
                            let result = response_py.borrow(py).clone_obj(py);
                            let _ = req.response.send(Ok(result));
                        }
                    }
                    _ => {
                        let _ = req
                            .response
                            .send(Err(pyo3::exceptions::PyException::new_err(
                                "Cant able to create request and response handler".to_string(),
                            )));
                    }
                }
            } else if let PyRequestWorker::Stream(req) = req_worker {
                match (Py::new(py, req.request), Py::new(py, req.response)) {
                    (Ok(request_py), Ok(response_py)) => {
                        for handler_method in 0..req.handler.len() {
                            if let Err(err) =
                                req.handler[handler_method].call1(py, (&request_py, &response_py))
                            {
                                println!("ERROR: {}", err);
                            }
                            break;
                        }
                    }
                    _ => {
                        println!("ERROR: Cant able to create reqeust and response handler");
                    }
                }
            } else {
                // websocket in future
            }
        }
    });
}
