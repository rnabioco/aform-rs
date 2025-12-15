# aform-rs

A terminal-based Stockholm alignment editor for RNA, DNA, and protein sequences, inspired by [Emacs ralee mode](https://github.com/samgriffithsjones/ralee).

## Features

- **Stockholm format support**: Parse and write Stockholm alignment files with full metadata support (#=GF, #=GS, #=GC, #=GR annotations), including [R2R](https://sourceforge.net/projects/weinberg-r2r/) extensions (R2R_LABEL, multiple SS_cons variants)
- **Vim-style keybindings**: Modal editing with normal, insert, and command modes
- **RNA/DNA/Protein support**: Auto-detects sequence type with appropriate coloring (nucleotide or amino acid colors)
- **Secondary structure visualization**: Color alignments by helix, base identity, or conservation
- **Structure-aware navigation**: Jump to paired bases in the secondary structure
- **Alignment editing**: Insert/delete gaps, shift sequences, undo/redo
- **Annotation bars**: Optional consensus sequence and conservation level visualization
- **Collapse identical sequences**: Group and collapse identical sequences to reduce clutter
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
aform [OPTIONS] [FILE]

Arguments:
  [FILE]  Stockholm alignment file to open

Options:
  -c, --color <COLOR>  Color scheme (none, structure, base, conservation, compensatory)
      --consensus      Show consensus sequence
      --conservation   Show conservation bar
      --cluster        Cluster sequences by similarity
      --collapse       Collapse identical sequences
      --tree           Show dendrogram tree (implies --cluster)
      --ruler          Show column ruler
      --rownum         Show row numbers
      --shortid        Show short IDs (strip /start-end suffix)
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
| `:e <path>` | Open file (Tab to complete path) |
| `:w <path>` | Save to new file |
| `:color <scheme>` | Set color scheme (see below) |
| `:set gap=<char>` | Set gap character |
| `:cluster` | Cluster sequences by similarity |
| `:uncluster` | Restore original sequence order |
| `:tree` | Toggle dendrogram tree display |
| `:collapse` | Toggle collapse of identical sequences |
| `:consensus` | Toggle consensus sequence bar |
| `:conservation` | Toggle conservation level bar |
| `:ruler` | Toggle column ruler |
| `:rownum` | Toggle row numbers |
| `:shortid` | Toggle short IDs (strip /start-end) |
| `:type <type>` | Set sequence type (rna/dna/protein/auto) |

## Color Schemes

Set with `:color <scheme>`:

- **none** / **off**: No coloring
- **structure** / **ss**: Color by secondary structure helix
- **base** / **nt**: Color by nucleotide (A=blue, C=green, G=pink, U=yellow)
- **protein** / **aa** / **residue**: Amino acid coloring (Taylor scheme)
- **conservation** / **cons**: Color by column conservation
- **compensatory** / **comp**: Highlight compensatory mutations

Note: `base`, `protein`, `aa`, and `residue` all use the same color scheme, which automatically adapts to the detected sequence type (nucleotide or amino acid colors).

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

## License

MIT

## Acknowledgments

Inspired by [ralee](https://github.com/samgriffithsjones/ralee) by Sam Griffiths-Jones.
