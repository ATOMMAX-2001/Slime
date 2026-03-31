from lib.slime import Slime, SlimeCompression

app = Slime(__file__)


@app.route(
    "/",
)
def land(req, resp):
    print(req.header)
    if req.method == "GET":
        resp.plain("hello" * 3000)
    else:
        resp.json({"status": "ok"})


@app.middle_before(path="/", method="GET")
def land_mb(req, resp):
    print("Middle Before working", flush=True)


if __name__ == "__main__":
    for route in app._get_routes():
        print(route)
    # app.serve(dev=True)
