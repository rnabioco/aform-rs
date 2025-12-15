# aform-rs

A terminal-based Stockholm alignment editor for RNA, DNA, and protein sequences.

![aform-rs editor demo](docs/images/editor.gif)

## Features

- **Stockholm format** with full annotation support (#=GF, #=GS, #=GC, #=GR)
- **Vim-style modal editing** (normal, insert, visual, command modes)
- **RNA/DNA/Protein** auto-detection with appropriate coloring
- **Secondary structure** visualization and paired-base navigation
- **Sequence clustering** with dendrogram display (UPGMA)
- **Collapse identical sequences** to reduce clutter

## Installation

```bash
cargo install aform-rs
```

Or build from source:

```bash
cargo build --release
```

## Quick Start

```bash
aform alignment.stk
```

Press `?` for help, `:q` to quit.

## Documentation

See the [full documentation](https://rnabioco.github.io/aform-rs/) for keybindings, commands, and color schemes.

## License

MIT

## Acknowledgments

Inspired by [ralee](https://github.com/samgriffithsjones/ralee) by Sam Griffiths-Jones.
