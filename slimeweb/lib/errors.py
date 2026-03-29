# AUTHOR: S.ABILASH
# Email: abinix01@gmail.com


class MethodException(Exception):
    def __init__(self, error: str) -> None:
        super().__init__(error)


class MultipleMiddlewareException(Exception):
    def __init__(self, error: str) -> None:
        super().__init__(error)


class InvalidMiddlewareHandlerType(Exception):
    def __init__(self, error: str) -> None:
        super().__init__(error)


class RouteHandlerNotFoundException(Exception):
    def __init__(self, error: str) -> None:
        super().__init__(error)


class InvalidHandler(Exception):
    def __init__(self, error: str) -> None:
        super().__init__(error)


class MultipleRouteException(Exception):
    def __init__(self, error: str) -> None:
        super().__init__(error)
