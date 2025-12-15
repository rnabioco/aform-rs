//! Color scheme implementations for alignment display.

use ratatui::style::Color;

use crate::app::ColorScheme;
use crate::stockholm::{Alignment, SequenceType};
use crate::structure::{CompensatoryChange, StructureCache, analyze_compensatory};

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
    ('A', Color::Rgb(0, 158, 115)), // #009E73 green (purine)
    ('a', Color::Rgb(0, 158, 115)),
    ('C', Color::Rgb(240, 228, 66)), // #F0E442 yellow (pyrimidine)
    ('c', Color::Rgb(240, 228, 66)),
    ('G', Color::Rgb(0, 114, 178)), // #0072B2 blue (purine)
    ('g', Color::Rgb(0, 114, 178)),
    ('U', Color::Rgb(213, 94, 0)), // #D55E00 orange (pyrimidine)
    ('u', Color::Rgb(213, 94, 0)),
    ('N', Color::Rgb(128, 128, 128)), // #808080 gray (unknown)
    ('n', Color::Rgb(128, 128, 128)),
];

/// Additional base colors for DNA (Okabe-Ito colorblind-friendly palette).
pub const DNA_BASE_COLORS: [(char, Color); 2] = [
    ('T', Color::Rgb(213, 94, 0)), // #D55E00 orange (same as U)
    ('t', Color::Rgb(213, 94, 0)),
];

/// Amino acid colors based on chemical properties.
/// Groups:
/// - Hydrophobic (nonpolar): A, I, L, M, F, W, V - orange/brown
/// - Polar uncharged: S, T, N, Q - green
/// - Charged positive: K, R, H - blue
/// - Charged negative: D, E - red
/// - Special: C (yellow), G (magenta), P (pink), Y (cyan)
pub const AMINO_ACID_COLORS: [(char, Color); 40] = [
    // Hydrophobic (nonpolar) - orange/brown
    ('A', Color::Rgb(230, 159, 0)), // Alanine - orange
    ('a', Color::Rgb(230, 159, 0)),
    ('I', Color::Rgb(204, 121, 0)), // Isoleucine - darker orange
    ('i', Color::Rgb(204, 121, 0)),
    ('L', Color::Rgb(204, 121, 0)), // Leucine - darker orange
    ('l', Color::Rgb(204, 121, 0)),
    ('M', Color::Rgb(230, 159, 0)), // Methionine - orange
    ('m', Color::Rgb(230, 159, 0)),
    ('F', Color::Rgb(166, 86, 40)), // Phenylalanine - brown
    ('f', Color::Rgb(166, 86, 40)),
    ('W', Color::Rgb(166, 86, 40)), // Tryptophan - brown
    ('w', Color::Rgb(166, 86, 40)),
    ('V', Color::Rgb(204, 121, 0)), // Valine - darker orange
    ('v', Color::Rgb(204, 121, 0)),
    // Polar uncharged - green
    ('S', Color::Rgb(0, 158, 115)), // Serine - green
    ('s', Color::Rgb(0, 158, 115)),
    ('T', Color::Rgb(0, 158, 115)), // Threonine - green (note: conflicts with DNA T)
    ('t', Color::Rgb(0, 158, 115)),
    ('N', Color::Rgb(86, 180, 133)), // Asparagine - light green
    ('n', Color::Rgb(86, 180, 133)),
    ('Q', Color::Rgb(86, 180, 133)), // Glutamine - light green
    ('q', Color::Rgb(86, 180, 133)),
    // Charged positive - blue
    ('K', Color::Rgb(0, 114, 178)), // Lysine - blue
    ('k', Color::Rgb(0, 114, 178)),
    ('R', Color::Rgb(0, 114, 178)), // Arginine - blue
    ('r', Color::Rgb(0, 114, 178)),
    ('H', Color::Rgb(86, 180, 233)), // Histidine - light blue
    ('h', Color::Rgb(86, 180, 233)),
    // Charged negative - red
    ('D', Color::Rgb(213, 94, 0)), // Aspartate - red-orange
    ('d', Color::Rgb(213, 94, 0)),
    ('E', Color::Rgb(204, 51, 17)), // Glutamate - red
    ('e', Color::Rgb(204, 51, 17)),
    // Special amino acids - distinct colors
    ('C', Color::Rgb(240, 228, 66)), // Cysteine - yellow
    ('c', Color::Rgb(240, 228, 66)),
    ('G', Color::Rgb(204, 121, 167)), // Glycine - pink/magenta
    ('g', Color::Rgb(204, 121, 167)),
    ('P', Color::Rgb(255, 182, 193)), // Proline - light pink
    ('p', Color::Rgb(255, 182, 193)),
    ('Y', Color::Rgb(0, 191, 196)), // Tyrosine - cyan
    ('y', Color::Rgb(0, 191, 196)),
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

/// Convert a PP (posterior probability) character to a color.
/// PP values: 0-9 (probability * 10), * = highest (>0.95).
/// Uses a red-yellow-green gradient.
pub fn pp_to_color(ch: char) -> Color {
    match ch {
        '*' => Color::Rgb(0, 255, 0),    // Bright green - highest confidence
        '9' => Color::Rgb(50, 220, 50),  // Green
        '8' => Color::Rgb(100, 200, 50), // Yellow-green
        '7' => Color::Rgb(150, 200, 50), // Yellow-green
        '6' => Color::Rgb(200, 200, 50), // Yellow
        '5' => Color::Rgb(220, 180, 50), // Yellow-orange
        '4' => Color::Rgb(220, 150, 50), // Orange
        '3' => Color::Rgb(220, 120, 50), // Orange
        '2' => Color::Rgb(200, 80, 50),  // Red-orange
        '1' => Color::Rgb(180, 50, 50),  // Dark red
        '0' => Color::Rgb(150, 50, 50),  // Dark red - lowest confidence
        '.' | '-' => Color::DarkGray,    // Gap
        _ => Color::Gray,                // Unknown
    }
}

/// Get color for a character based on the color scheme.
#[allow(clippy::too_many_arguments)]
pub fn get_color(
    scheme: ColorScheme,
    ch: char,
    col: usize,
    row: usize,
    alignment: &Alignment,
    cache: &StructureCache,
    gap_chars: &[char],
    reference_seq: usize,
    sequence_type: SequenceType,
) -> Option<Color> {
    match scheme {
        ColorScheme::None => None,
        ColorScheme::Structure => get_structure_color(col, cache),
        ColorScheme::Base => get_base_color(ch, gap_chars, sequence_type),
        ColorScheme::Conservation => get_conservation_color(col, alignment, gap_chars),
        ColorScheme::Compensatory => {
            get_compensatory_color(col, row, alignment, cache, gap_chars, reference_seq)
        }
        ColorScheme::PP => get_pp_color(ch, col, row, alignment, gap_chars),
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

/// Get color based on base/amino acid identity.
fn get_base_color(ch: char, gap_chars: &[char], sequence_type: SequenceType) -> Option<Color> {
    // Check if gap character - use dark gray background
    if gap_chars.contains(&ch) {
        return Some(BASE_GAP_COLOR);
    }

    match sequence_type {
        SequenceType::Protein => {
            // Check amino acid colors
            for (aa, color) in AMINO_ACID_COLORS {
                if ch == aa {
                    return Some(color);
                }
            }
            Some(BASE_GAP_COLOR)
        }
        SequenceType::DNA => {
            // Check DNA bases first, then RNA
            for (base, color) in DNA_BASE_COLORS {
                if ch == base {
                    return Some(color);
                }
            }
            for (base, color) in BASE_COLORS {
                if ch == base {
                    return Some(color);
                }
            }
            Some(BASE_GAP_COLOR)
        }
        SequenceType::RNA => {
            // Check RNA bases first, then DNA
            for (base, color) in BASE_COLORS {
                if ch == base {
                    return Some(color);
                }
            }
            for (base, color) in DNA_BASE_COLORS {
                if ch == base {
                    return Some(color);
                }
            }
            Some(BASE_GAP_COLOR)
        }
    }
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
pub fn calculate_conservation(col: usize, alignment: &Alignment, gap_chars: &[char]) -> f64 {
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

/// Get color based on per-residue PP (posterior probability) annotation.
fn get_pp_color(
    ch: char,
    col: usize,
    row: usize,
    alignment: &Alignment,
    gap_chars: &[char],
) -> Option<Color> {
    // Don't color gaps
    if gap_chars.contains(&ch) {
        return None;
    }

    // Look up PP annotation for this sequence
    let seq = alignment.sequences.get(row)?;
    if let Some(annotations) = alignment.residue_annotations.get(&seq.id) {
        for ann in annotations {
            if ann.tag == "PP"
                && let Some(pp_char) = ann.data.chars().nth(col)
            {
                return Some(pp_to_color(pp_char));
            }
        }
    }

    None // No PP annotation for this residue
}

/// Get consensus character for a column.
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

    // Use max_by to break ties deterministically by character
    counts
        .into_iter()
        .max_by(|(ch_a, count_a), (ch_b, count_b)| {
            count_a.cmp(count_b).then_with(|| ch_a.cmp(ch_b))
        })
        .map(|(ch, _)| ch)
        .unwrap_or('.')
}

/// Get consensus character with case indicating conservation level.
/// Uppercase if conservation >= threshold, lowercase otherwise.
pub fn get_consensus_char_with_case(
    col: usize,
    alignment: &Alignment,
    gap_chars: &[char],
    threshold: f64,
) -> char {
    let conservation = calculate_conservation(col, alignment, gap_chars);
    let ch = get_consensus_char(col, alignment, gap_chars);

    if conservation >= threshold {
        ch.to_ascii_uppercase()
    } else {
        ch.to_ascii_lowercase()
    }
}

/// Convert conservation score (0.0-1.0) to a block character and color.
/// Uses height-varying Unicode block characters with color gradient.
pub fn conservation_to_block(conservation: f64) -> (char, Color) {
    if conservation >= 0.95 {
        ('█', Color::Rgb(0, 255, 0)) // Full block - bright green
    } else if conservation >= 0.85 {
        ('▇', Color::Rgb(50, 205, 50)) // 7/8 - lime green
    } else if conservation >= 0.75 {
        ('▆', Color::Rgb(100, 200, 100)) // 6/8 - green
    } else if conservation >= 0.65 {
        ('▅', Color::Rgb(150, 200, 100)) // 5/8 - yellow-green
    } else if conservation >= 0.55 {
        ('▄', Color::Rgb(200, 200, 100)) // 4/8 - yellow
    } else if conservation >= 0.45 {
        ('▃', Color::Rgb(200, 150, 100)) // 3/8 - orange
    } else if conservation >= 0.35 {
        ('▂', Color::Rgb(200, 100, 100)) // 2/8 - red-orange
    } else if conservation >= 0.25 {
        ('▁', Color::Rgb(150, 80, 80)) // 1/8 - dark red
    } else {
        (' ', Color::DarkGray) // Empty for very low conservation
    }
}

/// Detect sequence type from alignment content.
/// Checks for protein-specific amino acids, then distinguishes RNA (U) from DNA (T).
pub fn detect_sequence_type(alignment: &Alignment, gap_chars: &[char]) -> SequenceType {
    // Amino acids only found in proteins (not in nucleotides)
    const PROTEIN_ONLY: &[char] = &[
        'E', 'e', 'F', 'f', 'I', 'i', 'L', 'l', 'P', 'p', 'Q', 'q', 'H', 'h', 'K', 'k', 'M', 'm',
        'R', 'r', 'S', 's', 'V', 'v', 'W', 'w', 'Y', 'y', 'D', 'd',
    ];

    let mut has_u = false;
    let mut has_t = false;
    let mut total_chars = 0;

    for seq in &alignment.sequences {
        for ch in seq.chars() {
            if gap_chars.contains(ch) {
                continue;
            }
            total_chars += 1;

            // Check for protein-specific characters
            if PROTEIN_ONLY.contains(ch) {
                return SequenceType::Protein;
            }

            let upper = ch.to_ascii_uppercase();
            if upper == 'U' {
                has_u = true;
            }
            if upper == 'T' {
                has_t = true;
            }
        }
    }

    if total_chars == 0 {
        return SequenceType::RNA; // Default
    }

    // RNA has U, DNA has T
    if has_u && !has_t {
        SequenceType::RNA
    } else if has_t && !has_u {
        SequenceType::DNA
    } else {
        // Default to RNA for ambiguous cases (both or neither)
        SequenceType::RNA
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stockholm::Sequence;
    use std::rc::Rc;

    #[test]
    fn test_base_colors() {
        let gap_chars = ['.', '-'];
        // RNA bases
        assert!(get_base_color('A', &gap_chars, SequenceType::RNA).is_some());
        assert!(get_base_color('C', &gap_chars, SequenceType::RNA).is_some());
        assert!(get_base_color('G', &gap_chars, SequenceType::RNA).is_some());
        assert!(get_base_color('U', &gap_chars, SequenceType::RNA).is_some());
        // DNA bases
        assert!(get_base_color('T', &gap_chars, SequenceType::DNA).is_some());
        // Protein amino acids
        assert!(get_base_color('M', &gap_chars, SequenceType::Protein).is_some());
        assert!(get_base_color('W', &gap_chars, SequenceType::Protein).is_some());
        // Gaps return dark gray background
        assert_eq!(
            get_base_color('.', &gap_chars, SequenceType::RNA),
            Some(Color::Rgb(40, 40, 40))
        );
    }

    #[test]
    fn test_conservation() {
        let mut alignment = Alignment::new();
        alignment
            .sequences
            .push(Rc::new(Sequence::new("s1", "AAAA")));
        alignment
            .sequences
            .push(Rc::new(Sequence::new("s2", "AAAA")));
        alignment
            .sequences
            .push(Rc::new(Sequence::new("s3", "AACA")));

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
