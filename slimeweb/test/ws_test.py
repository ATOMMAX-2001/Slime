import asyncio

import websockets


async def test():
    uri = "ws://localhost:3000/chat"
    async with websockets.connect(uri) as ws:
        await ws.send(b"hello")
        resp = await ws.recv()
        print(resp)

        await ws.send("hello world")
        resp = await ws.recv()
        print(resp)


asyncio.run(test())
