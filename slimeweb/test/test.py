from lib import slime

app = slime.Slime(__file__)


@app.route(path="/", method="GET")
def hello(req, resp):
    # return resp.plain("hello world")
    # resp.set_header("Server", "Slime")
    # resp.set_sign_cookie("hello", "world", "/", req.secret_key)
    return resp.json({"name": "abilash", "age": 24})


if __name__ == "__main__":
    app.serve()
