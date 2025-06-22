name = "cmake"
version = "3.20.0"
description = "CMake cross-platform build system"
authors = ["Kitware Inc."]

requires = []

tools = [
    "cmake",
    "ctest",
    "cpack",
]

def commands():
    import os
    env.PATH.prepend("{root}/bin")
    env.CMAKE_PREFIX_PATH.prepend("{root}")
