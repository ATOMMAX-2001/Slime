from lib.slime import Slime

app = Slime(__file__)


@app.route("/", method=["GET", "POST"])
def land(req, resp):
    if req.method == "GET":
        resp.plain("hello")
    else:
        resp.plain("world")


if __name__ == "__main__":
    app.serve(dev=True)
