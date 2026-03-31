# AUTHOR: S.ABILASH
# Email: abinix01@gmail.com

import inspect
from enum import Enum
from typing import Any, Callable, Dict, List, Literal, Tuple, Type

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


class Routes:
    def __init__(
        self,
        path: str = "/",
        method: str = "GET",
        stream: str | None = None,
        ws: bool = False,
        compression: SlimeCompression = SlimeCompression.NoCompression,
    ) -> None:
        global AVAILABLE_METHOD
        if method.upper() not in AVAILABLE_METHOD:
            raise MethodException(f"{method} is not Valid")

        self.path: str = path
        self.method: str = method
        self.stream: str | None = stream
        self.ws: bool = ws
        self.compression: int = compression.value

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

    def __apply_middleware(
        self,
        path: str,
        method: str,
        handler: Callable,
        is_async: bool,
        middle_kind: Literal["before", "after"],
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
                        error = f"Middle {middle_kind} handler should be of {'async' if call_handler['handler'][1] else 'sync'} type similar to route handler"
                        raise InvalidMiddlewareHandlerType(error)

        if not found:
            raise RouteHandlerNotFoundException(
                "You need to define the request handler to declare middleware"
            )

        # found: bool = False
        # for route in self.__routes:
        #     if (route.path == path and route.method == method) or (
        #         method == "*" and route.path == path
        #     ):
        #         call_handler = self.__routes.get(route)
        #         if call_handler is not None and call_handler["handler"] is not None:
        #             if call_handler["handler"][1] == is_async:
        #                 if call_handler[middle_kind] is None:
        #                     call_handler[middle_kind] = [
        #                         (
        #                             handler,
        #                             is_async,
        #                         )
        #                     ]
        #                     found = True
        #                     break
        #                 else:
        #                     call_handler[middle_kind].append((handler, is_async))
        #                     found = True
        #                     break
        #             else:
        #                 error = f"Middle {middle_kind} handler should be of {'async' if call_handler['handler'][1] else 'sync'} type similar to route handler"
        #                 raise InvalidMiddlewareHandlerType(error)
        # if not found:
        #     raise RouteHandlerNotFoundException(
        #         "You need to define the request handler to declare middleware"
        #     )

    def middle_before(
        self, path: str = "/", method: List[str] | str = "GET"
    ) -> Callable:
        def wrapper(middle_handler) -> Callable:
            if middle_handler is None or not callable(middle_handler):
                raise InvalidHandler(
                    f"Middleware handler should be a function for [Path: {path}, Method: {method}]"
                )
            # apply this middleware to all the available route or path
            if path == "*":
                for route in self.__routes:
                    call_handler = self.__routes.get(route)
                    if call_handler is not None:
                        if call_handler["before"] is None:
                            call_handler["before"] = [
                                (
                                    middle_handler,
                                    inspect.iscoroutinefunction(middle_handler),
                                )
                            ]
                        else:
                            call_handler["before"].append(
                                (
                                    middle_handler,
                                    inspect.iscoroutinefunction(middle_handler),
                                )
                            )
            else:
                is_async = inspect.iscoroutinefunction(middle_handler)
                if isinstance(method, list):
                    for method_col in dict.fromkeys(method):
                        self.__apply_middleware(
                            handler=middle_handler,
                            is_async=is_async,
                            method=method_col,
                            middle_kind="before",
                            path=path,
                        )
                else:
                    if method == "*":
                        global AVAILABLE_METHOD
                        for all_method in AVAILABLE_METHOD:
                            self.__apply_middleware(
                                handler=middle_handler,
                                is_async=is_async,
                                method=all_method,
                                middle_kind="before",
                                path=path,
                            )
                    else:
                        self.__apply_middleware(
                            handler=middle_handler,
                            is_async=is_async,
                            method=method,
                            middle_kind="before",
                            path=path,
                        )
            return middle_handler

        return wrapper

    def middle_after(
        self, path: str = "/", method: List[str] | str = "GET"
    ) -> Callable:
        def wrapper(middle_handler) -> Callable:
            if middle_handler is None or not callable(middle_handler):
                raise InvalidHandler(
                    f"Middleware handler should be a function for [Path: {path}, Method: {method}]"
                )
            if path == "*":
                for route in self.__routes:
                    call_handler = self.__routes.get(route)
                    if call_handler is not None:
                        if call_handler["after"] is None:
                            call_handler["after"] = [
                                (
                                    middle_handler,
                                    inspect.iscoroutinefunction(middle_handler),
                                )
                            ]
                        else:
                            call_handler["after"].append(
                                (
                                    middle_handler,
                                    inspect.iscoroutinefunction(middle_handler),
                                )
                            )
            else:
                is_async = inspect.iscoroutinefunction(middle_handler)
                if isinstance(method, list):
                    for method_col in dict.fromkeys(method):
                        self.__apply_middleware(
                            handler=middle_handler,
                            is_async=is_async,
                            method=method_col,
                            middle_kind="after",
                            path=path,
                        )
                else:
                    if method == "*":
                        global AVAILABLE_METHOD
                        for all_method in AVAILABLE_METHOD:
                            self.__apply_middleware(
                                handler=middle_handler,
                                is_async=is_async,
                                method=all_method,
                                middle_kind="after",
                                path=path,
                            )
                    else:
                        self.__apply_middleware(
                            handler=middle_handler,
                            is_async=is_async,
                            method=method,
                            middle_kind="after",
                            path=path,
                        )
            return middle_handler

        return wrapper

    def __apply_route(
        self,
        handler: Callable,
        path: str,
        method: str,
        stream: str | None,
        ws: bool,
        compression: SlimeCompression,
    ):
        new_route = Routes(path, method, stream, ws, compression)

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
    ) -> Callable:
        def wrapper(route_handler) -> Callable:
            if route_handler is None or not callable(route_handler):
                raise InvalidHandler(
                    f"Route handler should be a function for [Path: {path}, Method: {method}]"
                )
            if isinstance(method, list):
                for method_col in dict.fromkeys(method):
                    self.__apply_route(
                        handler=route_handler,
                        method=method_col,
                        path=path,
                        stream=stream,
                        ws=ws,
                        compression=compression,
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
                        )
                else:
                    self.__apply_route(
                        handler=route_handler,
                        method=method,
                        stream=stream,
                        ws=ws,
                        path=path,
                        compression=compression,
                    )
            return route_handler

        return wrapper

    def stream(
        self,
        path: str = "/",
        method: List[str] | str = "GET",
        content: str = "text/plain",
        compression: SlimeCompression = SlimeCompression.NoCompression,
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
            if isinstance(method, list):
                for method_col in dict.fromkeys(method):
                    self.__apply_route(
                        compression=compression,
                        handler=stream_handler,
                        method=method_col,
                        path=path,
                        stream=content,
                        ws=False,
                    )
            else:
                self.__apply_route(
                    compression=compression,
                    handler=stream_handler,
                    method=method,
                    path=path,
                    stream=content,
                    ws=False,
                )
            return stream_handler

        return wrapper

    def websocket(self, path: str = "/", method: str = "GET") -> Callable:
        def wrapper(websocket_handler) -> Callable:
            if websocket_handler is None or not callable(websocket_handler):
                raise InvalidHandler(
                    f"Websocket handler should be a function for [Path: {path}, Method: {method}]"
                )
            self.__apply_route(
                compression=SlimeCompression.NoCompression,
                handler=websocket_handler,
                method=method,
                path=path,
                stream=None,
                ws=True,
            )
            return websocket_handler

        return wrapper

    def _get_routes(
        self,
    ) -> Dict[Routes, Dict[str, List[Tuple[Callable, bool]] | None]]:
        return self.__routes

    def use(self, obj: Type, method: List[str] | str = "GET", path="*") -> None:
        if not isinstance(obj, type):
            raise InvalidMiddlewareHandlerType(
                'SlimePlugin has to be type class with "middle_before" or "middle_after" method'
            )

        plugin_instance = obj()
        found: bool = False
        if hasattr(plugin_instance, "middle_before"):
            found = True

            @self.middle_before(path=path, method=method)
            def before_plugin_handler(req, resp):
                plugin_instance.middle_before(req, resp)

        if hasattr(plugin_instance, "middle_after"):
            found = True

            @self.middle_after(path=path, method=method)
            def after_plugin_handler(req, resp):
                plugin_instance.middle_after(req, resp)

        if not found:
            raise InvalidMiddlewareHandlerType(
                "SlimePlugin class should have atleast one method middle_before or middle_after"
            )

    def serve(
        self,
        host: str = "127.0.0.1",
        port: int = 3000,
        secret_key: str | None = None,
        dev: bool = False,
        app_state: Dict[str, Any] = {},
    ) -> None:
        if secret_key is None:
            import secrets

            secret_key = secrets.token_urlsafe(30)
        import web

        web.init_web(self, host, port, secret_key, dev, app_state)
        print("Slime server is shutting down...")
        print("Finished")
