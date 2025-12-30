//! TUI rendering with ratatui.
#![allow(clippy::needless_range_loop)]

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::app::{ActivePane, App, ColorScheme, Mode, SplitMode, TerminalTheme};
use crate::color::{Rgb, get_color};

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

    // Render info overlay if active
    if app.show_info {
        render_info(frame, app);
    }
}

/// Height of the ruler in lines.
const RULER_HEIGHT: u16 = 2;

/// Formats the ID column (row number + sequence ID).
struct IdFormatter {
    row_width: usize,
    id_width: usize,
    show_row_numbers: bool,
    show_short_ids: bool,
    collapse_width: usize,
}

/// Format an annotation bar label with consistent styling.
fn format_annotation_label(
    name: &str,
    id_formatter: &IdFormatter,
    fg: Color,
    bg: Color,
) -> Line<'static> {
    let label = format!(
        "{:>row_w$} {:id_w$}",
        "═",
        name,
        row_w = id_formatter.row_width,
        id_w = id_formatter.id_width
    );
    Line::from(Span::styled(label, Style::reset().fg(fg).bg(bg)))
}

impl IdFormatter {
    fn new(
        num_sequences: usize,
        max_id_len: usize,
        show_row_numbers: bool,
        max_collapse_count: usize,
        show_short_ids: bool,
    ) -> Self {
        // Width for collapse count suffix: " (N)" where N is the max count
        let collapse_width = if max_collapse_count > 1 {
            // " (" + digits + ")"
            3 + max_collapse_count.to_string().len()
        } else {
            0
        };

        Self {
            row_width: if show_row_numbers {
                num_sequences.max(1).to_string().len()
            } else {
                0
            },
            id_width: max_id_len,
            show_row_numbers,
            show_short_ids,
            collapse_width,
        }
    }

    /// Total width of the formatted ID column.
    fn width(&self) -> usize {
        let base = if self.show_row_numbers {
            // Format: "row_num id " with spaces
            self.row_width + 1 + self.id_width + 1
        } else {
            // Format: "id " with trailing space
            self.id_width + 1
        };
        base + self.collapse_width
    }

    /// Format a row number and ID.
    fn format(&self, row: usize, id: &str) -> String {
        use crate::stockholm::short_id;
        let display_id = if self.show_short_ids {
            short_id(id)
        } else {
            id
        };
        if self.show_row_numbers {
            format!(
                "{:>row_w$} {:id_w$} ",
                row + 1,
                display_id,
                row_w = self.row_width,
                id_w = self.id_width
            )
        } else {
            format!("{:id_w$} ", display_id, id_w = self.id_width)
        }
    }
}

/// Render an alignment pane with the given viewport.
/// Layout: IDs | Alignment (with ruler above, SS_cons below) | Tree
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
        Style::default().fg(app.theme.border.active.to_color())
    } else {
        Style::default().fg(app.theme.border.inactive.to_color())
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
    let max_id_len = if app.show_short_ids {
        app.alignment.max_short_id_len().max(10)
    } else {
        app.alignment.max_id_len().max(10)
    };
    let max_collapse = app.max_collapse_count();
    let id_formatter = IdFormatter::new(
        num_seqs,
        max_id_len,
        app.show_row_numbers,
        max_collapse,
        app.show_short_ids,
    );
    let id_width = id_formatter.width();

    // Account for tree width if showing (separator + tree column)
    let tree_display_width = if app.show_tree && app.cluster_tree.is_some() {
        app.tree_width + 1
    } else {
        0
    };

    // Calculate alignment column width (cap at actual alignment width)
    let alignment_width = app.alignment.width();
    let available_width = (inner.width as usize)
        .saturating_sub(id_width + 1) // +1 for separator after IDs
        .saturating_sub(tree_display_width);
    let seq_width = alignment_width.min(available_width);

    // Vertical layout dimensions
    let ruler_height = if app.show_ruler { RULER_HEIGHT } else { 0 };
    let has_ss_cons = app.alignment.ss_cons().is_some();
    let ss_cons_height: u16 = if has_ss_cons { 1 } else { 0 };
    let has_rf = app.alignment.rf().is_some();
    let rf_height: u16 = if app.show_rf_bar && has_rf { 1 } else { 0 };
    let has_pp_cons = app.alignment.pp_cons().is_some();
    let pp_cons_height: u16 = if app.show_pp_cons && has_pp_cons {
        1
    } else {
        0
    };
    let consensus_height: u16 = if app.show_consensus { 1 } else { 0 };
    let conservation_height: u16 = if app.show_conservation_bar { 1 } else { 0 };

    // Calculate visible rows (inner height minus ruler and annotation bars)
    let visible_rows = (inner.height as usize)
        .saturating_sub(ruler_height as usize)
        .saturating_sub(ss_cons_height as usize)
        .saturating_sub(rf_height as usize)
        .saturating_sub(pp_cons_height as usize)
        .saturating_sub(consensus_height as usize)
        .saturating_sub(conservation_height as usize);

    // === Split horizontally: IDs | Alignment | Tree | Filler ===
    let h_constraints = if tree_display_width > 0 {
        vec![
            Constraint::Length(id_width as u16),       // IDs column
            Constraint::Length(1),                     // Separator
            Constraint::Length(seq_width as u16),      // Alignment column (capped)
            Constraint::Length(1),                     // Separator
            Constraint::Length(app.tree_width as u16), // Tree column
            Constraint::Min(0),                        // Filler (absorbs extra space)
        ]
    } else {
        vec![
            Constraint::Length(id_width as u16),  // IDs column
            Constraint::Length(1),                // Separator
            Constraint::Length(seq_width as u16), // Alignment column (capped)
            Constraint::Min(0),                   // Filler (absorbs extra space)
        ]
    };

    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(h_constraints)
        .split(inner);

    let ids_area = h_chunks[0];
    let align_area = h_chunks[2];
    let tree_area = if tree_display_width > 0 {
        Some(h_chunks[4])
    } else {
        None
    };

    // Total annotation bar height
    let annotation_height =
        ss_cons_height + rf_height + pp_cons_height + consensus_height + conservation_height;

    // Calculate actual sequence rows to display (may be less than visible_rows)
    let actual_seq_rows =
        (app.visible_sequence_count().saturating_sub(viewport_row)).min(visible_rows) as u16;

    // === Render IDs column (with vertical alignment to match sequences) ===
    render_ids_column(
        frame,
        app,
        ids_area,
        viewport_row,
        visible_rows,
        &id_formatter,
        ruler_height,
        annotation_height,
        actual_seq_rows,
    );

    // === Render separator line ===
    render_separator(
        frame,
        h_chunks[1],
        ruler_height,
        annotation_height,
        actual_seq_rows,
    );

    // === Render alignment column (with ruler above, annotation bars below) ===
    render_alignment_column(
        frame,
        app,
        align_area,
        viewport_row,
        viewport_col,
        visible_rows,
        seq_width,
        ruler_height,
        ss_cons_height,
        rf_height,
        pp_cons_height,
        consensus_height,
        conservation_height,
        is_active,
    );

    // === Render tree column if present ===
    if let Some(tree_rect) = tree_area {
        // Render separator before tree
        render_separator(
            frame,
            h_chunks[3],
            ruler_height,
            annotation_height,
            actual_seq_rows,
        );
        render_tree_column(
            frame,
            app,
            tree_rect,
            viewport_row,
            visible_rows,
            ruler_height,
            annotation_height,
            actual_seq_rows,
        );
    }
}

/// Render the IDs column (sequence identifiers).
#[allow(clippy::too_many_arguments)]
fn render_ids_column(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    viewport_row: usize,
    visible_rows: usize,
    id_formatter: &IdFormatter,
    ruler_height: u16,
    annotation_height: u16,
    actual_seq_rows: u16,
) {
    // Split to match alignment layout (blank space for ruler/annotation bars)
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(ruler_height),
            Constraint::Length(actual_seq_rows),
            Constraint::Length(annotation_height),
            Constraint::Min(0), // Filler
        ])
        .split(area);

    let ids_seq_area = v_chunks[1];
    let ids_annotation_area = v_chunks[2];

    // Render sequence IDs (with collapse count if enabled)
    // Get selection bounds for row highlighting in visual mode
    let selection_bounds = app.get_selection_bounds();

    let mut lines = Vec::new();
    for display_row in viewport_row..(viewport_row + visible_rows).min(app.visible_sequence_count())
    {
        let actual_row = app.display_to_actual_row(display_row);
        let seq = &app.alignment.sequences[actual_row];

        // Check if this row is in the visual selection
        let is_row_selected = selection_bounds
            .map(|(min_row, _, max_row, _)| display_row >= min_row && display_row <= max_row)
            .unwrap_or(false);

        let id_style = if is_row_selected {
            // Selection highlighting takes priority (includes cursor row in visual mode)
            Style::reset()
                .bg(app.theme.id_column.selected_bg.to_color())
                .fg(app.theme.id_column.selected_fg.to_color())
        } else if display_row == app.cursor_row {
            Style::reset().add_modifier(Modifier::BOLD)
        } else {
            Style::reset().fg(app.theme.id_column.text.to_color())
        };

        // Show collapse count if enabled and group has more than 1 member
        let collapse_count = app.get_collapse_count(display_row);
        let id_display = if collapse_count > 1 {
            format!(
                "{} ({})",
                id_formatter.format(display_row, &seq.id),
                collapse_count
            )
        } else {
            id_formatter.format(display_row, &seq.id)
        };
        lines.push(Line::from(Span::styled(id_display, id_style)));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, ids_seq_area);

    // Render annotation labels using helper
    let mut annotation_lines = Vec::new();

    if app.alignment.ss_cons().is_some() {
        annotation_lines.push(format_annotation_label(
            "#=GC SS_cons",
            id_formatter,
            app.theme.annotations.label_ss_cons_fg.to_color(),
            app.theme.annotations.ss_cons_bg.to_color(),
        ));
    }
    if app.show_rf_bar && app.alignment.rf().is_some() {
        annotation_lines.push(format_annotation_label(
            "#=GC RF",
            id_formatter,
            app.theme.annotations.label_rf_fg.to_color(),
            app.theme.annotations.rf_conserved_bg.to_color(),
        ));
    }
    if app.show_pp_cons && app.alignment.pp_cons().is_some() {
        annotation_lines.push(format_annotation_label(
            "#=GC PP_cons",
            id_formatter,
            app.theme.annotations.label_pp_cons_fg.to_color(),
            app.theme.annotations.pp_cons_bg.to_color(),
        ));
    }
    if app.show_consensus {
        annotation_lines.push(format_annotation_label(
            "Consensus",
            id_formatter,
            app.theme.annotations.label_consensus_fg.to_color(),
            app.theme.annotations.consensus_bg.to_color(),
        ));
    }
    if app.show_conservation_bar {
        annotation_lines.push(format_annotation_label(
            "Conservation",
            id_formatter,
            app.theme.annotations.label_conservation_fg.to_color(),
            app.theme.annotations.conservation_bg.to_color(),
        ));
    }

    if !annotation_lines.is_empty() {
        let label_para = Paragraph::new(annotation_lines);
        frame.render_widget(label_para, ids_annotation_area);
    }
}

/// Render a vertical separator line.
fn render_separator(
    frame: &mut Frame,
    area: Rect,
    ruler_height: u16,
    annotation_height: u16,
    actual_seq_rows: u16,
) {
    let mut lines = Vec::new();

    // Blank space for ruler area
    for _ in 0..ruler_height {
        lines.push(Line::from(Span::styled(
            "│",
            Style::reset().fg(Color::DarkGray),
        )));
    }

    // Separator for sequence rows
    for _ in 0..actual_seq_rows {
        lines.push(Line::from(Span::styled(
            "│",
            Style::reset().fg(Color::DarkGray),
        )));
    }

    // Separator for annotation bars
    for _ in 0..annotation_height {
        lines.push(Line::from(Span::styled(
            "│",
            Style::reset().fg(Color::DarkGray),
        )));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

/// Render the alignment column (ruler + sequences + annotation bars).
#[allow(clippy::too_many_arguments)]
fn render_alignment_column(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    viewport_row: usize,
    viewport_col: usize,
    visible_rows: usize,
    seq_width: usize,
    ruler_height: u16,
    ss_cons_height: u16,
    rf_height: u16,
    pp_cons_height: u16,
    consensus_height: u16,
    conservation_height: u16,
    is_active: bool,
) {
    // Total annotation bar height
    let annotation_height =
        ss_cons_height + rf_height + pp_cons_height + consensus_height + conservation_height;

    // Calculate actual sequence rows to display (may be less than visible_rows)
    let actual_seq_rows =
        (app.visible_sequence_count().saturating_sub(viewport_row)).min(visible_rows);

    // Split alignment area vertically: ruler | sequences | annotations | filler
    // Use Length for sequences so annotations follow immediately after
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(ruler_height),
            Constraint::Length(actual_seq_rows as u16),
            Constraint::Length(annotation_height),
            Constraint::Min(0), // Filler takes remaining space
        ])
        .split(area);

    let ruler_area = v_chunks[0];
    let seq_area = v_chunks[1];
    let annotation_area = v_chunks[2];

    // Further split annotation area
    let annotation_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(ss_cons_height),
            Constraint::Length(rf_height),
            Constraint::Length(pp_cons_height),
            Constraint::Length(consensus_height),
            Constraint::Length(conservation_height),
        ])
        .split(annotation_area);

    let ss_cons_area = annotation_chunks[0];
    let rf_area = annotation_chunks[1];
    let pp_cons_area = annotation_chunks[2];
    let consensus_area = annotation_chunks[3];
    let conservation_area = annotation_chunks[4];

    // Render ruler (no ID padding - ruler is only over alignment)
    if app.show_ruler {
        // Get cursor and paired column for base-pair display (only if this pane is active)
        let (cursor_col, paired_col) = if is_active {
            let paired = app.structure_cache.get_pair(app.cursor_col);
            (Some(app.cursor_col), paired)
        } else {
            (None, None)
        };
        let ruler_colors = (
            app.theme.ruler.numbers,
            app.theme.ruler.ticks,
            app.theme.ruler.pair_line,
        );
        let ruler_lines = render_ruler(
            0,
            seq_width,
            viewport_col,
            cursor_col,
            paired_col,
            ruler_colors,
        );
        let ruler_paragraph = Paragraph::new(ruler_lines);
        frame.render_widget(ruler_paragraph, ruler_area);
    }

    // Compute columns to render (handles hiding gap columns)
    let cols_to_render: Vec<usize> = if app.hide_gap_columns && !app.visible_columns.is_empty() {
        // viewport_col is in display column space when hiding
        app.visible_columns
            .iter()
            .skip(viewport_col)
            .take(seq_width)
            .copied()
            .collect()
    } else {
        (viewport_col..(viewport_col + seq_width).min(app.alignment.width())).collect()
    };

    // Render sequences
    let mut lines = Vec::new();
    for display_row in viewport_row..(viewport_row + visible_rows).min(app.visible_sequence_count())
    {
        let actual_row = app.display_to_actual_row(display_row);
        let seq = &app.alignment.sequences[actual_row];
        let mut spans = Vec::new();

        let seq_chars: Vec<char> = seq.chars().to_vec();
        for &col in &cols_to_render {
            let ch = seq_chars.get(col).copied().unwrap_or(' ');
            let is_cursor = is_active && display_row == app.cursor_row && col == app.cursor_col;

            let mut style = Style::reset();

            // Apply color scheme
            if let Some(color) = get_color(
                app.color_scheme,
                ch,
                col,
                actual_row,
                &app.alignment,
                &app.structure_cache,
                &app.gap_chars,
                app.reference_seq,
                app.sequence_type,
            ) {
                style = style.bg(color).fg(Color::Black);
            }

            // Highlight empty (all-gap) columns if enabled
            if app.highlight_gap_columns && app.alignment.is_empty_column(col, &app.gap_chars) {
                style = style.bg(app.theme.selection.gap_column_bg.to_color());
            }

            // Highlight search matches
            if let Some(is_current) = app.is_search_match(actual_row, col) {
                if is_current {
                    style = style
                        .bg(app.theme.selection.search_current_bg.to_color())
                        .fg(app.theme.selection.search_current_fg.to_color());
                } else {
                    style = style
                        .bg(app.theme.selection.search_other_bg.to_color())
                        .fg(app.theme.selection.search_other_fg.to_color());
                }
            }

            // Highlight visual selection
            if app.is_selected(display_row, col) {
                style = style.bg(app.theme.selection.visual_bg.to_color()).fg(app
                    .theme
                    .selection
                    .visual_fg
                    .to_color());
            }

            // Highlight paired column
            if let Some(paired_col) = app.structure_cache.get_pair(app.cursor_col)
                && col == paired_col
            {
                style = style
                    .bg(app.theme.selection.pair_highlight_bg.to_color())
                    .fg(app.theme.selection.pair_highlight_fg.to_color());
            }

            // Highlight cursor
            if is_cursor {
                style = style.add_modifier(Modifier::REVERSED);
            }

            spans.push(Span::styled(ch.to_string(), style));
        }

        lines.push(Line::from(spans));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, seq_area);

    // Render SS_cons
    if let Some(ss) = app.alignment.ss_cons() {
        let mut spans = Vec::new();

        let ss_chars: Vec<char> = ss.chars().collect();
        for &col in &cols_to_render {
            let ch = ss_chars.get(col).copied().unwrap_or(' ');
            let is_cursor_col = is_active && col == app.cursor_col;

            let mut style = Style::reset()
                .fg(app.theme.annotations.ss_cons_fg.to_color())
                .bg(app.theme.annotations.ss_cons_bg.to_color());

            // Highlight empty (all-gap) columns if enabled
            if app.highlight_gap_columns && app.alignment.is_empty_column(col, &app.gap_chars) {
                style = style.bg(app.theme.selection.gap_column_bg.to_color());
            }

            // Highlight paired bracket
            if let Some(paired_col) = app.structure_cache.get_pair(app.cursor_col)
                && col == paired_col
            {
                style = style
                    .fg(app.theme.annotations.ss_cons_paired_fg.to_color())
                    .bg(app.theme.annotations.ss_cons_paired_bg.to_color())
                    .add_modifier(Modifier::BOLD);
            }

            // Column indicator
            if is_cursor_col {
                style = style.add_modifier(Modifier::UNDERLINED);
            }

            spans.push(Span::styled(ch.to_string(), style));
        }

        let ss_line = Paragraph::new(Line::from(spans));
        frame.render_widget(ss_line, ss_cons_area);
    }

    // Render RF bar
    if app.show_rf_bar
        && let Some(rf) = app.alignment.rf()
    {
        render_rf_bar(
            frame,
            app,
            rf,
            rf_area,
            &cols_to_render,
            is_active,
            app.cursor_col,
        );
    }

    // Render PP_cons bar
    if app.show_pp_cons
        && let Some(pp) = app.alignment.pp_cons()
    {
        render_pp_cons_bar(
            frame,
            app,
            pp,
            pp_cons_area,
            &cols_to_render,
            is_active,
            app.cursor_col,
        );
    }

    // Render consensus bar
    if app.show_consensus {
        render_consensus_bar(frame, app, consensus_area, &cols_to_render, is_active);
    }

    // Render conservation bar
    if app.show_conservation_bar {
        render_conservation_bar(frame, app, conservation_area, &cols_to_render, is_active);
    }
}

/// Render the consensus bar (showing the most common character at each position).
fn render_consensus_bar(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    cols_to_render: &[usize],
    is_active: bool,
) {
    use crate::color::get_consensus_char_with_case;

    let mut spans = Vec::new();

    for &col in cols_to_render {
        let ch = get_consensus_char_with_case(
            col,
            &app.alignment,
            &app.gap_chars,
            app.consensus_threshold,
        );
        let is_cursor_col = is_active && col == app.cursor_col;

        let mut style = Style::reset()
            .fg(app.theme.annotations.consensus_fg.to_color())
            .bg(app.theme.annotations.consensus_bg.to_color());

        // Highlight empty (all-gap) columns if enabled
        if app.highlight_gap_columns && app.alignment.is_empty_column(col, &app.gap_chars) {
            style = style.bg(app.theme.selection.gap_column_bg.to_color());
        }

        if is_cursor_col {
            style = style.add_modifier(Modifier::UNDERLINED);
        }

        spans.push(Span::styled(ch.to_string(), style));
    }

    let line = Paragraph::new(Line::from(spans));
    frame.render_widget(line, area);
}

/// Render the conservation bar (showing conservation level with block characters).
fn render_conservation_bar(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    cols_to_render: &[usize],
    is_active: bool,
) {
    use crate::color::{calculate_conservation, conservation_to_block};

    let mut spans = Vec::new();

    for &col in cols_to_render {
        let conservation = calculate_conservation(col, &app.alignment, &app.gap_chars);
        let (ch, color) = conservation_to_block(conservation);
        let is_cursor_col = is_active && col == app.cursor_col;

        let mut style =
            Style::reset()
                .fg(color)
                .bg(app.theme.annotations.conservation_bg.to_color());

        // Highlight empty (all-gap) columns if enabled
        if app.highlight_gap_columns && app.alignment.is_empty_column(col, &app.gap_chars) {
            style = style.bg(app.theme.selection.gap_column_bg.to_color());
        }

        if is_cursor_col {
            style = style.add_modifier(Modifier::UNDERLINED);
        }

        spans.push(Span::styled(ch.to_string(), style));
    }

    let line = Paragraph::new(Line::from(spans));
    frame.render_widget(line, area);
}

/// Render the RF (reference sequence) bar.
fn render_rf_bar(
    frame: &mut Frame,
    app: &App,
    rf: &str,
    area: Rect,
    cols_to_render: &[usize],
    is_active: bool,
    cursor_col: usize,
) {
    let rf_chars: Vec<char> = rf.chars().collect();
    let mut spans = Vec::new();

    for &col in cols_to_render {
        let ch = rf_chars.get(col).copied().unwrap_or(' ');
        let is_cursor_col = is_active && col == cursor_col;

        // Uppercase or 'x'/'X' = conserved (green), lowercase/gaps = variable (gray)
        let mut style = if ch.is_uppercase() || ch == 'x' || ch == 'X' {
            Style::reset()
                .fg(app.theme.annotations.rf_conserved_fg.to_color())
                .bg(app.theme.annotations.rf_conserved_bg.to_color())
        } else {
            Style::reset()
                .fg(app.theme.annotations.rf_variable_fg.to_color())
                .bg(app.theme.annotations.rf_variable_bg.to_color())
        };

        // Highlight empty (all-gap) columns if enabled
        if app.highlight_gap_columns && app.alignment.is_empty_column(col, &app.gap_chars) {
            style = style.bg(app.theme.selection.gap_column_bg.to_color());
        }

        if is_cursor_col {
            style = style.add_modifier(Modifier::UNDERLINED);
        }

        spans.push(Span::styled(ch.to_string(), style));
    }

    let line = Paragraph::new(Line::from(spans));
    frame.render_widget(line, area);
}

/// Render the PP_cons (posterior probability consensus) bar.
fn render_pp_cons_bar(
    frame: &mut Frame,
    app: &App,
    pp: &str,
    area: Rect,
    cols_to_render: &[usize],
    is_active: bool,
    cursor_col: usize,
) {
    use crate::color::pp_to_color;

    let pp_chars: Vec<char> = pp.chars().collect();
    let mut spans = Vec::new();

    for &col in cols_to_render {
        let ch = pp_chars.get(col).copied().unwrap_or(' ');
        let is_cursor_col = is_active && col == cursor_col;

        let color = pp_to_color(ch);
        let mut style = Style::reset()
            .fg(color)
            .bg(app.theme.annotations.pp_cons_bg.to_color());

        // Highlight empty (all-gap) columns if enabled
        if app.highlight_gap_columns && app.alignment.is_empty_column(col, &app.gap_chars) {
            style = style.bg(app.theme.selection.gap_column_bg.to_color());
        }

        if is_cursor_col {
            style = style.add_modifier(Modifier::UNDERLINED);
        }

        spans.push(Span::styled(ch.to_string(), style));
    }

    let line = Paragraph::new(Line::from(spans));
    frame.render_widget(line, area);
}

/// Render the tree/dendrogram column.
#[allow(clippy::too_many_arguments)]
fn render_tree_column(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    viewport_row: usize,
    visible_rows: usize,
    ruler_height: u16,
    annotation_height: u16,
    actual_seq_rows: u16,
) {
    // Split to match alignment layout (blank space for ruler/annotation rows)
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(ruler_height),
            Constraint::Length(actual_seq_rows),
            Constraint::Length(annotation_height),
            Constraint::Min(0), // Filler
        ])
        .split(area);

    let tree_seq_area = v_chunks[1];

    // Render tree lines
    // Use collapsed_tree when collapse is enabled and available, otherwise use cluster_tree
    let mut lines = Vec::new();
    let tree_lines = if app.collapse_identical && app.collapsed_tree.is_some() {
        app.collapsed_tree.as_ref()
    } else {
        app.cluster_tree.as_ref()
    };

    if let Some(tree_lines) = tree_lines {
        for display_row in
            viewport_row..(viewport_row + visible_rows).min(app.visible_sequence_count())
        {
            // Tree lines are already in display order (clustered), use display_row directly
            if let Some(tree_str) = tree_lines.get(display_row) {
                let tree_color = match app.terminal_theme {
                    TerminalTheme::Dark => app.theme.misc.tree_dark_theme.to_color(),
                    TerminalTheme::Light => app.theme.misc.tree_light_theme.to_color(),
                };
                lines.push(Line::from(Span::styled(
                    tree_str.clone(),
                    Style::reset().fg(tree_color),
                )));
            } else {
                lines.push(Line::from(""));
            }
        }
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, tree_seq_area);
}

/// Render the position ruler (returns two lines: numbers and tick marks).
fn render_ruler(
    id_width: usize,
    seq_width: usize,
    viewport_col: usize,
    cursor_col: Option<usize>,
    paired_col: Option<usize>,
    ruler_colors: (Rgb, Rgb, Rgb), // (numbers, ticks, pair_line)
) -> Vec<Line<'static>> {
    let (numbers_color, ticks_color, pair_color) = ruler_colors;
    let mut lines = Vec::new();

    // First line: position numbers
    let mut number_spans = Vec::new();
    number_spans.push(Span::styled(
        " ".repeat(id_width),
        Style::reset().fg(numbers_color.to_color()),
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
        Style::reset().fg(numbers_color.to_color()),
    ));
    lines.push(Line::from(number_spans));

    // Second line: tick marks with base-pair overlay
    let mut tick_spans = Vec::new();
    tick_spans.push(Span::styled(
        " ".repeat(id_width),
        Style::reset().fg(ticks_color.to_color()),
    ));

    // Build tick characters
    let mut tick_chars: Vec<char> = Vec::with_capacity(seq_width);
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

    // Track which positions are part of base-pair display
    let mut is_pair_display: Vec<bool> = vec![false; seq_width];

    // Overlay base-pair connection if both cursor and paired positions exist
    if let (Some(cursor), Some(paired)) = (cursor_col, paired_col) {
        let viewport_end = viewport_col + seq_width;
        let (left, right) = if cursor < paired {
            (cursor, paired)
        } else {
            (paired, cursor)
        };

        // Check if any part of the pair is visible
        let left_visible = left >= viewport_col && left < viewport_end;
        let right_visible = right >= viewport_col && right < viewport_end;

        if left_visible || right_visible {
            // Determine display range (clipped to viewport)
            let display_start = left.saturating_sub(viewport_col);
            let display_end = if right < viewport_end {
                right - viewport_col
            } else {
                seq_width - 1
            };

            // Draw connecting line
            for i in display_start..=display_end {
                tick_chars[i] = '─';
                is_pair_display[i] = true;
            }

            // Draw arrows at endpoints (if visible)
            if left_visible {
                let idx = left - viewport_col;
                tick_chars[idx] = '↓';
            }
            if right_visible {
                let idx = right - viewport_col;
                tick_chars[idx] = '↓';
            }
        }
    }

    // Build spans with different styles for normal ticks vs pair display
    let tick_style = Style::reset().fg(ticks_color.to_color());
    let pair_style = Style::reset().fg(pair_color.to_color());

    let mut i = 0;
    while i < seq_width {
        let is_pair = is_pair_display[i];
        let start = i;
        while i < seq_width && is_pair_display[i] == is_pair {
            i += 1;
        }
        let segment: String = tick_chars[start..i].iter().collect();
        let style = if is_pair { pair_style } else { tick_style };
        tick_spans.push(Span::styled(segment, style));
    }

    lines.push(Line::from(tick_spans));

    lines
}

/// Render the status bar.
fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let modes = &app.theme.status_bar.modes;
    let mode_style = match app.mode {
        Mode::Normal => Style::default()
            .bg(modes.normal_bg.to_color())
            .fg(modes.normal_fg.to_color()),
        Mode::Insert => Style::default()
            .bg(modes.insert_bg.to_color())
            .fg(modes.insert_fg.to_color()),
        Mode::Command => Style::default()
            .bg(modes.command_bg.to_color())
            .fg(modes.command_fg.to_color()),
        Mode::Search => Style::default()
            .bg(modes.search_bg.to_color())
            .fg(modes.search_fg.to_color()),
        Mode::Visual => Style::default()
            .bg(modes.visual_bg.to_color())
            .fg(modes.visual_fg.to_color()),
    };

    let mode_span = Span::styled(format!(" {} ", app.mode.as_ref()), mode_style);

    // Position info
    let pos_info = format!(" {}:{} ", app.cursor_row + 1, app.cursor_col + 1);

    // Alignment info (show collapsed count if enabled)
    let align_info = if app.collapse_identical && !app.collapse_groups.is_empty() {
        format!(
            " [{}→{}]x{} ",
            app.alignment.num_sequences(),
            app.collapse_groups.len(),
            app.alignment.width()
        )
    } else {
        format!(
            " {}x{} ",
            app.alignment.num_sequences(),
            app.alignment.width()
        )
    };

    // Sequence type
    let type_info = format!(" {} ", app.sequence_type.as_str());

    // Color scheme
    let color_info = if app.color_scheme != ColorScheme::None {
        format!(" [{}] ", app.color_scheme.as_ref())
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
        Span::styled(
            pos_info,
            Style::default().fg(app.theme.status_bar.position.to_color()),
        ),
        Span::styled(
            align_info,
            Style::default().fg(app.theme.status_bar.alignment_info.to_color()),
        ),
        Span::styled(
            type_info,
            Style::default().fg(app.theme.status_bar.sequence_type.to_color()),
        ),
        Span::styled(
            color_info,
            Style::default().fg(app.theme.status_bar.color_scheme.to_color()),
        ),
        Span::styled(
            structure_info,
            Style::default().fg(app.theme.status_bar.structure_info.to_color()),
        ),
        Span::styled(
            selection_info,
            Style::default().fg(app.theme.status_bar.selection_info.to_color()),
        ),
        Span::raw(char_info),
    ];

    let status = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(app.theme.status_bar.background.to_color()));

    frame.render_widget(status, area);
}

/// Render the command/message line.
fn render_command_line(frame: &mut Frame, app: &App, area: Rect) {
    let content = match app.mode {
        Mode::Command => Line::from(vec![
            Span::styled(
                ":",
                Style::default().fg(app.theme.command_line.command_prefix.to_color()),
            ),
            Span::raw(&app.command_buffer),
            Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)),
        ]),
        Mode::Search => Line::from(vec![
            Span::styled(
                "/",
                Style::default().fg(app.theme.command_line.search_prefix.to_color()),
            ),
            Span::raw(&app.search.pattern),
            Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)),
        ]),
        _ => {
            if let Some(msg) = &app.status_message {
                Line::from(Span::raw(msg.as_str()))
            } else {
                // Show help hint
                Line::from(Span::styled(
                    "Press : for commands, ? for help, / for search",
                    Style::default().fg(app.theme.command_line.help_hint.to_color()),
                ))
            }
        }
    };

    let paragraph = Paragraph::new(content);
    frame.render_widget(paragraph, area);
}

/// Calculate visible dimensions for the alignment area.
#[allow(clippy::too_many_arguments)]
pub fn visible_dimensions(
    area: Rect,
    num_sequences: usize,
    max_id_len: usize,
    show_ruler: bool,
    show_row_numbers: bool,
    show_short_ids: bool,
    split_mode: Option<SplitMode>,
    has_ss_cons: bool,
    has_rf: bool,
    show_rf_bar: bool,
    has_pp_cons: bool,
    show_pp_cons: bool,
    show_consensus: bool,
    show_conservation_bar: bool,
    max_collapse_count: usize,
    tree_display_width: usize,
    alignment_width: usize,
) -> (usize, usize) {
    let id_formatter = IdFormatter::new(
        num_sequences,
        max_id_len.max(10),
        show_row_numbers,
        max_collapse_count,
        show_short_ids,
    );
    let ruler_height = if show_ruler { RULER_HEIGHT } else { 0 };
    let ss_cons_height: u16 = if has_ss_cons { 1 } else { 0 };
    let rf_height: u16 = if show_rf_bar && has_rf { 1 } else { 0 };
    let pp_cons_height: u16 = if show_pp_cons && has_pp_cons { 1 } else { 0 };
    let consensus_height: u16 = if show_consensus { 1 } else { 0 };
    let conservation_height: u16 = if show_conservation_bar { 1 } else { 0 };
    let annotation_height =
        ss_cons_height + rf_height + pp_cons_height + consensus_height + conservation_height;

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

    // Subtract borders (2), ruler height, annotation bar heights, and tree width
    // Cap at alignment width (no excess space beyond alignment)
    let inner_height = pane_height.saturating_sub(2 + ruler_height + annotation_height) as usize;
    let inner_width = (pane_width as usize)
        .saturating_sub(id_formatter.width() + 2)
        .saturating_sub(tree_display_width)
        .min(alignment_width);

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
        (":e <path>", "Open file (Tab=complete)"),
        ("?        ", "Show help               "),
        (":q       ", "Quit                    "),
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
        Line::from("  :color X    Set color (ss/base/protein/cons)"),
        Line::from("  :type X     Set seq type (rna/dna/protein/auto)"),
        Line::from("  :collapse   Toggle collapse identical seqs"),
        Line::from("  :consensus  Toggle consensus bar"),
        Line::from("  :conserv..  Toggle conservation bar"),
        Line::from("  :cluster    Cluster sequences by similarity"),
        Line::from("  :uncluster  Restore original order"),
        Line::from("  :tree       Toggle dendrogram tree"),
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

/// Render file info overlay.
fn render_info(frame: &mut Frame, app: &App) {
    let mut lines = vec![
        Line::from(Span::styled(
            "File Information",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    // Display key annotations
    let annotations = [
        ("ID", "Identifier"),
        ("AC", "Accession"),
        ("DE", "Description"),
        ("AU", "Author"),
        ("SE", "Source"),
        ("TP", "Type"),
    ];

    for (tag, label) in annotations {
        if let Some(value) = app.alignment.get_file_annotation(tag) {
            lines.push(Line::from(vec![
                Span::styled(format!("{label}: "), Style::default().fg(Color::Yellow)),
                Span::raw(value),
            ]));
        }
    }

    // Add CC (comments) if present - may span multiple lines
    let comments = app.alignment.get_file_annotations("CC");
    if !comments.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Comments:",
            Style::default().fg(Color::Yellow),
        )));
        for comment in comments.iter().take(5) {
            // Limit to 5 comment lines
            lines.push(Line::from(format!("  {comment}")));
        }
        if comments.len() > 5 {
            lines.push(Line::from(Span::styled(
                format!("  ... and {} more", comments.len() - 5),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    // Show file path and statistics
    lines.push(Line::from(""));
    if let Some(path) = &app.file_path {
        lines.push(Line::from(vec![
            Span::styled("File: ", Style::default().fg(Color::Yellow)),
            Span::raw(path.display().to_string()),
        ]));
    }
    lines.push(Line::from(vec![
        Span::styled("Sequences: ", Style::default().fg(Color::Yellow)),
        Span::raw(app.alignment.num_sequences().to_string()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Columns: ", Style::default().fg(Color::Yellow)),
        Span::raw(app.alignment.width().to_string()),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press any key to close",
        Style::default().fg(Color::DarkGray),
    )));

    // Calculate centered popup area
    let area = frame.area();
    let popup_width = 60.min(area.width.saturating_sub(4));
    let popup_height = (lines.len() as u16 + 2).min(area.height.saturating_sub(4));
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear the area and render popup
    frame.render_widget(Clear, popup_area);

    let info_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .style(Style::default().bg(Color::Black));

    let info_paragraph = Paragraph::new(lines)
        .block(info_block)
        .style(Style::default().bg(Color::Black));

    frame.render_widget(info_paragraph, popup_area);
}
