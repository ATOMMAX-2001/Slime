from lib.slime import Slime, SlimeCompression

app = Slime(__file__)


@app.route("/", method="*")
def land(req, resp):
    print(req.header)
    if req.method == "GET":
        resp.plain("hello" * 3000)
    else:
        resp.json({"status": "ok"})


if __name__ == "__main__":
    app.serve(dev=True)
