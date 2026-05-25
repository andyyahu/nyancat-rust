#!/usr/bin/env sh
set -eu

# Objective, deterministic render benchmark: counts retired instructions under
# valgrind's callgrind. Unlike wall-clock FPS (scripts/benchmark_matrix.sh) this
# is immune to CPU frequency scaling and scheduler decisions, so before/after
# comparisons are meaningful on any machine -- including laptops whose
# throughput can swing ~2x run to run. Prefer this for verifying perf changes.

BIN=${NYANCAT_BIN:-target/release/nyancat}
FRAMES=${1:-1000}

case $FRAMES in
    ''|*[!0-9]*)
        echo "frames must be a positive integer" >&2
        exit 1
        ;;
esac

if [ "$FRAMES" -eq 0 ]; then
    echo "frames must be a positive integer" >&2
    exit 1
fi

if ! command -v valgrind > /dev/null 2>&1; then
    echo "valgrind is required for this benchmark (e.g. pacman -S valgrind)" >&2
    exit 1
fi

if [ ! -x "$BIN" ]; then
    cargo build --release --locked
fi

TMP=${TMPDIR:-/tmp}

echo "# Callgrind Instruction Benchmark"
echo
echo "- Commit: \`$(git rev-parse --short HEAD 2>/dev/null || echo unknown)\`"
echo "- Tool: \`$(valgrind --version)\` (deterministic instruction count, frequency-independent)"
echo "- Frames: $FRAMES"
echo
echo "| Mode | Instructions | Per frame |"
echo "| :--- | ---: | ---: |"

measure() {
    label=$1
    term=$2
    shift 2

    out=$(mktemp "$TMP/nyancat-callgrind.XXXXXX")
    log=$(mktemp "$TMP/nyancat-callgrind-log.XXXXXX")

    env NO_COLOR= TERM="$term" valgrind --tool=callgrind \
        --callgrind-out-file="$out" --log-file="$log" \
        "$BIN" --benchmark --frames "$FRAMES" --no-title --no-clear --no-counter "$@" \
        > /dev/null 2> /dev/null

    ir=$(sed -n 's/.*I  *refs:[ ]*//p' "$log" | tr -d ', ')
    rm -f "$out" "$log"

    if [ -z "$ir" ]; then
        echo "failed to read instruction count for $label" >&2
        exit 1
    fi

    printf '| %s | %s | %s |\n' "$label" "$ir" "$((ir / FRAMES))"
}

measure "Xterm 256-color" xterm-256color
measure "TrueColor" xterm-256color --truecolor
measure "VT100 40x24" vt100 --width 40 --height 24
