import asyncio

from lib import slime
from pydantic import BaseModel

app = slime.Slime(__file__)


class Student(BaseModel):
    name: str
    age: int
    marks: int


class StudError(slime.SlimeException):
    def __init__(self, status=400, message="An unknown error was occurred") -> None:
        super().__init__(status, message)


class SampleMiddle(slime.SlimeMiddleware):
    def middle_before(self, req, resp):
        resp.set_header("plugin_before", "works")

    def middle_after(self, req, resp):
        resp.set_header("plugin_after", "works")


# @app.websocket(path="/chat", method="GET")
# async def chatty(req, resp):
#     await asyncio.sleep(1)

#     def read_me(msg):
#         if not resp.is_closed():
#             resp.send(msg)

#     resp.on_message(read_me)

#     def close_me():
#         print("closed")

#     def error_me(mes):
#         print(mes)

#     resp.on_error(error_me)
#     resp.on_close(close_me)


@app.route(path="/plain", method="GET")
async def land_plain(req, resp):
    return resp.plain("ok")


@app.route(path="/plain", method="POST")
def land_plain_post(req, resp):
    return resp.plain("hello world from post")


# @app.route(path="/json", method="GET")
# def land_json(req, resp):
#     return resp.json({"name": "abilash", "slimeversion": "V0.1.3"})


# @app.route(path="/render", method="GET")
# def land_render(req, resp):
#     html = req.render(
#         "hello.html",
#         **{"name": "abilash", "age": 24},
#     )
#     return resp.html(html)


# @app.route(path="/", method=["GET", "POST"])
# def land(req, resp):
#     req.validate(Student)
#     counter = req.get_state("counter")
#     html = req.render(
#         "hello.html", **{"name": "abilash", "age": 24, "counter": counter}
#     )
#     req.update_state("counter", counter + 1)
#     return resp.html(html)


# # @app.middle_after(path="/", method="GET")
# # async def land_after(req, resp):
# #     resp.set_header("BEFORE", "Request")


# # @app.middle_before(path="/", method="GET")
# # async def land_before(req, resp):
# #     resp.set_header("AFTER", "REQUEST")


# @app.route(path="/stream", method="GET", stream="text/plain")
# async def stream_me(req, resp):
#     await asyncio.sleep(1)
#     resp.start_stream()
#     for i in range(5):
#         resp.send(i)
#     resp.close()


# @app.route(path="/test", method="POST")
# def hello(req, resp):
#     # print("query", req.query)
#     # print("params", req.params)
#     # print("body", req.body)
#     # print("json", req.json)
#     # print("form", req.form)
#     # print("text", req.text)
#     # print("bytes", req.bytes)
#     # print("file", req.file)
#     # print("*" * 10)
#     for file in req.file:
#         print(file.filename)
#         print(file.content_type)
#         print(file.file_path)
#         print(file.file_size)
#         print(file.extension)
#         file.save(f"./testing_file.{file.extension}")
#     return resp.json({"status": "ok"})
#     # html = req.render("hello.html", **{"name": "abi", "age": 24})
#     # return resp.html(html)


# @app.route("/upload", method="POST", body_size=1024 * 1024 * 30)
# def upload_test(req, resp):
#     result = len(req.body)
#     return resp.plain(str(result))


# @app.start()
# def start_app():
#     print("app has been started")


# @app.end()
# def end_app(args):
#     print("app has been ended")


if __name__ == "__main__":
    app.use(SampleMiddle())

    app.serve(app_state={"counter": 0}, dev=True)
