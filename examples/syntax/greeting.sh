#!/usr/bin/env bash
# Display-only syntax fixture; never executed by tooling.
name="reader"
for index in 1 2 3; do
  printf 'Hello, %s (%s)\n' "$name" "$index"
done
