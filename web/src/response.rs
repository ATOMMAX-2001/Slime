use std::{collections::HashMap, io};

use axum::{
    body::Body,
    http::{
        HeaderValue, Response, StatusCode,
        header::{CONTENT_TYPE, SERVER, SET_COOKIE},
    },
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use bytes::Bytes;
use hmac::{Hmac, Mac};
use pyo3::prelude::*;

use pythonize::depythonize;
use sha2::Sha256;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::{constant::SERVER as CONST_SERVER, server::WebSocketConn};

#[pyclass]
pub struct SlimeStreamResponse {
    pub content_type: String,
    pub sender: Option<mpsc::Sender<Result<Bytes, io::Error>>>,
    pub tokio_handler: tokio::runtime::Handle,
    pub headers: HashMap<String, String>,
    pub is_started: bool,
    pub header_sender: mpsc::Sender<HashMap<String, String>>,
}

impl SlimeStreamResponse {
    pub fn new(
        content: String,
        tx: mpsc::Sender<Result<Bytes, io::Error>>,
        rt_handle: tokio::runtime::Handle,
        header_tx: mpsc::Sender<HashMap<String, String>>,
    ) -> SlimeStreamResponse {
        return SlimeStreamResponse {
            content_type: content,
            sender: Some(tx),
            headers: HashMap::with_capacity(2),
            is_started: false,
            tokio_handler: rt_handle,
            header_sender: header_tx,
        };
    }
}

#[pymethods]
impl SlimeStreamResponse {
    #[getter]
    fn content_type(&self) -> PyResult<String> {
        return Ok(self.content_type.to_owned());
    }

    #[getter]
    fn headers(&self) -> PyResult<HashMap<String, String>> {
        return Ok(self.headers.to_owned());
    }

    fn set_header(&mut self, key: String, value: String) -> PyResult<()> {
        if self.is_started {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "You can only set headers before the stream start",
            ));
        }
        self.headers.insert(key, value);
        return Ok(());
    }

    fn start_stream(&mut self) -> PyResult<()> {
        if self.is_started {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "You can start the stream only once",
            ));
        }
        self.is_started = true;
        let header_tx = self.header_sender.clone();
        let headers = self.headers.to_owned();
        self.tokio_handler.spawn(async move {
            let _ = header_tx.send(headers).await;
            header_tx.closed().await;
        });
        return Ok(());
    }
    #[pyo3(signature = (data, strict_order=true))]
    fn send(&self, py: Python, data: Py<PyAny>, strict_order: bool) -> PyResult<()> {
        if !self.is_started {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "You need to start the stream before streaming the data",
            ));
        }
        let value: serde_json::Value = depythonize(data.bind(py))?;
        let json_str = serde_json::to_string(&value).map_err(|err| {
            return PyErr::new::<pyo3::exceptions::PyException, _>(format!(
                "Json serialization error: {}",
                err
            ));
        })?;
        let tx = self.sender.clone();
        if tx.is_none() {
            return Err(PyErr::new::<pyo3::exceptions::PyException, _>(format!(
                "Failed to send stream: Channel is closed"
            )));
        }
        if strict_order {
            let result = tx
                .unwrap()
                .try_send(Ok(Bytes::copy_from_slice(json_str.as_bytes())));
            if let Err(err) = result {
                return Err(PyErr::new::<pyo3::exceptions::PyException, _>(format!(
                    "Failed to send stream: {}",
                    err.to_string()
                )));
            }
        } else {
            self.tokio_handler.spawn(async move {
                let result = tx
                    .unwrap()
                    .send(Ok(Bytes::copy_from_slice(json_str.as_bytes())))
                    .await;
                if let Err(err) = result {
                    println!("Stream Send Error: {}", err);
                }
            });
        }
        return Ok(());
    }
    fn close(&mut self) -> PyResult<()> {
        self.sender = None;
        return Ok(());
    }
}
#[derive(Debug)]
#[pyclass]
pub struct SlimeResponse {
    pub status: u16,
    pub is_stream: Option<mpsc::Receiver<Result<Bytes, io::Error>>>,
    pub headers: HashMap<String, HeaderValue>,
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
            .header(SERVER, CONST_SERVER);
        if self.header_size != 0 {
            for (key, value) in &self.headers {
                result = result.header(key, value);
            }
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

    pub fn clone_obj(&self) -> SlimeResponse {
        SlimeResponse {
            status: self.status,
            is_stream: None,
            headers: self.headers.clone(),
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
    pub fn new() -> SlimeResponse {
        SlimeResponse {
            status: 200,
            is_stream: None,
            headers: HashMap::with_capacity(3),
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

    fn set_header(&mut self, key: String, value: String) -> PyResult<()> {
        if let Ok(header_value) = HeaderValue::from_str(value.as_str()) {
            self.headers.insert(key, header_value);
        } else {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Cant able to convert proper header value",
            ));
        }

        self.header_size += 1;
        return Ok(());
    }

    fn set_status(&mut self, status: u16) -> PyResult<()> {
        self.status = status;
        return Ok(());
    }

    #[pyo3(signature = (resp_obj,status=200))]
    fn plain(&mut self, resp_obj: String, status: u16) -> PyResult<()> {
        self.status = status;
        self.body = Some(resp_obj);
        return Ok(());
    }

    #[pyo3(signature = (resp_obj,status=200))]
    fn json(&mut self, resp_obj: Py<PyAny>, status: u16, py: Python) -> PyResult<()> {
        let value: serde_json::Value = depythonize(resp_obj.bind(py))?;
        let json_str = serde_json::to_string(&value).map_err(|err| {
            return PyErr::new::<pyo3::exceptions::PyException, _>(format!(
                "Json serialization error: {}",
                err
            ));
        })?;
        self.status = status;
        self.body = Some(json_str);
        self.content_type = "application/json".to_string();
        return Ok(());
    }

    #[pyo3(signature = (resp_obj,status=200))]
    fn html(&mut self, resp_obj: String, status: u16) -> PyResult<()> {
        self.status = status;
        self.body = Some(resp_obj);
        self.content_type = "text/html; charset=utf-8".to_string();
        return Ok(());
    }
}

#[pyclass]
pub struct SlimeWebSocketResponse {
    pub conn: WebSocketConn,
    pub on_message_handler: Arc<Option<Py<PyAny>>>,
    pub on_close_handler: Arc<Option<Py<PyAny>>>,
    pub on_error_handler: Arc<Option<Py<PyAny>>>,
    pub on_ping_handler: Arc<Option<Py<PyAny>>>,
}

impl Clone for SlimeWebSocketResponse {
    fn clone(&self) -> Self {
        SlimeWebSocketResponse {
            conn: self.conn.clone(),
            on_message_handler: self.on_message_handler.clone(),
            on_close_handler: self.on_close_handler.clone(),
            on_error_handler: self.on_error_handler.clone(),
            on_ping_handler: self.on_ping_handler.clone(),
        }
    }
}
#[pymethods]
impl SlimeWebSocketResponse {
    #[getter]
    fn id(&self) -> PyResult<String> {
        return Ok(self.conn.id.to_string());
    }

    fn on_message(&mut self, handler: Py<PyAny>) -> PyResult<()> {
        self.on_message_handler = Arc::new(Some(handler));
        return Ok(());
    }
    fn on_close(&mut self, handler: Py<PyAny>) -> PyResult<()> {
        self.on_close_handler = Arc::new(Some(handler));
        return Ok(());
    }
    fn on_error(&mut self, handler: Py<PyAny>) -> PyResult<()> {
        self.on_error_handler = Arc::new(Some(handler));
        return Ok(());
    }

    fn send(&self, py: Python, message: Py<PyAny>) -> PyResult<()> {
        let value: serde_json::Value = depythonize(message.bind(py))?;
        let json_str = serde_json::to_string(&value).map_err(|err| {
            return PyErr::new::<pyo3::exceptions::PyException, _>(format!(
                "Json serialization error: {}",
                err
            ));
        })?;
        let _ = self.conn.sender.try_send(json_str.into());

        return Ok(());
    }

    fn is_closed(&self) -> PyResult<bool> {
        return Ok(self.conn.sender.is_closed());
    }
}
