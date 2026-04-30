#!/usr/bin/env python3
"""Run cargo test and capture output"""
import subprocess

result = subprocess.run(
    ["cargo", "test", "-p", "rez-next-version", "--lib", "range::tests"],
    capture_output=True,
    text=True
)

print("STDOUT:")
print(result.stdout[-3000:] if len(result.stdout) > 3000 else result.stdout)
print("\nSTDERR:")
print(result.stderr[-2000:] if len(result.stderr) > 2000 else result.stderr)
print(f"\nReturn code: {result.returncode}")
