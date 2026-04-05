from lib.slime import (
    BodySchema,
    QuerySchema,
    Slime,
    SlimeCompression,
    SlimeResponseType,
    SlimeSchema,
)

app = Slime(__file__)


class SubItem:
    is_item: bool
    how_long: int


class User:
    name: str
    age: int
    sub: dict[str, SubItem]


@app.docs(
    title="just checking",
    description="Simple landing page",
    response_type=SlimeResponseType.PlainResponse,
    schema=SlimeSchema(
        body=BodySchema(schema_name=User), query=[QuerySchema(name="name", type=str)]
    ),
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
