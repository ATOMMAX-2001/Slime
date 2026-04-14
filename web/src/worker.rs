use axum::{body::Body, response::Response};
use pyo3::prelude::*;
use pyo3_async_runtimes::{self as py_asyncio, TaskLocals};
use std::sync::Arc;

use crate::{response::SlimeResponse, server::PyRequestWorker};

pub async fn handle_async_handler(
    request_worker: PyRequestWorker,
    local_events: TaskLocals,
    async_pipeline: Arc<Py<PyAny>>,
) {
    match request_worker {
        PyRequestWorker::Http(req) => {
            let mut response_py = None;
            let coroutine = Python::attach(|py| {
                let request_py = Py::new(py, req.request).unwrap();
                let response_py_obj = Py::new(py, SlimeResponse::new()).unwrap();
                return match async_pipeline
                    .call1(py, (&(*req.handler), request_py, &response_py_obj))
                {
                    Ok(co) => {
                        response_py = Some(response_py_obj);
                        py_asyncio::into_future_with_locals(&local_events, co.into_bound(py))
                    }
                    Err(err) => Err(err),
                };
            });
            match coroutine {
                Ok(co_fut) => {
                    if let Err(err) = co_fut.await {
                        let _ = req.response.send(Err(err));
                    } else {
                        let _ = req.response.send(Ok(Response::new(Body::empty())));
                    }
                }
                Err(err) => {
                    println!("Error: {}", err);
                    let _ = req.response.send(Err(err));
                }
            }
        }
        _ => {}
    }
}
