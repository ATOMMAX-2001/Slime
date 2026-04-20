// # AUTHOR: S.ABILASH
// # Email: abinix01@gmail.com

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyModule;
use std::sync::Arc;
use tokio::runtime::{Builder, Handle};

mod request;
mod response;
mod server;

use pyo3_async_runtimes::{self as py_asyncio, TaskLocals};
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
    workers: usize,
    async_pipeline: Py<PyAny>,
    async_app_start: Py<PyAny>,
    static_path: String,
) -> PyResult<()> {
    println!("Initializing...");
    let slime_obj_bound = slime_obj.bind(py);
    let slime_routes = slime_obj_bound.call_method0("_get_routes")?;
    let slime_filename = slime_obj_bound.getattr("_Slime__filename")?.to_string();
    let routes = slime_routes.cast::<PyDict>()?;

    let no_of_cpu: usize = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    let worker_count = if workers != 0 {
        workers
    } else if is_dev {
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

    let local_event_loop_task = Python::attach(|py| {
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

    run_async_app_start(
        async_app_start,
        &local_event_loop_task,
        runtime_handler.clone(),
    );

    let worker_txs = server::spawn_python_workers(
        worker_count,
        runtime_handler.clone(),
        Arc::new(async_pipeline),
        local_event_loop_task,
    );

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

    py.detach(|| runtime.block_on(server.server_run(static_path)))?;
    Ok(())
}

fn run_async_app_start(
    async_app_start: Py<PyAny>,
    local_event_loop_task: &TaskLocals,
    runtime_handler: Handle,
) {
    Python::attach(|py| {
        if !async_app_start.is_none(py) {
            match async_app_start.call0(py) {
                Ok(co) => {
                    match py_asyncio::into_future_with_locals(
                        local_event_loop_task,
                        co.into_bound(py),
                    ) {
                        Ok(co_fut) => {
                            runtime_handler.block_on(async move {
                                if let Err(err) = co_fut.await {
                                    println!(
                                        "Error: Failed to run async app start handler (reason) -> {}",
                                        err
                                    );
                                    std::process::exit(1);
                                }
                            });
                        }
                        Err(err) => {
                            println!("Failed to start the application {}", err);
                            return;
                        }
                    }
                }
                Err(err) => {
                    println!("Failed to start the application {}", err);
                    return;
                }
            }
        }
    });
}

#[pymodule]
fn web(m: &Bound<PyModule>) -> PyResult<()> {
    m.add_class::<response::SlimeResponse>()?;
    m.add_class::<request::SlimeRequest>()?;
    m.add_function(wrap_pyfunction!(init_web, m)?)?;
    Ok(())
}
