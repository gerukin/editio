@echo off
rem A batch-file syntax sample; never run by tests.
set NAME=reader
if defined NAME (
    echo Hello, %NAME%!
)
