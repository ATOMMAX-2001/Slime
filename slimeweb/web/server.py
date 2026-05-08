import sys

from granian import Granian


class SlimeRequest:
    def __init__(self) -> None:
        pass


class SlimeResponse:
    def __init__(self, send) -> None:
        self.__build_header_response = {
            "type": "http.response.start",
            "status": 200,
            "headers": [(b"Server", b"Slime")],
        }
        self.__asgi_send = send

    def set_status(self, status: int) -> None:
        self.__build_header_response["status"] = status

    def add_header(self, key: str, value: str) -> None:
        self.__build_header_response["headers"].append(
            (key.encode("utf-8"), value.encode("utf-8"))
        )

    async def plain(self, body: str) -> None:
        await self.__asgi_send(self.__build_header_response)
        await self.__asgi_send(
            {
                "type": "http.response.body",
                "body": body.encode("utf-8"),
            }
        )


async def parse_request(scope, receive, send, slime_obj):
    if scope["type"] == "http":
        handlers = None
        for key, value in slime_obj._Slime__routes.items():
            if key.path == scope["path"]:
                handlers = value
        if handlers is not None:
            response = SlimeResponse(send)
            for handler in handlers["handler"]:
                await handler[0](SlimeRequest(), response)
    else:
        print(scope["type"])
        print("Yet to implement")


def compile_route_request(slime_obj):
    for key, value in slime_obj._Slime__routes.items():

def init_web(slime_obj, host: str, port: int, workers: int):
    import inspect

    import __main__

    slime_object_name = None
    for name, obj in inspect.currentframe().f_back.f_back.f_locals.items():  # type: ignore
        if obj is slime_obj:
            slime_object_name = name
            break

    module_name = __main__.__spec__.name
    if slime_object_name is None or module_name is None:
        raise ValueError("Cant able to find the slime object name")
    route_container = compile_route_request(slime_obj)
    server = Granian(
        target=f"{module_name}:{slime_object_name}",
        address=host,
        port=port,
        workers=workers,
        interface="asgi",  # type: ignore
    )
    server.serve()
