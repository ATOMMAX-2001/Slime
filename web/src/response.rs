use pyo3::prelude::*;
use pyo3::types::PyDict;

use axum::{
    body::Body,
    http::{HeaderValue, StatusCode},
    response::Response,
};
use pythonize::depythonize;

#[pyclass]
pub struct SlimeResponse {
    pub status: u16,
    pub headers: Py<PyDict>,
    pub header_size: usize,
    pub content_type: String,
    pub body: Option<String>,
}

impl SlimeResponse {
    pub fn _into_response(&self) -> Response<Body> {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::OK);
        let mut result = Response::builder().status(status);
        if self.header_size != 0 {
            result = Python::attach(|py| {
                if let Ok(headers_result) = self.headers.bind(py).cast::<PyDict>() {
                    for (k, v) in headers_result {
                        if let (Ok(key), Ok(value)) = (k.extract::<&str>(), v.extract::<&str>()) {
                            if let Ok(header_value) = HeaderValue::from_str(value) {
                                result = result.header(key, header_value);
                            }
                        }
                    }
                }
                return result;
            });
        }

        if let Some(body_data) = self.body.to_owned() {
            return result.body(Body::from(body_data)).unwrap();
        }
        return result.body(Body::from("")).unwrap();
    }

    pub fn clone_obj(&self, py: Python) -> SlimeResponse {
        SlimeResponse {
            status: self.status,
            headers: self.headers.clone_ref(py),
            header_size: self.header_size,
            content_type: self.content_type.to_owned(),
            body: self.body.clone(),
        }
    }
}

#[pymethods]
impl SlimeResponse {
    #[new]
    pub fn new(py: Python) -> SlimeResponse {
        SlimeResponse {
            status: 200,
            headers: PyDict::new(py).unbind(),
            header_size: 0,
            content_type: "text/plain".to_string(),
            body: None,
        }
    }

    fn set_header(&mut self, py: Python, key: String, value: String) -> PyResult<()> {
        let headers = self.headers.bind(py);
        headers.set_item(key, value)?;
        self.header_size += 1;
        return Ok(());
    }

    fn plain(&mut self, resp_obj: String) -> PyResult<()> {
        self.body = Some(resp_obj);
        return Ok(());
    }

    fn set_status(&mut self, status: u16) -> PyResult<()> {
        self.status = status;
        return Ok(());
    }

    fn json(&mut self, resp_obj: Py<PyAny>, py: Python) -> PyResult<()> {
        let value: serde_json::Value = depythonize(resp_obj.bind(py))?;
        let json_str = serde_json::to_string(&value).map_err(|err| {
            return PyErr::new::<pyo3::exceptions::PyException, _>(format!(
                "Json serialization error: {}",
                err
            ));
        })?;
        self.body = Some(json_str);
        self.content_type = "application/json".to_string();
        return Ok(());
    }
}
