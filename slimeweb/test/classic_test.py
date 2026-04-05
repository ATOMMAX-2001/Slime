from lib.slime import Slime, SlimeCompression, SlimeResponseType

app = Slime(__file__)


@app.docs(
    description="Simple landing page", response_type=SlimeResponseType.PlainResponse
)
@app.route("/", method="GET")
def land(req, resp):
    print(req.header)
    if req.method == "GET":
        resp.plain("hello" * 3000)
    else:
        resp.json({"status": "ok"})


@app.route("/home", method="POST")
def check(req, resp):
    pass


@app.middle_before(path="*", method="PUT")
def land_mb(req, resp):
    print("Middle Before working", flush=True)


class SimpleMiddle:
    def middle_after(self, req, resp):
        print("Middle after from class")


if __name__ == "__main__":
    app.use(SimpleMiddle, method=["POST", "GET"])
    app.serve(dev=True)
