# aform-rs

Terminal-based Stockholm alignment editor with vim-style modal editing.

![Main editor view](images/editor.gif)

## Features

- **Vim-style editing** - Modal editing with Normal, Insert, Visual, and Command modes
- **RNA structure** - Base pair highlighting from SS_cons annotations
- **Color schemes** - Structure, nucleotide, conservation, and compensatory coloring
- **Block selection** - Visual block mode for rectangular selections
- **Undo/redo** - Full history with efficient copy-on-write sequences

### Structure Coloring

![Structure coloring mode](images/structure-coloring.gif)

### Visual Block Selection

![Visual block selection](images/visual-mode.gif)

## Quick Start

```bash
# Open a Stockholm file
aform alignment.stk

# With structure coloring
aform --color structure alignment.stk
```

Press `?` for help within the editor.
