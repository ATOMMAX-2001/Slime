import logging
import os
import uuid
from logging.handlers import TimedRotatingFileHandler
from typing import Literal

from ..lib import SlimeMiddleware


class ReqLog(SlimeMiddleware):
    def __init__(
        self,
        log_kind: Literal["file", "stream"] = "stream",
    ) -> None:
        if log_kind not in ["file", "stream"]:
            raise ValueError("There is only 2 type of logger <File> or <Stream>")
        formatter = logging.Formatter("%(asctime)s - %(levelname)s - %(message)s")
        self.logger = logging.getLogger(f"SlimeLog_{uuid.uuid4()}")
        self.logger.propagate = False
        self.logger.setLevel(logging.INFO)
        if log_kind == "file":
            os.makedirs("logs", exist_ok=True)
            file_handler = TimedRotatingFileHandler(
                "logs/slimerequest.log", when="midnight", interval=1, backupCount=0
            )
            file_handler.setFormatter(formatter)
            file_handler.setLevel(logging.INFO)
            self.logger.addHandler(file_handler)
        else:
            stream_handler = logging.StreamHandler()
            stream_handler.setFormatter(formatter)
            stream_handler.setLevel(logging.INFO)
            self.logger.addHandler(stream_handler)

    def middle_before(self, req, resp):
        pass

    def middle_after(self, req, resp):
        message = f"{req.client} {req.path} {req.method} => {resp.status}"
        self.logger.info(message)
