# Changelog

All notable changes to aform-rs will be documented in this file.

## [Unreleased]

### Added
- Auto-configure display for protein alignments: base coloring, consensus, and conservation bar are enabled automatically on file load

### Fixed
- Index out of bounds when deleting sequences with active clustering and collapsed identical sequences

## [0.1.0-alpha.11] - 2026-01-23

### Added
- Visual Line mode (`V`) for selecting whole sequences with annotations
- Split panes with separate alignments for sequence extraction workflow
- Yank sequences in Visual Line mode captures complete sub-alignment (#=GS, #=GR, #=GC, #=GF)
- `:new` command to create empty alignment in split pane
- `:clipboard` command to inspect clipboard contents
- Paste (`p`) in secondary pane appends sequences from clipboard

### Fixed
- Ctrl-w + arrow keys now correctly switches between split panes

### Notes
- Clustering commands (`:cluster`, `:collapse`, `:tree`) are not supported in secondary pane

## [0.1.0-alpha.10] - 2026-01-17

### Added
- Light mode theme support: auto-detects terminal background and uses appropriate colors
- Auto-enable structure coloring for RNA files with SS_cons annotation

### Changed
- Updated `toml` dependency to 0.9

## [0.1.0-alpha.9] - 2025-12-30

### Added
- Improved dendrogram tree display with collapsed sequences
- Minimal dots style for dendrogram tree display

### Fixed
- Use musl for static Linux binaries

## [0.1.0-alpha.8] - 2025-12-30

### Added
- Trackpad scroll support for navigation

## [0.1.0-alpha.7] - 2025-12-18

### Added
- Configurable UI theme via `aform.toml`
- Base-pair ruler display showing paired column positions
- Support for simpler color formats in config (hex `#RRGGBB` and CSV `r,g,b`)

## [0.1.0-alpha.6] - 2025-12-18

### Added
- Multi-sequence deletion in visual mode with row ID highlighting
- PP (posterior probability) coloring
- Gap column hiding (`:hidegaps` / `:showgaps`)
- Info overlay (`?` key)

### Fixed
- Correct sequence indexing in clustered view
- Recompute clustering after sequence deletion

## [0.1.0-alpha.5] - 2025-12-15

### Added
- CLI flags for display options (`--consensus`, `--conservation`, `--ruler`, `--rownum`, `--shortid`)
- CLI flags for clustering (`--cluster`, `--collapse`, `--tree`)
- Tab completion for file paths in `:e` command

### Changed
- Upgraded to ratatui 0.30

## [0.1.0-alpha.4] - 2025-12-15

### Added
- Search history with Up/Down arrow navigation

### Changed
- Improved code organization and reduced duplication

## [0.1.0-alpha.3] - 2025-12-14

### Added
- Gzip file support (`.stk.gz`, `.sto.gz`)
- Protein sequence support with automatic detection
- Collapse identical sequences (`:collapse`)
- Consensus sequence display (`:consensus`)
- Conservation bar display (`:conservation`)

### Fixed
- Collapse counts with clustering
- Height-varying bars for conservation display
- Stay in command mode when backspacing to empty

## [0.1.0-alpha.2] - 2025-12-14

### Added
- Sequence clustering by similarity (`:cluster`, `:uncluster`)
- Dendrogram tree visualization (`:tree`)
- Scrollbar widgets for position feedback

### Fixed
- Clippy warnings

## [0.1.0-alpha.1] - 2025-12-14

### Added
- Initial release
- Stockholm format parsing and writing
- Vim-style modal editing (Normal, Insert, Command, Search, Visual modes)
- RNA secondary structure visualization with helix coloring
- Multiple color schemes: none, structure, base, conservation, compensatory
- Split screen view (`:split`, `:vsplit`)
- Visual block selection and trim commands
- Sequence search with `/` and `n`/`N` navigation (U/T tolerance)
- File browser with `:e` command
- Column ruler and row numbers
- Base-pair highlighting
- Help overlay (`?`)
- Command history
- Splash screen with rainbow RNA helix logo
