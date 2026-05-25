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
frames_err="$TMP/nyancat-rust-frames-error.err"
flag_value_err="$TMP/nyancat-rust-flag-value-error.err"
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

check_cksum() {
    file=$1
    expected_crc=$2
    expected_bytes=$3
    label=$4
    set -- $(cksum "$file")
    actual_crc=$1
    actual_bytes=$2
    if [ "$actual_crc" != "$expected_crc" ] || [ "$actual_bytes" != "$expected_bytes" ]; then
        echo "checksum mismatch for $label in $file: expected $expected_crc $expected_bytes, got $actual_crc $actual_bytes" >&2
        exit 1
    fi
}

check_contains() {
    file=$1
    pattern=$2
    label=$3
    if ! LC_ALL=C grep -F "$pattern" "$file" > /dev/null; then
        echo "missing $label in $file" >&2
        exit 1
    fi
}

check_absent() {
    file=$1
    pattern=$2
    label=$3
    if LC_ALL=C grep -F "$pattern" "$file" > /dev/null; then
        echo "unexpected $label in $file" >&2
        exit 1
    fi
}

check_char_count() {
    file=$1
    chars=$2
    expected=$3
    label=$4
    actual=$(LC_ALL=C tr -cd "$chars" < "$file" | wc -c | tr -d ' ')
    if [ "$actual" != "$expected" ]; then
        echo "character count mismatch for $label in $file: expected $expected, got $actual" >&2
        exit 1
    fi
}

echo "== cargo fmt --check =="
cargo fmt --check

echo "== cargo test =="
cargo test --locked

echo "== cargo clippy =="
cargo clippy --locked --all-targets --all-features -- -D warnings

echo "== cargo build --release =="
cargo build --release --locked

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

check_cksum "$normal_out" 3491497212 4002 "normal output"
check_cksum "$telnet_out" 3107447574 3067 "telnet output"
check_cksum "$truecolor_out" 1251626052 5175 "truecolor output"
check_cksum "$crop_out" 1400779159 4083 "crop output"
check_cksum "$benchmark_out" 3251515113 11916 "benchmark output"

esc=$(printf '\033')
telnet_iac_wont_echo=$(printf '\377\374\001')

check_contains "$normal_out" "${esc}[s${esc}[u${esc}[48;5;17m" "xterm frame prefix"
check_contains "$normal_out" "${esc}[48;5;196m" "xterm rainbow red"
check_absent "$normal_out" "${esc}[48;2;" "truecolor escape sequence"
check_absent "$normal_out" "You have nyaned for" "counter text"

check_contains "$truecolor_out" "${esc}[s${esc}[u${esc}[48;2;0;49;105m" "truecolor frame prefix"
check_contains "$truecolor_out" "${esc}[48;2;255;25;0m" "truecolor rainbow red"
check_absent "$truecolor_out" "${esc}[48;5;" "256-color escape sequence"

check_contains "$crop_out" "${esc}[s${esc}[u${esc}[48;5;17m" "cropped frame prefix"
check_absent "$benchmark_out" "You have nyaned for" "benchmark counter text"

check_contains "$telnet_out" "$telnet_iac_wont_echo" "telnet negotiation prefix"
check_contains "$telnet_out" "${esc}[s${esc}[u${esc}[104m" "telnet ANSI frame prefix"
check_char_count "$normal_out" '\000' 0 "normal NUL bytes"
check_char_count "$telnet_out" '\000' 23 "telnet NUL newline bytes"
check_char_count "$telnet_out" '\015' 23 "telnet CR newline bytes"

if "$BIN" --wat > "$cli_err" 2>&1; then
    echo "expected CLI error smoke to fail" >&2
    exit 1
fi

if "$BIN" --frames=-1 > "$frames_err" 2>&1; then
    echo "expected negative frame count smoke to fail" >&2
    exit 1
fi

if "$BIN" --no-counter=false > "$flag_value_err" 2>&1; then
    echo "expected flag value smoke to fail" >&2
    exit 1
fi

grep -F "nyancat: unknown option: --wat" "$cli_err" > /dev/null
grep -F "Try '$BIN --help' for usage." "$cli_err" > /dev/null
grep -F "nyancat: value for --frames must be positive: -1" "$frames_err" > /dev/null
grep -F "nyancat: unexpected value for --no-counter: false" "$flag_value_err" > /dev/null
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
