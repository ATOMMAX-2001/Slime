import asyncio

from lib import slime

app = slime.Slime(__file__)


@app.websocket(path="/chat", method="GET")
def chatty(req, resp):

    def read_me(msg):
        if not resp.is_closed():
            resp.send(msg)

    resp.on_message(read_me)

    def close_me():
        pass

    resp.on_close(close_me)


@app.route(path="/", method="GET")
async def land(req, resp):
    await asyncio.sleep(1)
    html = req.render("hello.html", **{"name": "abilash", "age": 24})
    return resp.html(html)


# @app.middle_after(path="/", method="GET")
# async def land_after(req, resp):
#     resp.set_header("BEFORE", "Request")


# @app.middle_before(path="/", method="GET")
# async def land_before(req, resp):
#     resp.set_header("AFTER", "REQUEST")


@app.route(path="/stream", method="GET", stream="text/plain")
async def stream_me(req, resp):
    resp.start_stream()
    for i in range(5):
        resp.send(i)
    resp.close()


@app.route(path="/test", method="POST")
def hello(req, resp):
    # print("query", req.query)
    # print("params", req.params)
    # print("body", req.body)
    # print("json", req.json)
    # print("form", req.form)
    # print("text", req.text)
    # print("bytes", req.bytes)
    # print("file",req.file)
    # print("*" * 10)
    # for i in ["abilash", "abi", "atom"]:
    file = req.file[0]
    print(file.filename)
    print(file.content_type)
    print(file.file_path)
    print(file.file_size)
    print(file.extension)
    file.save(f"testing_file.{file.extension}")
    return resp.json({"status": "ok"})
    # html = req.render("hello.html", **{"name": "abi", "age": 24})
    # return resp.html(html)


# return resp.json({"name": "abilash", "age": 24})


if __name__ == "__main__":
    app.serve()
