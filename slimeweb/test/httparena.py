# from slimeweb import Slime
from lib.slime import Slime
import json


app = Slime(__file__)

@app.route("/baseline11",method=["GET","POST"])
def baseline_test(req,resp):
    if req.method == "GET":
        result = 0
        for q_val in req.query.values():
            try:
                result += int(q_val)
            except ValueError: pass
        return resp.plain(str(result))
    else:
        result =0
        for q_val in req.query.values():
            try:
                result +=int(q_val)
            except ValueError: pass
        try:
            result += int(req.text)
        except ValueError: pass
        return resp.plain(str(result))


@app.route("/pipeline",method="GET")
def pipeline_test(req,resp):
    return resp.plain("ok")


@app.route("/upload",method="POST")
def upload_test(req,resp):
    result = len(req.body)
    return resp.plain(str(result))

@app.route("/json",method="GET")
def json_test(req,resp):
    data = load_json_processing_file()



def load_json_processing_file():
    with open("/data/dataset.json","r") as file:
        return json.load(file)


if __name__ == "__main__":
    app.serve(dev=True)
