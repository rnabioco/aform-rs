//! Application state and main loop.

use std::path::{Path, PathBuf};

use strum::AsRefStr;

use crate::color::Theme;
use crate::editor::History;
use crate::history::InputHistory;
use crate::stockholm::{Alignment, SequenceType};
use crate::structure::StructureCache;

/// Search state for pattern matching in sequences.
#[derive(Debug, Clone, Default)]
pub struct SearchState {
    /// Current search pattern.
    pub pattern: String,
    /// All match positions (row, start_col, end_col) - end_col is exclusive.
    pub matches: Vec<(usize, usize, usize)>,
    /// Current match index in matches.
    pub match_index: Option<usize>,
    /// Search history.
    pub history: InputHistory,
}

impl SearchState {
    /// Create a new empty search state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear search results and pattern.
    pub fn clear(&mut self) {
        self.pattern.clear();
        self.matches.clear();
        self.match_index = None;
    }

    /// Check if there's an active search with results.
    pub fn has_matches(&self) -> bool {
        !self.matches.is_empty() && !self.pattern.is_empty()
    }

    /// Navigate to previous history entry.
    pub fn history_prev(&mut self) {
        if let Some(entry) = self.history.prev(&self.pattern) {
            self.pattern = entry.to_string();
        }
    }

    /// Navigate to next history entry.
    pub fn history_next(&mut self) {
        if let Some(entry) = self.history.next() {
            self.pattern = entry.to_string();
        }
    }

    /// Check if a position is part of a search match.
    /// Returns Some(true) if it's the current match, Some(false) if it's another match, None if not a match.
    pub fn is_match(&self, row: usize, col: usize) -> Option<bool> {
        if !self.has_matches() {
            return None;
        }

        for (idx, &(match_row, start_col, end_col)) in self.matches.iter().enumerate() {
            if row == match_row && col >= start_col && col < end_col {
                return Some(self.match_index == Some(idx));
            }
        }

        None
    }
}

/// State for tab completion in command mode.
#[derive(Debug, Clone, Default)]
pub struct CompletionState {
    /// Available completion candidates.
    pub candidates: Vec<String>,
    /// Current index in candidates (for cycling).
    pub index: usize,
    /// Original prefix before completion started (for potential reset).
    #[allow(dead_code)]
    pub prefix: String,
}

/// Editor mode (vim-style).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, AsRefStr)]
#[strum(serialize_all = "UPPERCASE")]
pub enum Mode {
    #[default]
    Normal,
    Insert,
    Command,
    Search,
    /// Visual block selection mode.
    Visual,
}

/// Color scheme for the alignment display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum ColorScheme {
    #[default]
    None,
    /// Color by secondary structure (helix coloring).
    Structure,
    /// Color by base identity (A, C, G, U).
    Base,
    /// Color by conservation.
    Conservation,
    /// Color by compensatory changes.
    Compensatory,
    /// Color by per-residue posterior probability (#=GR PP).
    PP,
}

impl ColorScheme {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "none" | "off" => Some(ColorScheme::None),
            "structure" | "ss" => Some(ColorScheme::Structure),
            "base" | "nt" | "residue" | "aa" | "protein" => Some(ColorScheme::Base),
            "conservation" | "cons" => Some(ColorScheme::Conservation),
            "compensatory" | "comp" => Some(ColorScheme::Compensatory),
            "pp" | "probability" => Some(ColorScheme::PP),
            _ => None,
        }
    }
}

/// Split screen mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitMode {
    /// Top/bottom panes showing different rows.
    Horizontal,
    /// Left/right panes showing different columns.
    Vertical,
}

/// Which pane is currently active in split mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActivePane {
    #[default]
    Primary,
    Secondary,
}

/// Terminal color theme (detected at startup).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerminalTheme {
    /// Light background - use dark colors for contrast.
    Light,
    /// Dark background - use light colors for contrast.
    #[default]
    Dark,
}

/// Application state.
pub struct App {
    // === Public - Core data ===
    /// Current alignment.
    pub alignment: Alignment,
    /// File path (if loaded from file).
    pub file_path: Option<PathBuf>,
    /// Structure cache.
    pub structure_cache: StructureCache,

    // === Public - User configuration ===
    /// Gap character.
    pub gap_char: char,
    /// Characters considered as gaps.
    pub gap_chars: Vec<char>,
    /// Color scheme.
    pub color_scheme: ColorScheme,
    /// Show help overlay.
    pub show_help: bool,
    /// Show position ruler at top.
    pub show_ruler: bool,
    /// Show row numbers.
    pub show_row_numbers: bool,
    /// Show short IDs (strip coordinate suffix like /10000-20000).
    pub show_short_ids: bool,
    /// Reference sequence index for compensatory coloring.
    pub reference_seq: usize,
    /// Split screen mode (None = single pane).
    pub split_mode: Option<SplitMode>,
    /// Which pane is active in split mode.
    pub active_pane: ActivePane,

    // === Crate-internal ===
    /// Command line buffer (for command mode).
    pub(crate) command_buffer: String,
    /// Should quit.
    pub(crate) should_quit: bool,

    // === Internal state (crate-visible for impl App blocks) ===
    /// Whether the alignment has been modified.
    pub(crate) modified: bool,
    /// Current cursor row (sequence index).
    pub(crate) cursor_row: usize,
    /// Current cursor column.
    pub(crate) cursor_col: usize,
    /// Viewport offset (row).
    pub(crate) viewport_row: usize,
    /// Viewport offset (column).
    pub(crate) viewport_col: usize,
    /// Current editor mode.
    pub(crate) mode: Mode,
    /// Command history.
    pub(crate) command_history: InputHistory,
    /// Search state (pattern, matches, history).
    pub(crate) search: SearchState,
    /// Tab completion state for command mode.
    pub(crate) completion: Option<CompletionState>,
    /// Status message.
    pub(crate) status_message: Option<String>,
    /// Undo/redo history.
    pub(crate) history: History,
    /// Numeric count buffer for vim-style count prefixes (e.g., 50|).
    pub(crate) count_buffer: String,
    /// Secondary pane viewport row.
    pub(crate) secondary_viewport_row: usize,
    /// Secondary pane viewport column.
    pub(crate) secondary_viewport_col: usize,

    // === Visual selection state ===
    /// Selection anchor point (row, col) - set when entering visual mode.
    pub(crate) selection_anchor: Option<(usize, usize)>,
    /// Clipboard for yanked block (rectangular selection).
    pub(crate) clipboard: Option<Vec<Vec<char>>>,

    // === Clustering state ===
    /// Cluster-based display ordering (indices into alignment.sequences).
    /// When active, sequences are displayed in dendrogram order (similar sequences adjacent).
    pub(crate) cluster_order: Option<Vec<usize>>,
    /// Pre-computed ASCII tree characters for each display row.
    pub(crate) cluster_tree: Option<Vec<String>>,
    /// Pre-computed ASCII tree characters for collapsed view (one per group).
    pub(crate) collapsed_tree: Option<Vec<String>>,
    /// Width of the tree column in characters.
    pub(crate) tree_width: usize,
    /// Whether to show the dendrogram tree visualization.
    pub(crate) show_tree: bool,
    /// Group order when clustering with collapse (maps display_row -> group_index).
    pub(crate) cluster_group_order: Option<Vec<usize>>,
    /// Terminal color theme (detected at startup).
    pub terminal_theme: TerminalTheme,
    /// UI theme colors.
    pub theme: Theme,

    // === Collapse state ===
    /// Whether to collapse identical sequences in display.
    pub(crate) collapse_identical: bool,
    /// Mapping from display row to (representative_index, all_group_indices).
    pub(crate) collapse_groups: Vec<(usize, Vec<usize>)>,

    // === Annotation bar state ===
    /// Show consensus sequence bar.
    pub show_consensus: bool,
    /// Show conservation bar.
    pub show_conservation_bar: bool,
    /// Show RF (reference) annotation bar.
    pub show_rf_bar: bool,
    /// Show PP_cons (posterior probability consensus) bar.
    pub show_pp_cons: bool,
    /// Conservation threshold for uppercase in consensus (0.0-1.0).
    pub consensus_threshold: f64,

    // === Info overlay ===
    /// Show file info overlay.
    pub show_info: bool,

    // === Sequence type ===
    /// Detected sequence type (RNA, DNA, or Protein).
    pub sequence_type: SequenceType,

    // === Gap column state ===
    /// Highlight columns that contain only gaps.
    pub highlight_gap_columns: bool,
    /// Hide columns that contain only gaps from display.
    pub hide_gap_columns: bool,
    /// Precomputed list of visible (non-empty) column indices.
    /// Only populated when hide_gap_columns is true.
    pub(crate) visible_columns: Vec<usize>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            alignment: Alignment::new(),
            file_path: None,
            modified: false,
            cursor_row: 0,
            cursor_col: 0,
            viewport_row: 0,
            viewport_col: 0,
            mode: Mode::Normal,
            command_buffer: String::new(),
            command_history: InputHistory::new(),
            search: SearchState::new(),
            completion: None,
            status_message: None,
            gap_char: '.',
            gap_chars: vec!['.', '-', '_', '~', ':'],
            color_scheme: ColorScheme::None,
            structure_cache: StructureCache::new(),
            history: History::new(),
            should_quit: false,
            show_help: false,
            show_ruler: true,
            show_row_numbers: true,
            show_short_ids: false,
            reference_seq: 0,
            count_buffer: String::new(),
            split_mode: None,
            active_pane: ActivePane::Primary,
            secondary_viewport_row: 0,
            secondary_viewport_col: 0,
            selection_anchor: None,
            clipboard: None,
            cluster_order: None,
            cluster_tree: None,
            collapsed_tree: None,
            tree_width: 0,
            show_tree: false,
            cluster_group_order: None,
            terminal_theme: TerminalTheme::Dark,
            theme: Theme::default(),
            collapse_identical: false,
            collapse_groups: Vec::new(),
            show_consensus: false,
            show_conservation_bar: false,
            show_rf_bar: false,
            show_pp_cons: false,
            consensus_threshold: 0.7,
            show_info: false,
            sequence_type: SequenceType::RNA,
            highlight_gap_columns: false,
            hide_gap_columns: false,
            visible_columns: Vec::new(),
        }
    }
}

impl App {
    /// Create a new app with default state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load an alignment from a file.
    pub fn load_file(&mut self, path: &Path) -> Result<(), String> {
        let alignment = crate::stockholm::parser::parse_file(path)
            .map_err(|e| format!("Failed to parse file: {e}"))?;

        self.alignment = alignment;
        self.file_path = Some(path.to_path_buf());
        self.modified = false;
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.viewport_row = 0;
        self.viewport_col = 0;
        self.history.clear();

        // Reset collapse state
        self.collapse_identical = false;
        self.collapse_groups.clear();

        // Update structure cache (warn on parse errors)
        if let Some(ss) = self.alignment.ss_cons()
            && let Err(e) = self.structure_cache.update(ss)
        {
            eprintln!("Warning: Failed to parse SS_cons structure: {e}");
        }

        // Detect sequence type and precompute collapse groups
        self.detect_sequence_type();
        self.precompute_collapse_groups();

        self.set_status(format!(
            "Loaded {} ({} seqs, {:?}, SS_cons: {})",
            path.display(),
            self.alignment.num_sequences(),
            self.sequence_type,
            self.alignment.ss_cons().is_some()
        ));
        Ok(())
    }

    /// Save the alignment to a file.
    pub fn save_file(&mut self) -> Result<(), String> {
        let path = self.file_path.as_ref().ok_or("No file path set")?;
        crate::stockholm::writer::write_file(&self.alignment, path)
            .map_err(|e| format!("Failed to save file: {e}"))?;
        self.modified = false;
        self.set_status(format!("Saved {}", path.display()));
        Ok(())
    }

    /// Save the alignment to a new file.
    pub fn save_file_as(&mut self, path: PathBuf) -> Result<(), String> {
        crate::stockholm::writer::write_file(&self.alignment, &path)
            .map_err(|e| format!("Failed to save file: {e}"))?;
        self.file_path = Some(path.clone());
        self.modified = false;
        self.set_status(format!("Saved {}", path.display()));
        Ok(())
    }

    /// Set a status message.
    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = Some(message.into());
    }

    /// Clear the status message.
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Get the current character under the cursor.
    pub fn current_char(&self) -> Option<char> {
        let actual_row = self.display_to_actual_row(self.cursor_row);
        self.alignment.get_char(actual_row, self.cursor_col)
    }

    /// Check if the current character is a gap.
    pub fn is_current_gap(&self) -> bool {
        self.current_char()
            .map(|c| self.gap_chars.contains(&c))
            .unwrap_or(false)
    }

    /// Move cursor up.
    pub fn cursor_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
        }
    }

    /// Move cursor down.
    pub fn cursor_down(&mut self) {
        if self.cursor_row < self.visible_sequence_count().saturating_sub(1) {
            self.cursor_row += 1;
        }
    }

    /// Move cursor left.
    pub fn cursor_left(&mut self) {
        if self.hide_gap_columns && !self.visible_columns.is_empty() {
            // Find previous visible column
            if let Some(display_col) = self.actual_to_display_col(self.cursor_col)
                && display_col > 0
            {
                self.cursor_col = self.display_to_actual_col(display_col - 1);
            }
        } else if self.cursor_col > 0 {
            self.cursor_col -= 1;
        }
    }

    /// Move cursor right.
    pub fn cursor_right(&mut self) {
        if self.hide_gap_columns && !self.visible_columns.is_empty() {
            // Find next visible column
            if let Some(display_col) = self.actual_to_display_col(self.cursor_col)
                && display_col < self.visible_columns.len().saturating_sub(1)
            {
                self.cursor_col = self.display_to_actual_col(display_col + 1);
            }
        } else if self.cursor_col < self.alignment.width().saturating_sub(1) {
            self.cursor_col += 1;
        }
    }

    /// Move cursor to start of line.
    pub fn cursor_line_start(&mut self) {
        if self.hide_gap_columns && !self.visible_columns.is_empty() {
            // Go to first visible column
            self.cursor_col = self.display_to_actual_col(0);
        } else {
            self.cursor_col = 0;
        }
    }

    /// Move cursor to end of line.
    pub fn cursor_line_end(&mut self) {
        if self.hide_gap_columns && !self.visible_columns.is_empty() {
            // Go to last visible column
            self.cursor_col =
                self.display_to_actual_col(self.visible_columns.len().saturating_sub(1));
        } else {
            self.cursor_col = self.alignment.width().saturating_sub(1);
        }
    }

    /// Move cursor to first sequence.
    pub fn cursor_first_sequence(&mut self) {
        self.cursor_row = 0;
    }

    /// Move cursor to last sequence.
    pub fn cursor_last_sequence(&mut self) {
        self.cursor_row = self.visible_sequence_count().saturating_sub(1);
    }

    /// Jump to paired base.
    pub fn goto_pair(&mut self) {
        if let Some(paired) = self.structure_cache.get_pair(self.cursor_col) {
            self.cursor_col = paired;
        }
    }

    /// Jump to a specific column (1-indexed, like vim).
    pub fn goto_column(&mut self, col: usize) {
        if self.hide_gap_columns && !self.visible_columns.is_empty() {
            // When hiding, col refers to visible column index
            let max_display_col = self.visible_columns.len().saturating_sub(1);
            let target_display = col.saturating_sub(1).min(max_display_col);
            self.cursor_col = self.display_to_actual_col(target_display);
        } else {
            let max_col = self.alignment.width().saturating_sub(1);
            // Convert from 1-indexed to 0-indexed, clamping to valid range
            let target = col.saturating_sub(1).min(max_col);
            self.cursor_col = target;
        }
    }

    /// Jump to a specific row (1-indexed, like vim :N).
    pub fn goto_row(&mut self, row: usize) {
        let max_row = self.visible_sequence_count().saturating_sub(1);
        // Convert from 1-indexed to 0-indexed, clamping to valid range
        let target = row.saturating_sub(1).min(max_row);
        self.cursor_row = target;
    }

    /// Get the current count from the count buffer, or default to 1.
    pub fn take_count(&mut self) -> usize {
        let count = if self.count_buffer.is_empty() {
            1
        } else {
            self.count_buffer.parse().unwrap_or(1)
        };
        self.count_buffer.clear();
        count
    }

    /// Add a digit to the count buffer.
    pub fn push_count_digit(&mut self, digit: char) {
        self.count_buffer.push(digit);
    }

    /// Clear the count buffer.
    pub fn clear_count(&mut self) {
        self.count_buffer.clear();
    }

    /// Page down.
    pub fn page_down(&mut self, page_size: usize) {
        let max_row = self.visible_sequence_count().saturating_sub(1);
        self.cursor_row = (self.cursor_row + page_size).min(max_row);
    }

    /// Page up.
    pub fn page_up(&mut self, page_size: usize) {
        self.cursor_row = self.cursor_row.saturating_sub(page_size);
    }

    /// Half page down.
    pub fn half_page_down(&mut self, page_size: usize) {
        self.page_down(page_size / 2);
    }

    /// Half page up.
    pub fn half_page_up(&mut self, page_size: usize) {
        self.page_up(page_size / 2);
    }

    /// Scroll right.
    pub fn scroll_right(&mut self, amount: usize) {
        if self.hide_gap_columns && !self.visible_columns.is_empty() {
            // Move by visible columns
            if let Some(display_col) = self.actual_to_display_col(self.cursor_col) {
                let max_display = self.visible_columns.len().saturating_sub(1);
                let new_display = (display_col + amount).min(max_display);
                self.cursor_col = self.display_to_actual_col(new_display);
            }
        } else {
            let max_col = self.alignment.width().saturating_sub(1);
            self.cursor_col = (self.cursor_col + amount).min(max_col);
        }
    }

    /// Scroll left.
    pub fn scroll_left(&mut self, amount: usize) {
        if self.hide_gap_columns && !self.visible_columns.is_empty() {
            // Move by visible columns
            if let Some(display_col) = self.actual_to_display_col(self.cursor_col) {
                let new_display = display_col.saturating_sub(amount);
                self.cursor_col = self.display_to_actual_col(new_display);
            }
        } else {
            self.cursor_col = self.cursor_col.saturating_sub(amount);
        }
    }

    /// Enter insert mode.
    pub fn enter_insert_mode(&mut self) {
        self.mode = Mode::Insert;
    }

    /// Enter command mode.
    pub fn enter_command_mode(&mut self) {
        self.mode = Mode::Command;
        self.command_buffer.clear();
        self.command_history.reset_navigation();
    }

    /// Return to normal mode.
    pub fn enter_normal_mode(&mut self) {
        self.mode = Mode::Normal;
        self.command_buffer.clear();
    }

    /// Enter search mode.
    pub fn enter_search_mode(&mut self) {
        self.mode = Mode::Search;
        self.search.pattern.clear();
    }

    /// Enter visual selection mode.
    pub fn enter_visual_mode(&mut self) {
        self.mode = Mode::Visual;
        self.selection_anchor = Some((self.cursor_row, self.cursor_col));
    }

    /// Exit visual mode without taking action.
    pub fn exit_visual_mode(&mut self) {
        self.mode = Mode::Normal;
        self.selection_anchor = None;
    }

    /// Get the bounds of the current selection (`min_row`, `min_col`, `max_row`, `max_col`).
    /// Returns None if not in visual mode or no anchor set.
    pub fn get_selection_bounds(&self) -> Option<(usize, usize, usize, usize)> {
        let (anchor_row, anchor_col) = self.selection_anchor?;
        let min_row = anchor_row.min(self.cursor_row);
        let max_row = anchor_row.max(self.cursor_row);
        let min_col = anchor_col.min(self.cursor_col);
        let max_col = anchor_col.max(self.cursor_col);
        Some((min_row, min_col, max_row, max_col))
    }

    /// Check if a cell is within the current selection.
    pub fn is_selected(&self, row: usize, col: usize) -> bool {
        if self.mode != Mode::Visual {
            return false;
        }
        if let Some((min_row, min_col, max_row, max_col)) = self.get_selection_bounds() {
            row >= min_row && row <= max_row && col >= min_col && col <= max_col
        } else {
            false
        }
    }

    /// Get selection dimensions as a string for status bar.
    pub fn selection_info(&self) -> Option<String> {
        if self.mode != Mode::Visual {
            return None;
        }
        let (min_row, min_col, max_row, max_col) = self.get_selection_bounds()?;
        let rows = max_row - min_row + 1;
        let cols = max_col - min_col + 1;
        Some(format!("{rows}x{cols}"))
    }

    /// Yank (copy) the selected block to clipboard.
    pub fn yank_selection(&mut self) {
        let Some((min_row, min_col, max_row, max_col)) = self.get_selection_bounds() else {
            return;
        };

        let mut block = Vec::new();
        for row in min_row..=max_row {
            if let Some(seq) = self.alignment.sequences.get(row) {
                let chars: Vec<char> = (min_col..=max_col)
                    .map(|col| seq.get(col).unwrap_or(self.gap_char))
                    .collect();
                block.push(chars);
            }
        }

        let rows = block.len();
        let cols = if block.is_empty() { 0 } else { block[0].len() };
        self.clipboard = Some(block);
        self.exit_visual_mode();
        self.set_status(format!("Yanked {rows}x{cols} block"));
    }

    /// Delete the selected block (replace with gaps).
    pub fn delete_selection(&mut self) {
        let Some((min_row, min_col, max_row, max_col)) = self.get_selection_bounds() else {
            return;
        };

        // Save for undo
        self.history
            .save(&self.alignment, self.cursor_row, self.cursor_col);

        // Replace selected region with gaps
        for row in min_row..=max_row {
            if let Some(seq_rc) = self.alignment.sequences.get_mut(row) {
                let seq = std::rc::Rc::make_mut(seq_rc);
                for col in min_col..=max_col {
                    if col < seq.len() {
                        seq.set(col, self.gap_char);
                    }
                }
            }
        }

        let rows = max_row - min_row + 1;
        let cols = max_col - min_col + 1;
        self.modified = true;
        self.exit_visual_mode();
        self.set_status(format!("Deleted {rows}x{cols} block"));
    }

    /// Paste the clipboard at the cursor position.
    pub fn paste(&mut self) {
        let Some(ref block) = self.clipboard else {
            self.set_status("Nothing to paste");
            return;
        };

        // Save for undo
        self.history
            .save(&self.alignment, self.cursor_row, self.cursor_col);

        let block = block.clone();
        for (row_offset, row_data) in block.iter().enumerate() {
            let target_row = self.cursor_row + row_offset;
            if let Some(seq_rc) = self.alignment.sequences.get_mut(target_row) {
                let seq = std::rc::Rc::make_mut(seq_rc);
                for (col_offset, &ch) in row_data.iter().enumerate() {
                    let target_col = self.cursor_col + col_offset;
                    if target_col < seq.len() {
                        seq.set(target_col, ch);
                    }
                }
            }
        }

        let rows = block.len();
        let cols = if block.is_empty() { 0 } else { block[0].len() };
        self.modified = true;
        self.set_status(format!("Pasted {rows}x{cols} block"));
    }

    /// Clear search highlighting.
    pub fn clear_search(&mut self) {
        self.search.clear();
    }

    /// Execute the current search pattern.
    pub fn execute_search(&mut self) {
        if self.search.pattern.is_empty() {
            self.enter_normal_mode();
            return;
        }

        // Add to history (InputHistory handles deduplication)
        self.search.history.push(self.search.pattern.clone());

        self.search.matches = self.find_matches(&self.search.pattern.clone());

        if self.search.matches.is_empty() {
            self.set_status("Pattern not found (ignoring gaps)");
            self.search.match_index = None;
        } else {
            // Find first match at or after cursor position
            let first_match_idx = self
                .search
                .matches
                .iter()
                .position(|&(row, start_col, _)| {
                    (row, start_col) >= (self.cursor_row, self.cursor_col)
                })
                .unwrap_or(0);

            self.search.match_index = Some(first_match_idx);
            self.jump_to_current_match();
        }
    }

    /// Jump to the next search match relative to current cursor position.
    pub fn search_next(&mut self) {
        if self.search.matches.is_empty() {
            if !self.search.pattern.is_empty() {
                self.set_status("Pattern not found");
            }
            return;
        }

        // Find first match strictly after current cursor position
        let cursor_pos = (self.cursor_row, self.cursor_col);
        let next_idx = self
            .search
            .matches
            .iter()
            .position(|&(row, start_col, _)| (row, start_col) > cursor_pos)
            .unwrap_or(0); // Wrap to first match if none after cursor

        self.search.match_index = Some(next_idx);
        self.jump_to_current_match();
    }

    /// Jump to the previous search match relative to current cursor position.
    pub fn search_prev(&mut self) {
        if self.search.matches.is_empty() {
            if !self.search.pattern.is_empty() {
                self.set_status("Pattern not found");
            }
            return;
        }

        // Find last match strictly before current cursor position
        let cursor_pos = (self.cursor_row, self.cursor_col);
        let prev_idx = self
            .search
            .matches
            .iter()
            .rposition(|&(row, start_col, _)| (row, start_col) < cursor_pos)
            .unwrap_or(self.search.matches.len() - 1); // Wrap to last match if none before cursor

        self.search.match_index = Some(prev_idx);
        self.jump_to_current_match();
    }

    /// Find all matches of a pattern in the alignment.
    /// Case-insensitive, U/T tolerant (RNA/DNA equivalent), and ignores gap characters.
    /// Returns (row, start_col, end_col) where end_col is exclusive.
    fn find_matches(&self, pattern: &str) -> Vec<(usize, usize, usize)> {
        let pattern_normalized = Self::normalize_for_search(pattern);
        let pattern_chars: Vec<char> = pattern_normalized.chars().collect();
        let mut matches = Vec::new();

        if pattern_chars.is_empty() {
            return matches;
        }

        for (row, seq) in self.alignment.sequences.iter().enumerate() {
            let seq_chars: Vec<char> = seq.chars().to_vec();

            // Try matching starting at each position
            let mut col = 0;
            while col < seq_chars.len() {
                if let Some(end_col) = self.try_match_at(&seq_chars, col, &pattern_chars) {
                    matches.push((row, col, end_col));
                    // Move past the first non-gap character to find overlapping matches
                    col += 1;
                    while col < seq_chars.len() && self.gap_chars.contains(&seq_chars[col]) {
                        col += 1;
                    }
                } else {
                    col += 1;
                }
            }
        }

        matches
    }

    /// Try to match pattern starting at given column, skipping gaps.
    /// Returns the end column (exclusive) if match found, None otherwise.
    fn try_match_at(&self, seq: &[char], start_col: usize, pattern: &[char]) -> Option<usize> {
        let mut seq_idx = start_col;
        let mut pat_idx = 0;

        while pat_idx < pattern.len() {
            // Skip gaps in sequence
            while seq_idx < seq.len() && self.gap_chars.contains(&seq[seq_idx]) {
                seq_idx += 1;
            }

            if seq_idx >= seq.len() {
                return None; // Ran out of sequence
            }

            let seq_char = Self::normalize_char(seq[seq_idx]);
            if seq_char != pattern[pat_idx] {
                return None; // Mismatch
            }

            seq_idx += 1;
            pat_idx += 1;
        }

        Some(seq_idx)
    }

    /// Normalize a single character for search: uppercase and T→U.
    fn normalize_char(c: char) -> char {
        match c.to_ascii_uppercase() {
            'T' => 'U',
            other => other,
        }
    }

    /// Normalize a string for search: uppercase and T→U for RNA/DNA equivalence.
    fn normalize_for_search(s: &str) -> String {
        s.to_uppercase().replace('T', "U")
    }

    /// Check if a position is part of a search match.
    /// Returns Some(true) if it's the current match, Some(false) if it's another match, None if not a match.
    pub fn is_search_match(&self, row: usize, col: usize) -> Option<bool> {
        self.search.is_match(row, col)
    }

    /// Jump to the current match and update status.
    fn jump_to_current_match(&mut self) {
        if let Some(idx) = self.search.match_index
            && let Some(&(row, start_col, _end_col)) = self.search.matches.get(idx)
        {
            self.cursor_row = row;
            self.cursor_col = start_col;
            self.set_status(format!(
                "Match {}/{} (ignoring gaps)",
                idx + 1,
                self.search.matches.len()
            ));
        }
    }

    /// Execute a command from command mode.
    pub fn execute_command(&mut self) {
        let command = self.command_buffer.trim().to_string();
        self.command_buffer.clear();
        self.mode = Mode::Normal;

        if command.is_empty() {
            return;
        }

        // Add to history (InputHistory handles deduplication)
        self.command_history.push(command.clone());

        let parts: Vec<&str> = command.split_whitespace().collect();

        // Try each command category in order
        if self.execute_file_command(&parts, &command) {
            return;
        }
        if self.execute_display_command(&parts) {
            return;
        }
        if self.execute_transform_command(&parts) {
            return;
        }
        if self.execute_clustering_command(&parts) {
            return;
        }

        // Fallback: check for line number or unknown command
        if let Ok(line_num) = command.parse::<usize>() {
            self.goto_row(line_num);
        } else {
            self.set_status(format!("Unknown command: {command}"));
        }
    }

    /// Execute file-related commands (quit, write, edit). Returns true if handled.
    fn execute_file_command(&mut self, parts: &[&str], command: &str) -> bool {
        match parts {
            ["q" | "quit"] => {
                if self.split_mode.is_some() {
                    self.close_split();
                } else if self.modified {
                    self.set_status("No write since last change (use :q! to force)");
                } else {
                    self.should_quit = true;
                }
                true
            }
            ["q!"] => {
                if self.split_mode.is_some() {
                    self.close_split();
                } else {
                    self.should_quit = true;
                }
                true
            }
            ["w" | "write"] => {
                if let Err(e) = self.save_file() {
                    self.set_status(e);
                }
                true
            }
            ["w", path] => {
                if let Err(e) = self.save_file_as(PathBuf::from(*path)) {
                    self.set_status(e);
                }
                true
            }
            ["wq"] => {
                if let Err(e) = self.save_file() {
                    self.set_status(e);
                } else {
                    self.should_quit = true;
                }
                true
            }
            ["e" | "edit"] => {
                self.set_status("Usage: :e <path> (Tab to complete)");
                true
            }
            ["e" | "edit", path] => {
                if let Err(e) = self.load_file(Path::new(path)) {
                    self.set_status(e);
                }
                true
            }
            ["noh" | "nohlsearch"] => {
                self.clear_search();
                true
            }
            _ if command.starts_with('!') => {
                self.set_status("Shell commands not supported");
                true
            }
            _ => false,
        }
    }

    /// Execute display-related commands (ruler, rownum, color, etc.). Returns true if handled.
    fn execute_display_command(&mut self, parts: &[&str]) -> bool {
        match parts {
            ["?" | "help"] => {
                self.show_help = true;
                true
            }
            ["ruler"] => {
                self.show_ruler = !self.show_ruler;
                self.set_status(format!(
                    "Ruler: {}",
                    if self.show_ruler { "on" } else { "off" }
                ));
                true
            }
            ["rownum"] => {
                self.show_row_numbers = !self.show_row_numbers;
                self.set_status(format!(
                    "Row numbers: {}",
                    if self.show_row_numbers { "on" } else { "off" }
                ));
                true
            }
            ["shortid"] => {
                self.show_short_ids = !self.show_short_ids;
                self.set_status(format!(
                    "Short IDs: {}",
                    if self.show_short_ids { "on" } else { "off" }
                ));
                true
            }
            ["consensus"] => {
                self.show_consensus = !self.show_consensus;
                self.set_status(format!(
                    "Consensus bar: {}",
                    if self.show_consensus { "on" } else { "off" }
                ));
                true
            }
            ["conservation"] | ["consbar"] => {
                self.show_conservation_bar = !self.show_conservation_bar;
                self.set_status(format!(
                    "Conservation bar: {}",
                    if self.show_conservation_bar {
                        "on"
                    } else {
                        "off"
                    }
                ));
                true
            }
            ["rf"] => {
                self.show_rf_bar = !self.show_rf_bar;
                self.set_status(format!(
                    "RF bar: {}",
                    if self.show_rf_bar { "on" } else { "off" }
                ));
                true
            }
            ["ppcons"] | ["pp_cons"] => {
                self.show_pp_cons = !self.show_pp_cons;
                self.set_status(format!(
                    "PP_cons bar: {}",
                    if self.show_pp_cons { "on" } else { "off" }
                ));
                true
            }
            ["info"] => {
                self.show_info = !self.show_info;
                true
            }
            ["gapcols"] | ["gapcol"] => {
                self.highlight_gap_columns = !self.highlight_gap_columns;
                self.set_status(format!(
                    "Gap column highlighting: {}",
                    if self.highlight_gap_columns {
                        "on"
                    } else {
                        "off"
                    }
                ));
                true
            }
            ["hidegaps"] | ["hidegap"] => {
                self.hide_gap_columns = !self.hide_gap_columns;
                self.precompute_visible_columns();
                // Ensure cursor is on a visible column
                if self.hide_gap_columns
                    && !self.visible_columns.is_empty()
                    && self.actual_to_display_col(self.cursor_col).is_none()
                {
                    // Snap to nearest visible column
                    self.cursor_col = self.visible_columns.first().copied().unwrap_or(0);
                }
                self.set_status(format!(
                    "Hide gap columns: {}",
                    if self.hide_gap_columns { "on" } else { "off" }
                ));
                true
            }
            ["color", scheme] => {
                if let Some(s) = ColorScheme::from_str(scheme) {
                    self.color_scheme = s;
                    self.set_status(format!("Color scheme: {}", s.as_ref()));
                } else {
                    self.set_status(format!("Unknown color scheme: {scheme}"));
                }
                true
            }
            ["type"] => {
                self.set_status(format!("Sequence type: {:?}", self.sequence_type));
                true
            }
            ["type", t] => {
                self.execute_type_command(t);
                true
            }
            ["set", setting] => {
                self.execute_set_command(setting);
                true
            }
            ["split" | "sp"] => {
                self.horizontal_split();
                true
            }
            ["vsplit" | "vs" | "vsp"] => {
                self.vertical_split();
                true
            }
            ["only"] => {
                self.close_split();
                true
            }
            _ => false,
        }
    }

    /// Execute sequence type command.
    fn execute_type_command(&mut self, t: &str) {
        match t.to_lowercase().as_str() {
            "rna" => {
                self.sequence_type = SequenceType::RNA;
                self.set_status("Sequence type: RNA");
            }
            "dna" => {
                self.sequence_type = SequenceType::DNA;
                self.set_status("Sequence type: DNA");
            }
            "protein" | "aa" => {
                self.sequence_type = SequenceType::Protein;
                self.set_status("Sequence type: Protein");
            }
            "auto" => {
                self.detect_sequence_type();
                self.set_status(format!("Detected sequence type: {:?}", self.sequence_type));
            }
            _ => {
                self.set_status(format!(
                    "Unknown sequence type: {} (use rna, dna, protein, or auto)",
                    t
                ));
            }
        }
    }

    /// Execute set command (key=value settings).
    fn execute_set_command(&mut self, setting: &str) {
        if let Some((key, value)) = setting.split_once('=') {
            match key {
                "gap" => {
                    if let Some(c) = value.chars().next() {
                        self.gap_char = c;
                        self.set_status(format!("Gap character: '{c}'"));
                    }
                }
                _ => {
                    self.set_status(format!("Unknown setting: {key}"));
                }
            }
        }
    }

    /// Execute alignment transformation commands. Returns true if handled.
    fn execute_transform_command(&mut self, parts: &[&str]) -> bool {
        match parts {
            ["upper" | "uppercase"] => {
                self.uppercase_alignment();
                self.set_status("Converted to uppercase");
                true
            }
            ["lower" | "lowercase"] => {
                self.lowercase_alignment();
                self.set_status("Converted to lowercase");
                true
            }
            ["t2u"] => {
                self.convert_t_to_u();
                self.set_status("Converted T to U");
                true
            }
            ["u2t"] => {
                self.convert_u_to_t();
                self.set_status("Converted U to T");
                true
            }
            ["trimleft"] => {
                self.trim_left();
                true
            }
            ["trimright"] => {
                self.trim_right();
                true
            }
            ["trim"] => {
                self.trim();
                true
            }
            _ => false,
        }
    }

    /// Execute clustering-related commands. Returns true if handled.
    fn execute_clustering_command(&mut self, parts: &[&str]) -> bool {
        match parts {
            ["cluster"] => {
                self.cluster_sequences();
                self.set_status(format!(
                    "Clustered {} sequences by similarity",
                    self.alignment.num_sequences()
                ));
                true
            }
            ["uncluster"] => {
                self.uncluster();
                self.set_status("Clustering disabled");
                true
            }
            ["tree"] => {
                self.toggle_tree();
                if self.show_tree {
                    self.set_status("Tree visible");
                } else if self.cluster_tree.is_some() {
                    self.set_status("Tree hidden");
                }
                true
            }
            ["collapse"] => {
                self.toggle_collapse_identical();
                true
            }
            _ => false,
        }
    }

    /// Toggle help display.
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /// Enable horizontal split (top/bottom panes).
    pub fn horizontal_split(&mut self) {
        if self.split_mode.is_none() {
            // Initialize secondary viewport to current position
            self.secondary_viewport_row = self.viewport_row;
            self.secondary_viewport_col = self.viewport_col;
        }
        self.split_mode = Some(SplitMode::Horizontal);
        self.set_status("Horizontal split");
    }

    /// Enable vertical split (left/right panes).
    pub fn vertical_split(&mut self) {
        if self.split_mode.is_none() {
            // Initialize secondary viewport to current position
            self.secondary_viewport_row = self.viewport_row;
            self.secondary_viewport_col = self.viewport_col;
        }
        self.split_mode = Some(SplitMode::Vertical);
        self.set_status("Vertical split");
    }

    /// Close split and return to single pane.
    pub fn close_split(&mut self) {
        self.split_mode = None;
        self.active_pane = ActivePane::Primary;
        self.set_status("Split closed");
    }

    /// Switch between panes in split mode.
    pub fn switch_pane(&mut self) {
        if self.split_mode.is_some() {
            // Swap active pane and viewport positions
            self.active_pane = match self.active_pane {
                ActivePane::Primary => ActivePane::Secondary,
                ActivePane::Secondary => ActivePane::Primary,
            };
            // Swap cursor into the other viewport
            std::mem::swap(&mut self.viewport_row, &mut self.secondary_viewport_row);
            std::mem::swap(&mut self.viewport_col, &mut self.secondary_viewport_col);
        }
    }

    /// Navigate to previous command in history (Up arrow).
    pub fn command_history_prev(&mut self) {
        if let Some(entry) = self.command_history.prev(&self.command_buffer) {
            self.command_buffer = entry.to_string();
        }
    }

    /// Navigate to next command in history (Down arrow).
    pub fn command_history_next(&mut self) {
        if let Some(entry) = self.command_history.next() {
            self.command_buffer = entry.to_string();
        }
    }

    /// Navigate to previous search in history (Up arrow).
    pub fn search_history_prev(&mut self) {
        self.search.history_prev();
    }

    /// Navigate to next search in history (Down arrow).
    pub fn search_history_next(&mut self) {
        self.search.history_next();
    }

    /// Mark the alignment as modified.
    pub fn mark_modified(&mut self) {
        self.modified = true;
    }

    /// Update the structure cache if needed.
    pub fn update_structure_cache(&mut self) {
        if let Some(ss) = self.alignment.ss_cons()
            && !self.structure_cache.is_valid_for(ss)
            && let Err(e) = self.structure_cache.update(ss)
        {
            // Structure parsing failed - show status to user
            self.set_status(format!("Warning: SS_cons parse error: {e}"));
        }
    }

    /// Ensure cursor is within bounds.
    pub fn clamp_cursor(&mut self) {
        let max_row = self.visible_sequence_count().saturating_sub(1);
        self.cursor_row = self.cursor_row.min(max_row);

        if self.hide_gap_columns && !self.visible_columns.is_empty() {
            // Ensure cursor is on a visible column
            let max_display_col = self.visible_columns.len().saturating_sub(1);
            if self.actual_to_display_col(self.cursor_col).is_none() {
                // Snap to nearest visible column
                self.cursor_col = self
                    .visible_columns
                    .first()
                    .copied()
                    .unwrap_or(0)
                    .min(self.display_to_actual_col(max_display_col));
            }
        } else {
            let max_col = self.alignment.width().saturating_sub(1);
            self.cursor_col = self.cursor_col.min(max_col);
        }
    }

    /// Adjust viewport to keep cursor visible.
    pub fn adjust_viewport(&mut self, visible_rows: usize, visible_cols: usize) {
        // Vertical scrolling
        if self.cursor_row < self.viewport_row {
            self.viewport_row = self.cursor_row;
        } else if self.cursor_row >= self.viewport_row + visible_rows {
            self.viewport_row = self.cursor_row - visible_rows + 1;
        }

        // Horizontal scrolling
        if self.hide_gap_columns && !self.visible_columns.is_empty() {
            // When hiding, viewport_col is in display column space
            if let Some(cursor_display_col) = self.actual_to_display_col(self.cursor_col) {
                if cursor_display_col < self.viewport_col {
                    self.viewport_col = cursor_display_col;
                } else if cursor_display_col >= self.viewport_col + visible_cols {
                    self.viewport_col = cursor_display_col - visible_cols + 1;
                }
            }
        } else {
            // Normal mode - viewport_col is actual column
            if self.cursor_col < self.viewport_col {
                self.viewport_col = self.cursor_col;
            } else if self.cursor_col >= self.viewport_col + visible_cols {
                self.viewport_col = self.cursor_col - visible_cols + 1;
            }
        }
    }

    // === Clustering methods ===

    /// Map display row to actual sequence index.
    /// When collapse is active, maps to representative. When clustering is active, uses cluster order.
    pub fn display_to_actual_row(&self, display_row: usize) -> usize {
        if self.collapse_identical && !self.collapse_groups.is_empty() {
            // When clustering is also enabled, use group_order to find correct group
            let group_idx = if let Some(ref group_order) = self.cluster_group_order {
                group_order.get(display_row).copied().unwrap_or(display_row)
            } else {
                display_row
            };

            // Get representative from the group
            if group_idx < self.collapse_groups.len() {
                self.collapse_groups[group_idx].0
            } else {
                display_row
            }
        } else if let Some(ref order) = self.cluster_order {
            // Only clustering, no collapse
            order.get(display_row).copied().unwrap_or(display_row)
        } else {
            // No collapse, no clustering
            display_row
        }
    }

    /// Get the number of visible sequences (accounts for collapse).
    pub fn visible_sequence_count(&self) -> usize {
        if self.collapse_identical && !self.collapse_groups.is_empty() {
            self.collapse_groups.len()
        } else {
            self.alignment.num_sequences()
        }
    }

    /// Cluster sequences by similarity using hierarchical clustering.
    /// Uses precomputed collapse groups to avoid redundant distance calculations.
    pub fn cluster_sequences(&mut self) {
        if self.alignment.sequences.is_empty() {
            return;
        }

        // Get sequence chars for clustering
        let seq_chars: Vec<Vec<char>> = self
            .alignment
            .sequences
            .iter()
            .map(|s| s.chars().to_vec())
            .collect();

        // Compute cluster order and tree using UPGMA
        // Use collapse groups to cluster only unique sequences (optimization)
        let result = crate::clustering::cluster_sequences_with_collapse(
            &seq_chars,
            &self.gap_chars,
            &self.collapse_groups,
        );
        self.cluster_order = Some(result.order);
        self.cluster_tree = Some(result.tree_lines);
        self.collapsed_tree = result.collapsed_tree_lines;
        self.tree_width = result.tree_width;
        self.cluster_group_order = result.group_order;

        // Clamp cursor to valid range
        if self.cursor_row >= self.visible_sequence_count() {
            self.cursor_row = self.visible_sequence_count().saturating_sub(1);
        }
    }

    /// Disable clustering and restore original order.
    pub fn uncluster(&mut self) {
        self.cluster_order = None;
        self.cluster_tree = None;
        self.collapsed_tree = None;
        self.tree_width = 0;
        self.show_tree = false;
        self.cluster_group_order = None;
    }

    /// Toggle dendrogram tree visibility.
    pub fn toggle_tree(&mut self) {
        if self.cluster_tree.is_some() {
            self.show_tree = !self.show_tree;
        } else {
            self.status_message = Some("No tree available. Run :cluster first.".to_string());
        }
    }

    // === Collapse identical sequences ===

    /// Pre-compute collapse groups by grouping sequences with identical content.
    /// Called during load since sequences don't change during viewing.
    pub fn precompute_collapse_groups(&mut self) {
        use std::collections::HashMap;
        self.collapse_groups.clear();

        if self.alignment.sequences.is_empty() {
            return;
        }

        // Group by sequence content (chars as String for hashing)
        let mut content_map: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, seq) in self.alignment.sequences.iter().enumerate() {
            content_map.entry(seq.data()).or_default().push(idx);
        }

        // Build groups preserving original order (first occurrence is representative)
        let mut seen = std::collections::HashSet::new();
        for (idx, seq) in self.alignment.sequences.iter().enumerate() {
            let content = seq.data();
            if seen.insert(content.clone()) {
                let indices = content_map.remove(&content).unwrap();
                self.collapse_groups.push((idx, indices));
            }
        }
    }

    /// Get collapse count for a display row (1 if not collapsed or unique).
    pub fn get_collapse_count(&self, display_row: usize) -> usize {
        if self.collapse_identical && !self.collapse_groups.is_empty() {
            // When clustering is also enabled, use group_order to find correct group
            let group_idx = if let Some(ref group_order) = self.cluster_group_order {
                group_order.get(display_row).copied().unwrap_or(display_row)
            } else {
                display_row
            };

            if group_idx < self.collapse_groups.len() {
                self.collapse_groups[group_idx].1.len()
            } else {
                1
            }
        } else {
            1
        }
    }

    /// Get the maximum collapse count across all groups.
    pub fn max_collapse_count(&self) -> usize {
        if self.collapse_identical {
            self.collapse_groups
                .iter()
                .map(|(_, g)| g.len())
                .max()
                .unwrap_or(1)
        } else {
            1
        }
    }

    /// Toggle collapse identical sequences.
    pub fn toggle_collapse_identical(&mut self) {
        self.collapse_identical = !self.collapse_identical;
        // Groups are pre-computed during load, just flip the flag

        // Clamp cursor to visible range
        if self.cursor_row >= self.visible_sequence_count() {
            self.cursor_row = self.visible_sequence_count().saturating_sub(1);
        }

        let msg = if self.collapse_identical {
            format!(
                "Collapsed {} sequences into {} groups",
                self.alignment.num_sequences(),
                self.collapse_groups.len()
            )
        } else {
            "Collapse disabled".to_string()
        };
        self.status_message = Some(msg);
    }

    // === Sequence type detection ===

    /// Detect sequence type from alignment content.
    pub fn detect_sequence_type(&mut self) {
        self.sequence_type = crate::color::detect_sequence_type(&self.alignment, &self.gap_chars);
    }

    // === Gap column methods ===

    /// Precompute visible columns (call after loading alignment or toggling hide_gap_columns).
    pub fn precompute_visible_columns(&mut self) {
        if self.hide_gap_columns {
            self.visible_columns = (0..self.alignment.width())
                .filter(|&col| !self.alignment.is_empty_column(col, &self.gap_chars))
                .collect();
        } else {
            self.visible_columns.clear();
        }
    }

    /// Map display column index to actual column index.
    pub fn display_to_actual_col(&self, display_col: usize) -> usize {
        if self.hide_gap_columns && !self.visible_columns.is_empty() {
            self.visible_columns
                .get(display_col)
                .copied()
                .unwrap_or(display_col)
        } else {
            display_col
        }
    }

    /// Map actual column index to display column index (returns None if hidden).
    pub fn actual_to_display_col(&self, actual_col: usize) -> Option<usize> {
        if self.hide_gap_columns && !self.visible_columns.is_empty() {
            self.visible_columns.iter().position(|&c| c == actual_col)
        } else {
            Some(actual_col)
        }
    }

    /// Get number of visible columns.
    #[allow(dead_code)] // Part of public API for gap column hiding
    pub fn visible_column_count(&self) -> usize {
        if self.hide_gap_columns && !self.visible_columns.is_empty() {
            self.visible_columns.len()
        } else {
            self.alignment.width()
        }
    }
}
