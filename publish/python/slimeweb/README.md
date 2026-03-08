# Slime – A Rust + Python Hybrid Web Framework

Slime is a high-performance web framework that combines Rust and python

It is designed for developers who want Python developer experience with a Rust-powered server core.

Slime supports:

- synchronous Python handlers
- multipart/form-data (file uploads)
- JSON and form requests
- template rendering with context
- streaming
- background-style worker execution
- multiple Python worker threads


---

## Features

- Python handler functions
- Multiple worker pool model
- No asyncio required
- Multipart form support
- File uploads with temp storage
- Streaming
- Cookie signing
- Custom headers
- JSON / HTML / plain responses
- templates rendering with context
- Hot reload of templates in dev mode

---



## Basic Application

```python
@app.route(path="/", method="GET")
def index(req, resp):
    return resp.plain("Hello from Slime")

```
