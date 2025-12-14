# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Development Commands

```bash
# Build
cargo build
cargo build --release

# Run tests
cargo test
cargo test --all-features

# Linting and formatting
cargo clippy --all-features -- -D warnings
cargo fmt --all --check
cargo fmt  # auto-format

# Run the application
cargo run -- [FILE]
cargo run -- --color structure examples/example.stk
```

## Architecture Overview

aform-rs is a terminal-based Stockholm alignment editor for RNA sequences, using vim-style modal editing.

### Core Modules

- **`src/main.rs`**: Entry point, terminal setup (ratatui/crossterm), main event loop
- **`src/app.rs`**: Central `App` struct holding all application state (alignment, cursor, mode, viewport, history, search, clipboard). Implements vim-style modes (Normal, Insert, Command, Search, Browse, Visual)
- **`src/input.rs`**: Key event handling, dispatches to mode-specific handlers
- **`src/ui.rs`**: Terminal rendering with ratatui, viewport management, split pane support

### Data Layer

- **`src/stockholm/`**: Stockholm format I/O
  - `types.rs`: Core data structures - `Alignment`, `Sequence` (with `Rc` wrapper for copy-on-write), annotations (#=GF, #=GS, #=GC, #=GR)
  - `parser.rs`: Stockholm file parsing
  - `writer.rs`: Stockholm file writing

### Editor Features

- **`src/editor/`**: Editing operations
  - `commands.rs`: Alignment manipulation (gap insert/delete, shift, trim, case conversion)
  - `history.rs`: Undo/redo with structural sharing via `Rc<Sequence>`

- **`src/structure/`**: RNA secondary structure
  - `pairs.rs`: `StructureCache` for base pair lookup from SS_cons annotation
  - `parser.rs`: Dot-bracket notation parsing

- **`src/color/`**: Color schemes (none, structure/helix, base/nucleotide, conservation, compensatory)

- **`src/external/`**: ViennaRNA integration (RNAfold/RNAalifold)

### Key Design Patterns

- **Copy-on-write sequences**: `Alignment.sequences` uses `Vec<Rc<Sequence>>` for efficient undo/redo. Use `Rc::make_mut()` when modifying sequences.
- **Modal editing**: `Mode` enum controls input handling and UI display
- **Viewport scrolling**: Separate viewport offsets from cursor position; `adjust_viewport()` keeps cursor visible
- **Split panes**: Primary/secondary viewports with independent scroll positions
