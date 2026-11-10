#!/bin/sh
# Builds libawcore.a for both Mac architectures and joins them into one universal library:
#   tools/build-macos.sh <output directory>
# The Mac app links it; its scripts run this before `swift build`.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
out=${1:?usage: tools/build-macos.sh <output directory>}
# The app's minimum, so the linker doesn't warn that the objects need a newer macOS.
export MACOSX_DEPLOYMENT_TARGET=14.0
cd "$root"
mkdir -p "$out"
library="$out/libawcore.a"
stale=false
[ -f "$library" ] || stale=true
set --
for target in aarch64-apple-darwin x86_64-apple-darwin; do
    cargo build --quiet --release --locked -p awcore --target "$target"
    slice="target/$target/release/libawcore.a"
    [ "$slice" -nt "$library" ] && stale=true
    set -- "$@" "$slice"
done
if $stale; then
    lipo -create -output "$library" "$@"
fi
