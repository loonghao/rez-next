name = "witr"
version = "0.3.4"
description = "Why is this running? Process ancestry and diagnostics"
tools = ["witr"]
variants = [
    ["platform-windows", "arch-x86_64"],
    ["platform-windows", "arch-arm_64"],
    ["platform-linux", "arch-x86_64"],
    ["platform-linux", "arch-arm_64"],
    ["platform-osx", "arch-x86_64"],
    ["platform-osx", "arch-arm_64"],
]


def commands():
    env.PATH.prepend("{root}/payload/bin")
