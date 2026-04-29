from granian import Granian


def init_web(slime_obj):

    server = Granian(
        target="test.test:app",
        address="127.0.0.1",
        port=3000,
        workers=8,
        interface="asgi",
    )
    server.serve()
