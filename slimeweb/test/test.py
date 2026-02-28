from lib import slime

app = slime.Slime(__file__)


@app.route(path="/", method="POST")
def hello(req, resp):
    print(req.query)
    print(req.params)
    print(req.body)
    # for i in ["abilash", "abi", "atom"]:
    html = req.render("hello.html", **{"name": "abi", "age": 24})
    return resp.html(html)


# return resp.json({"name": "abilash", "age": 24})


if __name__ == "__main__":
    app.serve()
