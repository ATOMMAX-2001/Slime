import pydantic

import web
from slimeweb.lib.slime import SlimeException


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


def send_exception(self, custom_error: SlimeException):
    if not isinstance(custom_error, SlimeException):
        raise ValueError(
            "SlimeException has to be derived and need instance of the custom error"
        )
    if custom_error.message is not None:
        raise ValueError("SlimeException message attribute is undefined")
    return self.json({"status": custom_error.status, "message": custom_error.message})


web.SlimeResponse.exception = send_exception  # type: ignore
