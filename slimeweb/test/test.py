from lib import slime

app = slime.Slime(__file__)


@app.route(path="/", method="GET")
def hello():
    return "Hello World"


if __name__ == "__main__":
    app.serve()
