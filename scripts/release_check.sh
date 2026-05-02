#!/usr/bin/env sh
set -eu

BIN=${NYANCAT_BIN:-target/release/nyancat}
TMP=${TMPDIR:-/tmp}

normal_out="$TMP/nyancat-rust-smoke.out"
telnet_out="$TMP/nyancat-rust-telnet-smoke.out"
truecolor_out="$TMP/nyancat-rust-truecolor-smoke.out"
crop_out="$TMP/nyancat-rust-crop-smoke.out"
benchmark_out="$TMP/nyancat-rust-benchmark-smoke.out"
benchmark_err="$TMP/nyancat-rust-benchmark-smoke.err"
cli_err="$TMP/nyancat-rust-cli-error.err"
help_out="$TMP/nyancat-rust-help.out"

check_bytes() {
    file=$1
    expected=$2
    actual=$(wc -c < "$file" | tr -d ' ')
    if [ "$actual" != "$expected" ]; then
        echo "byte count mismatch for $file: expected $expected, got $actual" >&2
        exit 1
    fi
}

echo "== cargo fmt --check =="
cargo fmt --check

echo "== cargo test =="
cargo test

echo "== cargo clippy =="
cargo clippy --all-targets --all-features -- -D warnings

echo "== cargo build --release =="
cargo build --release

echo "== smoke tests =="
env TERM=xterm-256color "$BIN" --frames 1 --no-title --no-clear --no-counter > "$normal_out"
"$BIN" --telnet --skip-intro --frames 1 --no-title --no-clear --no-counter > "$telnet_out"
env TERM=xterm-256color "$BIN" --truecolor --frames 1 --no-title --no-clear --no-counter > "$truecolor_out"
env TERM=xterm-256color "$BIN" --frames 1 --width 40 --height 24 --no-title --no-clear --no-counter > "$crop_out"
env TERM=xterm-256color "$BIN" --benchmark --frames 3 --no-title --no-clear --no-counter > "$benchmark_out" 2> "$benchmark_err"
"$BIN" --help > "$help_out"

check_bytes "$normal_out" 4002
check_bytes "$telnet_out" 3067
check_bytes "$truecolor_out" 5175
check_bytes "$crop_out" 4083
check_bytes "$benchmark_out" 11916

if "$BIN" --wat > "$cli_err" 2>&1; then
    echo "expected CLI error smoke to fail" >&2
    exit 1
fi

grep -F "nyancat: unknown option: --wat" "$cli_err" > /dev/null
grep -F "Try '$BIN --help' for usage." "$cli_err" > /dev/null
grep -F "benchmark: frames=3" "$benchmark_err" > /dev/null
grep -F "usage: $BIN" "$help_out" > /dev/null
grep -F -- "-i, --intro" "$help_out" > /dev/null
grep -F -- "-I, --skip-intro" "$help_out" > /dev/null
grep -F -- "-t, --telnet" "$help_out" > /dev/null
grep -F -- "-T, --truecolor" "$help_out" > /dev/null
grep -F -- "-n, --no-counter" "$help_out" > /dev/null
grep -F -- "-s, --no-title" "$help_out" > /dev/null
grep -F -- "-e, --no-clear" "$help_out" > /dev/null
grep -F -- "-b, --benchmark" "$help_out" > /dev/null
grep -F -- "-d, --delay <ms>" "$help_out" > /dev/null
grep -F -- "-f, --frames <frames>" "$help_out" > /dev/null
grep -F -- "-r, --min-rows <row>" "$help_out" > /dev/null
grep -F -- "-R, --max-rows <row>" "$help_out" > /dev/null
grep -F -- "-c, --min-cols <col>" "$help_out" > /dev/null
grep -F -- "-C, --max-cols <col>" "$help_out" > /dev/null
grep -F -- "-W, --width <width>" "$help_out" > /dev/null
grep -F -- "-H, --height <height>" "$help_out" > /dev/null
grep -F -- "-h, --help" "$help_out" > /dev/null

echo "release check passed"
