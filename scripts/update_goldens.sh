#!/usr/bin/env sh
set -eu

# Regenerates the committed smoke-output golden files that scripts/release_check.sh
# compares against. Run this when a render/output change is intentional, then review
# the resulting git diff under tests/golden/ to confirm the byte changes are expected.

GOLDEN_DIR=${GOLDEN_DIR:-tests/golden}

if [ -z "${NYANCAT_BIN:-}" ]; then
    cargo build --release --locked
    BIN=target/release/nyancat
else
    BIN=$NYANCAT_BIN
    if [ ! -x "$BIN" ]; then
        echo "NYANCAT_BIN is not executable: $BIN" >&2
        exit 1
    fi
fi

mkdir -p "$GOLDEN_DIR"

env TERM=xterm-256color "$BIN" --frames 1 --no-title --no-clear --no-counter > "$GOLDEN_DIR/normal.out"
"$BIN" --telnet --skip-intro --frames 1 --no-title --no-clear --no-counter > "$GOLDEN_DIR/telnet.out"
env TERM=xterm-256color "$BIN" --truecolor --frames 1 --no-title --no-clear --no-counter > "$GOLDEN_DIR/truecolor.out"
env TERM=xterm-256color "$BIN" --frames 1 --width 40 --height 24 --no-title --no-clear --no-counter > "$GOLDEN_DIR/crop.out"
env TERM=xterm-256color "$BIN" --benchmark --frames 3 --no-title --no-clear --no-counter > "$GOLDEN_DIR/benchmark.out" 2>/dev/null

echo "updated goldens in $GOLDEN_DIR"
