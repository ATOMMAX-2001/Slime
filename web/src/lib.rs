use pyo3::prelude::*;

#[pymodule]
mod web {
    use axum::{Router, routing::get};
    use pyo3::{prelude::*, types::PyDict};
    use std::net::SocketAddr;
    use std::sync::Arc;
    use tokio::net::TcpListener;
    use tokio::runtime::Builder;

    struct Route {
        path: String,
        method: String,
        handler: Arc<Py<PyAny>>,
    }
    impl Clone for Route {
        fn clone(&self) -> Self {
            Route {
                path: self.path.clone(),
                method: self.method.clone(),
                handler: self.handler.clone(),
            }
        }
    }
    impl Route {
        pub fn new(path: String, method: String, handler: Py<PyAny>) -> Route {
            Route {
                path: path,
                method: method,
                handler: Arc::new(handler),
            }
        }
    }

    struct SlimeServer {
        routes: Vec<Route>,
        host: String,
        port: usize,
    }

    impl SlimeServer {
        pub fn new(host: String, port: usize) -> SlimeServer {
            SlimeServer {
                routes: Vec::with_capacity(5),
                host,
                port,
            }
        }
        pub fn load_routes(&mut self, routes: &Bound<PyDict>) -> PyResult<()> {
            let mut routes_collection: Vec<Route> = Vec::with_capacity(5);
            for (key, value) in routes {
                let path: String = key.getattr("path")?.extract()?;
                let method: String = key.getattr("method")?.extract()?;
                let handler = value.unbind();
                routes_collection.push(Route::new(path, method, handler));
            }
            self.routes = routes_collection;
            return Ok(());
        }

        fn set_server_routes(&self) -> Router {
            let mut server_router = Router::new();
            for route in &self.routes {
                let route = route.clone();
                let path = route.path;
                let method = route.method;
                let handler = route.handler;

                if method == "GET" {
                    server_router = server_router.route(
                        &path,
                        get(move || async move {
                            return tokio::task::spawn_blocking(move || {
                                return Python::attach(|py| {
                                    let bound = handler.clone_ref(py);
                                    let result = bound.call0(py)?;
                                    return result.extract::<String>(py);
                                });
                            })
                            .await
                            .unwrap()
                            .unwrap();
                        }),
                    );
                }
            }
            return server_router;
        }
        pub async fn server_run(self) -> PyResult<()> {
            let address: SocketAddr = format!("{}:{}", self.host, self.port).parse()?;
            let server_router = self.set_server_routes();
            let listener = TcpListener::bind(address).await.unwrap();
            #[cfg(target_family = "unix")]
            listener.set_reuseport(true);

            println!("Slime server is running at {}", address);
            let _ = axum::serve(listener, server_router).await;
            return Ok(());
        }
    }

    #[pyfunction]
    pub fn init_web(py: Python, slime_obj: Py<PyAny>, host: String, port: usize) -> PyResult<()> {
        let slime_obj_bound = slime_obj.bind(py);
        let slime_routes = slime_obj_bound.call_method0("_get_routes")?;
        let routes = slime_routes.cast::<PyDict>()?;
        let mut server = SlimeServer::new(host, port);
        server.load_routes(routes)?;
        let runtime = Builder::new_multi_thread().enable_all().build()?;
        runtime.block_on(server.server_run())?;
        return Ok(());
    }
}
