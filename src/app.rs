//! Application state and main loop.

use std::path::PathBuf;

use crate::editor::History;
use crate::stockholm::Alignment;
use crate::structure::StructureCache;

/// Editor mode (vim-style).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
    Command,
    Search,
}

impl Mode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
            Mode::Command => "COMMAND",
            Mode::Search => "SEARCH",
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
            "base" | "nt" => Some(ColorScheme::Base),
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
    /// All match positions (row, col).
    pub(crate) search_matches: Vec<(usize, usize)>,
    /// Current match index in search_matches.
    pub(crate) search_match_index: Option<usize>,
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
    pub fn load_file(&mut self, path: &PathBuf) -> Result<(), String> {
        let alignment = crate::stockholm::parser::parse_file(path)
            .map_err(|e| format!("Failed to parse file: {}", e))?;

        self.alignment = alignment;
        self.file_path = Some(path.clone());
        self.modified = false;
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.viewport_row = 0;
        self.viewport_col = 0;
        self.history.clear();

        // Update structure cache
        if let Some(ss) = self.alignment.ss_cons() {
            let _ = self.structure_cache.update(ss);
        }

        self.set_status(format!("Loaded {}", path.display()));
        Ok(())
    }

    /// Save the alignment to a file.
    pub fn save_file(&mut self) -> Result<(), String> {
        let path = self.file_path.as_ref().ok_or("No file path set")?;
        crate::stockholm::writer::write_file(&self.alignment, path)
            .map_err(|e| format!("Failed to save file: {}", e))?;
        self.modified = false;
        self.set_status(format!("Saved {}", path.display()));
        Ok(())
    }

    /// Save the alignment to a new file.
    pub fn save_file_as(&mut self, path: PathBuf) -> Result<(), String> {
        crate::stockholm::writer::write_file(&self.alignment, &path)
            .map_err(|e| format!("Failed to save file: {}", e))?;
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

    /// Execute the current search pattern.
    pub fn execute_search(&mut self) {
        if self.search_pattern.is_empty() {
            self.enter_normal_mode();
            return;
        }

        self.search_matches = self.find_matches(&self.search_pattern.clone());

        if self.search_matches.is_empty() {
            self.set_status("Pattern not found");
            self.search_match_index = None;
        } else {
            // Find first match at or after cursor position
            let cursor_pos = (self.cursor_row, self.cursor_col);
            let first_match_idx = self
                .search_matches
                .iter()
                .position(|&pos| pos >= cursor_pos)
                .unwrap_or(0);

            self.search_match_index = Some(first_match_idx);
            self.jump_to_current_match();
        }
    }

    /// Jump to the next search match.
    pub fn search_next(&mut self) {
        if self.search_matches.is_empty() {
            if !self.search_pattern.is_empty() {
                self.set_status("Pattern not found");
            }
            return;
        }

        let current_idx = self.search_match_index.unwrap_or(0);
        let next_idx = (current_idx + 1) % self.search_matches.len();
        self.search_match_index = Some(next_idx);
        self.jump_to_current_match();
    }

    /// Jump to the previous search match.
    pub fn search_prev(&mut self) {
        if self.search_matches.is_empty() {
            if !self.search_pattern.is_empty() {
                self.set_status("Pattern not found");
            }
            return;
        }

        let current_idx = self.search_match_index.unwrap_or(0);
        let prev_idx = if current_idx == 0 {
            self.search_matches.len() - 1
        } else {
            current_idx - 1
        };
        self.search_match_index = Some(prev_idx);
        self.jump_to_current_match();
    }

    /// Find all matches of a pattern in the alignment.
    /// Case-insensitive and U/T tolerant (RNA/DNA equivalent).
    fn find_matches(&self, pattern: &str) -> Vec<(usize, usize)> {
        let pattern_normalized = Self::normalize_for_search(pattern);
        let mut matches = Vec::new();

        for (row, seq) in self.alignment.sequences.iter().enumerate() {
            let data: String = seq.chars().iter().collect();
            let data_normalized = Self::normalize_for_search(&data);

            let mut start = 0;
            while let Some(pos) = data_normalized[start..].find(&pattern_normalized) {
                matches.push((row, start + pos));
                start += pos + 1;
            }
        }

        matches
    }

    /// Normalize a string for search: uppercase and T→U for RNA/DNA equivalence.
    fn normalize_for_search(s: &str) -> String {
        s.to_uppercase().replace('T', "U")
    }

    /// Get the length of the current search pattern.
    #[allow(dead_code)] // Public API
    pub fn search_pattern_len(&self) -> usize {
        self.search_pattern.len()
    }

    /// Check if a position is part of a search match.
    /// Returns Some(true) if it's the current match, Some(false) if it's another match, None if not a match.
    pub fn is_search_match(&self, row: usize, col: usize) -> Option<bool> {
        if self.search_matches.is_empty() || self.search_pattern.is_empty() {
            return None;
        }

        let pattern_len = self.search_pattern.len();
        let current_idx = self.search_match_index;

        for (idx, &(match_row, match_col)) in self.search_matches.iter().enumerate() {
            if row == match_row && col >= match_col && col < match_col + pattern_len {
                return Some(current_idx == Some(idx));
            }
        }

        None
    }

    /// Jump to the current match and update status.
    fn jump_to_current_match(&mut self) {
        if let Some(idx) = self.search_match_index {
            if let Some(&(row, col)) = self.search_matches.get(idx) {
                self.cursor_row = row;
                self.cursor_col = col;
                self.set_status(&format!(
                    "Match {}/{}",
                    idx + 1,
                    self.search_matches.len()
                ));
            }
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
            ["q"] | ["quit"] => {
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
            ["w"] | ["write"] => {
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
            ["color", scheme] => {
                if let Some(s) = ColorScheme::from_str(scheme) {
                    self.color_scheme = s;
                    self.set_status(format!("Color scheme: {}", s.as_str()));
                } else {
                    self.set_status(format!("Unknown color scheme: {}", scheme));
                }
            }
            ["set", setting] => {
                if let Some((key, value)) = setting.split_once('=') {
                    match key {
                        "gap" => {
                            if let Some(c) = value.chars().next() {
                                self.gap_char = c;
                                self.set_status(format!("Gap character: '{}'", c));
                            }
                        }
                        _ => {
                            self.set_status(format!("Unknown setting: {}", key));
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
            ["?"] | ["help"] => {
                self.show_help = true;
            }
            ["ruler"] => {
                self.show_ruler = !self.show_ruler;
                self.set_status(format!("Ruler: {}", if self.show_ruler { "on" } else { "off" }));
            }
            ["rownum"] => {
                self.show_row_numbers = !self.show_row_numbers;
                self.set_status(format!("Row numbers: {}", if self.show_row_numbers { "on" } else { "off" }));
            }
            ["split"] | ["sp"] => {
                self.horizontal_split();
            }
            ["vsplit"] | ["vs"] | ["vsp"] => {
                self.vertical_split();
            }
            ["only"] => {
                self.close_split();
            }
            ["upper"] | ["uppercase"] => {
                self.uppercase_alignment();
                self.set_status("Converted to uppercase");
            }
            ["lower"] | ["lowercase"] => {
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
            _ => {
                self.set_status(format!("Unknown command: {}", command));
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
                return;
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
        if let Some(ss) = self.alignment.ss_cons() {
            if !self.structure_cache.is_valid_for(ss) {
                let _ = self.structure_cache.update(ss);
            }
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
}
