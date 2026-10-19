#!/bin/sh
here="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
sys="$(uname -s)"
case "$sys" in
MINGW* | MSYS* | CYGWIN*) rel="x86_64-pc-windows-msvc/mem0.exe" ;;
Darwin*)
    case "$(uname -m)" in
    arm64) rel="aarch64-apple-darwin/mem0" ;;
    *) rel="x86_64-apple-darwin/mem0" ;;
    esac ;;
*)
    case "$(uname -m)" in
    aarch64 | arm64) rel="aarch64-unknown-linux-musl/mem0" ;;
    *) rel="x86_64-unknown-linux-musl/mem0" ;;
    esac ;;
esac
if [ -x "$here/$rel" ]; then exec "$here/$rel" "$@"; fi
for cand in "$here/target/release/mem0.exe" "$here/target/release/mem0" "$here/target/debug/mem0.exe" "$here/target/debug/mem0"; do
    if [ -x "$cand" ]; then exec "$cand" "$@"; fi
done
echo "mem0 binary not found in $here" >&2
exit 1
