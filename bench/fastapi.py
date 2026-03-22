import asyncio

from fastapi.responses import (
    HTMLResponse,
    JSONResponse,
    PlainTextResponse,
    StreamingResponse,
)
from fastapi.templating import Jinja2Templates

from fastapi import FastAPI, Request

app = FastAPI()
templates = Jinja2Templates(directory="templates")
templates.env.auto_reload = False
# initialize counter in app state
# app.state.counter = 0


@app.get("/plain", response_class=PlainTextResponse)
def land_plain():
    return "hello world"


@app.get("/json", response_class=JSONResponse)
def land_json():
    return {"name": "abilash", "fastversion": "V0.135.1"}


@app.get("/html", response_class=HTMLResponse)
def land_html(request: Request):
    return templates.TemplateResponse(
        "hello.html", {"request": request, "name": "abilash", "age": 24}
    )


@app.get("/stream")
def land_stream():
    def generate_stream():
        for i in range(5):
            yield f"{i}"

    return StreamingResponse(generate_stream(), media_type="text/plain")


# @app.get("/", response_class=HTMLResponse)
# def land(request: Request):

#     # get current counter
#     counter = app.state.counter

#     # render template
#     html = templates.TemplateResponse(
#         "hello.html",
#         {"request": request, "name": "abilash", "age": 24, "counter": counter}
#     )

#     # update counter
#     app.state.counter += 1

#     return html
