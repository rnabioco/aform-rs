# aform-rs

A terminal-based Stockholm alignment editor for RNA sequences, inspired by [Emacs ralee mode](https://github.com/samgriffithsjones/ralee).

## Features

- **Stockholm format support**: Parse and write Stockholm alignment files with full metadata support (#=GF, #=GS, #=GC, #=GR annotations), including [R2R](https://sourceforge.net/projects/weinberg-r2r/) extensions (R2R_LABEL, multiple SS_cons variants)
- **Vim-style keybindings**: Modal editing with normal, insert, and command modes
- **Secondary structure visualization**: Color alignments by helix, base identity, or conservation
- **Structure-aware navigation**: Jump to paired bases in the secondary structure
- **Alignment editing**: Insert/delete gaps, shift sequences, undo/redo
- **ViennaRNA integration**: Optional folding with RNAfold and RNAalifold (requires ViennaRNA package)
- **Sequence clustering**: Cluster sequences by similarity (UPGMA) with dendrogram visualization

## Installation

```bash
cargo install --path .
```

Or build from source:

```bash
cargo build --release
./target/release/aform-rs <file.stk>
```

## Usage

```bash
aform-rs [OPTIONS] [FILE]

Arguments:
  [FILE]  Stockholm alignment file to open

Options:
  -c, --color <COLOR>  Initial color scheme (none, structure, base, conservation) [default: none]
  -h, --help           Print help
  -V, --version        Print version
```

## Keybindings

### Normal Mode

| Key | Action |
|-----|--------|
| `h/j/k/l` | Move cursor left/down/up/right |
| `0` / `$` | Move to start/end of line |
| `gg` / `G` | Move to first/last sequence |
| `Ctrl-f/b` | Page down/up |
| `Ctrl-d/u` | Half page down/up |
| `w` / `b` | Jump 10 columns right/left |
| `gp` | Go to paired base |
| `i` | Enter insert mode |
| `x` | Delete gap at cursor |
| `I` | Insert gap column |
| `X` | Delete gap column (if all gaps) |
| `<` / `>` | Shift sequence left/right |
| `{` / `}` | Throw sequence left/right (shift to gap) |
| `dd` | Delete sequence |
| `u` | Undo |
| `Ctrl-r` | Redo |
| `:` | Enter command mode |
| `q` | Quit |
| `?` | Show help |

### Insert Mode

| Key | Action |
|-----|--------|
| `.` or `-` | Insert gap |
| `Backspace` | Delete gap behind cursor |
| `Esc` | Return to normal mode |

### Command Mode

| Command | Action |
|---------|--------|
| `:w` | Save file |
| `:q` | Quit (fails if unsaved changes) |
| `:q!` | Force quit |
| `:wq` | Save and quit |
| `:w <path>` | Save to new file |
| `:color <scheme>` | Set color scheme (none/ss/base/cons) |
| `:set gap=<char>` | Set gap character |
| `:fold` | Fold current sequence (requires RNAfold) |
| `:alifold` | Fold alignment (requires RNAalifold) |
| `:cluster` | Cluster sequences by similarity |
| `:uncluster` | Restore original sequence order |
| `:tree` | Toggle dendrogram tree display |

## Color Schemes

- **none**: No coloring
- **structure** (ss): Color by secondary structure helix
- **base** (nt): Color by nucleotide (A=blue, C=green, G=pink, U=yellow)
- **conservation** (cons): Color by column conservation

## Stockholm Format

aform-rs supports the standard Stockholm format:

```
# STOCKHOLM 1.0
#=GF ID    Example_alignment
#=GF AC    RF00001

seq1/1-50    ACGU...ACGU
seq2/1-50    ACGU...ACGU
#=GC SS_cons <<<<...>>>>
//
```

## Dependencies

- [ratatui](https://github.com/ratatui/ratatui) - Terminal UI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) - Terminal manipulation

Optional:
- [ViennaRNA](https://github.com/ViennaRNA/ViennaRNA) - For RNAfold/RNAalifold integration

## License

MIT

## Acknowledgments

Inspired by [ralee](https://github.com/samgriffithsjones/ralee) by Sam Griffiths-Jones.
