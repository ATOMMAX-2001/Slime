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
- App state
- Compression
- Middleware Plugin


---


## Project Structure
Project are created and init by uv after using below command, by default it runs in python no-gil mode, for max performance.

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

Use the below code to create simple GET request in dev environment

```python

from slimeweb import Slime

app = Slime(__file__)

@app.route(path="/", method="GET")
def home(req, resp):
    return resp.plain("Hello World from slime")

if __name__ == "__main__":
    app.serve(dev=True)

```



### Slime Cli

```bash
    slime new projectName   -> Create new project
    slime run main          -> Run slime without GIL
    slime rung main         -> Run slime with GIL
    slime add packageName   -> Add lib to the project deps
    slime use python3.12    -> Change the python runtime
```


## Basic Application
To create a route in slimeweb, Use **route()** method.
Route method contains

- path   ('/' as default)
- method ('GET' as default)
- stream (content-type)
- ws     (create websocket for this path)
- compression (SlimeCompression.NoCompression as default)

> **NOTE:** You can define only one handler per unique route-method combination, defining multiple handlers for the same path and method will raise an error.



```python
@app.route(path="/", method=["GET","POST"])
def index(req, resp):
    if req.method == "GET":
        return resp.plain("Hello from Slime")
    else:
        return resp.json({
            "status": "ok",
            "message": "Hello from Slime"
        })
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

In your Slime file handler, access uploaded files via the **req.file** attribute, it returns a list of **SlimeFile** objects (Refer below for SlimeFile **Api**).

> **Note**: For security, Slime automatically strips the original file extension and assigns a unique filename to each uploaded file.

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


### Compression

Slime supports response body compression to reduce payload size and improve performance.

```python
from slimeweb import SlimeCompression
@app.route(path="/",method="GET",compression=SlimeCompression.Gzip)
def land(req,resp):
    resp.plain("hello" * 5000)
```

In this example, Gzip compression is enabled for the route. If the client requested for compression, Slime will automatically compress the response body before sending it. Refer **Api** for types of compression available.


> **NOTE:** Compression body has a threshold slime will compress the body if the content size is above the threshold, to prevent  unnecessary CPU cycle.



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

> **NOTE:** SlimeState is not atomic. In concurrent scenario with multiple simultaneous request, race condition may occur during state updates, Potentially leading to incorrect values. For production when working with high concurrency, consider implementing your own synchronization or use external state store.

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

> **NOTE:** Middleware handlers must match the **route handler's type**. If your route handler is asynchronous, the middleware must also be async (and vice versa for sync).



### Middleware Plugin
 
Slime lets you create custom middleware plugins with the **use()** method. Define a plugin class with **middle_after** and **middle_before** methods. Both the mesthod should accept two argument SlimeRequest and SlimeResponse.

```python
class SimpleMiddle:
    def middle_after(self, req, resp):
        resp.set_header("PluginAfter","CustomPlugin")
    def middle_before(self, req, resp):
        resp.set_header("PluginBefore","CustomPlugin")
        
        
if __name__ == "__main__":
    app.use(SimpleMiddle)
    # or 
    app.use(SimpleMiddle,method=["POST","GET"],path="/home")

```

This example builds a custom middleware plugin named SimpleMiddle with both middle_after and middle_before methods. To apply the plugin, we are using **use()** method, which targets all routes and HTTP methods by default. We can limit the scope of the plugin by specifying the route and the path.

> **NOTE:** Plugin **use()** should be used after declaring the routes, otherwise error will be raised.



### Streaming

Streaming in slime is simple and straightforward. When declaring your route, specify the stream's **content-type**. 
You can add any headers to the response before calling **start_stream()**. Once start_stream is called, streaming begins to the user.
Use **send()** to stream data chunks and slime automatically serializes them before sending. 
Call **close()** when done to end the connection.

> **NOTE:** Updating headers after start_stream() will cause an error.

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

> **NOTE:** The on_message() callback must accept 1 argument which is the data sent by the client.

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

### Swagger Docs

Slime can automatically generate Swagger documentation and serve it at the **/docs** endpoint when your server is running in development mode **(dev=True)**.

To enable this, you simply need to define your documentation using the **@docs()** decorator.

```python


class SubItem:
    is_item: bool
    how_long: int


class User:
    name: str
    age: float
    sub: dict[str, SubItem]



 @app.docs(
    title="just checking",
    description="Simple landing page",
    response_type=SlimeResponseType.PlainResponse,
    schema=SlimeSchema(
        body=BodySchema(schema_name=User), query=[QuerySchema(name="name", type=str)]
    ),
)
@app.route("/", method=["GET"])
def land(req, resp):
    print(req.header)
    if req.method == "GET":
        resp.plain("hello" * 3000)
    else:
        resp.json({"status": "ok"})

```
In this example, documentation is attached to a route by providing details like **title, description, response_type, and schema** through the @docs() decorator.


For the schema, you can define both:

- **BodySchema** (Request payload)
- **QuerySchema**

These are available from the slimeweb package. Please check the **Api** reference for more details on how to define schemas.

> **NOTE:** You can define only one @docs() to route only, more than one can raise error.


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
  resp.set_status(status_id: int) -> None
  # Here status is optional parameter 
  resp.plain(data: str,status=200)
  resp.json(data: any,status=200) # any Pyobject which we can serialize
  resp.html(data: str,status=200)
  
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


### SlimeCompression
```python
from slimeweb import SlimeCompression #Enum

    SlimeCompression.NoCompression  (default)
    SlimeCompression.Gzip
    SlimeCompression.Brotli
    SlimeCompression.Zstd

```

### SlimeDocs
```python
from slimeweb import SlimeResponseType,SlimeSchema,BodySchema,QuerySchema

@docs(
    title: str= "",
    description: str="",
    response_type: SlimeResponseType = SlimeResponseType.JSON
    schema: SlimeSchema =SlimeSchema()
)
```

### SlimeResponseType

```python 
from slimeweb import SlimeResponseType #Enum 
    SlimeResponseType.PlainResponse 
    SlimeResponseType.JsonResponse 
    SlimeResponseType.HTMLResponse 
    SlimeResponseType.StreamResponse 
    SlimeResponseType.WebSocketResponse 
    SlimeResponseType.CsvResponse
    SlimeResponseType.XmlResponse
    SlimeResponseType.BinaryResponse
```
### SlimeSchema

```python
from slimeweb import SlimeSchema

SlimeSchema(
    query: list[QuerySchema]|None,
    body: BodySchema|None
)

```

### QuerySchema & BodySchema

```python
from slimeweb import QuerySchema,BodySchema

  BodySchema(schema_name: class)
  
  QuerySchema(
      name: str,
      type: str|int|bool,
      requiured: bool =True
  )
    
```



### Benchmark
[BenchMark Code with no-gil example:](https://github.com/Abilash2001/SlimeWeb/)

![Slimeweb benchmark with no-gil](https://raw.githubusercontent.com/Abilash2001/SlimeWeb/main/bench/slimebench.png)


### License

This project is licensed under the terms of **MIT** license


Thank you & enjoy using SlimeWeb ❤️ ~ Abilash Suresh
