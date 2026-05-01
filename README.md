# 🐱 Nyancat - Rust Edition

> A fast, colorful Nyancat animation for your terminal, written in Rust.

[![License](https://img.shields.io/badge/license-NCSA-blue.svg)](LICENSE)

![Nyancat](http://nyancat.dakko.us/nyancat.png)

## ✨ Features

- **High-performance animation** - Written in Rust for speed and efficiency
- **Terminal rendering** - Works in any ANSI-compatible terminal
- **TrueColor support** - Optional 24-bit high-definition rendering mode
- **Benchmark mode** - Zero-delay rendering for performance testing
- **Telnet mode** - Share Nyancat over the network through socket activation or inetd
- **Cross-platform** - Supports Linux, macOS, BSD, and other Unix-like systems
- **Minimal dependencies** - No external crates required

## ⚡ Performance

The Rust edition is optimized for high-throughput rendering. In our benchmarks, it significantly outperforms the original C implementation.

### Benchmark Results (100,000 frames)

| Implementation | Time (Total) | Throughput (FPS) |
| :--- | :--- | :--- |
| Original C (klange/nyancat) | 0.654s | ~152,905 FPS |
| **Rust Edition** | **0.109s** | **~917,431 FPS** |

*Test conditions: 100,000 frames rendered to `/dev/null` with delays disabled (using `-b` flag for Rust), running on Linux.*

## 🚀 Quick Start

### Build and Run

```bash
# Build the release version
cargo build --release

# Run the animation
./target/release/nyancat
```

Or run directly with Cargo:

```bash
cargo run --release
```

### Serve over telnet

```bash
# The -t flag speaks telnet over stdin/stdout; it does not open a listening port.
# Use the included systemd socket files, xinetd, or openbsd-inetd for network access.
sudo cp target/release/nyancat /usr/bin/nyancat
sudo cp systemd/nyancat.socket systemd/nyancat@.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now nyancat.socket
```

Then connect with:

```bash
telnet localhost 23
```

Example systemd service files are included in the `systemd/` directory.

## 📦 Installation

### From source

```bash
cargo build --release
# Optionally copy to your bin folder
cp target/release/nyancat /usr/local/bin/
```

## 💻 Usage

```bash
# Run the animation
nyancat

# Run with telnet negotiation on stdin/stdout
nyancat -t

# Run in benchmark mode (0ms delay)
nyancat -b

# Run in high-definition TrueColor mode
nyancat -T
```

### Command Line Options

| Flag | Long Option | Description |
| :--- | :--- | :--- |
| `-i` | `--intro` | Show introduction at startup |
| `-I` | `--skip-intro` | Skip the introduction in telnet mode |
| `-t` | `--telnet` | Enable Telnet protocol mode |
| `-T` | `--truecolor` | Enable 24-bit TrueColor rendering |
| `-n` | `--no-counter` | Do not display the timer |
| `-s` | `--no-title` | Do not set titlebar text |
| `-e` | `--no-clear` | Do not clear screen between frames |
| `-b` | `--benchmark` | Run with 0ms delay (Warning: high CPU) |
| `-d` | `--delay` | Set delay (10ms - 1000ms) |
| `-f` | `--frames` | Quit after N frames |
| `-r` | `--min-rows` | Crop the animation from the top |
| `-R` | `--max-rows` | Crop the animation from the bottom |
| `-c` | `--min-cols` | Crop the animation from the left |
| `-C` | `--max-cols` | Crop the animation from the right |
| `-W` | `--width` | Set animation width |
| `-H` | `--height` | Set animation height |
| `-h` | `--help` | Show help message |

## 🔧 Development

### Prerequisites

- Rust 1.85+ (2024 edition)

### Build

```bash
cargo build --release
```

### Project structure

- `src/main.rs` - Startup orchestration
- `src/animation.rs` - Frame data
- `src/cli.rs` - Command-line parsing and configuration
- `src/render.rs` - Palette setup and animation rendering
- `src/telnet.rs` - Telnet negotiation
- `src/terminal.rs` - Terminal size and type detection
- `src/runtime.rs` - Exit and signal handling
- `src/sys.rs` - Unix FFI bindings
- `systemd/` - Systemd service files for telnet server integration

## Credits

- **Original Nyancat animation**: [prguitarman](http://www.prguitarman.com/index.php?id=348)
- **Original implementation**: [Kevin Lange (klange)](https://github.com/klange/nyancat)
- **Rust rewrite**: This project

## 📜 License

Licensed under the [NCSA License](LICENSE).
