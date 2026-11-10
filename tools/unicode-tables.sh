#!/bin/sh
# Regenerates crates/apple-text/src/tables/{icu,folding,swift}.rs from this Mac's ICU, Swift
# runtime and Foundation, which are what the Mac app's formatter ran on. Run it on macOS when a new
# OS changes their Unicode data, then run the Mac app's differential tests.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
clang -O2 "$root/tools/unicode/icu.c" -licucore -o "$work/icu"
swiftc -O "$root/tools/unicode/swift.swift" -o "$work/swift"
"$work/icu" > "$work/icu.txt"
"$work/swift" > "$work/swift.txt"
python3 -I "$root/tools/unicode/tables.py" "$work/icu.txt" "$work/swift.txt" \
    "$root/crates/apple-text/src/tables"
cd "$root" && cargo fmt --all
