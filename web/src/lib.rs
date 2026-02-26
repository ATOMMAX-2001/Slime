use pyo3::prelude::*;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use axum::{Router, routing::get, response::IntoResponse, http::StatusCode};
use pyo3::types::PyDict;
use tokio::sync::{mpsc, oneshot};
use tokio::runtime::Builder;
use tokio::signal;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use rayon::ThreadPoolBuilder;


#[pymodule]
mod web {


    use super::*;

    struct Route {
        path: String,
        method: String,
        handler: Arc<Py<PyAny>>,
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

    struct PyRequest {
        handler: Arc<Py<PyAny>>,
        response: oneshot::Sender<PyResult<String>>,
    }

    struct SlimeServer {
        routes: Vec<Route>,
        host: String,
        port: usize,
        worker_txs: Arc<Vec<mpsc::Sender<PyRequest>>>,
        request_counter: Arc<AtomicUsize>,
    }

    impl SlimeServer {
        pub fn new(host: String, port: usize, worker_txs: Arc<Vec<mpsc::Sender<PyRequest>>>) -> Self {
            Self {
                routes: Vec::new(),
                host,
                port,
                worker_txs,
                request_counter: Arc::new(AtomicUsize::new(0)),
            }
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
            for route in &self.routes {
                let route = route.clone();
                let path = route.path;
                let method = route.method;
                let handler = route.handler.clone();
                let worker_txs = self.worker_txs.clone();
                let request_counter = self.request_counter.clone();
                let worker_count = worker_txs.len();

                if method == "GET" {
                    server_router = server_router.route(
                        &path,
                        get(move || {
                            let handler = handler.clone();
                            let worker_txs = worker_txs.clone();
                            async move {
                                let idx = request_counter.fetch_add(1, Ordering::Relaxed);
                                let worker_tx = &worker_txs[idx % worker_count];

                                let (resp_tx, resp_rx) = oneshot::channel();

                                if worker_tx.send(PyRequest { handler, response: resp_tx }).await.is_err() {
                                    return (StatusCode::INTERNAL_SERVER_ERROR, "Worker down".to_string()).into_response();
                                }

                                match resp_rx.await {
                                    Ok(Ok(result)) => (StatusCode::OK, result).into_response(),
                                    Ok(Err(err)) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
                                    Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Worker dropped".to_string()).into_response(),
                                }
                            }
                        }),
                    );
                }
            }
            server_router
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

    fn spawn_python_workers(worker_count: usize) -> Arc<Vec<mpsc::Sender<PyRequest>>> {
        let mut worker_txs = Vec::with_capacity(worker_count);
        let pool = ThreadPoolBuilder::new().num_threads(worker_count).build().unwrap();
        for _ in 0..worker_count {
            let (tx, mut rx) = mpsc::channel::<PyRequest>(1024);
            worker_txs.push(tx.clone());

            pool.spawn(move || {
                Python::attach(|py| {
                    while let Some(req) = rx.blocking_recv() {
                        let result = req.handler.call0(py).and_then(|r| r.extract::<String>(py));
                        let _ = req.response.send(result);
                    }
                });
            });
        }
        return Arc::new(worker_txs);
    }

    #[pyfunction]
    pub fn init_web(py: Python, slime_obj: Py<PyAny>, host: String, port: usize) -> PyResult<()> {
        let slime_obj_bound = slime_obj.bind(py);
        let slime_routes = slime_obj_bound.call_method0("_get_routes")?;
        let routes = slime_routes.cast::<PyDict>()?;

        let worker_count = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
        let worker_txs = spawn_python_workers(worker_count);

        let mut server = SlimeServer::new(host, port, worker_txs);
        server.load_routes(routes)?;

        let runtime = Builder::new_multi_thread()
            .worker_threads(worker_count)
            .enable_all()
            .build()?;

        py.detach(|| runtime.block_on(server.server_run()))?;
        Ok(())
    }

}