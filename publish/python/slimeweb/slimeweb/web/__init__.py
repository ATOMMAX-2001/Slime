from .web import *

__doc__ = web.__doc__
if hasattr(web, "__all__"):
    __all__ = web.__all__