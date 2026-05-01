# 🐱 Nyancat - Rust Edition

> A fast, colorful Nyancat animation for your terminal, written in Rust.

[![License](https://img.shields.io/badge/license-NCSA-blue.svg)](LICENSE)

![Nyancat](http://nyancat.dakko.us/nyancat.png)

## ✨ Features

- **High-performance animation** - Written in Rust for speed and efficiency
- **Terminal rendering** - Works in any ANSI-compatible terminal
- **Telnet server** - Share Nyancat over the network with telnet
- **Cross-platform** - Supports Linux, macOS, BSD, and other Unix-like systems
- **Minimal dependencies** - No external crates required

## 🚀 Quick Start

### Build from source

```bash
make
./src/nyancat
```

Or use Cargo directly:

```bash
cargo run --release
```

### Run as telnet server

```bash
./src/nyancat -t
```

Then connect with:

```bash
telnet localhost 23
```

For production setups, integrate with `systemd`, `xinetd`, or `openbsd-inetd`. Example systemd service files are included in the `systemd/` directory.

## 📦 Installation

### From source

```bash
make
sudo make install
```

## 💻 Usage

```bash
# Run the animation
nyancat

# Run as telnet server
nyancat -t
```

## 🔧 Development

### Prerequisites

- Rust 1.85+ (2024 edition)
- Make

### Build

```bash
cargo build --release
```

### Project structure

- `src/main.rs` - Main application logic with telnet protocol handling
- `src/animation.rs` - Frame data and animation rendering
- `systemd/` - Systemd service files for telnet server integration

## � Credits

- **Original Nyancat animation**: [prguitarman](http://www.prguitarman.com/index.php?id=348)
- **Original implementation**: [Kevin Lange (klange)](https://github.com/klange/nyancat)
- **Rust rewrite**: This project

## 📜 License

Licensed under the [NCSA License](LICENSE).
