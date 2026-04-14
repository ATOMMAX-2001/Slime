use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{
        ConnectInfo, FromRequest, State,
        ws::{Message, WebSocketUpgrade},
    },
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    routing::MethodRouter,
};
use bytes::Bytes;
use dashmap::DashMap;
use futures_util::StreamExt;
use pyo3::types::{PyBytes, PyDict, PyList};
use pyo3::{prelude::*, types::PyTuple};
use rayon::{ThreadPoolBuilder, yield_now};

use axum::extract::Path;
use minijinja::{AutoEscape, Environment, path_loader};
use std::path::Path as OsPath;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::{io, net::SocketAddr};
use tokio::{
    net::TcpListener,
    runtime::Handle,
    signal,
    sync::{mpsc, oneshot},
};
use tokio_stream::wrappers::ReceiverStream;
use tower_http::{compression::CompressionLayer, services::ServeDir};
use uuid::Uuid;

use crate::response::{SlimeResponse, SlimeStreamResponse, SlimeWebSocketResponse};
use crate::{constant::SERVER, request::SlimeState};
use crate::{
    request::{SlimeFile, SlimeRequest},
    worker,
};

use futures_util::SinkExt;
use pyo3_async_runtimes::{self as py_asyncio, TaskLocals};
use std::collections::HashMap;

pub struct Route {
    pub path: String,
    pub method: String,
    pub stream: Option<String>,
    pub ws: bool,
    pub compression: u8,
    pub body_size: usize,
    pub handler: Arc<Vec<Py<PyAny>>>,
    pub is_async: bool,
}

impl Clone for Route {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            method: self.method.clone(),
            stream: self.stream.to_owned(),
            ws: self.ws,
            compression: self.compression,
            body_size: self.body_size,
            handler: self.handler.clone(),
            is_async: self.is_async,
        }
    }
}

impl Route {
    pub fn new(
        path: String,
        method: String,
        stream: Option<String>,
        ws: bool,
        compression: u8,
        body_size: usize,
        handler: Vec<Py<PyAny>>,
        is_async: bool,
    ) -> Self {
        Self {
            path,
            method,
            stream,
            ws,
            compression,
            body_size,
            handler: Arc::new(handler),
            is_async: is_async,
        }
    }
}

pub enum PyRequestWorker {
    Http(PyRequest),
    Stream(PyRequestStream),
    WebSocket(PyRequestWebSocket),
}

pub struct PyRequestStream {
    pub handler: Arc<Vec<Py<PyAny>>>,
    pub request: SlimeRequest,
    pub response: SlimeStreamResponse,
}

pub struct PyRequest {
    pub handler: Arc<Vec<Py<PyAny>>>,
    pub request: SlimeRequest,
    pub response: oneshot::Sender<Result<Response<Body>, PyErr>>,
}

#[derive(Clone)]
pub struct WebSocketConn {
    pub id: Uuid,
    pub sender: mpsc::Sender<Bytes>,
}

pub struct PyRequestWebSocket {
    pub handler: Arc<Vec<Py<PyAny>>>,
    pub request: SlimeRequest,
    pub response: oneshot::Sender<Result<SlimeWebSocketResponse, PyErr>>,
    pub conn: WebSocketConn,
}

#[derive(Clone)]
pub struct WebSocketConnectionBook {
    connection: Arc<DashMap<Uuid, WebSocketConn>>,
}

impl WebSocketConnectionBook {
    pub fn add_conn(&self, value: WebSocketConn) {
        self.connection.insert(value.id, value);
    }
    pub fn remove_conn(&self, id: Uuid) {
        self.connection.remove(&id);
    }
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
    event_loop_task: TaskLocals,
    app_states: SlimeState,
    async_pipeline: Arc<Py<PyAny>>,
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
        app_states: Py<PyDict>,
        async_pipeline: Py<PyAny>,
    ) -> SlimeServer {
        let env = SlimeServer::get_template_environment(&filename);
        let local_event_loop = Python::attach(|py| {
            let asyncio_mod = py.import("asyncio").expect("Need asyncio lib ");
            #[cfg(target_os = "linux")]
            {
                let uv_loop_mod = py.import("uvloop").expect("Need uvloop lib");

                let policy = uv_loop_mod
                    .getattr("EventLoopPolicy")
                    .expect("Cant able to fetch policy")
                    .call0()
                    .expect("Cant able to fetch policy");

                asyncio_mod
                    .call_method1("set_event_loop_policy", (policy,))
                    .expect("failed to set async loop");

                let python_event_loop = asyncio_mod
                    .call_method0("new_event_loop")
                    .expect("Failed to create new event loop");

                asyncio_mod
                    .call_method1("set_event_loop", (&python_event_loop,))
                    .expect("Failed to set event loop");
            }
            let python_event_loop = match asyncio_mod.call_method0("get_running_loop") {
                Ok(event_loop) => event_loop,
                Err(_) => {
                    let new_event = asyncio_mod
                        .call_method0("new_event_loop")
                        .expect("Cant able to create event loop");
                    asyncio_mod
                        .call_method1("set_event_loop", (new_event.clone(),))
                        .expect("Cant able to init the event loop");
                    new_event
                }
            };
            let local_event: TaskLocals = TaskLocals::new(python_event_loop.clone());
            let unbind_event_loop = python_event_loop.unbind();
            std::thread::spawn(move || {
                Python::attach(|py| {
                    unbind_event_loop.call_method0(py, "run_forever").unwrap();
                });
            });
            return local_event;
        });
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
            event_loop_task: local_event_loop,
            app_states: SlimeState::new(app_states),
            async_pipeline: Arc::new(async_pipeline),
        }
    }
    // pub async fn new_worker(&mut self) {
    //     while let Some(old) = self.pool_channel.1.recv().await {
    //         let (tx, rx) = mpsc::channel::<PyRequestWorker>(1024 * 1024 * 10);
    //         let mut new_worker_tx = (*self.worker_txs).clone();
    //         new_worker_tx[old] = tx;
    //         let runtime_handler_clone = self.tokio_handler.clone();
    //         self.pool
    //             .spawn(move || handle_python_call(rx, runtime_handler_clone));
    //     }
    // }
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
            let ws: bool = key.getattr("ws")?.extract()?;
            let compression: u8 = key.getattr("compression")?.extract()?;
            let body_size: usize = key.getattr("body_size")?.extract()?;
            let handler = value.cast::<PyDict>()?;
            let mut handlers: Vec<Py<PyAny>> = Vec::with_capacity(3);
            let mut is_async = false;
            if let Ok(Some(before_handler)) = handler.get_item("before") {
                if !before_handler.is_none() {
                    let handler_object_collection: &Bound<PyList> =
                        before_handler.cast::<PyList>()?;
                    for before_middle in handler_object_collection {
                        let handler_object = before_middle.cast::<PyTuple>()?;
                        is_async = handler_object
                            .get_item(1)
                            .unwrap()
                            .extract::<bool>()
                            .unwrap_or(false);

                        handlers.push(handler_object.get_item(0).unwrap().unbind());
                    }
                }
            }
            if let Ok(Some(request_handler)) = handler.get_item("handler") {
                if !request_handler.is_none() {
                    let handler_object_collection = request_handler.cast::<PyList>()?;

                    let handler_object_item = handler_object_collection.get_item(0)?;
                    let handler_object = handler_object_item.cast::<PyTuple>()?;
                    is_async = handler_object
                        .get_item(1)
                        .unwrap()
                        .extract::<bool>()
                        .unwrap_or(false);
                    handlers.push(handler_object.get_item(0).unwrap().unbind());
                }
            }
            if let Ok(Some(after_handler)) = handler.get_item("after") {
                if !after_handler.is_none() {
                    let handler_object_collection: &Bound<PyList> =
                        after_handler.cast::<PyList>()?;

                    for after_middle in handler_object_collection {
                        let handler_object = after_middle.cast::<PyTuple>()?;
                        is_async = handler_object
                            .get_item(1)
                            .unwrap()
                            .extract::<bool>()
                            .unwrap_or(false);
                        handlers.push(handler_object.get_item(0).unwrap().unbind());
                    }
                }
            }

            routes_collection.push(Route::new(
                path,
                method,
                stream,
                ws,
                compression,
                body_size,
                handlers,
                is_async,
            ));
        }
        self.routes = routes_collection;
        Ok(())
    }

    fn set_server_routes(&self) -> Router<WebSocketConnectionBook> {
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
            let is_async = route.is_async;
            let compression = route.compression;
            let body_size = route.body_size;
            let worker_txs = self.worker_txs.clone();
            let request_counter = self.request_counter.clone();
            let worker_count = worker_txs.len();
            let secret_key = self.secret_key.clone();
            let mut template_engine = self.template.clone();
            let is_dev = self.is_dev;
            let filename = self.filename.to_owned();
            let tokio_runtime = self.tokio_handler.clone();
            let event_loop_task_local = self.event_loop_task.clone();
            let slime_app_state = self.app_states.clone();
            let request_type = if route.ws {
                "ws"
            } else if stream_content.is_some() {
                "stream"
            } else {
                "http"
            };
            let async_pipeline = self.async_pipeline.clone();
            let method_copy = method.to_owned();
            let path_copy = path.to_owned();
            let process_request = move |ConnectInfo(client): ConnectInfo<SocketAddr>,
                                        Path(params): Path<HashMap<String, String>>,
                                        State(app_state): State<WebSocketConnectionBook>,
                                        request: Request<Body>| {
                if is_dev {
                    println!(
                        "INFO: {} => {} {}",
                        &method,
                        &path,
                        &client.ip().to_string()
                    );
                    template_engine = Arc::new(SlimeServer::get_template_environment(&filename));
                }
                async move {
                    if request.method().as_str() != method {
                        return StatusCode::METHOD_NOT_ALLOWED.into_response();
                    }
                    let idx = request_counter.fetch_add(1, Ordering::Relaxed);
                    let worker_tx = &worker_txs[idx % worker_count];

                    let (resp_tx, resp_rx) = oneshot::channel();
                    let (parts, raw_body) = request.into_parts();
                    let mut parts_clone = None;
                    if request_type == "ws" {
                        parts_clone = Some(parts.clone());
                    }
                    let content_type = &parts
                        .headers
                        .get("content-type")
                        .and_then(|value| value.to_str().ok())
                        .unwrap_or("");

                    let body = match to_bytes(raw_body, body_size).await {
                        Ok(bod) => bod,
                        Err(_) => {
                            return (
                                StatusCode::BAD_REQUEST,
                                "The maximum request body size has been exceeded",
                            )
                                .into_response();
                        }
                    };
                    let mut json_body: Option<serde_json::Value> = None;
                    let mut form_body: Option<HashMap<String, String>> = None;
                    let mut file_body: Option<Vec<SlimeFile>> = None;
                    if content_type != &"" {
                        if content_type.starts_with("application/json") {
                            json_body = serde_json::from_slice::<serde_json::Value>(&body).ok();
                        } else if content_type.starts_with("application/x-www-form-urlencoded") {
                            form_body =
                                serde_urlencoded::from_bytes::<HashMap<String, String>>(&body).ok();
                        } else if content_type.starts_with("multipart/form-data") {
                            if let Ok(boundary) = multer::parse_boundary(content_type) {
                                let body_clone = body.clone();
                                let stream = futures_util::stream::once(async move {
                                    Ok::<_, std::io::Error>(body_clone)
                                });

                                let mut multipart = multer::Multipart::new(stream, boundary);

                                let mut text_fields = HashMap::new();
                                let mut files = Vec::with_capacity(2);
                                while let Some(mut field) =
                                    multipart.next_field().await.unwrap_or(None)
                                {
                                    let name = field.name().map(|s| s.to_string());

                                    if field.file_name().is_none() {
                                        if let (Some(name), Ok(text)) = (name, field.text().await) {
                                            text_fields.insert(name, text);
                                        }
                                    } else {
                                        // file uploads

                                        let content_type =
                                            field.content_type().map(|value| value.to_string());
                                        let filename =
                                            format!("slime_file_{}", uuid::Uuid::new_v4());
                                        let temp_path = std::env::temp_dir().join(&filename);
                                        if let Ok(mut file) =
                                            tokio::fs::File::create(&temp_path).await
                                        {
                                            let mut size = 0;
                                            while let Ok(Some(chunk)) = field.chunk().await {
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
                                            let file_content_type =
                                                content_type.unwrap_or("UNKNOWN".to_string());
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
                        states: slime_app_state,
                    };

                    // send request to python workers

                    match request_type {
                        "ws" => {
                            if let Ok(ws) = WebSocketUpgrade::from_request(
                                Request::from_parts(parts_clone.unwrap(), Body::empty()),
                                &app_state,
                            )
                            .await
                            {
                                return websocket_handler(
                                    ws,
                                    app_state,
                                    tokio_runtime.clone(),
                                    worker_tx.clone(),
                                    slime_request,
                                    handler,
                                    event_loop_task_local,
                                    async_pipeline,
                                )
                                .await
                                .into_response();
                            }
                        }
                        "stream" => {
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
                            if is_async {
                                tokio_runtime.spawn(handle_async_python_call(
                                    PyRequestWorker::Stream(PyRequestStream {
                                        handler,
                                        request: slime_request,
                                        response: new_slime_stream_resonse,
                                    }),
                                    event_loop_task_local,
                                    async_pipeline,
                                ));
                            } else {
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
                            }
                            if let Some(headers) = started_rx.recv().await {
                                let mut new_response = Response::builder()
                                    .header("content-type", stream_content_type)
                                    .header("Server", SERVER);
                                for (key, value) in headers {
                                    new_response = new_response.header(key, value);
                                }
                                drop(started_rx);
                                let stream = ReceiverStream::new(stream_rx);
                                let body = Body::from_stream(stream);
                                return new_response.body(body).unwrap();
                            }
                        }
                        "http" => {
                            if is_async {
                                tokio_runtime.spawn(worker::handle_async_handler(
                                    PyRequestWorker::Http(PyRequest {
                                        handler,
                                        request: slime_request,
                                        response: resp_tx,
                                    }),
                                    event_loop_task_local,
                                    async_pipeline,
                                ));
                                // tokio_runtime.spawn(handle_async_python_call(
                                //     PyRequestWorker::Http(PyRequest {
                                //         handler,
                                //         request: slime_request,
                                //         response: resp_tx,
                                //     }),
                                //     event_loop_task_local,
                                //     async_pipeline,
                                // ));
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
                        }
                        _ => {
                            return (StatusCode::BAD_REQUEST, "Unknown request".to_string())
                                .into_response();
                        }
                    }

                    // to client side response
                    match resp_rx.await {
                        Ok(Ok(result)) => {
                            return result;
                        }
                        Ok(Err(err)) => {
                            (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response()
                        }
                        Err(_) => {
                            // println!("INFO: Creating new worker...");
                            // let _ = pool_channel.send(idx % worker_count).await;
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Worker cant able to handle the response".to_string(),
                            )
                                .into_response();
                        }
                    }
                }
            };
            let mut method_router = MethodRouter::new();
            method_router = match method_copy.as_str() {
                "GET" => method_router.get(process_request),
                "HEAD" => method_router.head(process_request),
                "POST" => method_router.post(process_request),
                "PUT" => method_router.put(process_request),
                "PATCH" => method_router.patch(process_request),
                "DELETE" => method_router.delete(process_request),
                "OPTIONS" => method_router.options(process_request),
                _ => method_router,
            };

            if compression != 0 {
                let mut new_compression = CompressionLayer::new();
                match compression {
                    1 => {
                        new_compression = new_compression.gzip(true);
                    }
                    2 => {
                        new_compression = new_compression.br(true);
                    }
                    3 => {
                        new_compression = new_compression.zstd(true);
                    }
                    _ => {}
                }
                new_compression = new_compression.quality(tower_http::CompressionLevel::Best);
                method_router = method_router.layer(new_compression);
            }

            server_router = server_router.route(&path_copy, method_router)
        }
        return server_router;
    }

    pub async fn server_run(self) -> PyResult<()> {
        let address: SocketAddr = format!("{}:{}", self.host, self.port).parse()?;
        let server_router = self.set_server_routes();
        let listener = TcpListener::bind(address).await?;

        println!("Slime server is running at http://{}", address);
        let _ = axum::serve(
            listener,
            server_router
                .with_state(WebSocketConnectionBook {
                    connection: Arc::new(DashMap::with_capacity(5)),
                })
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal())
        .await;
        Ok(())
    }
}

async fn shutdown_signal() {
    signal::ctrl_c().await.expect("Failed to listen for ctrl_c");
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    state: WebSocketConnectionBook,
    tok_hand: Handle,
    worker: mpsc::Sender<PyRequestWorker>,
    slime_request: SlimeRequest,
    handler: Arc<Vec<Py<PyAny>>>,
    local_event: TaskLocals,
    async_pipeline: Arc<Py<PyAny>>,
) -> impl IntoResponse {
    ws.on_upgrade(async move |socket| {
        {
            let id = Uuid::new_v4();
            let (ws_tx, mut ws_rx) = mpsc::channel::<Bytes>(1024);

            let web_conn = WebSocketConn { id, sender: ws_tx };
            state.add_conn(web_conn.clone());

            let (mut sender, mut receiver) = socket.split();

            // send message
            let send_message_handler = tok_hand.spawn(async move {
                while let Some(msg) = ws_rx.recv().await {
                    if let Err(err) = sender.send(Message::Binary(msg)).await {
                        println!("Websocket send ERROR: {}", err.to_string());
                        break;
                    }
                }
            });
            let (worker_tx, worker_resp) = oneshot::channel::<PyResult<SlimeWebSocketResponse>>();

            if let Err(err) = worker
                .send(PyRequestWorker::WebSocket(PyRequestWebSocket {
                    handler: handler,
                    request: slime_request,
                    response: worker_tx,
                    conn: web_conn,
                }))
                .await
            {
                println!(
                    "Worker down cant able to handle the request (reason) => {}",
                    err.to_string()
                );
            }

            if let Ok(Ok(resp)) = worker_resp.await {
                // recevie message
                tok_hand.spawn(async move {
                    let error_handler =
                        |err: PyErr, handler: Arc<Option<Py<PyAny>>>, py: &Python| -> bool {
                            if let Some(error_handler) = &(*handler) {
                                if error_handler.call1(*py, (err,)).is_err() {
                                    return false;
                                }
                                return true;
                            }
                            return true;
                        };
                    while let Some(Ok(msg)) = receiver.next().await {
                        Python::attach(|py| match msg {
                            Message::Binary(data) => {
                                if let Some(handler_func) = &(*resp.on_message_handler) {
                                    if let Err(err) =
                                        handler_func.call1(py, (PyBytes::new(py, &data),))
                                    {
                                        error_handler(err, resp.on_error_handler.clone(), &py);
                                    }
                                }
                            }
                            Message::Text(data) => {
                                if let Some(handler_func) = &(*resp.on_message_handler) {
                                    if let Err(err) = handler_func.call1(py, (data.as_str(),)) {
                                        error_handler(err, resp.on_error_handler.clone(), &py);
                                    }
                                }
                            }
                            Message::Close(_) => {
                                send_message_handler.abort();
                                if let Some(handler_func) = &(*resp.on_close_handler) {
                                    if let Err(err) = handler_func.call0(py) {
                                        error_handler(err, resp.on_error_handler.clone(), &py);
                                    }
                                }
                                state.remove_conn(id);
                            }
                            Message::Ping(data) => {
                                if let Some(handler_func) = &(*resp.on_message_handler) {
                                    if let Err(err) =
                                        handler_func.call1(py, (PyBytes::new(py, &data),))
                                    {
                                        if let Some(handler_func) = &(*resp.on_close_handler) {
                                            if let Err(err) = handler_func.call1(py, (err,)) {
                                                error_handler(
                                                    err,
                                                    resp.on_error_handler.clone(),
                                                    &py,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        });
                    }
                });
            }
        }
    })
}

pub fn spawn_python_workers(
    worker_count: usize,
    runtime_handler: Handle,
) -> Arc<Vec<mpsc::Sender<PyRequestWorker>>> {
    let mut worker_txs = Vec::with_capacity(worker_count);
    let pool = ThreadPoolBuilder::new()
        .num_threads(worker_count)
        .build()
        .unwrap();

    for _ in 0..worker_count {
        let runtime_handler_clone = runtime_handler.clone();
        let (tx, rx) = mpsc::channel::<PyRequestWorker>(1024 * 1024 * 10);
        worker_txs.push(tx);
        pool.spawn(move || handle_python_call(rx, runtime_handler_clone.clone()));
    }
    return Arc::new(worker_txs);
}

async fn handle_async_python_call(
    req_worker: PyRequestWorker,
    local_event: TaskLocals,
    async_pipeline: Arc<Py<PyAny>>,
) {
    match req_worker {
        PyRequestWorker::Http(req) => {
            let mut is_error: Option<PyErr> = None;
            let mut main_response_py: Option<Py<SlimeResponse>> = None;
            let coroutine = Python::attach(|py| {
                let request_py = Py::new(py, req.request).unwrap();
                let response_py = Py::new(py, SlimeResponse::new()).unwrap();
                match async_pipeline.call1(py, (&(*req.handler), &request_py, &response_py)) {
                    Ok(corout) => {
                        main_response_py = Some(response_py);
                        py_asyncio::into_future_with_locals(&local_event, corout.into_bound(py))
                    }
                    Err(err) => Err(err),
                }
            });
            match coroutine {
                Ok(co_fut) => {
                    if let Err(err) = co_fut.await {
                        is_error = Some(err);
                    }
                }
                Err(err) => {
                    is_error = Some(err);
                }
            }

            if is_error.is_some() {
                println!("{}", is_error.as_ref().unwrap());
                let _ = &req.response.send(Err(is_error.unwrap()));
            } else {
                let result =
                    Python::attach(|py| main_response_py.unwrap().borrow(py)._into_response());
                let _ = req.response.send(Ok(result));
            }

            // match Python::attach(|py| {
            //     return (
            //         Py::new(py, SlimeResponse::new(py)),
            //         Py::new(py, req.request),
            //     );
            // }) {
            //     (Ok(response_py), Ok(request_py)) => {
            //         let mut is_error: Option<PyErr> = None;

            //         let coroutine = Python::attach(|py| {
            //             let handler_collections: Vec<Py<PyAny>> = req
            //                 .handler
            //                 .iter()
            //                 .map(|hand| hand.0.clone_ref(py))
            //                 .collect();
            //             match async_pipeline
            //                 .call1(py, (handler_collections, &request_py, &response_py))
            //             {
            //                 Ok(corout) => py_asyncio::into_future_with_locals(
            //                     &local_event,
            //                     corout.into_bound(py),
            //                 ),
            //                 Err(err) => Err(err),
            //             }
            //         });
            //         match coroutine {
            //             Ok(co_fut) => {
            //                 if let Err(err) = co_fut.await {
            //                     is_error = Some(err);
            //                 }
            //             }
            //             Err(err) => {
            //                 is_error = Some(err);
            //             }
            //         }

            //         if is_error.is_some() {
            //             println!("{}", is_error.as_ref().unwrap());
            //             let _ = &req.response.send(Err(is_error.unwrap()));
            //         } else {
            //             let result = Python::attach(|py| response_py.borrow(py).clone_obj(py));
            //             let _ = req.response.send(Ok(result));
            //         }
            //     }
            //     _ => {
            //         let _ = req
            //             .response
            //             .send(Err(pyo3::exceptions::PyException::new_err(
            //                 "Cant able to create request and response handler".to_string(),
            //             )));
            //     }
            // }
        }
        PyRequestWorker::Stream(req) => {
            match Python::attach(|py| (Py::new(py, req.request), Py::new(py, req.response))) {
                (Ok(request_py), Ok(response_py)) => {
                    let mut is_error: Option<PyErr> = None;
                    let co_collections = Python::attach(|py| {
                        let mut co_collections = Vec::with_capacity(req.handler.len());
                        for handler_method in 0..req.handler.len() {
                            co_collections.push(
                                req.handler[handler_method].call1(py, (&request_py, &response_py)),
                            );
                        }
                        return co_collections;
                    });

                    for co in co_collections {
                        match co {
                            Ok(co_handler) => {
                                let future = Python::attach(|py| {
                                    py_asyncio::into_future_with_locals(
                                        &local_event,
                                        co_handler.into_bound(py),
                                    )
                                    .unwrap()
                                });
                                if let Err(err) = future.await {
                                    is_error = Some(err);
                                    break;
                                }
                            }
                            Err(err) => {
                                is_error = Some(err);
                                break;
                            }
                        }
                        let _ = tokio::task::yield_now();
                    }
                    if is_error.is_some() {
                        println!("ERROR: {}", is_error.unwrap());
                    }
                }
                _ => {
                    println!("ERROR: Cant able to create reqeust and response handler");
                }
            }
        }
        PyRequestWorker::WebSocket(req) => {
            match Python::attach(|py| {
                (
                    Py::new(py, req.request),
                    Py::new(
                        py,
                        SlimeWebSocketResponse {
                            conn: req.conn,
                            on_message_handler: Arc::new(None),
                            on_close_handler: Arc::new(None),
                            on_error_handler: Arc::new(None),
                            on_ping_handler: Arc::new(None),
                        },
                    ),
                )
            }) {
                (Ok(request_py), Ok(response_py)) => {
                    let mut is_error: Option<PyErr> = None;
                    let co_collections = Python::attach(|py| {
                        let mut co_collections = Vec::with_capacity(req.handler.len());
                        for handler_method in 0..req.handler.len() {
                            co_collections.push(
                                req.handler[handler_method].call1(py, (&request_py, &response_py)),
                            );
                        }
                        return co_collections;
                    });
                    for co in co_collections {
                        match co {
                            Ok(co_handler) => {
                                let future = Python::attach(|py| {
                                    py_asyncio::into_future_with_locals(
                                        &local_event,
                                        co_handler.into_bound(py),
                                    )
                                    .unwrap()
                                });
                                if let Err(err) = future.await {
                                    is_error = Some(err);
                                    break;
                                }
                            }
                            Err(err) => {
                                is_error = Some(err);
                            }
                        }
                    }
                    if is_error.is_some() {
                        println!("ERROR: {}", is_error.unwrap());
                    }
                    let result = Python::attach(|py| response_py.borrow(py).clone());
                    let _ = req.response.send(Ok(result));
                }
                _ => {
                    println!("ERROR: Cant able to create reqeust and response handler");
                }
            }
        }
    }
}

#[inline]
fn handle_python_call(mut rx: mpsc::Receiver<PyRequestWorker>, _runtime_handler: Handle) {
    Python::attach(|py| {
        while let Some(req_worker) = py.detach(|| rx.blocking_recv()) {
            match req_worker {
                PyRequestWorker::Http(req) => {
                    match (Py::new(py, SlimeResponse::new()), Py::new(py, req.request)) {
                        (Ok(response_py), Ok(request_py)) => {
                            let mut is_error: Option<PyErr> = None;
                            for handler_method in 0..req.handler.len() {
                                if let Err(err) = req.handler[handler_method]
                                    .call1(py, (&request_py, &response_py))
                                {
                                    let path = request_py.getattr(py, "path").unwrap().to_string();
                                    let method =
                                        request_py.getattr(py, "method").unwrap().to_string();
                                    println!(
                                        "ERROR @ path: [{}] for method [{}]: {}",
                                        path, method, err
                                    );
                                    is_error = Some(err);
                                    break;
                                }
                                yield_now();
                            }
                            if is_error.is_some() {
                                let _ = &req.response.send(Err(is_error.unwrap()));
                            } else {
                                let result = response_py.borrow(py)._into_response();
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
                }
                PyRequestWorker::Stream(req) => {
                    match (Py::new(py, req.request), Py::new(py, req.response)) {
                        (Ok(request_py), Ok(response_py)) => {
                            for handler_method in 0..req.handler.len() {
                                if let Err(err) = req.handler[handler_method]
                                    .call1(py, (&request_py, &response_py))
                                {
                                    println!("ERROR: {}", err);
                                    break;
                                }
                                yield_now();
                            }
                        }
                        _ => {
                            println!("ERROR: Cant able to create reqeust and response handler");
                        }
                    }
                }
                PyRequestWorker::WebSocket(req) => {
                    let response_obj = Py::new(
                        py,
                        SlimeWebSocketResponse {
                            conn: req.conn,
                            on_message_handler: Arc::new(None),
                            on_close_handler: Arc::new(None),
                            on_error_handler: Arc::new(None),
                            on_ping_handler: Arc::new(None),
                        },
                    );
                    match (Py::new(py, req.request), response_obj) {
                        (Ok(request_py), Ok(response_py)) => {
                            for handler_method in 0..req.handler.len() {
                                if let Err(err) = req.handler[handler_method]
                                    .call1(py, (&request_py, &response_py))
                                {
                                    println!("ERROR: {}", err);
                                    break;
                                }
                                yield_now();
                            }
                            let result = response_py.borrow(py).clone();
                            let _ = req.response.send(Ok(result));
                        }
                        _ => {
                            println!("ERROR: Cant able to create reqeust and response handler");
                        }
                    }
                }
            }
        }
    });
}
