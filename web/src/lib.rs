use pyo3::prelude::*;
use pyo3::types::PyDict;

use tokio::runtime::Builder;

mod request;
mod response;
mod server;

use server::SlimeServer;
mod constant;

#[pymodule]
mod web {

    use super::*;

    #[pyfunction]
    pub fn init_web(
        py: Python,
        slime_obj: Py<PyAny>,
        host: String,
        port: usize,
        secret_key: String,
        is_dev: bool,
    ) -> PyResult<()> {
        let slime_obj_bound = slime_obj.bind(py);
        let slime_routes = slime_obj_bound.call_method0("_get_routes")?;
        let slime_filename = slime_obj_bound.getattr("_Slime__filename")?.to_string();
        let routes = slime_routes.cast::<PyDict>()?;

        let worker_count = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);

        let runtime = Builder::new_multi_thread()
            .worker_threads(worker_count)
            .enable_all()
            .build()?;

        let runtime_handler = runtime.handle().clone();
        let worker_txs = server::spawn_python_workers(worker_count);
        // let (async_tx, async_rx) = oneshot::channel::<P>();
        let mut server = SlimeServer::new(
            host,
            port,
            worker_txs,
            secret_key,
            slime_filename,
            is_dev,
            runtime_handler,
        );

        server.load_routes(routes)?;

        py.detach(|| runtime.block_on(server.server_run()))?;
        Ok(())
    }
}
