//! Base pair caching and higher-level structure operations.

use super::parser::{parse_structure, BasePair, StructureError};

/// Cached structure information for an alignment.
#[derive(Debug, Default)]
pub struct StructureCache {
    /// The structure string this cache was built from.
    cached_structure: String,
    /// Parsed base pairs.
    pairs: Vec<BasePair>,
    /// Lookup table: column -> paired column (None if unpaired).
    pair_lookup: Vec<Option<usize>>,
    /// Lookup table: column -> helix ID (None if unpaired).
    helix_lookup: Vec<Option<usize>>,
}

impl StructureCache {
    /// Create a new empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the cache with a new structure string.
    pub fn update(&mut self, structure: &str) -> Result<(), StructureError> {
        if structure == self.cached_structure {
            return Ok(());
        }

        self.pairs = parse_structure(structure)?;
        self.cached_structure = structure.to_string();

        // Build lookup tables
        let len = structure.len();
        self.pair_lookup = vec![None; len];
        self.helix_lookup = vec![None; len];

        for pair in &self.pairs {
            self.pair_lookup[pair.left] = Some(pair.right);
            self.pair_lookup[pair.right] = Some(pair.left);
            self.helix_lookup[pair.left] = Some(pair.helix_id);
            self.helix_lookup[pair.right] = Some(pair.helix_id);
        }

        Ok(())
    }

    /// Get the paired column for a given column, if any.
    pub fn get_pair(&self, col: usize) -> Option<usize> {
        self.pair_lookup.get(col).copied().flatten()
    }

    /// Get the helix ID for a given column, if any.
    pub fn get_helix(&self, col: usize) -> Option<usize> {
        self.helix_lookup.get(col).copied().flatten()
    }

    /// Get all base pairs.
    #[allow(dead_code)] // API for structure analysis
    pub fn pairs(&self) -> &[BasePair] {
        &self.pairs
    }

    /// Get the number of unique helices.
    #[allow(dead_code)] // API for structure analysis
    pub fn num_helices(&self) -> usize {
        self.pairs
            .iter()
            .map(|p| p.helix_id)
            .max()
            .map(|m| m + 1)
            .unwrap_or(0)
    }

    /// Check if a column is paired.
    pub fn is_paired(&self, col: usize) -> bool {
        self.pair_lookup.get(col).copied().flatten().is_some()
    }

    /// Clear the cache.
    #[allow(dead_code)] // API for cache management
    pub fn clear(&mut self) {
        self.cached_structure.clear();
        self.pairs.clear();
        self.pair_lookup.clear();
        self.helix_lookup.clear();
    }

    /// Check if the cache is valid for the given structure.
    pub fn is_valid_for(&self, structure: &str) -> bool {
        self.cached_structure == structure
    }
}

/// Check if two bases can form a Watson-Crick or wobble pair.
pub fn is_valid_pair(base1: char, base2: char) -> bool {
    let b1 = base1.to_ascii_uppercase();
    let b2 = base2.to_ascii_uppercase();

    matches!(
        (b1, b2),
        ('A', 'U') | ('U', 'A') |  // A-U
        ('A', 'T') | ('T', 'A') |  // A-T (DNA)
        ('G', 'C') | ('C', 'G') |  // G-C
        ('G', 'U') | ('U', 'G') |  // G-U wobble
        ('G', 'T') | ('T', 'G')    // G-T wobble (DNA)
    )
}

/// Analyze compensatory changes between two sequences at paired positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompensatoryChange {
    /// Both positions unchanged, valid pair.
    Unchanged,
    /// One base changed, still valid pair.
    SingleCompatible,
    /// Both bases changed, still valid pair.
    DoubleCompatible,
    /// One base changed, invalid pair.
    SingleIncompatible,
    /// Both bases changed, invalid pair.
    DoubleIncompatible,
    /// One or both positions involve a gap.
    InvolvesGap,
    /// Not a paired position.
    Unpaired,
}

/// Analyze a position for compensatory changes.
pub fn analyze_compensatory(
    ref_seq: &str,
    query_seq: &str,
    col: usize,
    cache: &StructureCache,
    gap_chars: &[char],
) -> CompensatoryChange {
    let paired_col = match cache.get_pair(col) {
        Some(p) => p,
        None => return CompensatoryChange::Unpaired,
    };

    let ref_chars: Vec<char> = ref_seq.chars().collect();
    let query_chars: Vec<char> = query_seq.chars().collect();

    if col >= ref_chars.len() || col >= query_chars.len() ||
       paired_col >= ref_chars.len() || paired_col >= query_chars.len() {
        return CompensatoryChange::Unpaired;
    }

    let ref_left = ref_chars[col];
    let ref_right = ref_chars[paired_col];
    let query_left = query_chars[col];
    let query_right = query_chars[paired_col];

    // Check for gaps
    if gap_chars.contains(&query_left) || gap_chars.contains(&query_right) {
        return CompensatoryChange::InvolvesGap;
    }

    let left_changed = ref_left.to_ascii_uppercase() != query_left.to_ascii_uppercase();
    let right_changed = ref_right.to_ascii_uppercase() != query_right.to_ascii_uppercase();
    let still_valid = is_valid_pair(query_left, query_right);

    match (left_changed, right_changed, still_valid) {
        (false, false, _) => CompensatoryChange::Unchanged,
        (true, true, true) => CompensatoryChange::DoubleCompatible,
        (true, true, false) => CompensatoryChange::DoubleIncompatible,
        (true, false, true) | (false, true, true) => CompensatoryChange::SingleCompatible,
        (true, false, false) | (false, true, false) => CompensatoryChange::SingleIncompatible,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_update() {
        let mut cache = StructureCache::new();
        cache.update("<<<>>>").unwrap();

        assert_eq!(cache.get_pair(0), Some(5));
        assert_eq!(cache.get_pair(1), Some(4));
        assert_eq!(cache.get_pair(2), Some(3));
        assert_eq!(cache.get_pair(3), Some(2));
        assert!(cache.is_paired(0));
    }

    #[test]
    fn test_helix_lookup() {
        let mut cache = StructureCache::new();
        cache.update("<<..<<..>>..>>").unwrap();

        // First helix
        assert_eq!(cache.get_helix(0), Some(0));
        assert_eq!(cache.get_helix(1), Some(0));
        assert_eq!(cache.get_helix(12), Some(0));
        assert_eq!(cache.get_helix(13), Some(0));

        // Second helix
        assert_eq!(cache.get_helix(4), Some(1));
        assert_eq!(cache.get_helix(5), Some(1));

        // Unpaired
        assert_eq!(cache.get_helix(2), None);
    }

    #[test]
    fn test_valid_pairs() {
        assert!(is_valid_pair('A', 'U'));
        assert!(is_valid_pair('U', 'A'));
        assert!(is_valid_pair('G', 'C'));
        assert!(is_valid_pair('G', 'U'));
        assert!(!is_valid_pair('A', 'A'));
        assert!(!is_valid_pair('A', 'C'));
    }

    #[test]
    fn test_compensatory_analysis() {
        let mut cache = StructureCache::new();
        cache.update("<<>>").unwrap();

        let gap_chars = ['.', '-'];

        // No change
        let result = analyze_compensatory("ACGU", "ACGU", 0, &cache, &gap_chars);
        assert_eq!(result, CompensatoryChange::Unchanged);

        // Double compatible (A-U -> G-C)
        let result = analyze_compensatory("AUUA", "GCGC", 0, &cache, &gap_chars);
        assert_eq!(result, CompensatoryChange::DoubleCompatible);
    }
}
