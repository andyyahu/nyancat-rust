#!/usr/bin/env sh
set -eu

BIN=${NYANCAT_BIN:-target/release/nyancat}
FRAMES=${1:-100000}

if [ ! -x "$BIN" ]; then
    cargo build --release --locked
fi

echo "# Benchmark Matrix"
echo
echo "- Commit: \`$(git rev-parse --short HEAD 2>/dev/null || echo unknown)\`"
echo "- Rust: \`$(rustc --version)\`"
echo "- OS: \`$(uname -srmo)\`"
echo "- Frames: $FRAMES"
echo "- Output: stdout redirected to \`/dev/null\`"
echo
echo "| Mode | Command suffix | Elapsed | FPS | Bytes | Avg frame bytes | Max frame bytes | Throughput |"
echo "| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |"

field() {
    printf '%s\n' "$report" | tr ' ' '\n' | sed -n "s/^$1=//p"
}

emit_row() {
    mode=$1
    suffix=$2
    shift 2

    output=$("$@" 2>&1 >/dev/null)
    report=$(printf '%s\n' "$output" | sed -n '/^benchmark: /p' | tail -n 1)

    if [ -z "$report" ]; then
        echo "missing benchmark report for $mode" >&2
        printf '%s\n' "$output" >&2
        exit 1
    fi

    elapsed=$(field elapsed_s)
    fps=$(field fps)
    bytes=$(field bytes)
    avg_frame_bytes=$(field avg_frame_bytes)
    max_frame_bytes=$(field max_frame_bytes)
    throughput=$(field throughput_mib_s)

    printf '| %s | `%s` | %ss | %s | %s | %s | %s | %s MiB/s |\n' \
        "$mode" "$suffix" "$elapsed" "$fps" "$bytes" "$avg_frame_bytes" "$max_frame_bytes" "$throughput"
}

emit_row \
    "Xterm 256-color" \
    "env TERM=xterm-256color ... --benchmark --frames $FRAMES --no-title --no-clear --no-counter" \
    env TERM=xterm-256color "$BIN" --benchmark --frames "$FRAMES" --no-title --no-clear --no-counter

emit_row \
    "TrueColor" \
    "env TERM=xterm-256color ... --benchmark --truecolor --frames $FRAMES --no-title --no-clear --no-counter" \
    env TERM=xterm-256color "$BIN" --benchmark --truecolor --frames "$FRAMES" --no-title --no-clear --no-counter

emit_row \
    "VT100 40x24" \
    "env TERM=vt100 ... --benchmark --frames $FRAMES --width 40 --height 24 --no-title --no-clear --no-counter" \
    env TERM=vt100 "$BIN" --benchmark --frames "$FRAMES" --width 40 --height 24 --no-title --no-clear --no-counter
