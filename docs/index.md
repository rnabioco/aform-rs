# aform-rs

Terminal-based Stockholm alignment editor for RNA, DNA, and protein sequences with vim-style modal editing.

![Main editor view](images/editor.gif)

## Features

- **Vim-style editing** - Modal editing with Normal, Insert, Visual, and Command modes
- **RNA/DNA/Protein support** - Auto-detects sequence type with appropriate coloring
- **RNA structure** - Base pair highlighting from SS_cons annotations
- **Color schemes** - Structure, nucleotide, amino acid, conservation, and compensatory coloring
- **Annotation bars** - Consensus sequence and conservation level visualization
- **Collapse identical** - Group and collapse identical sequences
- **Block selection** - Visual block mode for rectangular selections
- **Undo/redo** - Full history with efficient copy-on-write sequences
- **Sequence clustering** - Cluster sequences by similarity with dendrogram visualization

### Clustering with Dendrogram

Cluster sequences by similarity and visualize relationships with an ASCII dendrogram tree.

![Clustering with tree](images/clustering.gif)

### Annotation Bars

Display consensus sequence and conservation levels below the alignment.

![Annotation bars](images/annotation-bars.gif)

### Protein Support

Auto-detects protein sequences and enables amino acid coloring (Taylor scheme), consensus, and conservation bar automatically.

![Protein alignment](images/protein.gif)

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

# Protein files auto-enable coloring, consensus, and conservation
aform protein.stk
```

Press `?` for help within the editor.
