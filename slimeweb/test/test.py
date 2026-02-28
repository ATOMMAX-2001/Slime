from lib import slime

app = slime.Slime(__file__)


@app.route(path="/", method="GET")
def hello(req, resp):
    html = req.render("hello.html", **{"name": "abilash"})
    return resp.html(html)
    # return resp.json({"name": "abilash", "age": 24})


if __name__ == "__main__":
    app.serve()
