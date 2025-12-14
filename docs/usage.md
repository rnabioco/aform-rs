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

## Color Schemes

Set with `:color <scheme>` or `--color` flag.

| Scheme | Description |
|--------|-------------|
| `none` | No coloring |
| `structure` | Color by helix (rainbow) |
| `base` | Color by nucleotide |
| `conservation` | Color by column conservation |
| `compensatory` | Highlight compensatory mutations |

![Color schemes comparison](images/color-schemes.gif)
