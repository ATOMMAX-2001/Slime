from lib import slime

app = slime.Slime(__file__)


@app.route(path="/", method="GET")
def hello(req):
    print(req.method)
    print(req.path)
    print(req.header)
    print(req.body)
    return "Hello World"


if __name__ == "__main__":
    app.serve()
