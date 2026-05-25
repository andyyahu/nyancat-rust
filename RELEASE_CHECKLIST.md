# Release Checklist

This checklist is the baseline for merging release candidates and publishing builds. It is intentionally stricter than the minimum needed for local development.

## Release Inputs

- Confirm the target version in `Cargo.toml`.
- Confirm `CHANGELOG` has an entry for the target version.
- Confirm `README.md`, `nyancat.1`, and systemd files still match the current CLI and deployment model.
- Confirm `ARCHITECTURE.md` reflects any structural changes made during the release cycle.
- Confirm `ROADMAP.md` reflects any newly accepted engineering priorities or deferred work.
- Confirm the working tree is clean before final verification.

## Required Verification

Run the automated baseline from the repository root:

```bash
scripts/release_check.sh
```

The script covers:

- `cargo fmt --check`
- `cargo test --locked`, including CLI option coverage for `README.md` and `nyancat.1`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`
- `cargo build --release --locked`
- `sh -n` syntax checks for release helper scripts
- `cargo package --list --allow-dirty --locked`, including expected release files and excluding local-only dotfiles
- `scripts/release_archive.sh` with a temporary dist directory, followed by archive content checks
- Smoke tests, byte count and checksum checks, output marker checks, CLI error checks, and `--help` option coverage

GitHub Actions also runs the release check on stable Rust and a separate MSRV build/test job for Rust 1.85.0.

If you need to run the steps manually, use the commands in the sections below.

## Smoke Tests

Run the release binary after `cargo build --release`, or use `scripts/release_check.sh`:

```bash
env TERM=xterm-256color target/release/nyancat --frames 1 --no-title --no-clear --no-counter >/tmp/nyancat-rust-smoke.out
target/release/nyancat --telnet --skip-intro --frames 1 --no-title --no-clear --no-counter >/tmp/nyancat-rust-telnet-smoke.out
env TERM=xterm-256color target/release/nyancat --truecolor --frames 1 --no-title --no-clear --no-counter >/tmp/nyancat-rust-truecolor-smoke.out
env TERM=xterm-256color target/release/nyancat --frames 1 --width 40 --height 24 --no-title --no-clear --no-counter >/tmp/nyancat-rust-crop-smoke.out
env TERM=xterm-256color target/release/nyancat --benchmark --frames 3 --no-title --no-clear --no-counter >/tmp/nyancat-rust-benchmark-smoke.out 2>/tmp/nyancat-rust-benchmark-smoke.err
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

Expected current POSIX `cksum` values:

```text
3491497212 4002 /tmp/nyancat-rust-smoke.out
3107447574 3067 /tmp/nyancat-rust-telnet-smoke.out
1251626052 5175 /tmp/nyancat-rust-truecolor-smoke.out
1400779159 4083 /tmp/nyancat-rust-crop-smoke.out
3251515113 11916 /tmp/nyancat-rust-benchmark-smoke.out
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

The automated smoke checks also verify key output markers:

- xterm output uses 256-color escape sequences and does not emit TrueColor sequences.
- TrueColor output uses 24-bit escape sequences and does not emit 256-color sequences.
- telnet output starts with negotiation bytes and uses CR NUL LF newline markers.
- `--no-counter` smoke paths do not print counter text.

## Benchmarking

When performance changes are intentional, refresh the benchmark snapshot in this file.

Recommended commands:

```bash
scripts/benchmark_matrix.sh 100000 5
```

Record:

- Commit SHA
- Rust version
- OS and kernel
- CPU model and topology
- Build profile
- Output destination
- Runs per mode
- Full benchmark reports

### Current Snapshot

Benchmark snapshots are local measurements, not portable guarantees. Hardware, CPU governor, kernel, Rust version, terminal mode, build profile, and output destination can all change the numbers.

For comparable render-throughput measurements, build in release mode and redirect stdout to `/dev/null`. `scripts/benchmark_matrix.sh` runs each mode multiple times, verifies deterministic byte stats, and reports the sample with median elapsed time.

#### 2026-05-25

- Commit: `5f424b4`
- Rust: `rustc 1.95.0 (59807616e 2026-04-14)`
- OS: `Linux 7.0.10-1-cachyos x86_64 GNU/Linux`
- CPU: `Intel(R) Core(TM) Ultra 9 185H`
- CPU topology: 22 logical CPUs, 16 cores, 2 threads per core
- Build: `cargo build --release`
- Output: stdout redirected to `/dev/null`
- Frames: 100,000
- Runs per mode: 5

| Mode | Command suffix | Median elapsed | Median FPS | Bytes | Avg frame bytes | Max frame bytes | Median throughput | Elapsed range |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Xterm 256-color | `env TERM=xterm-256color ... --benchmark --frames 100000 --no-title --no-clear --no-counter` | 1.160864s | 86,142.70 | 401,883,158 | 4,018.83 | 4,152 | 330.16 MiB/s | 1.124866s-1.195199s |
| TrueColor | `env TERM=xterm-256color ... --benchmark --truecolor --frames 100000 --no-title --no-clear --no-counter` | 1.035395s | 96,581.46 | 520,949,705 | 5,209.50 | 5,424 | 479.83 MiB/s | 1.027166s-1.040219s |
| VT100 40x24 | `env TERM=vt100 ... --benchmark --frames 100000 --width 40 --height 24 --no-title --no-clear --no-counter` | 1.211560s | 82,538.22 | 308,616,555 | 3,086.17 | 3,170 | 242.93 MiB/s | 1.182115s-1.294123s |

## Packaging Checks

- Confirm `Cargo.lock` is committed.
- Confirm `LICENSE` is present.
- Confirm `Cargo.toml` has description, repository, homepage, readme, license, keywords, and categories metadata.
- Confirm `nyancat.1` documents all public CLI options; `cargo test` also checks README/manpage option names against `OPTION_SPECS`.
- Confirm `systemd/nyancat.socket` and `systemd/nyancat@.service` still reference the intended binary path and socket behavior.
- Confirm `cargo package --list --locked` contains the expected user docs, release scripts, source files, manpage, and systemd files.
- Confirm the package list excludes local-only files such as `.codex`, `.cargo/config.toml`, and GitHub Actions workflow metadata.
- Confirm release artifacts are built from a clean checkout or a clean working tree; `scripts/release_check.sh` builds a temporary archive and verifies its contents.

To build a local binary release archive:

```bash
scripts/release_archive.sh
tar -tzf target/dist/nyancat-vX.Y.Z-<host>.tar.gz
```

The archive contains:

- `bin/nyancat`
- `share/man/man1/nyancat.1`
- `systemd/nyancat.socket` and `systemd/nyancat@.service`
- user docs, release docs, license, changelog, and Cargo manifest files

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
