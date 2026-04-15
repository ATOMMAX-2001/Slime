// # AUTHOR: S.ABILASH
// # Email: abinix01@gmail.com

use axum::http;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use bytes::Bytes;
use hmac::{Hmac, Mac};
use minijinja::Environment;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use pythonize::{depythonize, pythonize};
use sha2::Sha256;
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;
use subtle::ConstantTimeEq;

#[derive(Clone)]
#[pyclass]
pub struct SlimeFile {
    pub filename: String,
    pub content_type: String,
    pub extension: String,
    pub temp_path: PathBuf,
    pub size: usize,
}
#[pymethods]
impl SlimeFile {
    /// get filename
    #[getter]
    fn filename(&self) -> PyResult<String> {
        return Ok(self.filename.to_owned());
    }

    /// get content_type
    #[getter]
    fn content_type(&self) -> PyResult<String> {
        return Ok(self.content_type.to_owned());
    }

    /// get file_path
    #[getter]
    fn file_path(&self) -> PyResult<String> {
        return Ok(self.temp_path.to_str().unwrap_or("").to_string());
    }

    ///get extension
    #[getter]
    fn extension(&self) -> PyResult<String> {
        return Ok(self.extension.to_owned());
    }

    #[getter]
    fn file_size(&self) -> PyResult<usize> {
        return Ok(self.size);
    }

    fn save(&self, new_filename: String) -> PyResult<()> {
        match std::fs::rename(&self.temp_path, &new_filename) {
            Ok(_) => {}
            Err(_) => {
                std::fs::copy(&self.temp_path, &new_filename)?;
            }
        }
        return Ok(());
    }

    fn clean(&self) -> PyResult<()> {
        std::fs::remove_file(&self.temp_path)?;
        return Ok(());
    }
}

pub struct SlimeState {
    pub app_state: Py<PyDict>,
}

impl SlimeState {
    pub fn new(app_state: Py<PyDict>) -> SlimeState {
        SlimeState {
            app_state: app_state,
        }
    }
}

impl Clone for SlimeState {
    fn clone(&self) -> Self {
        SlimeState {
            app_state: Python::attach(|py| self.app_state.clone_ref(py)),
        }
    }
}

#[pyclass]
pub struct SlimeRequest {
    pub uri: http::Uri,
    pub client: IpAddr,
    pub method: http::Method,
    pub header: Arc<http::HeaderMap>,
    pub body: Bytes,
    pub query: HashMap<String, String>,
    pub params: HashMap<String, String>,
    pub json_body: Option<serde_json::Value>,
    pub form: Option<HashMap<String, String>>,
    pub files: Option<Vec<SlimeFile>>,
    pub secret: Arc<Vec<u8>>,
    pub template: Arc<Environment<'static>>,
    pub states: SlimeState,
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
    fn client(&self) -> String {
        return self.client.to_string();
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
    fn bytes(&self) -> PyResult<Vec<u8>> {
        return Ok(self.body.to_vec());
    }

    #[getter]
    fn form(&self) -> PyResult<HashMap<String, String>> {
        return Ok(self.form.to_owned().unwrap_or_default());
    }

    #[getter]
    fn file(&self) -> PyResult<Vec<SlimeFile>> {
        return Ok(self.files.to_owned().unwrap_or_default());
    }

    #[getter]
    fn no_of_files_available(&self) -> PyResult<usize> {
        if let Some(data) = &self.files {
            return Ok(data.len());
        } else {
            return Ok(0);
        }
    }

    #[getter]
    fn json(&self, py: Python) -> PyResult<Py<PyAny>> {
        match &self.json_body {
            Some(value) => return Ok(pythonize(py, value)?.unbind()),
            None => return Ok(py.None()),
        }
    }

    #[getter]
    fn query(&self) -> PyResult<HashMap<String, String>> {
        return Ok(self.query.to_owned());
    }

    #[getter]
    fn params(&self) -> PyResult<HashMap<String, String>> {
        return Ok(self.params.to_owned());
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

    fn get_state(&self, py: Python, key: &str) -> PyResult<Option<Py<PyAny>>> {
        let bind_dict_state = self.states.app_state.bind(py);
        if let Ok(Some(value)) = bind_dict_state.get_item(key) {
            return Ok(Some(value.unbind()));
        } else {
            return Ok(None);
        }
    }

    fn update_state(&mut self, py: Python, key: &str, value: Py<PyAny>) -> PyResult<()> {
        let bind_dict_state = self.states.app_state.bind(py);
        bind_dict_state.set_item(key, value)?;
        return Ok(());
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
