//! Core types for Stockholm format alignments.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::rc::Rc;

/// Extract short ID from a Stockholm ID (strips coordinate suffix like /10000-20000).
pub fn short_id(id: &str) -> &str {
    id.split('/').next().unwrap_or(id)
}

/// Direction for shift operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftDirection {
    Left,
    Right,
}

/// Type of sequences in the alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(clippy::upper_case_acronyms)]
pub enum SequenceType {
    #[default]
    RNA,
    DNA,
    Protein,
}

impl SequenceType {
    /// Return a display string for the sequence type.
    pub fn as_str(&self) -> &'static str {
        match self {
            SequenceType::RNA => "RNA",
            SequenceType::DNA => "DNA",
            SequenceType::Protein => "Protein",
        }
    }
}

/// A Stockholm format alignment.
///
/// Sequences are wrapped in Rc for efficient copy-on-write cloning during undo/redo.
/// When an Alignment is cloned, sequences share data until modified.
#[derive(Debug, Clone, Default)]
pub struct Alignment {
    /// File-level annotations (#=GF)
    pub file_annotations: Vec<FileAnnotation>,
    /// Sequences in the alignment (Rc-wrapped for structural sharing)
    pub sequences: Vec<Rc<Sequence>>,
    /// Per-sequence annotations (#=GS)
    pub sequence_annotations: HashMap<String, Vec<SequenceAnnotation>>,
    /// Per-column annotations (#=GC)
    pub column_annotations: Vec<ColumnAnnotation>,
    /// Per-residue annotations (#=GR)
    pub residue_annotations: HashMap<String, Vec<ResidueAnnotation>>,
}

// Custom Serde for Alignment - unwrap Rc for serialization
impl serde::Serialize for Alignment {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Alignment", 5)?;
        state.serialize_field("file_annotations", &self.file_annotations)?;
        // Serialize sequences by dereferencing Rc
        let seqs: Vec<&Sequence> = self.sequences.iter().map(|rc| rc.as_ref()).collect();
        state.serialize_field("sequences", &seqs)?;
        state.serialize_field("sequence_annotations", &self.sequence_annotations)?;
        state.serialize_field("column_annotations", &self.column_annotations)?;
        state.serialize_field("residue_annotations", &self.residue_annotations)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for Alignment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct AlignmentHelper {
            file_annotations: Vec<FileAnnotation>,
            sequences: Vec<Sequence>,
            sequence_annotations: HashMap<String, Vec<SequenceAnnotation>>,
            column_annotations: Vec<ColumnAnnotation>,
            residue_annotations: HashMap<String, Vec<ResidueAnnotation>>,
        }
        let helper = AlignmentHelper::deserialize(deserializer)?;
        Ok(Alignment {
            file_annotations: helper.file_annotations,
            sequences: helper.sequences.into_iter().map(Rc::new).collect(),
            sequence_annotations: helper.sequence_annotations,
            column_annotations: helper.column_annotations,
            residue_annotations: helper.residue_annotations,
        })
    }
}

/// A sequence in the alignment.
#[derive(Debug, Clone)]
pub struct Sequence {
    /// Sequence identifier (may include coordinates like "id/start-end")
    pub id: String,
    /// Sequence data (with gaps) - stored as Vec<char> for O(1) access
    chars: Vec<char>,
}

// Custom Serde for Sequence - serialize chars as String
impl serde::Serialize for Sequence {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Sequence", 2)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("data", &self.data())?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for Sequence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct SequenceHelper {
            id: String,
            data: String,
        }
        let helper = SequenceHelper::deserialize(deserializer)?;
        Ok(Sequence::new(helper.id, helper.data))
    }
}

/// File-level annotation (#=GF tag value).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnnotation {
    pub tag: String,
    pub value: String,
}

/// Per-sequence annotation (#=GS seqid tag value).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceAnnotation {
    pub tag: String,
    pub value: String,
}

/// Per-column annotation (#=GC tag data).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnAnnotation {
    pub tag: String,
    pub data: String,
}

/// Per-residue annotation (#=GR seqid tag data).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResidueAnnotation {
    pub tag: String,
    pub data: String,
}

impl Alignment {
    /// Create a new empty alignment.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a mutable reference to a sequence, cloning if necessary (copy-on-write).
    ///
    /// This uses `Rc::make_mut` to implement structural sharing: if this is the only
    /// reference to the sequence, it returns a direct mutable reference. Otherwise,
    /// it clones the sequence first.
    #[allow(dead_code)] // Public API for copy-on-write access
    pub fn sequence_mut(&mut self, index: usize) -> Option<&mut Sequence> {
        self.sequences.get_mut(index).map(Rc::make_mut)
    }

    /// Get the number of sequences.
    pub fn num_sequences(&self) -> usize {
        self.sequences.len()
    }

    /// Get the alignment width (number of columns).
    pub fn width(&self) -> usize {
        self.sequences.first().map(|s| s.len()).unwrap_or(0)
    }

    /// Get the consensus secondary structure annotation if present.
    pub fn ss_cons(&self) -> Option<&str> {
        self.column_annotations
            .iter()
            .find(|a| a.tag == "SS_cons")
            .map(|a| a.data.as_str())
    }

    /// Get a mutable reference to the consensus secondary structure.
    #[allow(dead_code)] // API for structure editing
    pub fn ss_cons_mut(&mut self) -> Option<&mut String> {
        self.column_annotations
            .iter_mut()
            .find(|a| a.tag == "SS_cons")
            .map(|a| &mut a.data)
    }

    /// Get the reference sequence annotation if present.
    #[allow(dead_code)] // API for reference sequence access
    pub fn rf(&self) -> Option<&str> {
        self.column_annotations
            .iter()
            .find(|a| a.tag == "RF")
            .map(|a| a.data.as_str())
    }

    /// Check if all sequences have the same length.
    pub fn is_valid(&self) -> bool {
        if self.sequences.is_empty() {
            return true;
        }
        let width = self.sequences[0].len();
        self.sequences.iter().all(|s| s.len() == width)
            && self
                .column_annotations
                .iter()
                .all(|a| a.data.len() == width)
    }

    /// Get the maximum sequence ID length (for formatting).
    pub fn max_id_len(&self) -> usize {
        self.sequences.iter().map(|s| s.id.len()).max().unwrap_or(0)
    }

    /// Get the maximum short ID length (ID without coordinate suffix).
    pub fn max_short_id_len(&self) -> usize {
        self.sequences
            .iter()
            .map(|s| short_id(&s.id).len())
            .max()
            .unwrap_or(0)
    }

    /// Insert a gap at a specific position in all sequences and annotations.
    pub fn insert_gap_column(&mut self, col: usize, gap_char: char) {
        for seq in &mut self.sequences {
            Rc::make_mut(seq).insert_gap(col, gap_char);
        }
        for ann in &mut self.column_annotations {
            if col <= ann.data.len() {
                ann.data.insert(col, gap_char);
            }
        }
        for annotations in self.residue_annotations.values_mut() {
            for ann in annotations {
                if col <= ann.data.len() {
                    ann.data.insert(col, gap_char);
                }
            }
        }
    }

    /// Delete a column if it contains only gaps in all sequences.
    pub fn delete_gap_column(&mut self, col: usize, gap_chars: &[char]) -> bool {
        // Check if column is all gaps (O(1) per sequence now)
        let all_gaps = self
            .sequences
            .iter()
            .all(|s| s.get(col).map(|c| gap_chars.contains(&c)).unwrap_or(false));

        if !all_gaps {
            return false;
        }

        // Delete from all sequences
        for seq in &mut self.sequences {
            if col < seq.len() {
                Rc::make_mut(seq).chars_mut().remove(col);
            }
        }
        for ann in &mut self.column_annotations {
            if col < ann.data.len() {
                ann.data.remove(col);
            }
        }
        for annotations in self.residue_annotations.values_mut() {
            for ann in annotations {
                if col < ann.data.len() {
                    ann.data.remove(col);
                }
            }
        }

        true
    }

    /// Get character at a specific position (O(1)).
    pub fn get_char(&self, row: usize, col: usize) -> Option<char> {
        self.sequences.get(row)?.get(col)
    }

    /// Set character at a specific position (O(1)).
    #[allow(dead_code)] // API for direct character editing
    pub fn set_char(&mut self, row: usize, col: usize, ch: char) -> bool {
        if let Some(seq) = self.sequences.get_mut(row) {
            return Rc::make_mut(seq).set(col, ch);
        }
        false
    }

    /// Check if a column contains only gap characters.
    fn is_gap_column(&self, col: usize, gap_chars: &[char]) -> bool {
        self.sequences
            .iter()
            .all(|s| s.get(col).map(|c| gap_chars.contains(&c)).unwrap_or(true))
    }

    /// Remove leading gap-only columns from the alignment.
    /// Returns the number of columns removed.
    pub fn trim_left(&mut self, gap_chars: &[char]) -> usize {
        let width = self.width();
        if width == 0 {
            return 0;
        }

        // Find first non-gap column
        let first_non_gap = (0..width)
            .find(|&col| !self.is_gap_column(col, gap_chars))
            .unwrap_or(width);

        if first_non_gap == 0 {
            return 0;
        }

        // Remove columns from the front
        for seq in &mut self.sequences {
            let seq_mut = Rc::make_mut(seq);
            seq_mut.chars_mut().drain(0..first_non_gap);
        }
        for ann in &mut self.column_annotations {
            ann.data.drain(0..first_non_gap.min(ann.data.len()));
        }
        for annotations in self.residue_annotations.values_mut() {
            for ann in annotations {
                ann.data.drain(0..first_non_gap.min(ann.data.len()));
            }
        }

        first_non_gap
    }

    /// Remove trailing gap-only columns from the alignment.
    /// Returns the number of columns removed.
    pub fn trim_right(&mut self, gap_chars: &[char]) -> usize {
        let width = self.width();
        if width == 0 {
            return 0;
        }

        // Find last non-gap column
        let last_non_gap = (0..width)
            .rev()
            .find(|&col| !self.is_gap_column(col, gap_chars));

        let trim_from = match last_non_gap {
            Some(col) => col + 1,
            None => 0, // All columns are gaps
        };

        let to_remove = width - trim_from;
        if to_remove == 0 {
            return 0;
        }

        // Remove columns from the end
        for seq in &mut self.sequences {
            let seq_mut = Rc::make_mut(seq);
            seq_mut.chars_mut().truncate(trim_from);
        }
        for ann in &mut self.column_annotations {
            ann.data.truncate(trim_from);
        }
        for annotations in self.residue_annotations.values_mut() {
            for ann in annotations {
                ann.data.truncate(trim_from);
            }
        }

        to_remove
    }
}

impl Sequence {
    /// Create a new sequence.
    pub fn new(id: impl Into<String>, data: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            chars: data.into().chars().collect(),
        }
    }

    // === Accessor methods ===

    /// Get sequence data as a String (for output/serialization).
    pub fn data(&self) -> String {
        self.chars.iter().collect()
    }

    /// Get sequence characters as a slice.
    pub fn chars(&self) -> &[char] {
        &self.chars
    }

    /// Get mutable access to sequence characters.
    pub fn chars_mut(&mut self) -> &mut Vec<char> {
        &mut self.chars
    }

    /// Get the length of the sequence.
    pub fn len(&self) -> usize {
        self.chars.len()
    }

    /// Check if the sequence is empty.
    #[allow(dead_code)] // API completeness
    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    /// Get character at a specific position (O(1)).
    pub fn get(&self, index: usize) -> Option<char> {
        self.chars.get(index).copied()
    }

    /// Set character at a specific position (O(1)).
    pub fn set(&mut self, index: usize, ch: char) -> bool {
        if index < self.chars.len() {
            self.chars[index] = ch;
            true
        } else {
            false
        }
    }

    // === Mutation methods ===

    /// Insert a gap at a specific position.
    pub fn insert_gap(&mut self, pos: usize, gap_char: char) {
        if pos <= self.chars.len() {
            self.chars.insert(pos, gap_char);
        }
    }

    /// Delete a character at a specific position if it's a gap.
    pub fn delete_gap(&mut self, pos: usize, gap_chars: &[char]) -> bool {
        if let Some(&ch) = self.chars.get(pos)
            && gap_chars.contains(&ch)
        {
            self.chars.remove(pos);
            return true;
        }
        false
    }

    /// Shift sequence in the given direction (moves content to next gap).
    pub fn shift(&mut self, col: usize, direction: ShiftDirection, gap_chars: &[char]) -> bool {
        // Find the nearest gap in the specified direction
        let gap_pos = match direction {
            ShiftDirection::Left => (0..col).rev().find(|&i| gap_chars.contains(&self.chars[i])),
            ShiftDirection::Right => {
                ((col + 1)..self.chars.len()).find(|&i| gap_chars.contains(&self.chars[i]))
            }
        };

        if let Some(gp) = gap_pos {
            // Remove gap at gp, insert gap at col
            self.chars.remove(gp);
            self.chars.insert(col, gap_chars[0]);
            return true;
        }

        false
    }

    /// Shift sequence left by one position (moves content to next gap on left).
    #[allow(dead_code)] // Convenience method, used by tests
    pub fn shift_left(&mut self, col: usize, gap_chars: &[char]) -> bool {
        self.shift(col, ShiftDirection::Left, gap_chars)
    }

    /// Shift sequence right by one position (moves content to next gap on right).
    #[allow(dead_code)] // Convenience method, used by tests
    pub fn shift_right(&mut self, col: usize, gap_chars: &[char]) -> bool {
        self.shift(col, ShiftDirection::Right, gap_chars)
    }

    /// Convert sequence to uppercase.
    pub fn make_uppercase(&mut self) {
        for ch in &mut self.chars {
            *ch = ch.to_ascii_uppercase();
        }
    }

    /// Convert sequence to lowercase.
    pub fn make_lowercase(&mut self) {
        for ch in &mut self.chars {
            *ch = ch.to_ascii_lowercase();
        }
    }

    /// Replace all occurrences of one character with another.
    pub fn replace_char(&mut self, from: char, to: char) {
        for ch in &mut self.chars {
            if *ch == from {
                *ch = to;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alignment_width() {
        let mut alignment = Alignment::new();
        alignment
            .sequences
            .push(Rc::new(Sequence::new("seq1", "ACGU..ACGU")));
        alignment
            .sequences
            .push(Rc::new(Sequence::new("seq2", "ACGU..ACGU")));
        assert_eq!(alignment.width(), 10);
    }

    #[test]
    fn test_insert_gap_column() {
        let mut alignment = Alignment::new();
        alignment
            .sequences
            .push(Rc::new(Sequence::new("seq1", "ACGU")));
        alignment.insert_gap_column(2, '.');
        assert_eq!(alignment.sequences[0].data(), "AC.GU");
    }

    #[test]
    fn test_sequence_shift_left() {
        let mut seq = Sequence::new("test", "A.CGU");
        assert!(seq.shift_left(2, &['.']));
        assert_eq!(seq.data(), "AC.GU");
    }

    #[test]
    fn test_sequence_shift_right() {
        let mut seq = Sequence::new("test", "ACG.U");
        assert!(seq.shift_right(2, &['.']));
        assert_eq!(seq.data(), "AC.GU");
    }
}
