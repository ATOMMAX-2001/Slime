from typing import Callable, Dict


class Routes:
    def __init__(self, path: str = "/", method: str = "GET") -> None:
        if path is None or method is None:
            raise ValueError("Path and Method should not be empty")
        if method.upper() not in ["GET", "POST", "PATCH", "PUT", "DELETE", "OPTIONS"]:
            raise ValueError(f"{method} is not Valid")

        self.path: str = path
        self.method: str = method

    def __str__(self) -> str:
        return f"""
            Path: {self.path}
            Method: {self.method}
        """


class Slime:
    def __init__(self, filename: str) -> None:
        if filename is None or not isinstance(filename, str):
            raise ValueError("Need argument as __file__ ")

        # for templating purpose we can fetch file path with /template
        self.__filename: str | None = filename

        # => you can multiple or same path with different request
        # like let saay user can assign  a path /name
        # with methods like GET,POST under /name
        # => for each handler there can be only one method
        self.__routes: Dict[Routes, Callable] = {}

    def route(
        self,
        path: str = "/",
        method: str = "GET",
        handler_view: Callable | None = None,
    ) -> Callable:
        def wrapper(route_handler) -> Callable:
            if handler_view is None or not callable(handler_view):
                raise ValueError(
                    f"View handler should be a function for [Path: {path}, Method: {method}]"
                )

            self.__routes[Routes(path, method)] = handler_view
            return handler_view

        return wrapper

    def serve(self, host: str = "localhost", port: int = 3000) -> None:
        print(f"Slime Server is running at {host}:{port}")
