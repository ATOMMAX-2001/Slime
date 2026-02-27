use axum::http;
use bytes::Bytes;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use std::sync::Arc;

#[pyclass]
pub struct SlimeRequest {
    pub uri: http::Uri,
    pub method: http::Method,
    pub header: Arc<http::HeaderMap>,
    pub body: Bytes,
}
#[pymethods]
impl SlimeRequest {
    #[getter]
    fn method(&self) -> String {
        return self.method.to_string();
    }
    #[getter]
    fn path(&self) -> String {
        return self.uri.to_string();
    }

    #[getter]
    fn header<'py>(&self, py: Python<'py>) -> PyResult<Py<PyDict>> {
        let req_header = PyDict::new(py);

        for (key, value) in self.header.iter() {
            let header_value = value.to_str().unwrap_or("");
            req_header.set_item(key.as_str(), header_value)?;
        }
        return Ok(req_header.unbind());
    }

    #[getter]
    fn body(&self, py: Python) -> PyResult<Py<PyBytes>> {
        return Ok(PyBytes::new(py, &self.body).unbind());
    }
    #[getter]
    fn text(&self) -> PyResult<String> {
        return Ok(String::from_utf8_lossy(&self.body).to_string());
    }

    fn __repr__(&self) -> PyResult<String> {
        return Ok(format!(
            "SlimeRequest <path: {} method: {}>",
            self.path(),
            self.method()
        ));
    }
}
