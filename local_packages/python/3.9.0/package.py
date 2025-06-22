name = "python"
version = "3.9.0"
description = "Python programming language interpreter"
authors = ["Python Software Foundation"]

requires = []

tools = [
    "python",
    "pip",
    "python3",
]

def commands():
    import os
    env.PATH.prepend("{root}/bin")
    env.PYTHONPATH.prepend("{root}/lib/python3.9/site-packages")
