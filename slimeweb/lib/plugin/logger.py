import logging
from typing import Literal

from ..slime import SlimeMiddleware


class ReqLog(SlimeMiddleware):
    def __init__(
        self,
        log_at: Literal["before", "after", "both"] = "before",
        log_kind: Literal["file", "stream"] = "file",
    ) -> None:
        if log_at not in ["before", "after", "both"]:
            raise ValueError(
                "logger kind should be one of the list [before,after,both]"
            )

    def middle_before(self, req, resp):
        pass

    def middle_after(self, req, resp):
        pass
