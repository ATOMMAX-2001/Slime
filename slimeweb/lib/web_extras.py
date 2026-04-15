# AUTHOR: S.ABILASH
# Email: abinix01@gmail.com


import pydantic

import web

# for prod from .web import web


def validate_me(self, obj: pydantic.BaseModel, raise_err: bool = True):
    try:
        obj.model_validate(self.json)
        return None
    except pydantic.ValidationError as e:
        result = {
            "detail": [
                {
                    "type": err["type"],
                    "loc": list(err["loc"]),
                    "msg": err["msg"],
                    "input": err["input"],
                }
                for err in e.errors()
            ]
        }

        if raise_err:
            raise Exception(result)
        return result


web.SlimeRequest.validate = validate_me  # type: ignore


async def slime_async_pipeline(handlers, req, resp):
    for handler in handlers:
        await handler(req, resp)
