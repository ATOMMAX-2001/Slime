use axum::{
    body::Body,
    http::{
        HeaderValue, StatusCode,
        header::{CONTENT_TYPE, SERVER, SET_COOKIE},
    },
    response::Response,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pythonize::depythonize;
use sha2::Sha256;

#[pyclass]
pub struct SlimeResponse {
    pub status: u16,
    pub headers: Py<PyDict>,
    pub header_size: usize,
    pub cookies: Vec<String>,
    pub content_type: String,
    pub body: Option<String>,
}

impl SlimeResponse {
    fn sign_cookie_values(&self, secret: &[u8], value: &String) -> String {
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(secret).unwrap();
        mac.update(value.as_bytes());
        let signature = mac.finalize().into_bytes();
        return URL_SAFE_NO_PAD.encode(signature);
    }

    pub fn _into_response(&self) -> Response<Body> {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::OK);
        let mut result = Response::builder()
            .status(status)
            .header(CONTENT_TYPE, &self.content_type)
            .header(SERVER, "SlimeV0.1");
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

        let mut final_response = if let Some(body_data) = self.body.to_owned() {
            result.body(Body::from(body_data)).unwrap()
        } else {
            result.body(Body::from("")).unwrap()
        };

        for cookie in &self.cookies {
            if let Ok(header_value) = HeaderValue::from_str(cookie) {
                final_response
                    .headers_mut()
                    .append(SET_COOKIE, header_value);
            }
        }
        return final_response;
    }

    pub fn clone_obj(&self, py: Python) -> SlimeResponse {
        SlimeResponse {
            status: self.status,
            headers: self.headers.clone_ref(py),
            header_size: self.header_size,
            cookies: self.cookies.to_owned(),
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
            cookies: Vec::new(),
            content_type: "text/plain".to_string(),
            body: None,
        }
    }

    fn set_cookie(&mut self, key: String, value: String, path: String) -> PyResult<()> {
        let cookie = format!("{}={};Path={}; HttpOnly", key, value, path);
        self.cookies.push(cookie);
        return Ok(());
    }

    fn set_sign_cookie(
        &mut self,
        key: String,
        value: String,
        path: String,
        secret: String,
    ) -> PyResult<()> {
        let cookie = format!(
            "{}={}.{};Path={}; HttpOnly",
            key,
            &value,
            self.sign_cookie_values(secret.as_bytes(), &value),
            path
        );
        self.cookies.push(cookie);
        return Ok(());
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

    fn html(&mut self, body: String) -> PyResult<()> {
        self.body = Some(body);
        self.content_type = "text/html; charset=utf-8".to_string();
        return Ok(());
    }
}
