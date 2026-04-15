//! SVG export for alignment visualization.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

use ratatui::style::Color;

use crate::app::App;
use crate::color::{
    calculate_conservation, conservation_to_block, get_color, get_consensus_char_with_case,
    pp_to_color,
};
use crate::stockholm::short_id;

/// Layout constants for the SVG grid.
const CELL_WIDTH: f64 = 9.6;
const CELL_HEIGHT: f64 = 18.0;
const FONT_SIZE: f64 = 14.0;
const FONT_FAMILY: &str = "monospace";

/// Convert a ratatui Color to a CSS hex string.
fn color_to_hex(color: Color) -> String {
    match color {
        Color::Rgb(r, g, b) => format!("#{r:02X}{g:02X}{b:02X}"),
        Color::Black => "#000000".to_string(),
        Color::White => "#FFFFFF".to_string(),
        Color::DarkGray => "#808080".to_string(),
        Color::Gray => "#A0A0A0".to_string(),
        Color::Red => "#FF0000".to_string(),
        Color::Green => "#00FF00".to_string(),
        Color::Blue => "#0000FF".to_string(),
        Color::Yellow => "#FFFF00".to_string(),
        Color::Cyan => "#00FFFF".to_string(),
        Color::Magenta => "#FF00FF".to_string(),
        Color::LightRed => "#FF8080".to_string(),
        Color::LightGreen => "#80FF80".to_string(),
        Color::LightBlue => "#8080FF".to_string(),
        Color::LightYellow => "#FFFF80".to_string(),
        Color::LightCyan => "#80FFFF".to_string(),
        Color::LightMagenta => "#FF80FF".to_string(),
        _ => "#808080".to_string(),
    }
}

/// Escape special XML characters in text content.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Export the current alignment as an SVG file.
pub fn export_svg(app: &App, path: &Path) -> io::Result<()> {
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);
    write_svg(app, &mut w)
}

fn write_svg<W: Write>(app: &App, w: &mut W) -> io::Result<()> {
    let alignment = &app.alignment;
    if alignment.num_sequences() == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Empty alignment",
        ));
    }

    // Determine columns to render
    let cols: Vec<usize> = if app.hide_gap_columns && !app.visible_columns.is_empty() {
        app.visible_columns.clone()
    } else {
        (0..alignment.width()).collect()
    };
    let num_cols = cols.len();

    // Determine rows to render
    let num_display_rows = app.visible_sequence_count();

    // Calculate ID column width
    let max_id_len = if app.show_short_ids {
        alignment.max_short_id_len()
    } else {
        alignment.max_id_len()
    };
    let row_num_width = if app.show_row_numbers {
        num_display_rows.max(1).to_string().len() + 1 // +1 for space after number
    } else {
        0
    };
    // Collapse count width
    let max_collapse_count = if app.collapse_identical {
        (0..num_display_rows)
            .map(|r| app.get_collapse_count(r))
            .max()
            .unwrap_or(1)
    } else {
        1
    };
    let collapse_width = if max_collapse_count > 1 {
        3 + max_collapse_count.to_string().len() // " (N)"
    } else {
        0
    };
    let id_chars = row_num_width + max_id_len + 1 + collapse_width; // +1 trailing space

    // Count annotation rows
    let has_ss_cons = alignment.ss_cons().is_some();
    let has_rf = app.show_rf_bar && alignment.rf().is_some();
    let has_pp_cons = app.show_pp_cons && alignment.pp_cons().is_some();
    let has_consensus = app.show_consensus;
    let has_conservation = app.show_conservation_bar;

    let annotation_rows = has_ss_cons as usize
        + has_rf as usize
        + has_pp_cons as usize
        + has_consensus as usize
        + has_conservation as usize;

    let ruler_rows: usize = if app.show_ruler { 2 } else { 0 };

    // Total layout
    let separator_chars = 1; // "│" separator between ID and sequence
    let total_char_cols = id_chars + separator_chars + num_cols;
    let total_rows = ruler_rows + num_display_rows + annotation_rows;

    let svg_width = total_char_cols as f64 * CELL_WIDTH;
    let svg_height = total_rows as f64 * CELL_HEIGHT;

    // SVG header
    writeln!(w, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
    writeln!(
        w,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{svg_width}\" height=\"{svg_height}\" viewBox=\"0 0 {svg_width} {svg_height}\">"
    )?;

    // Background
    let bg_color = if app.terminal_theme == crate::app::TerminalTheme::Light {
        "#FFFFFF"
    } else {
        "#1E1E1E"
    };
    writeln!(
        w,
        "<rect width=\"{svg_width}\" height=\"{svg_height}\" fill=\"{bg_color}\"/>"
    )?;

    // Common text style
    writeln!(
        w,
        "<style>text {{ font-family: {FONT_FAMILY}; font-size: {FONT_SIZE}px; dominant-baseline: central; }}</style>"
    )?;

    let seq_x_offset = (id_chars + separator_chars) as f64 * CELL_WIDTH;
    let id_text_color = color_to_hex(app.theme.id_column.text.to_color());
    let default_text_color = if app.terminal_theme == crate::app::TerminalTheme::Light {
        "#000000"
    } else {
        "#D4D4D4"
    };

    // ─── Ruler ───
    let mut current_row = 0usize;
    if app.show_ruler {
        let ruler_color = color_to_hex(app.theme.ruler.numbers.to_color());
        let tick_color = color_to_hex(app.theme.ruler.ticks.to_color());

        // Numbers line
        let y = current_row as f64 * CELL_HEIGHT + CELL_HEIGHT / 2.0;
        for (display_idx, &col) in cols.iter().enumerate() {
            let pos = col + 1; // 1-based
            if pos % 10 == 0 {
                let pos_str = pos.to_string();
                // Right-align number ending at this position
                let label_end = display_idx;
                let label_start = label_end.saturating_sub(pos_str.len() - 1);
                let x = seq_x_offset + label_start as f64 * CELL_WIDTH;
                writeln!(
                    w,
                    "<text x=\"{x}\" y=\"{y}\" fill=\"{ruler_color}\">{}</text>",
                    xml_escape(&pos_str)
                )?;
            }
        }
        current_row += 1;

        // Tick line
        let y = current_row as f64 * CELL_HEIGHT + CELL_HEIGHT / 2.0;
        for (display_idx, &col) in cols.iter().enumerate() {
            let pos = col + 1;
            let ch = if pos % 10 == 0 {
                '|'
            } else if pos % 5 == 0 {
                '+'
            } else {
                '\u{00B7}' // middle dot
            };
            let x = seq_x_offset + display_idx as f64 * CELL_WIDTH;
            writeln!(
                w,
                "<text x=\"{x}\" y=\"{y}\" fill=\"{tick_color}\">{}</text>",
                xml_escape(&ch.to_string())
            )?;
        }
        current_row += 1;
    }

    // ─── Separator line (vertical) ───
    let sep_x = id_chars as f64 * CELL_WIDTH;
    let sep_color = color_to_hex(app.theme.misc.separator.to_color());
    writeln!(
        w,
        "<line x1=\"{sep_x}\" y1=\"0\" x2=\"{sep_x}\" y2=\"{svg_height}\" stroke=\"{sep_color}\" stroke-width=\"1\"/>"
    )?;

    // ─── Sequence IDs + Sequence grid ───
    for display_row in 0..num_display_rows {
        let actual_row = app.display_to_actual_row(display_row);
        let seq = &alignment.sequences[actual_row];
        let y_top = (current_row + display_row) as f64 * CELL_HEIGHT;
        let y_text = y_top + CELL_HEIGHT / 2.0;

        // ID text
        let display_id = if app.show_short_ids {
            short_id(&seq.id)
        } else {
            &seq.id
        };
        let id_text = if app.show_row_numbers {
            let row_str = format!("{:>w$}", display_row + 1, w = row_num_width - 1);
            format!("{row_str} {display_id}")
        } else {
            display_id.to_string()
        };
        let id_text = if max_collapse_count > 1 {
            let count = app.get_collapse_count(display_row);
            if count > 1 {
                format!("{id_text} ({count})")
            } else {
                id_text
            }
        } else {
            id_text
        };
        writeln!(
            w,
            "<text x=\"2\" y=\"{y_text}\" fill=\"{id_text_color}\">{}</text>",
            xml_escape(&id_text)
        )?;

        // Sequence cells
        let seq_chars: Vec<char> = seq.chars().to_vec();
        for (display_idx, &col) in cols.iter().enumerate() {
            let ch = seq_chars.get(col).copied().unwrap_or(' ');
            let x = seq_x_offset + display_idx as f64 * CELL_WIDTH;

            // Get background color from color scheme
            let bg = get_color(
                app.color_scheme,
                ch,
                col,
                actual_row,
                alignment,
                &app.structure_cache,
                &app.gap_chars,
                app.reference_seq,
                app.sequence_type,
                app.terminal_theme,
            );

            // Highlight gap columns
            let bg = if bg.is_none()
                && app.highlight_gap_columns
                && alignment.is_empty_column(col, &app.gap_chars)
            {
                Some(app.theme.selection.gap_column_bg.to_color())
            } else {
                bg
            };

            if let Some(color) = bg {
                let hex = color_to_hex(color);
                writeln!(
                    w,
                    "<rect x=\"{x}\" y=\"{y_top}\" width=\"{CELL_WIDTH}\" height=\"{CELL_HEIGHT}\" fill=\"{hex}\"/>"
                )?;
            }

            // Text
            let fg = if bg.is_some() {
                "#000000"
            } else {
                default_text_color
            };
            if ch != ' ' && !ch.is_whitespace() {
                writeln!(
                    w,
                    "<text x=\"{x}\" y=\"{y_text}\" fill=\"{fg}\">{}</text>",
                    xml_escape(&ch.to_string())
                )?;
            }
        }
    }
    current_row += num_display_rows;

    // ─── Annotation bars ───

    // SS_cons
    if let Some(ss) = alignment.ss_cons() {
        let ss_chars: Vec<char> = ss.chars().collect();
        let y_top = current_row as f64 * CELL_HEIGHT;
        let y_text = y_top + CELL_HEIGHT / 2.0;
        let fg_hex = color_to_hex(app.theme.annotations.ss_cons_fg.to_color());
        let bg_hex = color_to_hex(app.theme.annotations.ss_cons_bg.to_color());

        // Label
        writeln!(
            w,
            "<rect x=\"0\" y=\"{y_top}\" width=\"{sep_x}\" height=\"{CELL_HEIGHT}\" fill=\"{bg_hex}\"/>"
        )?;
        let label_fg = color_to_hex(app.theme.annotations.label_ss_cons_fg.to_color());
        writeln!(
            w,
            "<text x=\"2\" y=\"{y_text}\" fill=\"{label_fg}\">#=GC SS_cons</text>"
        )?;

        // Bar
        for (display_idx, &col) in cols.iter().enumerate() {
            let ch = ss_chars.get(col).copied().unwrap_or(' ');
            let x = seq_x_offset + display_idx as f64 * CELL_WIDTH;
            writeln!(
                w,
                "<rect x=\"{x}\" y=\"{y_top}\" width=\"{CELL_WIDTH}\" height=\"{CELL_HEIGHT}\" fill=\"{bg_hex}\"/>"
            )?;
            if ch != ' ' {
                writeln!(
                    w,
                    "<text x=\"{x}\" y=\"{y_text}\" fill=\"{fg_hex}\">{}</text>",
                    xml_escape(&ch.to_string())
                )?;
            }
        }
        current_row += 1;
    }

    // RF bar
    if let Some(rf) = {
        if app.show_rf_bar {
            alignment.rf()
        } else {
            None
        }
    } {
        let rf_chars: Vec<char> = rf.chars().collect();
        let y_top = current_row as f64 * CELL_HEIGHT;
        let y_text = y_top + CELL_HEIGHT / 2.0;

        // Label
        let label_bg = color_to_hex(app.theme.annotations.rf_conserved_bg.to_color());
        let label_fg = color_to_hex(app.theme.annotations.label_rf_fg.to_color());
        writeln!(
            w,
            "<rect x=\"0\" y=\"{y_top}\" width=\"{sep_x}\" height=\"{CELL_HEIGHT}\" fill=\"{label_bg}\"/>"
        )?;
        writeln!(
            w,
            "<text x=\"2\" y=\"{y_text}\" fill=\"{label_fg}\">#=GC RF</text>"
        )?;

        let conserved_fg = color_to_hex(app.theme.annotations.rf_conserved_fg.to_color());
        let conserved_bg = color_to_hex(app.theme.annotations.rf_conserved_bg.to_color());
        let variable_fg = color_to_hex(app.theme.annotations.rf_variable_fg.to_color());
        let variable_bg = color_to_hex(app.theme.annotations.rf_variable_bg.to_color());

        for (display_idx, &col) in cols.iter().enumerate() {
            let ch = rf_chars.get(col).copied().unwrap_or(' ');
            let x = seq_x_offset + display_idx as f64 * CELL_WIDTH;
            let is_conserved = ch.is_uppercase() || ch == 'x' || ch == 'X';
            let (fg, bg) = if is_conserved {
                (&conserved_fg, &conserved_bg)
            } else {
                (&variable_fg, &variable_bg)
            };
            writeln!(
                w,
                "<rect x=\"{x}\" y=\"{y_top}\" width=\"{CELL_WIDTH}\" height=\"{CELL_HEIGHT}\" fill=\"{bg}\"/>"
            )?;
            if ch != ' ' {
                writeln!(
                    w,
                    "<text x=\"{x}\" y=\"{y_text}\" fill=\"{fg}\">{}</text>",
                    xml_escape(&ch.to_string())
                )?;
            }
        }
        current_row += 1;
    }

    // PP_cons bar
    if let Some(pp) = {
        if app.show_pp_cons {
            alignment.pp_cons()
        } else {
            None
        }
    } {
        let pp_chars: Vec<char> = pp.chars().collect();
        let y_top = current_row as f64 * CELL_HEIGHT;
        let y_text = y_top + CELL_HEIGHT / 2.0;
        let bg_hex = color_to_hex(app.theme.annotations.pp_cons_bg.to_color());

        // Label
        let label_fg = color_to_hex(app.theme.annotations.label_pp_cons_fg.to_color());
        writeln!(
            w,
            "<rect x=\"0\" y=\"{y_top}\" width=\"{sep_x}\" height=\"{CELL_HEIGHT}\" fill=\"{bg_hex}\"/>"
        )?;
        writeln!(
            w,
            "<text x=\"2\" y=\"{y_text}\" fill=\"{label_fg}\">#=GC PP_cons</text>"
        )?;

        for (display_idx, &col) in cols.iter().enumerate() {
            let ch = pp_chars.get(col).copied().unwrap_or(' ');
            let x = seq_x_offset + display_idx as f64 * CELL_WIDTH;
            let fg_color = pp_to_color(ch);
            let fg_hex = color_to_hex(fg_color);
            writeln!(
                w,
                "<rect x=\"{x}\" y=\"{y_top}\" width=\"{CELL_WIDTH}\" height=\"{CELL_HEIGHT}\" fill=\"{bg_hex}\"/>"
            )?;
            if ch != ' ' {
                writeln!(
                    w,
                    "<text x=\"{x}\" y=\"{y_text}\" fill=\"{fg_hex}\">{}</text>",
                    xml_escape(&ch.to_string())
                )?;
            }
        }
        current_row += 1;
    }

    // Consensus bar
    if app.show_consensus {
        let y_top = current_row as f64 * CELL_HEIGHT;
        let y_text = y_top + CELL_HEIGHT / 2.0;
        let fg_hex = color_to_hex(app.theme.annotations.consensus_fg.to_color());
        let bg_hex = color_to_hex(app.theme.annotations.consensus_bg.to_color());

        // Label
        let label_fg = color_to_hex(app.theme.annotations.label_consensus_fg.to_color());
        writeln!(
            w,
            "<rect x=\"0\" y=\"{y_top}\" width=\"{sep_x}\" height=\"{CELL_HEIGHT}\" fill=\"{bg_hex}\"/>"
        )?;
        writeln!(
            w,
            "<text x=\"2\" y=\"{y_text}\" fill=\"{label_fg}\">Consensus</text>"
        )?;

        for (display_idx, &col) in cols.iter().enumerate() {
            let ch = get_consensus_char_with_case(
                col,
                alignment,
                &app.gap_chars,
                app.consensus_threshold,
            );
            let x = seq_x_offset + display_idx as f64 * CELL_WIDTH;
            writeln!(
                w,
                "<rect x=\"{x}\" y=\"{y_top}\" width=\"{CELL_WIDTH}\" height=\"{CELL_HEIGHT}\" fill=\"{bg_hex}\"/>"
            )?;
            if ch != ' ' {
                writeln!(
                    w,
                    "<text x=\"{x}\" y=\"{y_text}\" fill=\"{fg_hex}\">{}</text>",
                    xml_escape(&ch.to_string())
                )?;
            }
        }
        current_row += 1;
    }

    // Conservation bar
    if app.show_conservation_bar {
        let y_top = current_row as f64 * CELL_HEIGHT;
        let y_text = y_top + CELL_HEIGHT / 2.0;
        let bg_hex = color_to_hex(app.theme.annotations.conservation_bg.to_color());

        // Label
        let label_fg = color_to_hex(app.theme.annotations.label_conservation_fg.to_color());
        writeln!(
            w,
            "<rect x=\"0\" y=\"{y_top}\" width=\"{sep_x}\" height=\"{CELL_HEIGHT}\" fill=\"{bg_hex}\"/>"
        )?;
        writeln!(
            w,
            "<text x=\"2\" y=\"{y_text}\" fill=\"{label_fg}\">Conservation</text>"
        )?;

        for (display_idx, &col) in cols.iter().enumerate() {
            let conservation = calculate_conservation(col, alignment, &app.gap_chars);
            let (ch, color) = conservation_to_block(conservation);
            let x = seq_x_offset + display_idx as f64 * CELL_WIDTH;
            let fg_hex = color_to_hex(color);
            writeln!(
                w,
                "<rect x=\"{x}\" y=\"{y_top}\" width=\"{CELL_WIDTH}\" height=\"{CELL_HEIGHT}\" fill=\"{bg_hex}\"/>"
            )?;
            if ch != ' ' {
                writeln!(
                    w,
                    "<text x=\"{x}\" y=\"{y_text}\" fill=\"{fg_hex}\">{}</text>",
                    xml_escape(&ch.to_string())
                )?;
            }
        }
        // current_row += 1; // not needed, last section
    }

    // Close SVG
    writeln!(w, "</svg>")?;
    w.flush()
}
