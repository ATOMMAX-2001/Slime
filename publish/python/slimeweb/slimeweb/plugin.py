# AUTHOR: S.ABILASH
# Email: abinix01@gmail.com


from typing import List

from .slime import SlimeMiddleware


class Cors(SlimeMiddleware):
    def __init__(
        self,
        origins: List[str] | str = "*",
        methods: List[str] = [
            "GET",
            "POST",
            "PATCH",
            "PUT",
            "DELETE",
            "OPTIONS",
            "HEAD",
        ],
        headers: List[str] = [],
        enable_cred: bool = False,
        age: int = 86400,
    ) -> None:
        if origins == "*" and enable_cred:
            raise ValueError("You cant allow all origin when cred is true")
        if enable_cred not in [True, False]:
            raise ValueError("enable_cred is a bool type")
        if not isinstance(methods, list):
            raise ValueError("methods is a list type")
        if not isinstance(origins, list) and not isinstance(origins, str):
            raise ValueError("origins is a list type")
        if not isinstance(headers, list):
            raise ValueError("headers is a list type")

        if not isinstance(age, int):
            raise ValueError("age is a int type represent in seconds")

        self.origins = origins
        self.methods = methods
        self.headers = headers
        self.headers.append("Content-Type")
        self.cred = enable_cred
        self.age = age

    def __apply_cors(self, req, resp):
        origin_req = req.header.get("origin")
        if origin_req is None:
            return
        if origin_req in self.origins:
            resp.set_header("Access-Control-Allow-Origin", origin_req)
        elif self.origins == "*":
            resp.set_header("Access-Control-Allow-Origin", "*")
        resp.set_header("Access-Control-Allow-Methods", ", ".join(self.methods))
        resp.set_header("Access-Control-Allow-Headers", ", ".join(self.headers))
        if self.cred:
            resp.set_header("Access-Control-Allow-Credentials", "true")
        resp.set_header("Vary", "Origin")
        resp.set_header("Access-Control-Max-Age", str(self.age))

    def middle_before(self, req, resp):
        self.__apply_cors(req, resp)

    def middle_after(self, req, resp):
        self.__apply_cors(req, resp)
