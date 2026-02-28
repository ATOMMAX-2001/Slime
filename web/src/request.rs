use axum::http;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use bytes::Bytes;
use hmac::{Hmac, Mac};

use minijinja::Environment;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use pythonize::depythonize;
use sha2::Sha256;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use subtle::ConstantTimeEq;

#[pyclass]
pub struct SlimeRequest {
    pub uri: http::Uri,
    pub method: http::Method,
    pub header: Arc<http::HeaderMap>,
    pub body: Bytes,
    pub secret: Arc<Vec<u8>>,
    pub template: Arc<Environment<'static>>,
}

impl SlimeRequest {
    fn verify_sign_cookie_value(&self, value: &str, signature: &str) -> bool {
        let Ok(signature_bytes) = URL_SAFE_NO_PAD.decode(signature) else {
            return false;
        };
        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(&self.secret).unwrap();
        mac.update(value.as_bytes());
        return mac
            .finalize()
            .into_bytes()
            .ct_eq(signature_bytes.as_slice())
            .into();
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

    #[getter]
    fn secret_key(&self) -> PyResult<String> {
        let secret = &**self.secret;
        return Ok(std::str::from_utf8(secret)?.to_string());
    }

    fn get_cookies(&self) -> PyResult<HashMap<String, String>> {
        let mut cookies = HashMap::new();
        if let Some(cookie_headers) = self.header.get("cookie") {
            if let Ok(cookie_values) = cookie_headers.to_str() {
                for cookie_pairs in cookie_values.split(";") {
                    let mut parts = cookie_pairs.trim().splitn(2, "=");
                    if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                        cookies.insert(key.to_string(), value.to_string());
                    }
                }
            }
        }

        return Ok(cookies);
    }

    fn get_signed_cookie(&self, key: &str) -> PyResult<Option<String>> {
        let cookies = self.get_cookies()?;
        let Some(cookie_result) = cookies.get(key) else {
            return Ok(None);
        };
        let mut parts = cookie_result.splitn(2, '.');
        let Some(value) = parts.next() else {
            return Ok(None);
        };

        let Some(sig) = parts.next() else {
            return Ok(None);
        };

        if self.verify_sign_cookie_value(value, sig) {
            Ok(Some(value.to_string()))
        } else {
            Ok(None)
        }
    }
    #[pyo3(signature = (template_name, **kwargs))]
    fn render(
        &mut self,
        py: Python,
        template_name: String,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<String> {
        let context_dict = match kwargs {
            Some(d) => d,
            None => &PyDict::new(py),
        };
        let context: serde_json::Value = depythonize(context_dict)
            .map_err(|err| pyo3::exceptions::PyValueError::new_err(err.to_string()))?;

        let template = self
            .template
            .get_template(&template_name)
            .map_err(|err| pyo3::exceptions::PyValueError::new_err(err.to_string()))?;
        let render_output = template
            .render(context)
            .map_err(|err| pyo3::exceptions::PyValueError::new_err(err.to_string()))?;

        return Ok(render_output);
    }

    fn __repr__(&self) -> PyResult<String> {
        return Ok(format!(
            "SlimeRequest <path: {} method: {}>",
            self.path(),
            self.method()
        ));
    }
}
