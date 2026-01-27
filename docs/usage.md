# Usage

## Opening Files

```bash
aform alignment.stk
aform --color structure alignment.stk
aform --cluster --tree --conservation alignment.stk
```

Use `:e <path>` to open files from within the editor (Tab completes paths).

## Modes

aform-rs uses vim-style modal editing:

| Mode | Enter | Description |
|------|-------|-------------|
| Normal | `Esc` | Navigation and commands |
| Insert | `i` | Edit sequence characters |
| Visual | `v` | Block selection |
| Visual Line | `V` | Row selection (whole sequences) |
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

### Visual Mode (Block Selection)

| Key | Action |
|-----|--------|
| `v` | Enter visual mode (block selection) |
| `y` | Yank (copy) selection |
| `d` / `x` | Delete selection |
| `Esc` | Exit visual mode |

In Normal mode, `p` pastes the yanked block at the cursor position, replacing characters in place.

### Visual Line Mode (Sequence Selection)

| Key | Action |
|-----|--------|
| `V` | Enter visual line mode (selects whole rows) |
| `j` / `k` | Extend selection up/down |
| `y` | Yank selected sequences (with all annotations) |
| `d` | Delete selected sequences |
| `Esc` | Exit visual mode |

Visual Line mode (`V`) selects complete sequences including their IDs and all annotations (#=GS, #=GR). When you yank with `y`, the clipboard contains a complete sub-alignment that can be used to create a new split pane (see Split Panes below).

### Structure

| Key | Action |
|-----|--------|
| `gp` | Go to base pair partner |
| `[` / `]` | Previous/next helix |

### Split Panes

| Key | Action |
|-----|--------|
| `Ctrl-w s` | Horizontal split |
| `Ctrl-w v` | Vertical split |
| `Ctrl-w w` | Switch between panes |
| `Ctrl-w` + arrow | Switch between panes |
| `Ctrl-w q` | Close current pane |

## Commands

| Command | Description |
|---------|-------------|
| `:w` | Save file |
| `:q` | Quit |
| `:wq` | Save and quit |
| `:e <path>` | Open file (Tab completes path) |
| `:color <scheme>` | Set color scheme |
| `:trim` | Remove gap-only columns (both ends) |
| `:trimleft` | Remove leading gap-only columns |
| `:trimright` | Remove trailing gap-only columns |
| `:upper` | Convert to uppercase |
| `:lower` | Convert to lowercase |
| `:t2u` | Convert T to U |
| `:u2t` | Convert U to T |
| `:noh` | Clear search highlighting |
| `:cluster` | Cluster sequences by similarity |
| `:uncluster` | Restore original sequence order |
| `:tree` | Toggle dendrogram tree display |
| `:collapse` | Toggle collapse of identical sequences |
| `:consensus` | Toggle consensus sequence bar |
| `:conservation` | Toggle conservation level bar |
| `:ruler` | Toggle column ruler |
| `:rownum` | Toggle row numbers |
| `:shortid` | Toggle short IDs (strip /start-end suffix) |
| `:type <type>` | Set sequence type (rna/dna/protein/auto) |
| `:split` | Horizontal split (uses clipboard if linewise yank) |
| `:vsplit` | Vertical split (uses clipboard if linewise yank) |
| `:new` | Create new empty alignment in split pane |
| `:only` | Close split, keep current pane |
| `:clipboard` | Show clipboard contents (for debugging) |

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

aform-rs auto-detects the sequence type (RNA, DNA, or Protein) when loading a file. Protein alignments automatically enable base coloring, consensus, and conservation bar. RNA/DNA alignments with SS_cons automatically enable structure coloring. You can also manually set the type:

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
| `:conservation` | Show conservation level with height-varying bars (▁▂▃▄▅▆▇█) |

## Collapse Identical Sequences

Use `:collapse` to group identical sequences together, showing only one representative with a count indicator (e.g., `seq1 (5)` means 5 identical sequences). This reduces visual clutter in alignments with many duplicates.

## Split Panes

aform-rs supports vim-style split panes for viewing and extracting sequences.

### Basic Splits (Viewport Only)

Use `Ctrl-w s` (horizontal) or `Ctrl-w v` (vertical) to split the view. Both panes show the same alignment with independent scroll positions. Use `Ctrl-w w` or `Ctrl-w` + arrow keys to switch between panes.

### Extracting Sequences to a New File

The real power of splits comes from combining Visual Line mode with splits to extract a subset of sequences:

1. **Select sequences** - Navigate to the first sequence you want, press `V` to enter Visual Line mode, then use `j`/`k` to extend the selection
2. **Yank** - Press `y` to copy. You'll see "Yanked X of Y sequence(s) [linewise]"
3. **Split** - Type `:split` or `:vsplit`. The new pane opens with only the yanked sequences
4. **Save** - In the new pane, type `:w subset.sto` to save the extracted sequences

The yanked sub-alignment includes:
- Selected sequences with their IDs
- All #=GS annotations for those sequences
- All #=GR annotations for those sequences
- All #=GC column annotations (SS_cons, RF, etc.)
- All #=GF file annotations

### Example Workflow

```
# Open a large alignment
aform alignment.sto

# Navigate to row 10
10G

# Enter Visual Line mode
V

# Select rows 10-15
5j

# Yank the 6 sequences
y

# Create split with yanked sequences
:split

# (Now in secondary pane with 6 sequences)
# Save to new file
:w subset.sto

# Close the split and return to original
:q
```

### Split Commands

| Command | Description |
|---------|-------------|
| `:split` / `:sp` | Horizontal split |
| `:vsplit` / `:vs` | Vertical split |
| `:new` | Create empty alignment in new split |
| `:only` | Close all splits |
| `:q` | Close current pane (or quit if no split) |
| `:w <path>` | Save current pane's alignment |
