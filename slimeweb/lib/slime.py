from typing import Callable, Dict


class Routes:
    def __init__(self, path: str = "/", method: str = "GET") -> None:
        if path is None or method is None:
            raise ValueError("Path and Method should not be empty")
        if method.upper() not in ["GET", "POST", "PATCH", "PUT", "DELETE", "OPTIONS"]:
            raise ValueError(f"{method} is not Valid")

        self.path: str = path
        self.method: str = method

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
    ) -> Callable:
        def wrapper(route_handler) -> Callable:
            if route_handler is None or not callable(route_handler):
                raise ValueError(
                    f"View handler should be a function for [Path: {path}, Method: {method}]"
                )

            self.__routes[Routes(path, method)] = route_handler
            return route_handler

        return wrapper

    def _get_routes(self) -> Dict[Routes, Callable]:
        return self.__routes

    def serve(self, host: str = "127.0.0.1", port: int = 3000) -> None:
        import web

        web.init_web(self, host, port)
        print("Slime server is shutting down...")
        print("Finished")
