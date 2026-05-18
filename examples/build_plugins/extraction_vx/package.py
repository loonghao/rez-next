"""Example Rez package using rez_next.build_plugins.ExtractionBuilder."""

name = "vx"
version = "0.0.1"
description = "Example binary extraction package"
tools = ["vx"]

tests = {
    "run": "vx",
}


def commands():
    env.PATH.prepend("{root}/bin")
