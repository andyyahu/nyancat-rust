# Release Checklist

This checklist is the baseline for merging release candidates and publishing builds. It is intentionally stricter than the minimum needed for local development.

## Release Inputs

- Confirm the target version in `Cargo.toml`.
- Confirm `CHANGELOG` has an entry for the target version.
- Confirm `README.md`, `nyancat.1`, and systemd files still match the current CLI and deployment model.
- Confirm `RUSTIFICATION_PLAN.md` and `ARCHITECTURE.md` reflect any structural changes made during the release cycle.
- Confirm the working tree is clean before final verification.

## Required Verification

Run from the repository root:

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

## Smoke Tests

Run the release binary after `cargo build --release`:

```bash
target/release/nyancat --frames 1 --no-title --no-clear --no-counter >/tmp/nyancat-rust-smoke.out
target/release/nyancat --telnet --skip-intro --frames 1 --no-title --no-clear --no-counter >/tmp/nyancat-rust-telnet-smoke.out
target/release/nyancat --truecolor --frames 1 --no-title --no-clear --no-counter >/tmp/nyancat-rust-truecolor-smoke.out
target/release/nyancat --frames 1 --width 40 --height 24 --no-title --no-clear --no-counter >/tmp/nyancat-rust-crop-smoke.out
target/release/nyancat --benchmark --frames 3 --no-title --no-clear --no-counter >/tmp/nyancat-rust-benchmark-smoke.out 2>/tmp/nyancat-rust-benchmark-smoke.err
target/release/nyancat --wat
```

Expected current byte counts:

```text
 4002 /tmp/nyancat-rust-smoke.out
 3067 /tmp/nyancat-rust-telnet-smoke.out
 5175 /tmp/nyancat-rust-truecolor-smoke.out
 4083 /tmp/nyancat-rust-crop-smoke.out
11916 /tmp/nyancat-rust-benchmark-smoke.out
```

The CLI error smoke must return a non-zero status and print:

```text
nyancat: unknown option: --wat
Try 'target/release/nyancat --help' for usage.
```

The benchmark smoke stderr must include:

```text
benchmark: frames=3
```

## Benchmark Snapshot

When performance changes are intentional, refresh `BENCHMARKS.md`.

Recommended commands:

```bash
cargo build --release
env TERM=xterm-256color target/release/nyancat --benchmark --frames 100000 --no-title --no-clear --no-counter >/dev/null
env TERM=xterm-256color target/release/nyancat --benchmark --truecolor --frames 100000 --no-title --no-clear --no-counter >/dev/null
env TERM=vt100 target/release/nyancat --benchmark --frames 100000 --width 40 --height 24 --no-title --no-clear --no-counter >/dev/null
```

Record:

- Commit SHA
- Rust version
- OS and kernel
- CPU model and topology
- Build profile
- Output destination
- Full benchmark reports

## Packaging Checks

- Confirm `Cargo.lock` is committed.
- Confirm `LICENSE` is present.
- Confirm `nyancat.1` documents all public CLI options.
- Confirm `systemd/nyancat.socket` and `systemd/nyancat@.service` still reference the intended binary path and socket behavior.
- Confirm release artifacts are built from a clean checkout or a clean working tree.

## Tagging

Before tagging:

```bash
git status --short
git log -1 --oneline
```

After verification, tag with the package version:

```bash
git tag -a vX.Y.Z -m "Release vX.Y.Z"
```

Only push tags after the release artifact and documentation have been reviewed.
