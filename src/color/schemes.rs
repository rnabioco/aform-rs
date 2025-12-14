//! Color scheme implementations for alignment display.

use ratatui::style::Color;

use crate::app::ColorScheme;
use crate::stockholm::Alignment;
use crate::structure::{analyze_compensatory, CompensatoryChange, StructureCache};

/// Colors for helix highlighting (cycling through these).
pub const HELIX_COLORS: &[Color] = &[
    Color::Rgb(135, 206, 235), // skyblue
    Color::Rgb(144, 238, 144), // lightgreen
    Color::Rgb(255, 182, 193), // pink
    Color::Rgb(255, 255, 0),   // yellow
    Color::Rgb(238, 130, 238), // violet
    Color::Rgb(255, 215, 0),   // gold
    Color::Rgb(245, 222, 179), // wheat
    Color::Rgb(0, 255, 255),   // cyan
    Color::Rgb(169, 169, 169), // gray
];

/// Colors for base identity (Okabe-Ito colorblind-friendly palette).
/// See: https://github.com/rnabioco/squiggy-positron
pub const BASE_COLORS: [(char, Color); 10] = [
    ('A', Color::Rgb(0, 158, 115)),   // #009E73 green (purine)
    ('a', Color::Rgb(0, 158, 115)),
    ('C', Color::Rgb(240, 228, 66)),  // #F0E442 yellow (pyrimidine)
    ('c', Color::Rgb(240, 228, 66)),
    ('G', Color::Rgb(0, 114, 178)),   // #0072B2 blue (purine)
    ('g', Color::Rgb(0, 114, 178)),
    ('U', Color::Rgb(213, 94, 0)),    // #D55E00 orange (pyrimidine)
    ('u', Color::Rgb(213, 94, 0)),
    ('N', Color::Rgb(128, 128, 128)), // #808080 gray (unknown)
    ('n', Color::Rgb(128, 128, 128)),
];

/// Additional base colors for DNA (Okabe-Ito colorblind-friendly palette).
pub const DNA_BASE_COLORS: [(char, Color); 2] = [
    ('T', Color::Rgb(213, 94, 0)),    // #D55E00 orange (same as U)
    ('t', Color::Rgb(213, 94, 0)),
];

/// Conservation thresholds and colors.
pub const CONSERVATION_HIGH: f64 = 0.8;
pub const CONSERVATION_MED: f64 = 0.6;
pub const CONSERVATION_LOW: f64 = 0.4;

pub const CONSERVATION_HIGH_COLOR: Color = Color::Rgb(0, 255, 255); // cyan
pub const CONSERVATION_MED_COLOR: Color = Color::Rgb(135, 206, 235); // skyblue
pub const CONSERVATION_LOW_COLOR: Color = Color::Rgb(169, 169, 169); // gray

/// Compensatory change colors.
pub const COMP_DOUBLE_COMPATIBLE: Color = Color::Green;
pub const COMP_SINGLE_COMPATIBLE: Color = Color::Rgb(144, 238, 144); // lightgreen
pub const COMP_DOUBLE_INCOMPATIBLE: Color = Color::Red;
pub const COMP_SINGLE_INCOMPATIBLE: Color = Color::Rgb(255, 165, 0); // orange
pub const COMP_GAP: Color = Color::Magenta;

/// Get color for a character based on the color scheme.
pub fn get_color(
    scheme: ColorScheme,
    ch: char,
    col: usize,
    row: usize,
    alignment: &Alignment,
    cache: &StructureCache,
    gap_chars: &[char],
    reference_seq: usize,
) -> Option<Color> {
    match scheme {
        ColorScheme::None => None,
        ColorScheme::Structure => get_structure_color(col, cache),
        ColorScheme::Base => get_base_color(ch, gap_chars),
        ColorScheme::Conservation => get_conservation_color(col, alignment, gap_chars),
        ColorScheme::Compensatory => {
            get_compensatory_color(col, row, alignment, cache, gap_chars, reference_seq)
        }
    }
}

/// Get color based on secondary structure (helix coloring).
fn get_structure_color(col: usize, cache: &StructureCache) -> Option<Color> {
    cache
        .get_helix(col)
        .map(|helix_id| HELIX_COLORS[helix_id % HELIX_COLORS.len()])
}

/// Background color for gap characters in base coloring mode.
const BASE_GAP_COLOR: Color = Color::Rgb(40, 40, 40); // dark gray

/// Get color based on base identity.
fn get_base_color(ch: char, gap_chars: &[char]) -> Option<Color> {
    // Check if gap character - use dark gray background
    if gap_chars.contains(&ch) {
        return Some(BASE_GAP_COLOR);
    }
    // Check RNA bases
    for (base, color) in BASE_COLORS {
        if ch == base {
            return Some(color);
        }
    }
    // Check DNA bases
    for (base, color) in DNA_BASE_COLORS {
        if ch == base {
            return Some(color);
        }
    }
    Some(BASE_GAP_COLOR) // Unknown chars also get explicit background
}

/// Get color based on conservation at a column.
fn get_conservation_color(col: usize, alignment: &Alignment, gap_chars: &[char]) -> Option<Color> {
    let conservation = calculate_conservation(col, alignment, gap_chars);

    if conservation >= CONSERVATION_HIGH {
        Some(CONSERVATION_HIGH_COLOR)
    } else if conservation >= CONSERVATION_MED {
        Some(CONSERVATION_MED_COLOR)
    } else if conservation >= CONSERVATION_LOW {
        Some(CONSERVATION_LOW_COLOR)
    } else {
        None
    }
}

/// Calculate conservation at a column (0.0 to 1.0).
fn calculate_conservation(col: usize, alignment: &Alignment, gap_chars: &[char]) -> f64 {
    if alignment.sequences.is_empty() {
        return 0.0;
    }

    let mut counts: std::collections::HashMap<char, usize> = std::collections::HashMap::new();
    let mut total = 0;

    for seq in &alignment.sequences {
        if let Some(ch) = seq.get(col) {
            let upper: char = ch.to_ascii_uppercase();
            if !gap_chars.contains(&ch) {
                *counts.entry(upper).or_insert(0) += 1;
                total += 1;
            }
        }
    }

    if total == 0 {
        return 0.0;
    }

    // Conservation is the frequency of the most common base
    let max_count = counts.values().max().copied().unwrap_or(0);
    max_count as f64 / total as f64
}

/// Get color based on compensatory changes.
fn get_compensatory_color(
    col: usize,
    row: usize,
    alignment: &Alignment,
    cache: &StructureCache,
    gap_chars: &[char],
    reference_seq: usize,
) -> Option<Color> {
    if row == reference_seq {
        // Reference sequence - no compensatory analysis
        return None;
    }

    let ref_seq = alignment.sequences.get(reference_seq)?;
    let query_seq = alignment.sequences.get(row)?;

    let change = analyze_compensatory(&ref_seq.data(), &query_seq.data(), col, cache, gap_chars);

    match change {
        CompensatoryChange::Unchanged => None,
        CompensatoryChange::DoubleCompatible => Some(COMP_DOUBLE_COMPATIBLE),
        CompensatoryChange::SingleCompatible => Some(COMP_SINGLE_COMPATIBLE),
        CompensatoryChange::DoubleIncompatible => Some(COMP_DOUBLE_INCOMPATIBLE),
        CompensatoryChange::SingleIncompatible => Some(COMP_SINGLE_INCOMPATIBLE),
        CompensatoryChange::InvolvesGap => Some(COMP_GAP),
        CompensatoryChange::Unpaired => None,
    }
}

/// Get consensus character for a column.
#[allow(dead_code)] // API utility for future consensus display
pub fn get_consensus_char(col: usize, alignment: &Alignment, gap_chars: &[char]) -> char {
    if alignment.sequences.is_empty() {
        return ' ';
    }

    let mut counts: std::collections::HashMap<char, usize> = std::collections::HashMap::new();

    for seq in &alignment.sequences {
        if let Some(ch) = seq.get(col) {
            let upper: char = ch.to_ascii_uppercase();
            if !gap_chars.contains(&ch) {
                *counts.entry(upper).or_insert(0) += 1;
            }
        }
    }

    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(ch, _)| ch)
        .unwrap_or('.')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stockholm::Sequence;
    use std::rc::Rc;

    #[test]
    fn test_base_colors() {
        let gap_chars = ['.', '-'];
        assert!(get_base_color('A', &gap_chars).is_some());
        assert!(get_base_color('C', &gap_chars).is_some());
        assert!(get_base_color('G', &gap_chars).is_some());
        assert!(get_base_color('U', &gap_chars).is_some());
        // Gaps return dark gray background
        assert_eq!(get_base_color('.', &gap_chars), Some(Color::Rgb(40, 40, 40)));
    }

    #[test]
    fn test_conservation() {
        let mut alignment = Alignment::new();
        alignment.sequences.push(Rc::new(Sequence::new("s1", "AAAA")));
        alignment.sequences.push(Rc::new(Sequence::new("s2", "AAAA")));
        alignment.sequences.push(Rc::new(Sequence::new("s3", "AACA")));

        let gap_chars = ['.', '-'];

        // Column 0: 100% A
        let cons = calculate_conservation(0, &alignment, &gap_chars);
        assert!((cons - 1.0).abs() < 0.01);

        // Column 2: 66% A, 33% C
        let cons = calculate_conservation(2, &alignment, &gap_chars);
        assert!((cons - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_structure_colors() {
        let mut cache = StructureCache::new();
        cache.update("<<<>>>").unwrap();

        assert!(get_structure_color(0, &cache).is_some());
        assert!(get_structure_color(3, &cache).is_some());
    }
}
