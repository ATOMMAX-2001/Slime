<div align="center">
  <img src="https://raw.githubusercontent.com/Abilash2001/SlimeWeb/main/bench/slime_photo.png" width="300">
</div>

# Slime – A Rust + Python Hybrid Web Framework

Slime is a high-performance web framework that combines Rust and python

It is designed for developers who want Python developer experience with a Rust-powered server core. Slime is ideal for building:

- Api
- Real-time application
- Web backends
- Microservices
- Streaming Services


## Installation
```bash
    pip install slimeweb
    slime new ProjectName
    cd ProjectName
    slime run ProjectName
```
After running these commands, open your browser and navigate to **localhost:3000**. You'll see this message displayed:

```plain
Hello World from slime
```


---

## Features

- Python handler functions 
- Rust powered HTTP server
- Multiple worker pool model
- Sync & Async handler
- Multipart form support
- File uploads
- Streaming Response
- Cookie signing
- Custom headers
- JSON / HTML / raw responses
- Templates rendering with context
- Static serving
- Hot reload of templates in dev mode
- WebSocket

---


## Project Structure
Project are created and init by uv after
```bash
slime new ProjectName
``` 

```plain
root/
│
├── .venv/                 # Virtual environment
├── static/                # Static files (CSS, JS, images)
├── templates/             # HTML templates
│
├── .gitignore             # Git ignore rules
├── .python-version        # Python version specification
├── main.py                # Main application entry point
├── pyproject.toml         # Project configuration and dependencies
├── README.md              # Project documentation
└── uv.lock                # Locked dependency versions
```



## Getting Started

```python

from slimeweb import Slime

app = Slime(__file__)

@app.route(path="/", method="GET")
def home(req, resp):
    return resp.plain("Hello World from slime")

if __name__ == "__main__":
    app.serve(dev=True)

```





## Basic Application
To create a route in slimeweb, Use **route()** method.
Route method contains

- path   ('/' as default)
- method ('GET' as default)
- stream (content-type)
- ws     (create websocket for this path)

**NOTE:** You can set only one path and method at a same time, For multiple method for same path you need to create different handler.

```python
@app.route(path="/", method="GET")
def index(req, resp):
    return resp.plain("Hello from Slime")
```

Handlers in slimeweb can be written as either regular synchronous functions or asynchronous ones. Async handlers run using Python's asyncio event loop for efficient, non-blocking execution.

Every handler receives exactly two arguments:

- **SlimeRequest**: The incoming request object, containing details like headers, body, and query parameters.

- **SlimeResponse**: The response object you'll use to build and send the output back to the client.

The exact way you handle and populate the response depends on the route type (e.g., HTTP,Streaming or Websocket). Check the **API & Examples** reference below for type-specific details.

To start the Slime server we should use **server()** method.

```python
 app.serve()
```
**serve()** has few optional argument you can pass

- host (default 127.0.0.1)
- port (default 3000)
- secret_key (default None, used for cookie sign)
- dev (default False)
- app_state (default {})

**Worker:**

You can control the number of workers using the **SLIME_WORKER** environment variable.

- In development mode, it defaults to 1 worker
- In production, it automatically uses the number of CPU cores

```bash
 export SLIME_WORKER=3
 
 OR
 
 $ENV:SLIME_WORKER="3"
```


## Request Body

Slime supports all kinds of request body
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

In your Slime file handler, access uploaded files via the **req.file** attribute, it returns a list of **SlimeFile** objects (Refer below for SlimeFile **api**).

**Note**: For security, Slime automatically strips the original file extension and assigns a unique filename to each uploaded file.

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
Few examples on rendering
```html
<h1>Hello {{ name }}!</h1>
```
above will result like 
```html
<h1>Hello abilash!</h1>
```
**Logic**
```html
{% if user %}
  Hello {{ user }}
{% else %}
  Hello Guest
{% endif %}
```
**Loops**
```html
<ul>
{% for item in items %}
  <li>{{ item }}</li>
{% endfor %}
</ul>
```

You can also generate 

- Markdown
- SQL
- Custom Code
- YAML
- JSON
- Config File
- HTML & etc..

### App State

App state allows you to maintain shared data across requests during your app's lifecycle.

Initialize app state when starting your server

```python
app.serve(app_state={"counter": 0})
```
Within each handler, the current app state is automatically injected into the **SlimeRequest** object. Use these methods to interact with it:

- **req.get_state(key)**, To retrieve the current value for a given key.
- **req.update_state(key,value)**, To update the value for a given key.

**NOTE:** SlimeState is not atomic. In concurrent scenario with multiple simultaneous request, race condition may occur during state updates, Potentially leading to incorrect values. For production when working with high concurrency, consider implementing your own synchronization or use external state store.

### Middleware

Middleware  should be declared after declaring route handler 

**LifeCycle of request handler**

Middle before request **->** Router handler **->** Middle after request

```python
@app.middle_before(path="/", method="GET")
def land_before(req, resp):
    resp.set_header("AFTER", "REQUEST")


@app.middle_after(path="/", method="GET")
def land_after(req, resp):
    resp.set_header("BEFORE", "Request")

```

**NOTE:** Middleware handlers must match the **route handler's type**. If your route handler is asynchronous, the middleware must also be async (and vice versa for sync).

### Streaming

Streaming in slime is simple and straightforward. When declaring your route, specify the stream's **content-type**. 
You can add any headers to the response before calling **start_stream()**. Once start_stream is called, streaming begins to the user.
Use **send()** to stream data chunks and slime automatically serializes them before sending. 
Call **close()** when done to end the connection.

**NOTE:** Updating headers after start_stream() will cause an error.

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

WebSockets in Slime are event-driven, meaning Slime calls specific callback methods when key events happen.

You'll typically need two optional callbacks (not required):
- One for when data is received from the client
- One for when the client disconnects

In the echo example below, the **read_me()** callback first checks if the client is still connected. If yes, it echoes back the exact message it received.

**NOTE:** The on_message() callback must accept 1 argument which is the data sent by the client.

```python
@app.websocket(path="/chat", method="GET")
def chatty(req, resp):

    def read_me(msg):
        if not resp.is_closed():
            resp.send(msg)    

    def close_me():
        pass

    resp.on_message(read_me)
    resp.on_close(close_me)
```

## Api

###  Slime Request
```python
  req.method -> str
  req.path -> str
  req.client -> str # client address
  req.header -> Dict[str,str]
  req.body -> Bytes
  req.bytes -> [Bytes] (u8)
  req.form -> Dict[str,str]
  req.file -> [SlimeFile]
  req.json -> Any
  req.query -> Dict[str,str]
  req.params -> Dict[str,str]
  req.text -> str
  req.secret_key -> str
  req.get_cookies() -> Dict[str,str]
  req.get_signed_cookie(key: str) -> str|None
  req.render(template_name: str,Dict[str,any]|None)
  req.no_of_files_available() -> int
  req.get_state(key: str) -> Any
  req.update_state(key: str,value: Any)
```



###  HTTP Slime Response
```python
  resp.set_cookie(key: str,value: str) -> None
  resp.set_sign_cookie(key:str,value: str,secret: str) -> None
  resp.set_header(key: str,value: str) -> None
  resp.plain(data: str)
  resp.json(data: any) # any Pyobject which we can serialize
  resp.html(data: str)
  
```

### Stream Slime Response
```python
  resp.content_type ->  str
  resp.headers -> Dict[str,str]
  resp.set_header(key: str,value: str)
  resp.start_stream() # to start the stream
  resp.send(data: any) # any Pyobject which we can serialize
  
  #send has optional parameter strict_order which is default as True 
  #if your streaming doesnt need to be in order it will create task for each send and run in async internally.
  # NOTE: If strict_order=False and send() is failed it will return silently with error in terminal.
  
  resp.close() # close stream
  
```


### Websocket Slime Response
```python
   resp.id -> str
   resp.on_message(handler: Callable) -> None
   resp.on_close(handler: Callable) -> None
   resp.send(data: any) # any Pyobject that we can serialize
   resp.is_closed() -> bool
  
```

### SlimeFile
```python
    slimefile_obj.filename -> str
    slimefile_obj.content_type -> str
    slimefile_obj.file_path -> str
    slimefile_obj.file_size -> int
    slimefile_obj.extension -> str
    slimefile_obj.save(new_filename: str) -> None
    slimefile_obj.clean() -> None # remove temp file
```


### Benchmark
[BenchMark Code with slime example:](https://github.com/Abilash2001/SlimeWeb/)

![Slimeweb benchmark](https://raw.githubusercontent.com/Abilash2001/SlimeWeb/main/bench/slimebench.png)


### License

This project is licensed under the terms of **MIT** license


Thank You & enjoy using SlimeWeb ❤️
