#!/usr/bin/env sh
set -eu

FRAMES=${1:-100000}
RUNS=${2:-3}

case $FRAMES in
    ''|*[!0-9]*)
        echo "frames must be a positive integer" >&2
        exit 1
        ;;
esac

case $RUNS in
    ''|*[!0-9]*)
        echo "runs must be a positive integer" >&2
        exit 1
        ;;
esac

if [ "$FRAMES" -eq 0 ]; then
    echo "frames must be a positive integer" >&2
    exit 1
fi

if [ "$RUNS" -eq 0 ]; then
    echo "runs must be a positive integer" >&2
    exit 1
fi

TMP=${TMPDIR:-/tmp}
results=$(mktemp "$TMP/nyancat-rust-benchmark.XXXXXX")
trap 'rm -f "$results"' EXIT HUP INT TERM

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

echo "# Benchmark Matrix"
echo
echo "- Commit: \`$(git rev-parse --short HEAD 2>/dev/null || echo unknown)\`"
echo "- Rust: \`$(rustc --version)\`"
echo "- OS: \`$(uname -srmo)\`"
echo "- Frames: $FRAMES"
echo "- Runs per mode: $RUNS"
echo "- Output: stdout redirected to \`/dev/null\`"
echo
echo "| Mode | Command suffix | Median elapsed | Median FPS | Bytes | Avg frame bytes | Max frame bytes | Median throughput | Elapsed range |"
echo "| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"

field() {
    printf '%s\n' "$report" | tr ' ' '\n' | sed -n "s/^$1=//p"
}

emit_row() {
    mode=$1
    suffix=$2
    shift 2

    : > "$results"
    run=1
    expected_bytes=
    expected_avg_frame_bytes=
    expected_max_frame_bytes=

    while [ "$run" -le "$RUNS" ]; do
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

        if [ "$run" -eq 1 ]; then
            expected_bytes=$bytes
            expected_avg_frame_bytes=$avg_frame_bytes
            expected_max_frame_bytes=$max_frame_bytes
        elif [ "$bytes" != "$expected_bytes" ] \
            || [ "$avg_frame_bytes" != "$expected_avg_frame_bytes" ] \
            || [ "$max_frame_bytes" != "$expected_max_frame_bytes" ]; then
            echo "non-deterministic byte stats for $mode" >&2
            printf '%s\n' "$report" >&2
            exit 1
        fi

        printf '%s %s %s %s %s %s\n' \
            "$elapsed" "$fps" "$bytes" "$avg_frame_bytes" "$max_frame_bytes" "$throughput" >> "$results"
        run=$((run + 1))
    done

    median_index=$(((RUNS + 1) / 2))
    sorted=$(LC_ALL=C sort -n -k1,1 "$results")
    median=$(printf '%s\n' "$sorted" | sed -n "${median_index}p")
    min_elapsed=$(printf '%s\n' "$sorted" | sed -n '1p' | awk '{ print $1 }')
    max_elapsed=$(printf '%s\n' "$sorted" | sed -n '$p' | awk '{ print $1 }')

    set -- $median
    elapsed=$1
    fps=$2
    bytes=$3
    avg_frame_bytes=$4
    max_frame_bytes=$5
    throughput=$6

    printf '| %s | `%s` | %ss | %s | %s | %s | %s | %s MiB/s | %ss-%ss |\n' \
        "$mode" "$suffix" "$elapsed" "$fps" "$bytes" "$avg_frame_bytes" "$max_frame_bytes" "$throughput" "$min_elapsed" "$max_elapsed"
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
