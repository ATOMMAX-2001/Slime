import pydantic

import web


def validate_me(self, obj, raise_err=True):
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
