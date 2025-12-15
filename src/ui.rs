//! TUI rendering with ratatui.
#![allow(clippy::needless_range_loop)]

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::app::{ActivePane, App, ColorScheme, Mode, SplitMode};
use crate::color::get_color;

/// Render the application UI.
pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // Alignment view
            Constraint::Length(1), // Status bar
            Constraint::Length(1), // Command/message line
        ])
        .split(frame.area());

    // If in browse mode, render file explorer
    if app.mode == Mode::Browse {
        render_file_explorer(frame, app, chunks[0]);
        render_status_bar(frame, app, chunks[1]);
        render_command_line(frame, app, chunks[2]);
        return;
    }

    // Handle split mode
    match app.split_mode {
        None => {
            // Single pane
            render_alignment_pane(
                frame,
                app,
                chunks[0],
                app.viewport_row,
                app.viewport_col,
                true, // always active
                None, // no pane indicator
            );
        }
        Some(SplitMode::Horizontal) => {
            // Top/bottom split
            let panes = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[0]);

            render_alignment_pane(
                frame,
                app,
                panes[0],
                app.viewport_row,
                app.viewport_col,
                app.active_pane == ActivePane::Primary,
                Some("Primary"),
            );
            render_alignment_pane(
                frame,
                app,
                panes[1],
                app.secondary_viewport_row,
                app.secondary_viewport_col,
                app.active_pane == ActivePane::Secondary,
                Some("Secondary"),
            );
        }
        Some(SplitMode::Vertical) => {
            // Left/right split
            let panes = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[0]);

            render_alignment_pane(
                frame,
                app,
                panes[0],
                app.viewport_row,
                app.viewport_col,
                app.active_pane == ActivePane::Primary,
                Some("Primary"),
            );
            render_alignment_pane(
                frame,
                app,
                panes[1],
                app.secondary_viewport_row,
                app.secondary_viewport_col,
                app.active_pane == ActivePane::Secondary,
                Some("Secondary"),
            );
        }
    }

    render_status_bar(frame, app, chunks[1]);
    render_command_line(frame, app, chunks[2]);

    // Render help overlay if active
    if app.show_help {
        render_help(frame);
    }
}

/// Height of the ruler in lines.
const RULER_HEIGHT: u16 = 2;

/// Formats the ID column (row number + sequence ID).
struct IdFormatter {
    row_width: usize,
    id_width: usize,
    show_row_numbers: bool,
}

impl IdFormatter {
    fn new(num_sequences: usize, max_id_len: usize, show_row_numbers: bool) -> Self {
        Self {
            row_width: if show_row_numbers {
                num_sequences.max(1).to_string().len()
            } else {
                0
            },
            id_width: max_id_len,
            show_row_numbers,
        }
    }

    /// Total width of the formatted ID column.
    fn width(&self) -> usize {
        if self.show_row_numbers {
            // Format: "row_num id " with spaces
            self.row_width + 1 + self.id_width + 1
        } else {
            // Format: "id " with trailing space
            self.id_width + 1
        }
    }

    /// Format a row number and ID.
    fn format(&self, row: usize, id: &str) -> String {
        if self.show_row_numbers {
            format!(
                "{:>row_w$} {:id_w$} ",
                row + 1,
                id,
                row_w = self.row_width,
                id_w = self.id_width
            )
        } else {
            format!("{:id_w$} ", id, id_w = self.id_width)
        }
    }
}

/// Render an alignment pane with the given viewport.
fn render_alignment_pane(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    viewport_row: usize,
    viewport_col: usize,
    is_active: bool,
    pane_label: Option<&str>,
) {
    // Build title with file info and optional pane label
    let file_info = format!(
        " {} {} ",
        app.file_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "[No file]".to_string()),
        if app.modified { "[+]" } else { "" }
    );

    let title = match pane_label {
        Some(label) => format!("{} [{}]", file_info, label),
        None => file_info,
    };

    // Use different border color for active vs inactive pane
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.alignment.sequences.is_empty() {
        render_splash(frame, inner);
        return;
    }

    // Calculate widths using a formatter helper
    let num_seqs = app.alignment.num_sequences();
    let max_id_len = app.alignment.max_id_len().max(10);
    let id_formatter = IdFormatter::new(num_seqs, max_id_len, app.show_row_numbers);
    let id_width = id_formatter.width();
    let seq_width = (inner.width as usize).saturating_sub(id_width);

    // Split inner area into ruler (fixed top), sequences (flexible), SS_cons (fixed bottom)
    let ruler_height = if app.show_ruler { RULER_HEIGHT } else { 0 };
    let has_ss_cons = app.alignment.ss_cons().is_some();
    let ss_cons_height: u16 = if has_ss_cons { 1 } else { 0 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(ruler_height),
            Constraint::Min(1),
            Constraint::Length(ss_cons_height),
        ])
        .split(inner);

    let ruler_area = chunks[0];
    let seq_area = chunks[1];
    let ss_cons_area = chunks[2];
    let visible_rows = seq_area.height as usize;

    // Render sticky ruler (if enabled)
    if app.show_ruler {
        let ruler_lines = render_ruler(id_width, seq_width, viewport_col);
        let ruler_paragraph = Paragraph::new(ruler_lines);
        frame.render_widget(ruler_paragraph, ruler_area);
    }

    // Render sequences
    let mut lines = Vec::new();
    for display_row in viewport_row..(viewport_row + visible_rows).min(app.visible_sequence_count()) {
        // Map display row to actual sequence index (for clustering support)
        let actual_row = app.display_to_actual_row(display_row);
        let seq = &app.alignment.sequences[actual_row];
        let mut spans = Vec::new();

        // Row number and sequence ID (show display row number, not actual)
        let id_style = if display_row == app.cursor_row {
            Style::reset().add_modifier(Modifier::BOLD)
        } else {
            Style::reset().fg(Color::Cyan)
        };
        let id_display = id_formatter.format(display_row, &seq.id);
        spans.push(Span::styled(id_display, id_style));

        // Sequence data
        let seq_chars: Vec<char> = seq.chars().to_vec();
        for col in viewport_col..(viewport_col + seq_width).min(seq_chars.len()) {
            let ch = seq_chars[col];
            // Only show cursor in active pane (cursor_row is in display coordinates)
            let is_cursor = is_active && display_row == app.cursor_row && col == app.cursor_col;

            let mut style = Style::reset();

            // Apply color scheme (use actual_row for alignment data)
            if let Some(color) = get_color(
                app.color_scheme,
                ch,
                col,
                actual_row,
                &app.alignment,
                &app.structure_cache,
                &app.gap_chars,
                app.reference_seq,
            ) {
                style = style.bg(color).fg(Color::Black);
            }

            // Highlight search matches (use actual_row since matches are on actual sequences)
            if let Some(is_current) = app.is_search_match(actual_row, col) {
                if is_current {
                    // Current match: bright yellow background
                    style = style.bg(Color::Yellow).fg(Color::Black);
                } else {
                    // Other matches: dimmer highlight
                    style = style.bg(Color::Rgb(100, 100, 50)).fg(Color::White);
                }
            }

            // Highlight visual selection (use display_row since selection is visual)
            if app.is_selected(display_row, col) {
                style = style.bg(Color::Rgb(80, 80, 140)).fg(Color::White);
            }

            // Highlight paired column (works across all panes)
            if let Some(paired_col) = app.structure_cache.get_pair(app.cursor_col) {
                if col == paired_col {
                    // Bright magenta background to clearly show the paired base
                    style = style.bg(Color::Magenta).fg(Color::White);
                }
            }

            // Highlight cursor (on top of everything)
            if is_cursor {
                style = style.add_modifier(Modifier::REVERSED);
            }

            spans.push(Span::styled(ch.to_string(), style));
        }

        lines.push(Line::from(spans));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, seq_area);

    // Render SS_cons annotation bar (fixed at bottom, like ruler)
    if let Some(ss) = app.alignment.ss_cons() {
        let mut spans = Vec::new();
        // Use blank row number area + SS_cons label with distinct background
        let ss_label = format!(
            "{:>row_w$} {:id_w$} ",
            "═",
            "#=GC SS_cons",
            row_w = id_formatter.row_width,
            id_w = id_formatter.id_width
        );
        spans.push(Span::styled(
            ss_label,
            Style::reset().fg(Color::Yellow).bg(Color::Rgb(30, 30, 40)),
        ));

        let ss_chars: Vec<char> = ss.chars().collect();
        for col in viewport_col..(viewport_col + seq_width).min(ss_chars.len()) {
            let ch = ss_chars[col];
            // Only show cursor column highlight in active pane
            let is_cursor_col = is_active && col == app.cursor_col;

            let mut style = Style::reset().fg(Color::Yellow).bg(Color::Rgb(30, 30, 40));

            // Highlight the paired bracket (works across all panes)
            if let Some(paired_col) = app.structure_cache.get_pair(app.cursor_col) {
                if col == paired_col {
                    style = style
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD);
                }
            }

            // Column indicator (not cursor - just shows which column)
            if is_cursor_col {
                style = style.add_modifier(Modifier::UNDERLINED);
            }

            spans.push(Span::styled(ch.to_string(), style));
        }

        let ss_line = Paragraph::new(Line::from(spans));
        frame.render_widget(ss_line, ss_cons_area);
    }
}

/// Render the position ruler (returns two lines: numbers and tick marks).
fn render_ruler(id_width: usize, seq_width: usize, viewport_col: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // First line: position numbers
    let mut number_spans = Vec::new();
    number_spans.push(Span::styled(
        " ".repeat(id_width),
        Style::reset().fg(Color::DarkGray),
    ));

    let mut number_chars = vec![' '; seq_width];
    for col in viewport_col..(viewport_col + seq_width) {
        let pos = col + 1; // 1-based position
        if pos % 10 == 0 {
            let pos_str = pos.to_string();
            let local_col = col - viewport_col;
            // Place the number so it ends at the marker position
            let start = local_col.saturating_sub(pos_str.len() - 1);
            for (i, ch) in pos_str.chars().enumerate() {
                if start + i < seq_width {
                    number_chars[start + i] = ch;
                }
            }
        }
    }
    number_spans.push(Span::styled(
        number_chars.into_iter().collect::<String>(),
        Style::reset().fg(Color::DarkGray),
    ));
    lines.push(Line::from(number_spans));

    // Second line: tick marks
    let mut tick_spans = Vec::new();
    tick_spans.push(Span::styled(
        " ".repeat(id_width),
        Style::reset().fg(Color::DarkGray),
    ));

    let mut tick_chars = String::with_capacity(seq_width);
    for col in viewport_col..(viewport_col + seq_width) {
        let pos = col + 1; // 1-based position
        if pos % 10 == 0 {
            tick_chars.push('|');
        } else if pos % 5 == 0 {
            tick_chars.push('+');
        } else {
            tick_chars.push('·');
        }
    }
    tick_spans.push(Span::styled(tick_chars, Style::reset().fg(Color::DarkGray)));
    lines.push(Line::from(tick_spans));

    lines
}

/// Render the status bar.
fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let mode_style = match app.mode {
        Mode::Normal => Style::default().bg(Color::Blue).fg(Color::White),
        Mode::Insert => Style::default().bg(Color::Green).fg(Color::Black),
        Mode::Command => Style::default().bg(Color::Yellow).fg(Color::Black),
        Mode::Search => Style::default().bg(Color::Magenta).fg(Color::White),
        Mode::Browse => Style::default().bg(Color::Cyan).fg(Color::Black),
        Mode::Visual => Style::default()
            .bg(Color::Rgb(100, 100, 180))
            .fg(Color::White),
    };

    let mode_span = Span::styled(format!(" {} ", app.mode.as_str()), mode_style);

    // Position info
    let pos_info = format!(" {}:{} ", app.cursor_row + 1, app.cursor_col + 1);

    // Alignment info
    let align_info = format!(
        " {}x{} ",
        app.alignment.num_sequences(),
        app.alignment.width()
    );

    // Color scheme
    let color_info = if app.color_scheme != ColorScheme::None {
        format!(" [{}] ", app.color_scheme.as_str())
    } else {
        String::new()
    };

    // Structure info
    let structure_info = if app.structure_cache.is_paired(app.cursor_col) {
        if let Some(paired) = app.structure_cache.get_pair(app.cursor_col) {
            format!(" pair:{} ", paired + 1)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    // Current character
    let char_info = app
        .current_char()
        .map(|c| format!(" '{}' ", c))
        .unwrap_or_default();

    // Selection info (in visual mode)
    let selection_info = app
        .selection_info()
        .map(|s| format!(" [{}] ", s))
        .unwrap_or_default();

    let spans = vec![
        mode_span,
        Span::raw(pos_info),
        Span::styled(align_info, Style::default().fg(Color::Cyan)),
        Span::styled(color_info, Style::default().fg(Color::Magenta)),
        Span::styled(structure_info, Style::default().fg(Color::Yellow)),
        Span::styled(selection_info, Style::default().fg(Color::LightBlue)),
        Span::raw(char_info),
    ];

    let status = Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::DarkGray));

    frame.render_widget(status, area);
}

/// Render the command/message line.
fn render_command_line(frame: &mut Frame, app: &App, area: Rect) {
    let content = match app.mode {
        Mode::Command => Line::from(vec![
            Span::styled(":", Style::default().fg(Color::Yellow)),
            Span::raw(&app.command_buffer),
            Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)),
        ]),
        Mode::Search => Line::from(vec![
            Span::styled("/", Style::default().fg(Color::Magenta)),
            Span::raw(&app.search_pattern),
            Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)),
        ]),
        _ => {
            if let Some(msg) = &app.status_message {
                Line::from(Span::raw(msg.as_str()))
            } else {
                // Show help hint
                Line::from(Span::styled(
                    "Press : for commands, ? for help, / for search",
                    Style::default().fg(Color::DarkGray),
                ))
            }
        }
    };

    let paragraph = Paragraph::new(content);
    frame.render_widget(paragraph, area);
}

/// Calculate visible dimensions for the alignment area.
pub fn visible_dimensions(
    area: Rect,
    num_sequences: usize,
    max_id_len: usize,
    show_ruler: bool,
    show_row_numbers: bool,
    split_mode: Option<SplitMode>,
    has_ss_cons: bool,
) -> (usize, usize) {
    let id_formatter = IdFormatter::new(num_sequences, max_id_len.max(10), show_row_numbers);
    let ruler_height = if show_ruler { RULER_HEIGHT } else { 0 };
    let ss_cons_height: u16 = if has_ss_cons { 1 } else { 0 };

    // Calculate the alignment area (total - status - command)
    let alignment_area_height = area.height.saturating_sub(2); // status + command
    let alignment_area_width = area.width;

    // Calculate pane dimensions based on split mode
    let (pane_height, pane_width) = match split_mode {
        None => (alignment_area_height, alignment_area_width),
        Some(SplitMode::Horizontal) => {
            // Each pane gets ~50% of height
            (alignment_area_height / 2, alignment_area_width)
        }
        Some(SplitMode::Vertical) => {
            // Each pane gets ~50% of width
            (alignment_area_height, alignment_area_width / 2)
        }
    };

    // Subtract borders (2), ruler height, and SS_cons height
    let inner_height = pane_height.saturating_sub(2 + ruler_height + ss_cons_height) as usize;
    let inner_width = (pane_width as usize).saturating_sub(id_formatter.width() + 2);

    (inner_height, inner_width)
}

/// Render splash screen when no file is loaded.
fn render_splash(frame: &mut Frame, area: Rect) {
    // Rainbow colors for the helix
    let helix_colors = [
        Color::Rgb(255, 0, 0),   // Red
        Color::Rgb(255, 127, 0), // Orange
        Color::Rgb(255, 255, 0), // Yellow
        Color::Rgb(0, 255, 0),   // Green
        Color::Rgb(0, 127, 255), // Blue
        Color::Rgb(127, 0, 255), // Purple
    ];

    let version = env!("CARGO_PKG_VERSION");
    let description = "Terminal Stockholm alignment editor";

    let mut lines: Vec<Line> = Vec::new();

    // Add some vertical padding
    let vertical_padding = area.height.saturating_sub(20) / 2;
    for _ in 0..vertical_padding {
        lines.push(Line::from(""));
    }

    // Calculate horizontal padding for centering
    // Logo block is about 38 chars wide
    let logo_width = 38;
    let h_pad = (area.width as usize).saturating_sub(logo_width) / 2;
    let pad = " ".repeat(h_pad);

    // RNA helix + aform logo (hand-crafted ASCII art)
    let logo_lines = [
        ("  A───U  ", "                             "),
        (" G─┐ ┌─C ", "  __ _ / _|___  _ _ _ __     "),
        ("   │×│   ", " / _` |  _/ _ \\| '_| '  \\   "),
        (" C─┘ └─G ", " \\__,_|_| \\___/|_| |_|_|_|  "),
        ("  U───A  ", "                             "),
        (" A─┐ ┌─U ", "                             "),
        ("   │×│   ", "                             "),
        (" G─┘ └─C ", "                             "),
    ];

    for (i, (helix, text)) in logo_lines.iter().enumerate() {
        let helix_color = helix_colors[i % helix_colors.len()];
        lines.push(Line::from(vec![
            Span::raw(pad.clone()),
            Span::styled(*helix, Style::default().fg(helix_color)),
            Span::styled(
                *text,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    lines.push(Line::from(""));

    // Version and description (centered)
    let ver_str = format!("v{version}");
    let ver_pad = " ".repeat((area.width as usize).saturating_sub(ver_str.len()) / 2);
    lines.push(Line::from(vec![
        Span::raw(ver_pad),
        Span::styled(ver_str, Style::default().fg(Color::DarkGray)),
    ]));

    let desc_pad = " ".repeat((area.width as usize).saturating_sub(description.len()) / 2);
    lines.push(Line::from(vec![
        Span::raw(desc_pad),
        Span::styled(description, Style::default().fg(Color::Gray)),
    ]));
    lines.push(Line::from(""));

    // Quick start (centered)
    let qs_title = "Quick Start";
    let qs_pad = " ".repeat((area.width as usize).saturating_sub(qs_title.len()) / 2);
    lines.push(Line::from(vec![
        Span::raw(qs_pad),
        Span::styled(
            qs_title,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(""));

    // Commands - use fixed width format for alignment
    let commands = [
        (":e       ", "Browse and open a file"),
        (":e <file>", "Open a specific file  "),
        ("?        ", "Show help             "),
        (":q       ", "Quit                  "),
    ];
    let cmd_width = 35;
    let cmd_pad = " ".repeat((area.width as usize).saturating_sub(cmd_width) / 2);

    for (cmd, desc) in commands {
        lines.push(Line::from(vec![
            Span::raw(cmd_pad.clone()),
            Span::styled(cmd, Style::default().fg(Color::Green)),
            Span::raw("  "),
            Span::raw(desc),
        ]));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

/// Render file explorer for browse mode.
fn render_file_explorer(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(ref explorer) = app.file_explorer {
        let block = Block::default()
            .title(" Open File (Enter=select, Esc=cancel) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(&explorer.widget(), inner);
    }
}

/// Render help overlay.
fn render_help(frame: &mut Frame) {
    let help_text = vec![
        Line::from(Span::styled(
            "aform-rs Help",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Navigation",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )),
        Line::from("  h/j/k/l     Move cursor"),
        Line::from("  0 ^ / $     Start/end of line"),
        Line::from("  gg / G      First/last sequence"),
        Line::from("  Ctrl-f/b    Page down/up"),
        Line::from("  Ctrl-d/u    Half page down/up"),
        Line::from("  gp          Go to paired base"),
        Line::from("  N|          Go to column N"),
        Line::from(""),
        Line::from(Span::styled(
            "Search",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )),
        Line::from("  /           Search (U/T tolerant)"),
        Line::from("  n / N       Next/previous match"),
        Line::from(""),
        Line::from(Span::styled(
            "Split Windows",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )),
        Line::from("  Ctrl-w s    Horizontal split (:sp)"),
        Line::from("  Ctrl-w v    Vertical split (:vs)"),
        Line::from("  Ctrl-w hjkl Switch pane (or arrows)"),
        Line::from("  Ctrl-w q    Close split (:q or :only)"),
        Line::from(""),
        Line::from(Span::styled(
            "Editing",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )),
        Line::from("  i           Insert mode (then . for gap)"),
        Line::from("  x           Delete gap at cursor"),
        Line::from("  I           Insert gap column"),
        Line::from("  X           Delete gap column"),
        Line::from("  < / >       Shift sequence left/right"),
        Line::from("  { / }       Throw sequence left/right"),
        Line::from("  u           Undo"),
        Line::from("  Ctrl-r      Redo"),
        Line::from(""),
        Line::from(Span::styled(
            "Commands",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )),
        Line::from("  :w          Save file"),
        Line::from("  :q          Quit (:q! to force)"),
        Line::from("  :wq         Save and quit"),
        Line::from("  :color X    Set color (ss/base/cons/off)"),
        Line::from("  :ruler      Toggle position ruler"),
        Line::from("  :rownum     Toggle row numbers"),
        Line::from("  :help       Show this help"),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    // Calculate centered popup area
    let area = frame.area();
    let popup_width = 50.min(area.width.saturating_sub(4));
    let popup_height = (help_text.len() as u16 + 2).min(area.height.saturating_sub(4));
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear the area and render popup
    frame.render_widget(Clear, popup_area);

    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    let help_paragraph = Paragraph::new(help_text)
        .block(help_block)
        .style(Style::default().bg(Color::Black));

    frame.render_widget(help_paragraph, popup_area);
}
