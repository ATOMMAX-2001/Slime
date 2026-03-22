from flask import Flask, Response, jsonify, render_template

app = Flask(__name__)


@app.route("/plain")
def land_plain():
    return "hello world", 200, {"Content-Type": "text/plain"}


@app.route("/json")
def land_json():
    return jsonify({"name": "abilash", "fastversion": "V0.135.1"})


@app.route("/html")
def land_html():
    return render_template("hello.html", name="abilash", age=24)


@app.route("/stream")
def land_stream():
    def generate_stream():
        for i in range(5):
            yield f"{i}"

    return Response(generate_stream(), mimetype="text/plain")
