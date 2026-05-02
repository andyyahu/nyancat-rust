# Benchmark Snapshots

Benchmark snapshots are local measurements, not portable guarantees. Hardware, CPU governor, kernel, Rust version, terminal mode, build profile, and output destination can all change the numbers.

For comparable render-throughput measurements, build in release mode and redirect stdout to `/dev/null`:

```bash
cargo build --release
env TERM=xterm-256color target/release/nyancat --benchmark --frames 100000 --no-title --no-clear --no-counter >/dev/null
```

## 2026-05-02

- Commit: `79cc902`
- Rust: `rustc 1.95.0 (59807616e 2026-04-14)`
- OS: `Linux 7.0.2-2-cachyos x86_64 GNU/Linux`
- CPU: `Intel(R) Core(TM) Ultra 9 185H`
- CPU topology: 22 logical CPUs, 16 cores, 2 threads per core
- Build: `cargo build --release`
- Output: stdout redirected to `/dev/null`
- Frames: 100,000

| Mode | Command suffix | Elapsed | FPS | Bytes | Avg frame bytes | Max frame bytes | Throughput |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| Xterm 256-color | `env TERM=xterm-256color ... --benchmark --frames 100000 --no-title --no-clear --no-counter` | 0.311109s | 321,430.64 | 401,883,158 | 4,018.83 | 4,152 | 1,231.93 MiB/s |
| TrueColor | `env TERM=xterm-256color ... --benchmark --truecolor --frames 100000 --no-title --no-clear --no-counter` | 0.532235s | 187,886.96 | 520,949,705 | 5,209.50 | 5,424 | 933.45 MiB/s |
| VT100 40x24 | `env TERM=vt100 ... --benchmark --frames 100000 --width 40 --height 24 --no-title --no-clear --no-counter` | 0.530758s | 188,409.80 | 308,616,555 | 3,086.17 | 3,170 | 554.53 MiB/s |
