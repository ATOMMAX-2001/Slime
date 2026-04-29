import asyncio

import websockets


async def run(uri):
    async with websockets.connect(uri) as ws:
        pong = await ws.ping(b"Hello")
        print(await pong)

        await ws.send(b"hello")
        resp = await ws.recv()
        print(resp)
        await ws.send("hello world")
        resp = await ws.recv()
        print(resp)


async def test():
    uri = "ws://localhost:8080/ws"
    try:
        await run(uri)
    except:
        uri = "ws://localhost:3000/chat"
        await run(uri)


asyncio.run(test())
