from lib import slime

app = slime.Slime(__file__)


@app.route(path="/", method="GET")
def land(req, resp):
    html = req.render("hello.html", **{"name": "abilash", "age": 24})
    return resp.html(html)


@app.route(path="/stream", method="GET")
def stream_me(req, resp):
    stream_obj = resp.stream("text/plain")
    for i in range(5):
        stream_obj.send(str(i) + "\n")
    # return resp.plain("hello")


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
