from lib import slime

app = slime.Slime(__file__)


@app.route(path="/", method="GET")
def hello(req, resp):
    # print("query", req.query)
    # print("params", req.params)
    # print("body", req.body)
    # print("json", req.json)
    # print("form", req.form)
    # print("text", req.text)
    # print("bytes", req.bytes)
    # print("*" * 10)
    # for i in ["abilash", "abi", "atom"]:
    html = req.render("hello.html", **{"name": "abi", "age": 24})
    return resp.html(html)


# return resp.json({"name": "abilash", "age": 24})


if __name__ == "__main__":
    app.serve()
