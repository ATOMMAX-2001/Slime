from slimeweb import Slime, SlimeCompression, SlimeDocs, SlimeMiddleware, SlimeTls

# from slimeweb.plugin.cors import Cors
# from slimeweb.plugin.logger import ReqLog

app = Slime(__file__)


@app.route(path="/plaint", method="GET")
async def land_plaint(req, resp):
    return await resp.plain("ok")


if __name__ == "__main__":
    app.serve(
        app_state={"counter": 0},
        # https=SlimeTls(
        #     cert="../../certs/localhost+1.cert", key="../../certs/localhost+1-key.key"
        # ),
    )
