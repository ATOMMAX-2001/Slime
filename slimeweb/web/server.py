from granian import Granian


async def parse_request(scope, receive, send):
    if scope["type"] != "http":
        return
    await send(
        {
            "type": "http.response.start",
            "status": 200,
            "headers": [(b"content-type", b"text/plain")],
        }
    )

    await send(
        {
            "type": "http.response.body",
            "body": b"Hello from Slimes",
        }
    )


def init_web(slime_obj):

    server = Granian(
        target="test.test:app",
        address="127.0.0.1",
        port=3000,
        workers=1,
        interface="asgi",
    )
    server.serve()
