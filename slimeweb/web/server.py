from granian import Granian

from slimeweb.lib.slime import Slime


class SlimeRequest:
    def __init__(self) -> None:
        pass


class SlimeResponse:
    def __init__(self, send) -> None:
        self.__build_response = {
            "type": "http.response.start",
            "status": 200,
            "headers": [(b"Server", b"Slime")],
        }
        self.__asgi_send = send

    def set_status(self, status: int) -> None:
        self.__build_response["status"] = status

    def add_header(self, key: str, value: str) -> None:
        self.__build_response["headers"].append(
            (key.encode("utf-8"), value.encode("utf-8"))
        )

    async def plain(self, body: str) -> None:
        await self.__asgi_send(self.__build_response)
        await self.__asgi_send(
            {
                "type": "http.response.body",
                "body": body.encode("utf-8"),
            }
        )


async def parse_request(scope, receive, send, slime_obj):
    if scope["type"] == "http":
        handlers = None
        for key, value in slime_obj._get_routes().items():
            if key.path == scope["path"]:
                handlers = value
        if handlers is not None:
            response = SlimeResponse(send)
            for handler in handlers["handler"]:
                await handler[0](SlimeRequest(), response)
    else:
        print(scope["type"])
        print("Yet to implement")
        pass


def init_web(slime_obj):
    server = Granian(
        target="test.check:app",
        address="127.0.0.1",
        port=3000,
        workers=1,
        interface="asgi",
    )
    server.serve()
