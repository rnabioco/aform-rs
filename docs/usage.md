# Usage

## Opening Files

```bash
aform alignment.stk
aform --color structure alignment.stk
```

Or use `:e` to browse files from within the editor.

![File browser](images/file-browser.gif)

## Modes

aform-rs uses vim-style modal editing:

| Mode | Enter | Description |
|------|-------|-------------|
| Normal | `Esc` | Navigation and commands |
| Insert | `i` | Edit sequence characters |
| Visual | `v` | Block selection |
| Command | `:` | Ex-style commands |
| Search | `/` | Pattern search |

## Key Bindings

### Navigation

| Key | Action |
|-----|--------|
| `h/j/k/l` | Move cursor |
| `0` / `$` | Line start/end |
| `gg` / `G` | First/last sequence |
| `Ctrl-f/b` | Page down/up |
| `Ctrl-d/u` | Half page down/up |
| `w` / `b` | Jump 10 columns right/left |

### Editing (Insert Mode)

| Key | Action |
|-----|--------|
| `a-z`, `A-Z` | Insert nucleotide |
| `-`, `.` | Insert gap |
| `Backspace` | Delete character |

### Visual Mode

| Key | Action |
|-----|--------|
| `v` | Enter visual mode |
| `y` | Yank (copy) selection |
| `d` / `x` | Delete selection |
| `Esc` | Exit visual mode |

In Normal mode, `p` pastes the yanked block.

### Structure

| Key | Action |
|-----|--------|
| `gp` | Go to base pair partner |
| `[` / `]` | Previous/next helix |

## Commands

| Command | Description |
|---------|-------------|
| `:w` | Save file |
| `:q` | Quit |
| `:wq` | Save and quit |
| `:e [path]` | Open file browser |
| `:color <scheme>` | Set color scheme |
| `:trim` | Remove gap-only columns (both ends) |
| `:trimleft` | Remove leading gap-only columns |
| `:trimright` | Remove trailing gap-only columns |
| `:upper` | Convert to uppercase |
| `:lower` | Convert to lowercase |
| `:t2u` | Convert T to U |
| `:u2t` | Convert U to T |
| `:fold` | Run RNAfold on current sequence |
| `:alifold` | Run RNAalifold on alignment |
| `:noh` | Clear search highlighting |
| `:cluster` | Cluster sequences by similarity |
| `:uncluster` | Restore original sequence order |
| `:tree` | Toggle dendrogram tree display |
| `:collapse` | Toggle collapse of identical sequences |
| `:consensus` | Toggle consensus sequence bar |
| `:conservation` | Toggle conservation level bar |
| `:type <type>` | Set sequence type (rna/dna/protein/auto) |

## Clustering

Cluster sequences by pairwise similarity using `:cluster`. This reorders sequences using hierarchical agglomerative clustering (UPGMA algorithm with Hamming distance), grouping similar sequences together.

Use `:tree` to show a dendrogram alongside the alignment, visualizing sequence relationships. The tree uses ASCII box-drawing characters and adapts to your terminal's color scheme.

To restore the original sequence order, use `:uncluster`.

## Color Schemes

Set with `:color <scheme>` or `--color` flag.

| Scheme | Aliases | Description |
|--------|---------|-------------|
| `none` | `off` | No coloring |
| `structure` | `ss` | Color by helix (rainbow) |
| `base` | `nt`, `protein`, `aa`, `residue` | Color by nucleotide or amino acid |
| `conservation` | `cons` | Color by column conservation |
| `compensatory` | `comp` | Highlight compensatory mutations |

The `base` scheme automatically uses nucleotide colors for RNA/DNA or amino acid colors (Taylor scheme) for protein sequences based on auto-detection.

![Color schemes comparison](images/color-schemes.gif)

## Sequence Type

aform-rs auto-detects the sequence type (RNA, DNA, or Protein) when loading a file. You can also manually set it:

| Command | Description |
|---------|-------------|
| `:type` | Show current sequence type |
| `:type rna` | Set to RNA |
| `:type dna` | Set to DNA |
| `:type protein` | Set to Protein |
| `:type auto` | Auto-detect from alignment |

## Annotation Bars

Toggle annotation bars below the alignment:

| Command | Description |
|---------|-------------|
| `:consensus` | Show consensus sequence (uppercase = high conservation) |
| `:conservation` | Show conservation level with block characters (░▒▓█) |

## Collapse Identical Sequences

Use `:collapse` to group identical sequences together, showing only one representative with a count indicator (e.g., `seq1 (5)` means 5 identical sequences). This reduces visual clutter in alignments with many duplicates.
