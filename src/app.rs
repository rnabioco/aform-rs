//! Application state and main loop.

use std::path::{Path, PathBuf};

use ratatui_explorer::FileExplorer;

use crate::editor::History;
use crate::stockholm::{Alignment, SequenceType};
use crate::structure::StructureCache;

/// Editor mode (vim-style).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
    Command,
    Search,
    /// File browser mode for opening files.
    Browse,
    /// Visual block selection mode.
    Visual,
}

impl Mode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
            Mode::Command => "COMMAND",
            Mode::Search => "SEARCH",
            Mode::Browse => "BROWSE",
            Mode::Visual => "VISUAL",
        }
    }
}

/// Color scheme for the alignment display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
}

impl ColorScheme {
    pub fn as_str(&self) -> &'static str {
        match self {
            ColorScheme::None => "none",
            ColorScheme::Structure => "structure",
            ColorScheme::Base => "base",
            ColorScheme::Conservation => "conservation",
            ColorScheme::Compensatory => "compensatory",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "none" | "off" => Some(ColorScheme::None),
            "structure" | "ss" => Some(ColorScheme::Structure),
            "base" | "nt" | "residue" | "aa" | "protein" => Some(ColorScheme::Base),
            "conservation" | "cons" => Some(ColorScheme::Conservation),
            "compensatory" | "comp" => Some(ColorScheme::Compensatory),
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
    pub(crate) command_history: Vec<String>,
    /// Current position in command history (None = new command).
    pub(crate) command_history_index: Option<usize>,
    /// Saved command buffer when browsing history.
    pub(crate) command_history_saved: String,
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

    // === Search state ===
    /// Current search pattern.
    pub(crate) search_pattern: String,
    /// All match positions (row, start_col, end_col) - end_col is exclusive.
    pub(crate) search_matches: Vec<(usize, usize, usize)>,
    /// Current match index in search_matches.
    pub(crate) search_match_index: Option<usize>,

    // === File browser state ===
    /// File explorer for browsing files.
    pub(crate) file_explorer: Option<FileExplorer>,

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
    /// Width of the tree column in characters.
    pub(crate) tree_width: usize,
    /// Whether to show the dendrogram tree visualization.
    pub(crate) show_tree: bool,
    /// Terminal color theme (detected at startup).
    pub terminal_theme: TerminalTheme,

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
    /// Conservation threshold for uppercase in consensus (0.0-1.0).
    pub consensus_threshold: f64,

    // === Sequence type ===
    /// Detected sequence type (RNA, DNA, or Protein).
    pub sequence_type: SequenceType,
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
            command_history: Vec::new(),
            command_history_index: None,
            command_history_saved: String::new(),
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
            reference_seq: 0,
            count_buffer: String::new(),
            split_mode: None,
            active_pane: ActivePane::Primary,
            secondary_viewport_row: 0,
            secondary_viewport_col: 0,
            search_pattern: String::new(),
            search_matches: Vec::new(),
            search_match_index: None,
            file_explorer: None,
            selection_anchor: None,
            clipboard: None,
            cluster_order: None,
            cluster_tree: None,
            tree_width: 0,
            show_tree: false,
            terminal_theme: TerminalTheme::Dark,
            collapse_identical: false,
            collapse_groups: Vec::new(),
            show_consensus: false,
            show_conservation_bar: false,
            consensus_threshold: 0.7,
            sequence_type: SequenceType::RNA,
        }
    }
}

impl App {
    /// Create a new app with default state.
    pub fn new() -> Self {
        Self::default()
    }

    // === Getters for internal state (public API for external crates) ===

    /// Get whether the alignment has been modified.
    #[allow(dead_code)] // Public API
    pub fn modified(&self) -> bool {
        self.modified
    }

    /// Get current cursor row.
    #[allow(dead_code)] // Public API
    pub fn cursor_row(&self) -> usize {
        self.cursor_row
    }

    /// Get current cursor column.
    #[allow(dead_code)] // Public API
    pub fn cursor_col(&self) -> usize {
        self.cursor_col
    }

    /// Get viewport row offset.
    #[allow(dead_code)] // Public API
    pub fn viewport_row(&self) -> usize {
        self.viewport_row
    }

    /// Get viewport column offset.
    #[allow(dead_code)] // Public API
    pub fn viewport_col(&self) -> usize {
        self.viewport_col
    }

    /// Get current editor mode.
    #[allow(dead_code)] // Public API
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Get current status message.
    #[allow(dead_code)] // Public API
    pub fn status_message(&self) -> Option<&str> {
        self.status_message.as_deref()
    }

    /// Get secondary viewport row offset.
    #[allow(dead_code)] // Public API
    pub fn secondary_viewport_row(&self) -> usize {
        self.secondary_viewport_row
    }

    /// Get secondary viewport column offset.
    #[allow(dead_code)] // Public API
    pub fn secondary_viewport_col(&self) -> usize {
        self.secondary_viewport_col
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

        // Update structure cache
        if let Some(ss) = self.alignment.ss_cons() {
            let _ = self.structure_cache.update(ss);
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
        self.alignment.get_char(self.cursor_row, self.cursor_col)
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
        if self.cursor_row < self.alignment.num_sequences().saturating_sub(1) {
            self.cursor_row += 1;
        }
    }

    /// Move cursor left.
    pub fn cursor_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        }
    }

    /// Move cursor right.
    pub fn cursor_right(&mut self) {
        if self.cursor_col < self.alignment.width().saturating_sub(1) {
            self.cursor_col += 1;
        }
    }

    /// Move cursor to start of line.
    pub fn cursor_line_start(&mut self) {
        self.cursor_col = 0;
    }

    /// Move cursor to end of line.
    pub fn cursor_line_end(&mut self) {
        self.cursor_col = self.alignment.width().saturating_sub(1);
    }

    /// Move cursor to first sequence.
    pub fn cursor_first_sequence(&mut self) {
        self.cursor_row = 0;
    }

    /// Move cursor to last sequence.
    pub fn cursor_last_sequence(&mut self) {
        self.cursor_row = self.alignment.num_sequences().saturating_sub(1);
    }

    /// Jump to paired base.
    pub fn goto_pair(&mut self) {
        if let Some(paired) = self.structure_cache.get_pair(self.cursor_col) {
            self.cursor_col = paired;
        }
    }

    /// Jump to a specific column (1-indexed, like vim).
    pub fn goto_column(&mut self, col: usize) {
        let max_col = self.alignment.width().saturating_sub(1);
        // Convert from 1-indexed to 0-indexed, clamping to valid range
        let target = col.saturating_sub(1).min(max_col);
        self.cursor_col = target;
    }

    /// Jump to a specific row (1-indexed, like vim :N).
    pub fn goto_row(&mut self, row: usize) {
        let max_row = self.alignment.num_sequences().saturating_sub(1);
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
        let max_row = self.alignment.num_sequences().saturating_sub(1);
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
        let max_col = self.alignment.width().saturating_sub(1);
        self.cursor_col = (self.cursor_col + amount).min(max_col);
    }

    /// Scroll left.
    pub fn scroll_left(&mut self, amount: usize) {
        self.cursor_col = self.cursor_col.saturating_sub(amount);
    }

    /// Enter insert mode.
    pub fn enter_insert_mode(&mut self) {
        self.mode = Mode::Insert;
    }

    /// Enter command mode.
    pub fn enter_command_mode(&mut self) {
        self.mode = Mode::Command;
        self.command_buffer.clear();
        self.command_history_index = None;
        self.command_history_saved.clear();
    }

    /// Return to normal mode.
    pub fn enter_normal_mode(&mut self) {
        self.mode = Mode::Normal;
        self.command_buffer.clear();
    }

    /// Enter search mode.
    pub fn enter_search_mode(&mut self) {
        self.mode = Mode::Search;
        self.search_pattern.clear();
    }

    /// Enter file browser mode.
    pub fn enter_browse_mode(&mut self) {
        match FileExplorer::new() {
            Ok(explorer) => {
                self.file_explorer = Some(explorer);
                self.mode = Mode::Browse;
            }
            Err(e) => {
                self.set_status(format!("Failed to open file browser: {e}"));
            }
        }
    }

    /// Exit file browser mode without selecting a file.
    pub fn exit_browse_mode(&mut self) {
        self.file_explorer = None;
        self.mode = Mode::Normal;
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
        self.search_pattern.clear();
        self.search_matches.clear();
        self.search_match_index = None;
    }

    /// Execute the current search pattern.
    pub fn execute_search(&mut self) {
        if self.search_pattern.is_empty() {
            self.enter_normal_mode();
            return;
        }

        self.search_matches = self.find_matches(&self.search_pattern.clone());

        if self.search_matches.is_empty() {
            self.set_status("Pattern not found (ignoring gaps)");
            self.search_match_index = None;
        } else {
            // Find first match at or after cursor position
            let first_match_idx = self
                .search_matches
                .iter()
                .position(|&(row, start_col, _)| {
                    (row, start_col) >= (self.cursor_row, self.cursor_col)
                })
                .unwrap_or(0);

            self.search_match_index = Some(first_match_idx);
            self.jump_to_current_match();
        }
    }

    /// Jump to the next search match relative to current cursor position.
    pub fn search_next(&mut self) {
        if self.search_matches.is_empty() {
            if !self.search_pattern.is_empty() {
                self.set_status("Pattern not found");
            }
            return;
        }

        // Find first match strictly after current cursor position
        let cursor_pos = (self.cursor_row, self.cursor_col);
        let next_idx = self
            .search_matches
            .iter()
            .position(|&(row, start_col, _)| (row, start_col) > cursor_pos)
            .unwrap_or(0); // Wrap to first match if none after cursor

        self.search_match_index = Some(next_idx);
        self.jump_to_current_match();
    }

    /// Jump to the previous search match relative to current cursor position.
    pub fn search_prev(&mut self) {
        if self.search_matches.is_empty() {
            if !self.search_pattern.is_empty() {
                self.set_status("Pattern not found");
            }
            return;
        }

        // Find last match strictly before current cursor position
        let cursor_pos = (self.cursor_row, self.cursor_col);
        let prev_idx = self
            .search_matches
            .iter()
            .rposition(|&(row, start_col, _)| (row, start_col) < cursor_pos)
            .unwrap_or(self.search_matches.len() - 1); // Wrap to last match if none before cursor

        self.search_match_index = Some(prev_idx);
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
        if self.search_matches.is_empty() || self.search_pattern.is_empty() {
            return None;
        }

        let current_idx = self.search_match_index;

        for (idx, &(match_row, start_col, end_col)) in self.search_matches.iter().enumerate() {
            if row == match_row && col >= start_col && col < end_col {
                return Some(current_idx == Some(idx));
            }
        }

        None
    }

    /// Jump to the current match and update status.
    fn jump_to_current_match(&mut self) {
        if let Some(idx) = self.search_match_index
            && let Some(&(row, start_col, _end_col)) = self.search_matches.get(idx)
        {
            self.cursor_row = row;
            self.cursor_col = start_col;
            self.set_status(format!(
                "Match {}/{} (ignoring gaps)",
                idx + 1,
                self.search_matches.len()
            ));
        }
    }

    /// Execute a command from command mode.
    pub fn execute_command(&mut self) {
        let command = self.command_buffer.trim().to_string();
        self.command_buffer.clear();
        self.command_history_index = None;
        self.mode = Mode::Normal;

        if command.is_empty() {
            return;
        }

        // Add to history (avoid consecutive duplicates)
        if self.command_history.last() != Some(&command) {
            self.command_history.push(command.clone());
        }

        let parts: Vec<&str> = command.split_whitespace().collect();
        match parts.as_slice() {
            ["q" | "quit"] => {
                if self.split_mode.is_some() {
                    // In split mode, :q closes the current pane
                    self.close_split();
                } else if self.modified {
                    self.set_status("No write since last change (use :q! to force)");
                } else {
                    self.should_quit = true;
                }
            }
            ["q!"] => {
                if self.split_mode.is_some() {
                    // In split mode, :q! closes the current pane (no save check needed)
                    self.close_split();
                } else {
                    self.should_quit = true;
                }
            }
            ["w" | "write"] => {
                if let Err(e) = self.save_file() {
                    self.set_status(e);
                }
            }
            ["w", path] => {
                if let Err(e) = self.save_file_as(PathBuf::from(path)) {
                    self.set_status(e);
                }
            }
            ["wq"] => {
                if let Err(e) = self.save_file() {
                    self.set_status(e);
                } else {
                    self.should_quit = true;
                }
            }
            ["e" | "edit"] => {
                // Open file browser
                self.enter_browse_mode();
            }
            ["e" | "edit", path] => {
                // Open specific file
                if let Err(e) = self.load_file(Path::new(path)) {
                    self.set_status(e);
                }
            }
            ["color", scheme] => {
                if let Some(s) = ColorScheme::from_str(scheme) {
                    self.color_scheme = s;
                    self.set_status(format!("Color scheme: {}", s.as_str()));
                } else {
                    self.set_status(format!("Unknown color scheme: {scheme}"));
                }
            }
            ["set", setting] => {
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
            ["fold"] => {
                self.fold_current_sequence();
            }
            ["alifold"] => {
                self.fold_alignment();
            }
            ["?" | "help"] => {
                self.show_help = true;
            }
            ["ruler"] => {
                self.show_ruler = !self.show_ruler;
                self.set_status(format!(
                    "Ruler: {}",
                    if self.show_ruler { "on" } else { "off" }
                ));
            }
            ["rownum"] => {
                self.show_row_numbers = !self.show_row_numbers;
                self.set_status(format!(
                    "Row numbers: {}",
                    if self.show_row_numbers { "on" } else { "off" }
                ));
            }
            ["split" | "sp"] => {
                self.horizontal_split();
            }
            ["vsplit" | "vs" | "vsp"] => {
                self.vertical_split();
            }
            ["only"] => {
                self.close_split();
            }
            ["upper" | "uppercase"] => {
                self.uppercase_alignment();
                self.set_status("Converted to uppercase");
            }
            ["lower" | "lowercase"] => {
                self.lowercase_alignment();
                self.set_status("Converted to lowercase");
            }
            ["t2u"] => {
                self.convert_t_to_u();
                self.set_status("Converted T to U");
            }
            ["u2t"] => {
                self.convert_u_to_t();
                self.set_status("Converted U to T");
            }
            ["trimleft"] => {
                self.trim_left();
            }
            ["trimright"] => {
                self.trim_right();
            }
            ["trim"] => {
                self.trim();
            }
            ["noh" | "nohlsearch"] => {
                self.clear_search();
            }
            ["cluster"] => {
                self.cluster_sequences();
                self.set_status(format!(
                    "Clustered {} sequences by similarity",
                    self.alignment.num_sequences()
                ));
            }
            ["uncluster"] => {
                self.uncluster();
                self.set_status("Clustering disabled");
            }
            ["tree"] => {
                self.toggle_tree();
                if self.show_tree {
                    self.set_status("Tree visible");
                } else if self.cluster_tree.is_some() {
                    self.set_status("Tree hidden");
                }
            }
            ["collapse"] => {
                self.toggle_collapse_identical();
            }
            ["consensus"] => {
                self.show_consensus = !self.show_consensus;
                self.set_status(format!(
                    "Consensus bar: {}",
                    if self.show_consensus { "on" } else { "off" }
                ));
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
            }
            ["type"] => {
                self.set_status(format!("Sequence type: {:?}", self.sequence_type));
            }
            ["type", t] => match t.to_lowercase().as_str() {
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
            },
            _ => {
                // Check if command is a line number (e.g., :1, :42)
                if let Ok(line_num) = command.parse::<usize>() {
                    self.goto_row(line_num);
                } else {
                    self.set_status(format!("Unknown command: {command}"));
                }
            }
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
        if self.command_history.is_empty() {
            return;
        }

        match self.command_history_index {
            None => {
                // Save current input and go to most recent history
                self.command_history_saved = self.command_buffer.clone();
                self.command_history_index = Some(self.command_history.len() - 1);
            }
            Some(0) => {
                // Already at oldest, stay there
                return;
            }
            Some(i) => {
                self.command_history_index = Some(i - 1);
            }
        }

        if let Some(i) = self.command_history_index {
            self.command_buffer = self.command_history[i].clone();
        }
    }

    /// Navigate to next command in history (Down arrow).
    pub fn command_history_next(&mut self) {
        match self.command_history_index {
            None => {
                // Not in history, do nothing
            }
            Some(i) if i >= self.command_history.len() - 1 => {
                // At end of history, restore saved input
                self.command_history_index = None;
                self.command_buffer = self.command_history_saved.clone();
            }
            Some(i) => {
                self.command_history_index = Some(i + 1);
                self.command_buffer = self.command_history[i + 1].clone();
            }
        }
    }

    /// Fold current sequence using RNAfold.
    fn fold_current_sequence(&mut self) {
        self.set_status("RNAfold integration not yet implemented");
    }

    /// Fold alignment using RNAalifold.
    fn fold_alignment(&mut self) {
        self.set_status("RNAalifold integration not yet implemented");
    }

    /// Mark the alignment as modified.
    pub fn mark_modified(&mut self) {
        self.modified = true;
    }

    /// Update the structure cache if needed.
    pub fn update_structure_cache(&mut self) {
        if let Some(ss) = self.alignment.ss_cons()
            && !self.structure_cache.is_valid_for(ss)
        {
            let _ = self.structure_cache.update(ss);
        }
    }

    /// Ensure cursor is within bounds.
    pub fn clamp_cursor(&mut self) {
        let max_row = self.alignment.num_sequences().saturating_sub(1);
        let max_col = self.alignment.width().saturating_sub(1);
        self.cursor_row = self.cursor_row.min(max_row);
        self.cursor_col = self.cursor_col.min(max_col);
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
        if self.cursor_col < self.viewport_col {
            self.viewport_col = self.cursor_col;
        } else if self.cursor_col >= self.viewport_col + visible_cols {
            self.viewport_col = self.cursor_col - visible_cols + 1;
        }
    }

    // === Clustering methods ===

    /// Map display row to actual sequence index.
    /// When collapse is active, maps to representative. When clustering is active, uses cluster order.
    pub fn display_to_actual_row(&self, display_row: usize) -> usize {
        // First apply collapse mapping (if enabled)
        let row = if self.collapse_identical && display_row < self.collapse_groups.len() {
            self.collapse_groups[display_row].0
        } else {
            display_row
        };

        // Then apply cluster mapping (if enabled)
        if let Some(ref order) = self.cluster_order {
            order.get(row).copied().unwrap_or(row)
        } else {
            row
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
        let result = crate::clustering::cluster_sequences_with_tree(&seq_chars, &self.gap_chars);
        self.cluster_order = Some(result.order);
        self.cluster_tree = Some(result.tree_lines);
        self.tree_width = result.tree_width;

        // Clamp cursor to valid range
        if self.cursor_row >= self.visible_sequence_count() {
            self.cursor_row = self.visible_sequence_count().saturating_sub(1);
        }
    }

    /// Disable clustering and restore original order.
    pub fn uncluster(&mut self) {
        self.cluster_order = None;
        self.cluster_tree = None;
        self.tree_width = 0;
        self.show_tree = false;
    }

    /// Toggle dendrogram tree visibility.
    pub fn toggle_tree(&mut self) {
        if self.cluster_tree.is_some() {
            self.show_tree = !self.show_tree;
        } else {
            self.status_message = Some("No tree available. Run :cluster first.".to_string());
        }
    }

    /// Check if clustering is currently active.
    #[allow(dead_code)]
    pub fn is_clustered(&self) -> bool {
        self.cluster_order.is_some()
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
        if self.collapse_identical && display_row < self.collapse_groups.len() {
            self.collapse_groups[display_row].1.len()
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
}
