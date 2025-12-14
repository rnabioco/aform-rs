//! Secondary structure bracket notation parser.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum StructureError {
    #[error("Unmatched opening bracket at position {0}")]
    UnmatchedOpen(usize),
    #[error("Unmatched closing bracket at position {0}")]
    UnmatchedClose(usize),
    #[allow(dead_code)] // Variant for future pseudoknot validation
    #[error("Bracket type mismatch at position {0}")]
    BracketMismatch(usize),
}

/// A base pair with positions (0-indexed) and helix ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BasePair {
    /// 5' position (opening bracket)
    pub left: usize,
    /// 3' position (closing bracket)
    pub right: usize,
    /// Helix identifier for coloring
    pub helix_id: usize,
}

/// Opening bracket types.
const OPEN_BRACKETS: &[char] = &['<', '(', '[', '{'];
/// Closing bracket types.
const CLOSE_BRACKETS: &[char] = &['>', ')', ']', '}'];

/// Check if a character is an opening bracket.
#[allow(dead_code)] // API utility for bracket validation
pub fn is_open_bracket(c: char) -> bool {
    OPEN_BRACKETS.contains(&c)
}

/// Check if a character is a closing bracket.
#[allow(dead_code)] // API utility for bracket validation
pub fn is_close_bracket(c: char) -> bool {
    CLOSE_BRACKETS.contains(&c)
}

/// Get the matching closing bracket for an opening bracket.
#[allow(dead_code)] // API utility for bracket matching
pub fn matching_close(open: char) -> Option<char> {
    OPEN_BRACKETS
        .iter()
        .position(|&c| c == open)
        .map(|i| CLOSE_BRACKETS[i])
}

/// Get the matching opening bracket for a closing bracket.
#[allow(dead_code)] // API utility for bracket matching
pub fn matching_open(close: char) -> Option<char> {
    CLOSE_BRACKETS
        .iter()
        .position(|&c| c == close)
        .map(|i| OPEN_BRACKETS[i])
}

/// Parse a secondary structure string into base pairs.
///
/// Handles nested bracket notation with multiple bracket types.
/// Returns base pairs sorted by left position.
pub fn parse_structure(ss: &str) -> Result<Vec<BasePair>, StructureError> {
    let mut pairs = Vec::new();
    let mut stacks: [Vec<usize>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];

    for (pos, ch) in ss.chars().enumerate() {
        if let Some(bracket_type) = OPEN_BRACKETS.iter().position(|&c| c == ch) {
            stacks[bracket_type].push(pos);
        } else if let Some(bracket_type) = CLOSE_BRACKETS.iter().position(|&c| c == ch) {
            if let Some(left) = stacks[bracket_type].pop() {
                pairs.push(BasePair {
                    left,
                    right: pos,
                    helix_id: 0, // Will be assigned later
                });
            } else {
                return Err(StructureError::UnmatchedClose(pos));
            }
        }
    }

    // Check for unmatched opening brackets
    for (_bracket_type, stack) in stacks.iter().enumerate() {
        if let Some(&pos) = stack.first() {
            return Err(StructureError::UnmatchedOpen(pos));
        }
    }

    // Sort by left position
    pairs.sort_by_key(|p| p.left);

    // Assign helix IDs based on contiguity
    assign_helix_ids(&mut pairs);

    Ok(pairs)
}

/// Assign helix IDs to base pairs.
///
/// Pairs that are adjacent (consecutive positions on both sides) belong to the same helix.
fn assign_helix_ids(pairs: &mut [BasePair]) {
    if pairs.is_empty() {
        return;
    }

    let mut current_helix = 0;
    pairs[0].helix_id = current_helix;

    for i in 1..pairs.len() {
        let prev = pairs[i - 1];
        let curr = pairs[i];

        // Check if this pair is adjacent to the previous one
        // Adjacent means: left positions are consecutive AND right positions are consecutive (in reverse)
        let is_adjacent = curr.left == prev.left + 1 && curr.right + 1 == prev.right;

        if !is_adjacent {
            current_helix += 1;
        }
        pairs[i].helix_id = current_helix;
    }
}

/// Find the paired position for a given column.
#[allow(dead_code)] // API utility, used in tests
pub fn find_pair(pairs: &[BasePair], col: usize) -> Option<usize> {
    for pair in pairs {
        if pair.left == col {
            return Some(pair.right);
        }
        if pair.right == col {
            return Some(pair.left);
        }
    }
    None
}

/// Get the helix ID for a given column.
#[allow(dead_code)] // API utility for structure analysis
pub fn get_helix_id(pairs: &[BasePair], col: usize) -> Option<usize> {
    for pair in pairs {
        if pair.left == col || pair.right == col {
            return Some(pair.helix_id);
        }
    }
    None
}

/// Count the number of unique helices.
#[allow(dead_code)] // API utility for structure analysis
pub fn count_helices(pairs: &[BasePair]) -> usize {
    pairs.iter().map(|p| p.helix_id).max().map(|m| m + 1).unwrap_or(0)
}

/// Check if the structure string is valid (balanced brackets).
#[allow(dead_code)] // API utility for structure validation
pub fn is_valid_structure(ss: &str) -> bool {
    parse_structure(ss).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_helix() {
        let pairs = parse_structure("<<<>>>").unwrap();
        assert_eq!(pairs.len(), 3);
        assert_eq!(pairs[0], BasePair { left: 0, right: 5, helix_id: 0 });
        assert_eq!(pairs[1], BasePair { left: 1, right: 4, helix_id: 0 });
        assert_eq!(pairs[2], BasePair { left: 2, right: 3, helix_id: 0 });
    }

    #[test]
    fn test_nested_helices() {
        let pairs = parse_structure("<<..<<..>>..>>").unwrap();
        assert_eq!(pairs.len(), 4);
        // First helix: 0-13, 1-12
        assert_eq!(pairs[0].helix_id, 0);
        assert_eq!(pairs[1].helix_id, 0);
        // Second helix: 4-9, 5-8
        assert_eq!(pairs[2].helix_id, 1);
        assert_eq!(pairs[3].helix_id, 1);
    }

    #[test]
    fn test_multiple_bracket_types() {
        let pairs = parse_structure("<([{}>])").unwrap();
        assert_eq!(pairs.len(), 4);
    }

    #[test]
    fn test_find_pair() {
        let pairs = parse_structure("<<<>>>").unwrap();
        assert_eq!(find_pair(&pairs, 0), Some(5));
        assert_eq!(find_pair(&pairs, 5), Some(0));
        assert_eq!(find_pair(&pairs, 1), Some(4));
    }

    #[test]
    fn test_unmatched_close() {
        let result = parse_structure("<<>>");
        assert!(result.is_ok()); // This is actually valid

        let result = parse_structure("<<>>>");
        assert!(matches!(result, Err(StructureError::UnmatchedClose(_))));
    }

    #[test]
    fn test_unmatched_open() {
        let result = parse_structure("<<<>>");
        assert!(matches!(result, Err(StructureError::UnmatchedOpen(_))));
    }

    #[test]
    fn test_with_unpaired() {
        let pairs = parse_structure("<<...>>").unwrap();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], BasePair { left: 0, right: 6, helix_id: 0 });
        assert_eq!(pairs[1], BasePair { left: 1, right: 5, helix_id: 0 });
    }
}
