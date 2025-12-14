//! Vim-style input handling.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, Mode};

/// Handle a key event.
pub fn handle_key(app: &mut App, key: KeyEvent, page_size: usize) {
    // Close help overlay on any keypress
    if app.show_help {
        app.show_help = false;
        return;
    }

    match app.mode {
        Mode::Normal => handle_normal_mode(app, key, page_size),
        Mode::Insert => handle_insert_mode(app, key),
        Mode::Command => handle_command_mode(app, key),
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
    ) || (matches!(key.code, KeyCode::Char('0')) && !app.count_buffer.is_empty());

    // Clear count for non-digit keys (except | which consumes it)
    let is_pipe = matches!(
        (key.modifiers, key.code),
        (KeyModifiers::NONE, KeyCode::Char('|')) | (KeyModifiers::SHIFT, KeyCode::Char('|'))
    );
    if !is_count_digit && !is_pipe {
        app.clear_count();
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
        (KeyModifiers::NONE, KeyCode::Char('|'))
        | (KeyModifiers::SHIFT, KeyCode::Char('|')) => {
            let col = app.take_count();
            app.goto_column(col);
        }

        // Movement - basic
        (KeyModifiers::NONE, KeyCode::Char('h')) | (KeyModifiers::NONE, KeyCode::Left) => {
            app.cursor_left();
        }
        (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
            app.cursor_down();
        }
        (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
            app.cursor_up();
        }
        (KeyModifiers::NONE, KeyCode::Char('l')) | (KeyModifiers::NONE, KeyCode::Right) => {
            app.cursor_right();
        }

        // Movement - line
        (KeyModifiers::NONE, KeyCode::Char('0'))
        | (KeyModifiers::SHIFT, KeyCode::Char('^'))
        | (KeyModifiers::NONE, KeyCode::Char('^')) => {
            // Only reaches here if count_buffer is empty (handled above otherwise)
            app.cursor_line_start();
        }
        (KeyModifiers::NONE, KeyCode::Char('$')) | (KeyModifiers::SHIFT, KeyCode::Char('$')) => {
            app.cursor_line_end();
        }
        (KeyModifiers::NONE, KeyCode::Home) => {
            app.cursor_line_start();
        }
        (KeyModifiers::NONE, KeyCode::End) => {
            app.cursor_line_end();
        }

        // Movement - document
        (KeyModifiers::NONE, KeyCode::Char('g')) => {
            // Waiting for second 'g'
            app.set_status("g...");
        }
        (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
            app.cursor_last_sequence();
        }

        // Movement - scrolling
        (KeyModifiers::CONTROL, KeyCode::Char('f')) | (KeyModifiers::NONE, KeyCode::PageDown) => {
            app.page_down(page_size);
        }
        (KeyModifiers::CONTROL, KeyCode::Char('b')) | (KeyModifiers::NONE, KeyCode::PageUp) => {
            app.page_up(page_size);
        }
        (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
            app.half_page_down(page_size);
        }
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            app.half_page_up(page_size);
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

        // Go to pair
        (KeyModifiers::NONE, KeyCode::Char('p')) => {
            // Check if previous key was 'g'
            if pending_status.as_deref() == Some("g...") {
                app.goto_pair();
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
        (KeyModifiers::NONE, KeyCode::Char(':')) | (KeyModifiers::SHIFT, KeyCode::Char(':')) => {
            app.enter_command_mode();
        }

        // Delete line
        (KeyModifiers::NONE, KeyCode::Char('d')) => {
            // Waiting for second 'd'
            app.set_status("d...");
        }

        // Help (some terminals send ? without SHIFT modifier)
        (KeyModifiers::SHIFT, KeyCode::Char('?'))
        | (KeyModifiers::NONE, KeyCode::Char('?')) => {
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
            ("Ctrl-w...", KeyCode::Char('w'))
            | ("Ctrl-w...", KeyCode::Left)
            | ("Ctrl-w...", KeyCode::Right)
            | ("Ctrl-w...", KeyCode::Up)
            | ("Ctrl-w...", KeyCode::Down)
            | ("Ctrl-w...", KeyCode::Char('h'))
            | ("Ctrl-w...", KeyCode::Char('j'))
            | ("Ctrl-w...", KeyCode::Char('k'))
            | ("Ctrl-w...", KeyCode::Char('l')) => {
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
        KeyCode::Char('.') | KeyCode::Char('-') => {
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
            app.enter_normal_mode();
        }
        KeyCode::Enter => {
            app.execute_command();
        }
        KeyCode::Backspace => {
            app.command_buffer.pop();
            if app.command_buffer.is_empty() {
                app.enter_normal_mode();
            }
        }
        KeyCode::Up => {
            app.command_history_prev();
        }
        KeyCode::Down => {
            app.command_history_next();
        }
        KeyCode::Char(c) => {
            app.command_buffer.push(c);
        }
        _ => {}
    }
}

