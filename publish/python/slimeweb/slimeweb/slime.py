# AUTHOR: S.ABILASH
# Email: abinix01@gmail.com

import inspect
from typing import Callable, Dict, Tuple


class Routes:
    def __init__(
        self,
        path: str = "/",
        method: str = "GET",
        stream: str | None = None,
        ws: bool = False,
    ) -> None:
        if method.upper() not in ["GET", "POST", "PATCH", "PUT", "DELETE", "OPTIONS"]:
            raise ValueError(f"{method} is not Valid")

        self.path: str = path
        self.method: str = method
        self.stream: str | None = stream
        self.ws: bool = ws

    def __hash__(self) -> int:
        return hash((self.path, self.method, self.stream))

    def __eq__(self, value: object) -> bool:
        return (
            isinstance(value, Routes)
            and self.path == value.path
            and self.method == value.method
            and self.stream == value.stream
            and self.ws == value.ws
        )

    def __str__(self) -> str:
        return f"""
            Path: {self.path}
            Method: {self.method}
            {f"Stream: {self.stream}" if self.stream is not None else ""}
            {f"Websocket: {self.ws}" if self.ws is not None else ""}
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
        # => for each handler there can be only one method
        self.__routes: Dict[Routes, Dict[str, Tuple[Callable, bool] | None]] = {}

    def middle_before(self, path: str = "/", method: str = "GET") -> Callable:
        def wrapper(middle_handler) -> Callable:
            if middle_handler is None or not callable(middle_handler):
                raise ValueError(
                    f"Middleware handler should be a function for [Path: {path}, Method: {method}]"
                )
            found: bool = False
            if path == "*":
                for route in self.__routes:
                    call_handler = self.__routes.get(route)
                    if call_handler is not None:
                        call_handler["before"] = (
                            middle_handler,
                            inspect.iscoroutinefunction(middle_handler),
                        )
            else:
                is_async = inspect.iscoroutinefunction(middle_handler)
                for route in self.__routes:
                    if route.path == path and route.method == method:
                        call_handler = self.__routes.get(route)
                        if (
                            call_handler is not None
                            and call_handler["handler"] is not None
                        ):
                            if call_handler["handler"][1] == is_async:
                                if call_handler["before"] is None:
                                    call_handler["before"] = (
                                        middle_handler,
                                        is_async,
                                    )
                                    found = True
                                    break
                                else:
                                    error = f"Multiple middle before definition found for same Path: {path}, method: {method}"
                                    raise ValueError(error)
                            else:
                                error = f"Middle before handler should be of {'async' if call_handler['handler'][1] else 'sync'} type similar to route handler"
                                raise ValueError(error)
                if not found:
                    raise ValueError(
                        "You need to define the request handler to declare middleware"
                    )
            return middle_handler

        return wrapper

    def middle_after(self, path: str = "/", method: str = "GET") -> Callable:
        def wrapper(middle_handler) -> Callable:
            if middle_handler is None or not callable(middle_handler):
                raise ValueError(
                    f"Middleware handler should be a function for [Path: {path}, Method: {method}]"
                )

            found: bool = False
            if path == "*":
                for route in self.__routes:
                    call_handler = self.__routes.get(route)
                    if call_handler is not None:
                        call_handler["after"] = (
                            middle_handler,
                            inspect.iscoroutinefunction(middle_handler),
                        )
            else:
                is_async = inspect.iscoroutinefunction(middle_handler)
                for route in self.__routes:
                    if route.path == path and route.method == method:
                        call_handler = self.__routes.get(route)
                        if (
                            call_handler is not None
                            and call_handler["handler"] is not None
                        ):
                            if call_handler["handler"][1] == is_async:
                                if call_handler["after"] is None:
                                    call_handler["after"] = (
                                        middle_handler,
                                        is_async,
                                    )
                                    found = True
                                    break
                                else:
                                    error = f"Multiple middle after definition found for same Path: {path}, method: {method}"
                                    raise ValueError(error)
                            else:
                                error = f"Middle after handler should be of {'async' if call_handler['handler'][1] else 'sync'} type similar to route handler"
                                raise ValueError(error)
                if not found:
                    raise ValueError(
                        "You need to define the request handler to declare middleware"
                    )
            return middle_handler

        return wrapper

    def route(
        self,
        path: str = "/",
        method: str = "GET",
        stream: str | None = None,
        ws: bool = False,
    ) -> Callable:
        def wrapper(route_handler) -> Callable:
            if route_handler is None or not callable(route_handler):
                raise ValueError(
                    f"Route handler should be a function for [Path: {path}, Method: {method}]"
                )
            self.__routes[Routes(path, method, stream, ws)] = {
                "handler": (route_handler, inspect.iscoroutinefunction(route_handler)),
                "before": None,
                "after": None,
            }
            return route_handler

        return wrapper

    def stream(
        self, path: str = "/", method: str = "GET", content: str = "text/plain"
    ) -> Callable:
        def wrapper(stream_handler) -> Callable:
            if stream_handler is None or not callable(stream_handler):
                raise ValueError(
                    f"Stream handler should be a function for [Path: {path}, Method: {method}]"
                )
            if not isinstance(content, str):
                raise ValueError(
                    f"Stream content type should be of type <String> with MIME for [Path: {path}, Method: {method}]"
                )
            self.__routes[Routes(path, method, stream=content)] = {
                "handler": (
                    stream_handler,
                    inspect.iscoroutinefunction(stream_handler),
                ),
                "before": None,
                "after": None,
            }
            return stream_handler

        return wrapper

    def websocket(self, path: str = "/", method: str = "GET") -> Callable:
        def wrapper(websocket_handler) -> Callable:
            if websocket_handler is None or not callable(websocket_handler):
                raise ValueError(
                    f"Websocket handler should be a function for [Path: {path}, Method: {method}]"
                )
            self.__routes[Routes(path, method, stream=None, ws=True)] = {
                "handler": (
                    websocket_handler,
                    inspect.iscoroutinefunction(websocket_handler),
                ),
                "before": None,
                "after": None,
            }
            return websocket_handler

        return wrapper

    def _get_routes(self) -> Dict[Routes, Dict[str, Tuple[Callable, bool] | None]]:
        return self.__routes

    def serve(
        self,
        host: str = "127.0.0.1",
        port: int = 3000,
        secret_key: str | None = None,
        dev: bool = False,
    ) -> None:
        if secret_key is None:
            import secrets

            secret_key = secrets.token_urlsafe(30)

        from . import web

        web.init_web(self, host, port, secret_key, dev)
        print("Slime server is shutting down...")
        print("Finished")
