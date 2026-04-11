use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyModule;

use tokio::runtime::Builder;

mod request;
mod response;
mod server;

use server::SlimeServer;
mod constant;
use mimalloc_rust::*;

#[global_allocator]
static GLOBAL_MIMALLOC: GlobalMiMalloc = GlobalMiMalloc;

#[pyfunction]
pub fn init_web(
    py: Python,
    slime_obj: Py<PyAny>,
    host: String,
    port: usize,
    secret_key: String,
    is_dev: bool,
    app_states: Py<PyDict>,
) -> PyResult<()> {
    println!("Initializing...");
    let slime_obj_bound = slime_obj.bind(py);
    let slime_routes = slime_obj_bound.call_method0("_get_routes")?;
    let slime_filename = slime_obj_bound.getattr("_Slime__filename")?.to_string();
    let routes = slime_routes.cast::<PyDict>()?;

    let no_of_cpu: usize = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    let worker_count = if is_dev {
        1
    } else {
        match std::env::var("SLIME_WORKER") {
            Ok(data) => data.parse::<usize>().unwrap_or(no_of_cpu),
            Err(_) => no_of_cpu,
        }
    };

    println!("No of workers created: {}", worker_count);

    let runtime = Builder::new_multi_thread()
        .worker_threads(worker_count)
        .enable_all()
        .build()?;

    let runtime_handler = runtime.handle().clone();
    let worker_txs = server::spawn_python_workers(worker_count, runtime_handler.clone());
    let mut server = SlimeServer::new(
        host,
        port,
        worker_txs,
        secret_key,
        slime_filename,
        is_dev,
        runtime_handler,
        app_states,
    );

    server.load_routes(routes)?;

    py.detach(|| runtime.block_on(server.server_run()))?;
    Ok(())
}

#[pymodule]
fn web(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<response::SlimeResponse>()?;
    m.add_class::<request::SlimeRequest>()?;
    m.add_function(wrap_pyfunction!(init_web, m)?)?;
    Ok(())
}

// #[pymodule]
// mod web {

//     use super::*;

//     ///
//     ///
//     /// Initilize the Slime web server
//     /// ```python
//     /// init_web(host: str="localhost",port: int=3000,secret_key:str,is_dev: bool)
//     /// ```

//     #[pyfunction]
//     pub fn init_web(
//         py: Python,
//         slime_obj: Py<PyAny>,
//         host: String,
//         port: usize,
//         secret_key: String,
//         is_dev: bool,
//         app_states: Py<PyDict>,
//     ) -> PyResult<()> {
//         println!("Initializing...");
//         let slime_obj_bound = slime_obj.bind(py);
//         let slime_routes = slime_obj_bound.call_method0("_get_routes")?;
//         let slime_filename = slime_obj_bound.getattr("_Slime__filename")?.to_string();
//         let routes = slime_routes.cast::<PyDict>()?;

//         let no_of_cpu: usize = std::thread::available_parallelism()
//             .map(|n| n.get())
//             .unwrap_or(1);

//         let worker_count = if is_dev {
//             1
//         } else {
//             match std::env::var("SLIME_WORKER") {
//                 Ok(data) => data.parse::<usize>().unwrap_or(no_of_cpu),
//                 Err(_) => no_of_cpu,
//             }
//         };

//         println!("No of workers created: {}", worker_count);

//         let runtime = Builder::new_multi_thread()
//             .worker_threads(worker_count)
//             .enable_all()
//             .build()?;

//         let runtime_handler = runtime.handle().clone();
//         let worker_txs = server::spawn_python_workers(worker_count, runtime_handler.clone());
//         let mut server = SlimeServer::new(
//             host,
//             port,
//             worker_txs,
//             secret_key,
//             slime_filename,
//             is_dev,
//             runtime_handler,
//             app_states,
//         );

//         server.load_routes(routes)?;

//         py.detach(|| runtime.block_on(server.server_run()))?;
//         Ok(())
//     }
// }
