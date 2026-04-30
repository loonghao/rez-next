#!/usr/bin/env python3
"""Run cargo test and capture output"""
import subprocess

result = subprocess.run(
    ["cargo", "test", "-p", "rez-next-version", "--lib", "test_range_"],
    capture_output=True,
    text=True
)

print("STDOUT:")
print(result.stdout)
print("\nSTDERR:")
print(result.stderr)
print(f"\nReturn code: {result.returncode}")
