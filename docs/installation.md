# Installation

## Pre-built Binaries

Download the latest release for your platform from the [releases page](https://github.com/rnabioco/aform-rs/releases).

| Platform | Download |
|----------|----------|
| Linux (x86_64) | `aform-x86_64-unknown-linux-gnu.tar.gz` |
| Linux (ARM64) | `aform-aarch64-unknown-linux-gnu.tar.gz` |
| macOS (Intel) | `aform-x86_64-apple-darwin.tar.gz` |
| macOS (Apple Silicon) | `aform-aarch64-apple-darwin.tar.gz` |
| Windows | `aform-x86_64-pc-windows-msvc.zip` |

## From Source

Requires [Rust](https://rustup.rs/) 1.70+.

```bash
# Install from GitHub
cargo install --git https://github.com/rnabioco/aform-rs

# Or clone and build
git clone https://github.com/rnabioco/aform-rs
cd aform-rs
cargo build --release
```

The binary will be at `target/release/aform`.

## Optional: ViennaRNA

For RNA folding features (`:fold`, `:alifold`), install [ViennaRNA](https://www.tbi.univie.ac.at/RNA/).

```bash
# macOS
brew install viennarna

# Ubuntu/Debian
sudo apt install viennarna
```
