# AUTHOR: S.ABILASH
# Email: abinix01@gmail.com

import copy
import inspect
from enum import Enum
from typing import Any, Callable, Dict, List, Literal, Set, Tuple, Type

from .errors import (
    InvalidHandler,
    InvalidMiddlewareHandlerType,
    MethodException,
    MultipleRouteException,
    RouteHandlerNotFoundException,
)

AVAILABLE_METHOD = ["GET", "POST", "PATCH", "PUT", "DELETE", "OPTIONS", "HEAD"]


class SlimeCompression(Enum):
    NoCompression = 0
    Gzip = 1
    Brotli = 2
    Zstd = 3


class SlimeMiddleware:
    def middle_before(self, req, resp):
        pass

    def middle_after(self, req, resp):
        pass


class SlimeException(Exception):
    def __init__(self, status=400, message="An unknown error occurred") -> None:
        self.status = status
        self.message = message


class SlimeResponseType(Enum):
    PlainResponse = 0
    JsonResponse = 1
    HTMLResponse = 2
    StreamResponse = 3
    WebSocketResponse = 4
    CsvResponse = 5
    XmlResponse = 6
    BinaryResponse = 7


class QuerySchema:
    def __init__(self, name, type, required=True) -> None:
        self.name: str = name
        self.type: str = ""
        if type is int:
            self.type = "integer"
        elif type is str:
            self.type = "string"
        elif type is bool:
            self.type = "boolean"
        else:
            raise ValueError("Query schema can have only int|str|bool type")
        self.required: bool = required

    def compile(self):
        return {
            "name": self.name,
            "in": "query",
            "required": self.required,
            "schema": {"type": self.type, "title": self.name.upper()},
        }


class BodySchema:
    def __init__(self, schema_name: Type) -> None:
        if not isinstance(schema_name, type):
            raise ValueError("Need proper schema body")
        self.schema_name = schema_name.__name__
        self.schema_struture = schema_name.__annotations__
        self.external_schema: Set[BodySchema] = set()

    def required(self) -> List[str]:
        return list(dict.fromkeys(self.schema_struture))

    def compile(self) -> Dict[str, Dict[str, str]]:
        result = {}
        for key, value in self.schema_struture.items():
            result[key] = {}
            result[key]["title"] = key
            if value is int:
                result[key]["type"] = "number"
            elif value is str:
                result[key]["type"] = "string"
            elif value is bool:
                result[key]["type"] = "boolean"
            elif value is float:
                result[key]["type"] = "number"
                result[key]["format"] = "double"
            elif hasattr(value, "__origin__") and (
                value.__origin__ is list or value.__origin__ is dict
            ):
                is_dict = value.__origin__ is dict
                if is_dict:
                    result[key]["type"] = "object"
                else:
                    result[key]["type"] = "array"

                kind = ""
                index = 0
                if is_dict:
                    index = 1
                    kind = "additionalProperties"
                else:
                    index = 0
                    kind = "items"
                if value.__args__[index] is int:
                    result[key][kind] = {"type": "integer"}
                elif value.__args__[index] is bool:
                    result[key][kind] = {"type": "boolean"}
                elif value.__args__[index] is str:
                    result[key][kind] = {"type": "string"}
                elif value.__args__[index] is float:
                    result[key][kind] = {"type": "number", "format": "double"}
                elif isinstance(value.__args__[index], type):
                    result[key][kind] = {
                        "$ref": f"#/components/schemas/{value.__args__[index].__name__}"
                    }
                    self.external_schema.add(BodySchema(value.__args__[index]))
                else:
                    raise ValueError("Unknown type, Need proper value ")

            elif isinstance(value, type):
                self.external_schema.add(BodySchema(value))
                result[key] = {"$ref": f"#/components/schemas/{value.__name__}"}
            else:
                raise ValueError(
                    f"Invalid definition need value with BodySchema instance {value}"
                )
        return result


class SlimeSchema:
    def __init__(
        self, query: List[QuerySchema] | None = None, body: BodySchema | None = None
    ) -> None:
        if isinstance(query, list) or query is None:
            self.query: List[QuerySchema] | None = query
        else:
            raise ValueError("QuerySchema has to be in list")
        self.body: BodySchema | None = body


class SlimeDocs:
    def __init__(
        self,
        handler_name: str,
        title: str = "",
        description: str = "",
        path: str = "",
        method: List[str] = [],
        response_type: SlimeResponseType = SlimeResponseType.JsonResponse,
        schema: SlimeSchema = SlimeSchema(),
    ) -> None:
        self.handler_name = handler_name
        self.title = title
        self.description = description
        self.path = path
        self.method = (
            copy.deepcopy(AVAILABLE_METHOD)
            if len(method) == 1 and method[0] == "*"
            else method
        )
        self.response_type = response_type
        self.schema = schema

    def get_response_content(self) -> str:
        if self.response_type == SlimeResponseType.HTMLResponse:
            return "text/html"
        elif self.response_type in [
            SlimeResponseType.PlainResponse,
            SlimeResponseType.WebSocketResponse,
        ]:
            return "text/plain"

        elif self.response_type == SlimeResponseType.CsvResponse:
            return "text/csv"
        elif self.response_type == SlimeResponseType.XmlResponse:
            return "text/xml"
        elif self.response_type == SlimeResponseType.BinaryResponse:
            return "application/octet-stream"
        else:
            return "application/json"


class Routes:
    def __init__(
        self,
        path: str = "/",
        method: str = "GET",
        stream: str | None = None,
        ws: bool = False,
        compression: SlimeCompression = SlimeCompression.NoCompression,
        body_size: int = 1024 * 1024 * 10,
    ) -> None:
        global AVAILABLE_METHOD
        if method.upper() not in AVAILABLE_METHOD:
            raise MethodException(f"{method} is not Valid")

        self.path: str = path
        self.method: str = method
        self.stream: str | None = stream
        self.ws: bool = ws
        self.compression: int = compression.value
        self.body_size: int = body_size

    def __hash__(self) -> int:
        return hash((self.path, self.method))

    def __eq__(self, value: object) -> bool:
        return (
            isinstance(value, Routes)
            and self.path == value.path
            and self.method == value.method
        )

    def __str__(self) -> str:
        return f"""
            Path: {self.path}
            Method: {self.method}
            BodySize: {self.body_size}
            {f"Stream: {self.stream}" if self.stream is not None else ""}
            {f"Websocket: {self.ws}" if self.ws is not None else ""}
            {f"Compression: {self.compression}" if self.compression is not None else ""}
        """


class Slime:
    def __init__(self, filename: str) -> None:
        if filename is None or not isinstance(filename, str):
            raise ValueError("Need argument as __file__ ")

        # for templating purpose we can fetch file path with /templates and /static
        self.__filename: str | None = filename

        # => you can define multiple or same path with different request
        # like let saay user can assign  a path /name
        # with methods like GET,POST under /name
        self.__routes: Dict[Routes, Dict[str, List[Tuple[Callable, bool]] | None]] = {}

        # to generate swagger docs user can  create
        # custom definition
        self.__docs: List[SlimeDocs] = []

        # app start and end

        self.__app_start: Callable | None = None
        self.__app_end: Callable | None = None

    def __apply_middleware(
        self,
        path: str,
        method: str,
        handler: Callable,
        is_async: bool,
        middle_kind: Literal["before", "after"],
        is_plugin=False,
    ):
        found: bool = False
        for route in self.__routes:
            if route.path == path and route.method == method:
                found = True
                call_handler = self.__routes[route]
                if call_handler["handler"] is not None:
                    if call_handler["handler"][0][1] == is_async:
                        result = call_handler[middle_kind]
                        if result is None:
                            call_handler[middle_kind] = [(handler, is_async)]
                        else:
                            result.append((handler, is_async))
                        break
                    else:
                        error = f"Middle {middle_kind} handler should be of {'async' if call_handler['handler'][0] else 'sync'} type similar to route handler for Path: {path},Method: {method}"
                        raise InvalidMiddlewareHandlerType(error)

        if not found and not is_plugin:
            raise RouteHandlerNotFoundException(
                f"You need to define the request handler to declare middleware for Path: {path}, Method: {method}"
            )

    def middle_before(
        self, path: str = "/", method: List[str] | str = "GET", _is_plugin: bool = False
    ) -> Callable:
        def wrapper(middle_handler) -> Callable:
            is_async = inspect.iscoroutinefunction(middle_handler)
            if isinstance(method, list):
                if path == "*":
                    for route in self.__routes:
                        if route.method in method:
                            self.__apply_middleware(
                                handler=middle_handler,
                                is_async=is_async,
                                method=route.method,
                                middle_kind="before",
                                path=route.path,
                            )
                else:
                    for method_col in method:
                        self.__apply_middleware(
                            handler=middle_handler,
                            is_async=is_async,
                            method=method_col,
                            middle_kind="before",
                            path=path,
                            is_plugin=_is_plugin,
                        )
            elif isinstance(method, str):
                if path == "*":
                    for route in self.__routes:
                        if route.method == method:
                            self.__apply_middleware(
                                handler=middle_handler,
                                is_async=is_async,
                                method=method,
                                middle_kind="before",
                                path=route.path,
                                is_plugin=_is_plugin,
                            )
                else:
                    self.__apply_middleware(
                        handler=middle_handler,
                        is_async=is_async,
                        method=method,
                        middle_kind="before",
                        path=path,
                        is_plugin=_is_plugin,
                    )
            else:
                raise InvalidHandler(
                    "Method should be of type : String or List[String]"
                )
            return middle_handler

        return wrapper

    def middle_after(
        self, path: str = "/", method: List[str] | str = "GET", _is_plugin: bool = False
    ) -> Callable:
        def wrapper(middle_handler) -> Callable:
            is_async = inspect.iscoroutinefunction(middle_handler)
            if isinstance(method, list):
                if path == "*":
                    for route in self.__routes:
                        if route.method in method:
                            self.__apply_middleware(
                                handler=middle_handler,
                                is_async=is_async,
                                method=route.method,
                                middle_kind="after",
                                path=route.path,
                                is_plugin=_is_plugin,
                            )
                else:
                    for method_col in method:
                        self.__apply_middleware(
                            handler=middle_handler,
                            is_async=is_async,
                            method=method_col,
                            middle_kind="after",
                            path=path,
                            is_plugin=_is_plugin,
                        )
            elif isinstance(method, str):
                if path == "*":
                    for route in self.__routes:
                        if route.method == method:
                            self.__apply_middleware(
                                handler=middle_handler,
                                is_async=is_async,
                                method=method,
                                middle_kind="after",
                                path=route.path,
                                is_plugin=_is_plugin,
                            )
                else:
                    self.__apply_middleware(
                        handler=middle_handler,
                        is_async=is_async,
                        method=method,
                        middle_kind="after",
                        path=path,
                        is_plugin=_is_plugin,
                    )
            else:
                raise InvalidHandler(
                    "Method should be of type : String or List[String]"
                )
            return middle_handler

        return wrapper

    def _check_handler_type_for_route(self, path: str, method: str) -> bool:
        for route, handler in self.__routes.items():
            if (
                route.path == path
                and route.method == method
                or (path == "*" and route.method == method)
            ):
                if handler["handler"] is not None and handler["handler"][0] is not None:
                    return handler["handler"][0][1]
        return False

    def __apply_plugin(
        self,
        is_async: bool,
        path: str,
        method: str | List[str],
        plugin_instance: SlimeMiddleware,
    ):
        found: bool = False

        if hasattr(plugin_instance, "middle_before"):
            found = True
            if is_async:

                @self.middle_before(path=path, method=method, _is_plugin=True)
                async def before_plugin_async_handler(req, resp):
                    plugin_instance.middle_before(req, resp)
            else:

                @self.middle_before(path=path, method=method, _is_plugin=True)
                def before_plugin_handler(req, resp):
                    plugin_instance.middle_before(req, resp)

        if hasattr(plugin_instance, "middle_after"):
            found = True
            if is_async:

                @self.middle_after(path=path, method=method, _is_plugin=True)
                async def after_plugin_async_handler(req, resp):
                    plugin_instance.middle_after(req, resp)
            else:

                @self.middle_after(path=path, method=method, _is_plugin=True)
                def after_plugin_handler(req, resp):
                    plugin_instance.middle_after(req, resp)

        if not found:
            raise InvalidMiddlewareHandlerType(
                "SlimePlugin class should have atleast one method middle_before or middle_after"
            )

    def use(
        self, plugin_instance: Any, method: List[str] | str = "*", path="*"
    ) -> None:
        if not isinstance(plugin_instance, SlimeMiddleware):
            print(type(plugin_instance))
            raise ValueError(
                "Not a valid plugin definition, It has to be derived from SlimeMiddleware class"
            )
        if path == "*":
            route_details = {
                (route.path, route.method): handler["handler"][0][1]  # type: ignore
                for (route, handler) in self.__routes.items()
            }

            for route, is_async in route_details.items():
                if method == "*":
                    self.__apply_plugin(
                        is_async=is_async,
                        method=route[1],
                        path=route[0],
                        plugin_instance=plugin_instance,
                    )
                else:
                    if isinstance(method, str) and route[1] == method:
                        self.__apply_plugin(
                            is_async=is_async,
                            method=method,
                            path=route[0],
                            plugin_instance=plugin_instance,
                        )
                    else:
                        if route[1] in method:
                            self.__apply_plugin(
                                is_async=is_async,
                                method=route[1],
                                path=route[0],
                                plugin_instance=plugin_instance,
                            )

        else:
            self.__apply_plugin(
                is_async=self._check_handler_type_for_route(
                    method=method if isinstance(method, str) else method[0], path=path
                ),
                method=method,
                path=path,
                plugin_instance=plugin_instance,
            )

    def __apply_route(
        self,
        handler: Callable,
        path: str,
        method: str,
        stream: str | None,
        ws: bool,
        compression: SlimeCompression,
        body_size: int,
    ):
        if not isinstance(body_size, int):
            raise ValueError("body_size should be of type <int> represent bytes")
        new_route = Routes(path, method, stream, ws, compression, body_size)

        if new_route not in self.__routes:
            self.__routes[new_route] = {
                "handler": [
                    (
                        handler,
                        inspect.iscoroutinefunction(handler),
                    )
                ],
                "before": None,
                "after": None,
            }
        else:
            raise MultipleRouteException(
                f"Multiple route definition for Path: {path} and Method: {method}"
            )

    def route(
        self,
        path: str = "/",
        method: str | List[str] = "GET",
        stream: str | None = None,
        ws: bool = False,
        compression: SlimeCompression = SlimeCompression.NoCompression,
        body_size: int = 1024 * 1024 * 10,
    ) -> Callable:
        def wrapper(route_handler) -> Callable:
            if route_handler is None or not callable(route_handler):
                raise InvalidHandler(
                    f"Route handler should be a function for [Path: {path}, Method: {method}]"
                )
            setattr(route_handler, "__path", path)
            setattr(route_handler, "__method", method)
            setattr(route_handler, "__set_docs", False)
            if isinstance(method, list):
                for method_col in dict.fromkeys(method):
                    self.__apply_route(
                        handler=route_handler,
                        method=method_col,
                        path=path,
                        stream=stream,
                        ws=ws,
                        compression=compression,
                        body_size=body_size,
                    )
            else:
                if method == "*":
                    global AVAILABLE_METHOD
                    for all_method in AVAILABLE_METHOD:
                        self.__apply_route(
                            handler=route_handler,
                            method=all_method,
                            stream=stream,
                            ws=ws,
                            path=path,
                            compression=compression,
                            body_size=body_size,
                        )
                else:
                    self.__apply_route(
                        handler=route_handler,
                        method=method,
                        stream=stream,
                        ws=ws,
                        path=path,
                        compression=compression,
                        body_size=body_size,
                    )
            return route_handler

        return wrapper

    def stream(
        self,
        path: str = "/",
        method: List[str] | str = "GET",
        content: str = "text/plain",
        compression: SlimeCompression = SlimeCompression.NoCompression,
        body_size: int = 1024 * 1024 * 10,
    ) -> Callable:
        def wrapper(stream_handler) -> Callable:
            if stream_handler is None or not callable(stream_handler):
                raise InvalidHandler(
                    f"Stream handler should be a function for [Path: {path}, Method: {method}]"
                )
            if not isinstance(content, str):
                raise ValueError(
                    f"Stream content type should be of type <String> with MIME for [Path: {path}, Method: {method}]"
                )
            setattr(stream_handler, "__path", path)
            setattr(stream_handler, "__method", method)
            setattr(stream_handler, "__set_docs", False)
            if isinstance(method, list):
                for method_col in dict.fromkeys(method):
                    self.__apply_route(
                        compression=compression,
                        handler=stream_handler,
                        method=method_col,
                        path=path,
                        stream=content,
                        ws=False,
                        body_size=body_size,
                    )
            else:
                self.__apply_route(
                    compression=compression,
                    handler=stream_handler,
                    method=method,
                    path=path,
                    stream=content,
                    ws=False,
                    body_size=body_size,
                )
            return stream_handler

        return wrapper

    def websocket(
        self, path: str = "/", method: str = "GET", body_size: int = 1024 * 1024 * 10
    ) -> Callable:
        def wrapper(websocket_handler) -> Callable:
            if websocket_handler is None or not callable(websocket_handler):
                raise InvalidHandler(
                    f"Websocket handler should be a function for [Path: {path}, Method: {method}]"
                )
            setattr(websocket_handler, "__path", path)
            setattr(websocket_handler, "__method", method)
            setattr(websocket_handler, "__set_docs", False)
            self.__apply_route(
                compression=SlimeCompression.NoCompression,
                handler=websocket_handler,
                method=method,
                path=path,
                stream=None,
                ws=True,
                body_size=body_size,
            )
            return websocket_handler

        return wrapper

    def _get_routes(
        self,
    ) -> Dict[Routes, Dict[str, List[Tuple[Callable, bool]] | None]]:
        return self.__routes

    def docs(
        self,
        title: str = "",
        description: str = "",
        response_type: SlimeResponseType = SlimeResponseType.JsonResponse,
        schema: SlimeSchema = SlimeSchema(),
    ):
        def wrapper(handler):
            if not callable(handler):
                raise RuntimeError("@docs needs to be define above @route.")
            is_docs_sett = getattr(handler, "__set_docs", False)
            if is_docs_sett:
                raise ValueError("@docs has been already defined for this route")
            path = getattr(handler, "__path", None)
            method = getattr(handler, "__method", None)

            function_name = handler.__name__

            if path is None or method is None:
                raise ValueError("@docs can be defined only for routes.")

            self.__docs.append(
                SlimeDocs(
                    handler_name=function_name,
                    description=description,
                    title=title,
                    method=[method] if isinstance(method, str) else method,
                    path=path,
                    response_type=response_type,
                    schema=schema,
                )
            )
            setattr(handler, "__set_docs", True)
            return handler

        return wrapper

    def __generate_docs_path(self):
        api = {
            "openapi": "3.0.3",
            "info": {"title": "SlimeWeb Api Docs", "version": "0.1"},
            "paths": {},
        }
        all_paths = list(set(self.__docs))
        for path in all_paths:
            api["paths"][path.path] = {}
            schema_result = {}
            if path.schema.body is not None:
                schema_result = {
                    path.schema.body.schema_name: {
                        "properties": path.schema.body.compile(),
                        "type": "object",
                        "required": path.schema.body.required(),
                    }
                }
                for ext_schema in path.schema.body.external_schema:
                    schema_result[ext_schema.schema_name] = {
                        "properties": ext_schema.compile(),
                        "type": "object",
                        "required": ext_schema.required(),
                    }
                api["components"] = {
                    "schemas": schema_result,
                }
            query_schema_result = []
            if path.schema.query is not None:
                query_schema_result = [query.compile() for query in path.schema.query]

            for method in path.method:
                api["paths"][path.path][method.lower()] = {}
                result = {
                    "summary": path.title,
                    "operationId": path.handler_name,
                    "parameters": query_schema_result,
                    "requestBody": {
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": f"#components/schemas/{path.schema.body.schema_name}"
                                }
                            }
                        }
                    }
                    if path.schema.body is not None
                    else {},
                    "responses": {
                        "200": {
                            "description": path.description,
                            "content": {path.get_response_content(): {"schame": {}}},
                        }
                    },
                }
                api["paths"][path.path][method.lower()] = copy.deepcopy(result)
        return api

    def start(self):
        def wrapper(handler):
            if not callable(handler):
                raise InvalidHandler("Application start handler should be a function")
            self.__app_start = handler
            return handler

        return wrapper

    def end(self):
        def wrapper(handler):
            if not callable(handler):
                raise InvalidHandler("Application end handler should be a function")
            if len(list(inspect.signature(handler).parameters)) != 1:
                raise InvalidHandler(
                    "Application end handler should have one argument of type None|Exception"
                )
            self.__app_end = handler
            return handler

        return wrapper

    def generate_docs(self):
        api = self.__generate_docs_path()
        HTML_BODY = """
        <!DOCTYPE html>
        <html>
        <head>
          <title>SlimeWeb Api Docs</title>
          <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist/swagger-ui.css" />
        </head>
        <body>
          <div id="swagger"></div>

          <script src="https://unpkg.com/swagger-ui-dist/swagger-ui-bundle.js"></script>
          <script>
            SwaggerUIBundle({
              url: "/openapi.json",
              dom_id: "#swagger",
              deepLinking: true,
                presets: [
                  SwaggerUIBundle.presets.apis,
                  SwaggerUIBundle.SwaggerUIStandalonePreset
                ],
                layout: "BaseLayout"

            });
          </script>
        </body>
        </html>
        """

        @self.route("/docs", method="GET")
        def land_docs(req, resp):
            return resp.html(HTML_BODY)

        @self.route("/openapi.json", method="GET")
        def land_api_schema(req, resp):
            return resp.json(api)

    def serve(
        self,
        host: str = "127.0.0.1",
        port: int = 3000,
        secret_key: str | None = None,
        dev: bool = False,
        app_state: Dict[str, Any] = {},
        workers: int = 0,
    ) -> None:
        if dev and len(self.__docs) != 0:
            self.generate_docs()

        if secret_key is None:
            import secrets

            secret_key = secrets.token_urlsafe(30)

        if not isinstance(workers, int):
            raise ValueError("worker needs to be in int type")
        if self.__app_start is not None:
            self.__app_start()

        from . import web_extras

        try:
            web_extras.web.init_web(
                self, host, port, secret_key, dev, app_state, workers
            )
        except Exception as e:
            if self.__app_end is not None:
                self.__app_end(e)
            raise e
        if self.__app_end is not None:
            self.__app_end(None)
        print("Slime server is shutting down...")
        print("Finished")
