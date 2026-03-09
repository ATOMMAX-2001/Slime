# Slime – A Rust + Python Hybrid Web Framework

Slime is a high-performance web framework that combines Rust and python

It is designed for developers who want Python developer experience with a Rust-powered server core.

Slime supports:

- synchronous Python handlers
- multipart/form-data (file uploads)
- JSON and form requests
- template rendering with context
- streaming response
- background-style worker execution
- multiple Python worker threads
- websocket


---

## Features

- Python handler functions
- Multiple worker pool model
- No asyncio required
- Multipart form support
- File uploads
- Streaming Response
- Cookie signing
- Custom headers
- JSON / HTML / plain responses and others
- templates rendering with context
- Hot reload of templates in dev mode
- WebSocket

---



## Basic Application

```python
@app.route(path="/", method="GET")
def index(req, resp):
    return resp.plain("Hello from Slime")
```


## Request Body

```python
@app.route(path="/test", method="POST")
def hello(req, resp):
    print("query", req.query)
    print("params", req.params)
    print("body", req.body)
    print("json", req.json)
    print("form", req.form)
    print("text", req.text)
    print("bytes", req.bytes)
    print("file",req.file)
    return resp.json({"status": "ok"})
```



### File Upload
```python
@app.route(path="/test", method="POST")
def hello(req, resp):
    file = req.file[0] # use can upload multiple files
    print(file.filename)
    print(file.content_type)
    print(file.file_path)
    print(file.file_size)
    print(file.extension)
    file.save(f"testing_file.{file.extension}")
    return resp.json({"status": "ok"})

```


### Template Render

```python
@app.route(path="/", method="GET")
def land(req, resp):
    html = req.render("hello.html", **{"name": "abilash", "slimeVersion": "0.0.1"})
    return resp.html(html)

```

### Middleware

```python
#NOTE: middleware  should be declared after declaring route handler 
# LifeCycle handler => 
# middle before request -> router handler -> middle after request

@app.middle_after(path="/", method="GET")
def land_after(req, resp):
    resp.set_header("BEFORE", "Request")


@app.middle_before(path="/", method="GET")
def land_before(req, resp):
    resp.set_header("AFTER", "REQUEST")

```

### Streaming

```python
@app.route(path="/stream", method="GET", stream="text/plain")
def stream_me(req, resp):
    resp.start_stream()
    for i in range(5):
        resp.send(i)
    resp.close()
    
    # OR u can use @app.stream

@app.stream(path="/stream", method="GET", content="text/plain")
def stream_me(req, resp):
    resp.start_stream()
    for i in range(5):
        resp.send(i)
    resp.close()    
```



### WebSocket
```python
@app.websocket(path="/chat", method="GET")
def chatty(req, resp):

    def read_me(msg):
        if not resp.is_closed():
            resp.send(msg)

    resp.on_message(read_me)

    def close_me():
        pass

    resp.on_close(close_me)
```
