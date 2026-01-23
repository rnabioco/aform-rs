//! Vim-style input handling.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, Mode};

/// Handle movement keys common to normal and visual modes.
/// Returns true if the key was handled as a movement.
fn handle_movement_keys(app: &mut App, key: KeyEvent, page_size: usize) -> bool {
    match (key.modifiers, key.code) {
        // Basic movement (hjkl and arrows)
        (KeyModifiers::NONE, KeyCode::Char('h') | KeyCode::Left) => {
            app.cursor_left();
            true
        }
        (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
            app.cursor_down();
            true
        }
        (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
            app.cursor_up();
            true
        }
        (KeyModifiers::NONE, KeyCode::Char('l') | KeyCode::Right) => {
            app.cursor_right();
            true
        }

        // Line movement
        (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char('^')) => {
            app.cursor_line_start();
            true
        }
        (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char('$')) => {
            app.cursor_line_end();
            true
        }
        (KeyModifiers::NONE, KeyCode::Home) => {
            app.cursor_line_start();
            true
        }
        (KeyModifiers::NONE, KeyCode::End) => {
            app.cursor_line_end();
            true
        }

        // Document movement
        (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
            app.cursor_last_sequence();
            true
        }

        // Page movement
        (KeyModifiers::CONTROL, KeyCode::Char('f')) | (KeyModifiers::NONE, KeyCode::PageDown) => {
            app.page_down(page_size);
            true
        }
        (KeyModifiers::CONTROL, KeyCode::Char('b')) | (KeyModifiers::NONE, KeyCode::PageUp) => {
            app.page_up(page_size);
            true
        }
        (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
            app.half_page_down(page_size);
            true
        }
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            app.half_page_up(page_size);
            true
        }

        // Word-like movement (jump by 10 columns)
        (KeyModifiers::NONE, KeyCode::Char('w')) => {
            app.scroll_right(10);
            true
        }
        (KeyModifiers::NONE, KeyCode::Char('b')) => {
            app.scroll_left(10);
            true
        }

        _ => false,
    }
}

/// Handle a key event.
pub fn handle_key(app: &mut App, key: KeyEvent, page_size: usize) {
    // Close help overlay on any keypress
    if app.show_help {
        app.show_help = false;
        return;
    }

    // Close info overlay on any keypress
    if app.show_info {
        app.show_info = false;
        return;
    }

    match app.mode {
        Mode::Normal => handle_normal_mode(app, key, page_size),
        Mode::Insert => handle_insert_mode(app, key),
        Mode::Command => handle_command_mode(app, key),
        Mode::Search => handle_search_mode(app, key),
        Mode::Visual | Mode::VisualLine => handle_visual_mode(app, key, page_size),
    }
}

/// Handle keys in normal mode.
fn handle_normal_mode(app: &mut App, key: KeyEvent, page_size: usize) {
    // Save pending status for two-key sequences before clearing
    let pending_status = app.status_message.clone();
    app.clear_status();

    // Check if this is a digit key for count prefix
    let is_count_digit = matches!(
        (key.modifiers, key.code),
        (KeyModifiers::NONE, KeyCode::Char('1'..='9'))
    ) || (matches!(key.code, KeyCode::Char('0'))
        && !app.count_buffer.is_empty());

    // Clear count for non-digit keys (except | which consumes it)
    let is_pipe = matches!(
        (key.modifiers, key.code),
        (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char('|'))
    );
    if !is_count_digit && !is_pipe {
        app.clear_count();
    }

    // Check if we're in a two-key sequence (like "Ctrl-w..." or "g...")
    let in_two_key_sequence = pending_status
        .as_ref()
        .map(|s| s.ends_with("..."))
        .unwrap_or(false);

    // Try shared movement keys first (unless it's a key with special normal-mode handling
    // or we're in a two-key sequence)
    let is_special_normal_key = matches!(
        (key.modifiers, key.code),
        (KeyModifiers::NONE, KeyCode::Char('0' | 'g' | 'w' | 'b'))
            | (KeyModifiers::CONTROL, KeyCode::Char('w'))
    );
    if !in_two_key_sequence && !is_special_normal_key && handle_movement_keys(app, key, page_size) {
        return;
    }

    match (key.modifiers, key.code) {
        // Quit
        (KeyModifiers::NONE, KeyCode::Char('q')) => {
            if app.modified {
                app.set_status("No write since last change (use :q! to force)");
            } else {
                app.should_quit = true;
            }
        }

        // Count prefix digits (1-9 start a count, 0 continues a count)
        (KeyModifiers::NONE, KeyCode::Char(c @ '1'..='9')) => {
            app.push_count_digit(c);
        }
        (KeyModifiers::NONE, KeyCode::Char('0')) if !app.count_buffer.is_empty() => {
            app.push_count_digit('0');
        }

        // Go to column (vim |)
        (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char('|')) => {
            let col = app.take_count();
            app.goto_column(col);
        }

        // Movement - line start (0 only when not building count)
        (KeyModifiers::NONE, KeyCode::Char('0')) => {
            app.cursor_line_start();
        }

        // Movement - document (g starts two-key sequence)
        (KeyModifiers::NONE, KeyCode::Char('g')) => {
            app.set_status("g...");
        }

        // Split window prefix (Ctrl-w)
        (KeyModifiers::CONTROL, KeyCode::Char('w')) => {
            app.set_status("Ctrl-w...");
        }

        // Movement - word-like (jump by 10 columns)
        (KeyModifiers::NONE, KeyCode::Char('w')) => {
            app.scroll_right(10);
        }
        (KeyModifiers::NONE, KeyCode::Char('b')) => {
            app.scroll_left(10);
        }

        // Go to pair (gp) or paste
        (KeyModifiers::NONE, KeyCode::Char('p')) => {
            if pending_status.as_deref() == Some("g...") {
                app.goto_pair();
            } else {
                app.paste();
            }
        }

        // Insert mode
        (KeyModifiers::NONE, KeyCode::Char('i')) => {
            app.enter_insert_mode();
        }

        // Delete gap
        (KeyModifiers::NONE, KeyCode::Char('x')) => {
            app.delete_gap();
        }

        // Insert gap column
        (KeyModifiers::SHIFT, KeyCode::Char('I')) => {
            app.insert_gap_column();
        }

        // Delete gap column
        (KeyModifiers::SHIFT, KeyCode::Char('X')) => {
            app.delete_gap_column();
        }

        // Shift sequence
        (KeyModifiers::SHIFT, KeyCode::Char('<')) => {
            app.shift_sequence_left();
        }
        (KeyModifiers::SHIFT, KeyCode::Char('>')) => {
            app.shift_sequence_right();
        }

        // Throw sequence
        (KeyModifiers::SHIFT, KeyCode::Char('{')) => {
            app.throw_sequence_left();
        }
        (KeyModifiers::SHIFT, KeyCode::Char('}')) => {
            app.throw_sequence_right();
        }

        // Undo/Redo
        (KeyModifiers::NONE, KeyCode::Char('u')) => {
            app.undo();
        }
        (KeyModifiers::CONTROL, KeyCode::Char('r')) => {
            app.redo();
        }

        // Command mode
        (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(':')) => {
            app.enter_command_mode();
        }

        // Delete line
        (KeyModifiers::NONE, KeyCode::Char('d')) => {
            app.set_status("d...");
        }

        // Search
        (KeyModifiers::NONE, KeyCode::Char('/')) => {
            app.enter_search_mode();
        }
        (KeyModifiers::NONE, KeyCode::Char('n')) => {
            app.search_next();
        }
        (KeyModifiers::SHIFT, KeyCode::Char('N')) => {
            app.search_prev();
        }

        // Visual mode
        (KeyModifiers::NONE, KeyCode::Char('v')) => {
            app.enter_visual_mode();
        }

        // Visual line mode
        (KeyModifiers::SHIFT, KeyCode::Char('V')) => {
            app.enter_visual_line_mode();
        }

        // Help (some terminals send ? without SHIFT modifier)
        (KeyModifiers::SHIFT | KeyModifiers::NONE, KeyCode::Char('?')) => {
            app.toggle_help();
        }

        _ => {}
    }

    // Handle two-key sequences
    if let Some(status) = pending_status.as_deref() {
        match (status, key.code) {
            ("g...", KeyCode::Char('g')) => {
                app.cursor_first_sequence();
            }
            ("d...", KeyCode::Char('d')) => {
                app.delete_sequence();
            }
            // Ctrl-w sequences for split management
            ("Ctrl-w...", KeyCode::Char('s')) => {
                app.horizontal_split();
            }
            ("Ctrl-w...", KeyCode::Char('v')) => {
                app.vertical_split();
            }
            (
                "Ctrl-w...",
                KeyCode::Char('w' | 'h' | 'j' | 'k' | 'l')
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::Up
                | KeyCode::Down,
            ) => {
                app.switch_pane();
            }
            ("Ctrl-w...", KeyCode::Char('q')) => {
                app.close_split();
            }
            _ => {}
        }
    }
}

/// Handle keys in insert mode.
fn handle_insert_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.enter_normal_mode();
        }
        KeyCode::Char('.' | '-') => {
            app.insert_gap();
        }
        KeyCode::Backspace => {
            // Delete gap behind cursor
            if app.cursor_col > 0 {
                app.cursor_left();
                app.delete_gap();
            }
        }
        KeyCode::Left => {
            app.cursor_left();
        }
        KeyCode::Right => {
            app.cursor_right();
        }
        KeyCode::Up => {
            app.cursor_up();
        }
        KeyCode::Down => {
            app.cursor_down();
        }
        _ => {}
    }
}

/// Handle keys in command mode.
fn handle_command_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.completion = None;
            app.enter_normal_mode();
        }
        KeyCode::Enter => {
            app.completion = None;
            app.execute_command();
        }
        KeyCode::Backspace => {
            app.completion = None;
            app.command_buffer.pop();
        }
        KeyCode::Up => {
            app.command_history_prev();
        }
        KeyCode::Down => {
            app.command_history_next();
        }
        KeyCode::Tab => {
            handle_tab_completion(app);
        }
        KeyCode::Char(c) => {
            app.completion = None;
            app.command_buffer.push(c);
        }
        _ => {}
    }
}

/// Handle tab completion for file paths in command mode.
fn handle_tab_completion(app: &mut App) {
    use crate::app::CompletionState;

    // Check if we're completing a file path command
    let buffer = app.command_buffer.clone();
    let (cmd, partial_path) = if let Some(rest) = buffer.strip_prefix("e ") {
        ("e ", rest)
    } else if let Some(rest) = buffer.strip_prefix("edit ") {
        ("edit ", rest)
    } else if let Some(rest) = buffer.strip_prefix("w ") {
        ("w ", rest)
    } else if let Some(rest) = buffer.strip_prefix("write ") {
        ("write ", rest)
    } else {
        return; // Not a file command
    };

    // If we have existing completion state, cycle through candidates
    if let Some(ref mut state) = app.completion
        && !state.candidates.is_empty()
    {
        state.index = (state.index + 1) % state.candidates.len();
        app.command_buffer = format!("{}{}", cmd, state.candidates[state.index]);
        return;
    }

    // Get completions
    let candidates = complete_path(partial_path);

    if candidates.is_empty() {
        app.set_status("No matches");
        return;
    }

    if candidates.len() == 1 {
        // Single match - complete it
        app.command_buffer = format!("{}{}", cmd, candidates[0]);
        app.completion = None;
    } else {
        // Multiple matches - start cycling
        app.command_buffer = format!("{}{}", cmd, candidates[0]);
        app.completion = Some(CompletionState {
            candidates,
            index: 0,
            prefix: partial_path.to_string(),
        });
        // Show available options in status
        if let Some(ref state) = app.completion {
            let preview: Vec<&str> = state
                .candidates
                .iter()
                .take(5)
                .map(|s| s.as_str())
                .collect();
            let msg = if state.candidates.len() > 5 {
                format!(
                    "{} ... ({} more)",
                    preview.join("  "),
                    state.candidates.len() - 5
                )
            } else {
                preview.join("  ")
            };
            app.set_status(msg);
        }
    }
}

/// Complete a partial file path, returning sorted candidates.
fn complete_path(partial: &str) -> Vec<String> {
    use std::path::Path;

    let path = Path::new(partial);

    // Determine directory to search and prefix to match
    let (dir, prefix) = if partial.is_empty() {
        (Path::new("."), "")
    } else if partial.ends_with('/') || partial.ends_with(std::path::MAIN_SEPARATOR) {
        (path, "")
    } else {
        // parent() returns Some("") for bare filenames, which is not a valid dir
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let file_prefix = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        (parent, file_prefix)
    };

    let mut candidates: Vec<String> = std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|name| name.starts_with(prefix) && !name.starts_with('.'))
        })
        .map(|e| {
            let mut result = if dir == Path::new(".") {
                e.file_name().to_string_lossy().to_string()
            } else {
                dir.join(e.file_name()).display().to_string()
            };
            // Add trailing slash for directories
            if e.file_type().is_ok_and(|t| t.is_dir()) {
                result.push('/');
            }
            result
        })
        .collect();

    candidates.sort();
    candidates
}

/// Handle keys in search mode.
fn handle_search_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.enter_normal_mode();
        }
        KeyCode::Enter => {
            app.execute_search();
            app.enter_normal_mode();
        }
        KeyCode::Backspace => {
            app.search.pattern.pop();
            if app.search.pattern.is_empty() {
                app.enter_normal_mode();
            }
        }
        KeyCode::Up => {
            app.search_history_prev();
        }
        KeyCode::Down => {
            app.search_history_next();
        }
        KeyCode::Char(c) => {
            app.search.pattern.push(c);
        }
        _ => {}
    }
}

/// Handle keys in visual selection mode.
fn handle_visual_mode(app: &mut App, key: KeyEvent, page_size: usize) {
    // Handle '0' for line start (visual mode doesn't have count buffer)
    if matches!(
        (key.modifiers, key.code),
        (KeyModifiers::NONE, KeyCode::Char('0'))
    ) {
        app.cursor_line_start();
        return;
    }

    // Capture pending status for two-key sequences
    let pending_status = app.status_message.clone();

    // Handle two-key sequences first
    if let Some(status) = pending_status.as_deref() {
        match (status, key.code) {
            ("g...", KeyCode::Char('g')) => {
                app.cursor_first_sequence();
                return;
            }
            ("d...", KeyCode::Char('d')) => {
                // dd in visual mode = delete entire sequences
                app.delete_selected_sequences();
                return;
            }
            _ => {
                // Clear pending status on unrecognized sequence
                app.clear_status();
            }
        }
    }

    // Try shared movement keys first (except 'g' which needs two-key handling)
    if !matches!(
        (key.modifiers, key.code),
        (KeyModifiers::NONE, KeyCode::Char('g'))
    ) && handle_movement_keys(app, key, page_size)
    {
        return;
    }

    match (key.modifiers, key.code) {
        // Exit visual mode
        (KeyModifiers::NONE, KeyCode::Esc | KeyCode::Char('v')) => {
            app.exit_visual_mode();
        }
        // Exit visual line mode with V
        (KeyModifiers::SHIFT, KeyCode::Char('V')) => {
            app.exit_visual_mode();
        }

        // Document movement (g starts two-key sequence for gg)
        (KeyModifiers::NONE, KeyCode::Char('g')) => {
            app.set_status("g...");
        }

        // Yank (copy) selection
        (KeyModifiers::NONE, KeyCode::Char('y')) => {
            app.yank_selection();
        }

        // Delete: first 'd' starts sequence, 'x' deletes cells immediately
        (KeyModifiers::NONE, KeyCode::Char('d')) => {
            app.set_status("d...");
        }
        (KeyModifiers::NONE, KeyCode::Char('x')) => {
            app.delete_selection();
        }

        _ => {}
    }
}
