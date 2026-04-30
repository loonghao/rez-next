@echo off
cd /d g:\PycharmProjects\github\rez-next
cargo test -p rez-next-build --lib > test_output.txt 2>&1
echo Exit code: %ERRORLEVEL% >> test_output.txt
type test_output.txt
