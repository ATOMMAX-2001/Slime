use axum::http;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use bytes::Bytes;
use hmac::{Hmac, Mac};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::Arc;

#[pyclass]
pub struct SlimeRequest {
    pub uri: http::Uri,
    pub method: http::Method,
    pub header: Arc<http::HeaderMap>,
    pub body: Bytes,
}

impl SlimeRequest {
    fn sign_cookie_values(&self, secret: &[u8], value: &str) -> String {
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::from(secret).unwrap();
        mac.update(value.as_bytes());
        let signature = mac.finalize().into_bytes();
        return URL_SAFE_NO_PAD.encode(signature);
    }
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

    fn get_cookies(&self) -> PyResult<HashMap<String, String>> {
        let mut cookies = HashMap::new();
        if let Some(cookie_headers) = self.header.get("cookie") {
            if let Ok(cookie_values) = cookie_headers.to_str() {
                for cookie_pairs in cookie_values.split("; ") {
                    let mut parts = cookie_pairs.splitn(2, "=");
                    if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                        cookies.insert(key.to_string(), value.to_string());
                    }
                }
            }
        }

        return Ok(cookies);
    }

    fn __repr__(&self) -> PyResult<String> {
        return Ok(format!(
            "SlimeRequest <path: {} method: {}>",
            self.path(),
            self.method()
        ));
    }
}
