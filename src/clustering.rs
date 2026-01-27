//! Sequence clustering using hierarchical agglomerative clustering.
//!
//! Uses Hamming distance and UPGMA (average linkage) to group similar sequences.
//! Identical sequences are collapsed before clustering to reduce O(n²) distance computation.

use kodama::{Method, linkage};

/// Result of clustering: leaf order and optional tree visualization.
#[derive(Debug, Clone)]
pub struct ClusterResult {
    /// Sequence indices in dendrogram order (similar sequences adjacent).
    pub order: Vec<usize>,
    /// ASCII tree characters for each row (if requested).
    pub tree_lines: Vec<String>,
    /// Width of the tree in characters.
    pub tree_width: usize,
    /// Group order when clustering with collapse (maps display_row -> group_index).
    /// Only populated when clustering collapsed groups.
    pub group_order: Option<Vec<usize>>,
    /// Tree lines for collapsed view (one per group, not per sequence).
    /// Only populated when clustering with collapse groups.
    pub collapsed_tree_lines: Option<Vec<String>>,
}

/// Compute Hamming distance between two sequences (count mismatches).
/// Gaps vs gaps count as match, gaps vs residue count as mismatch.
pub fn hamming_distance(seq1: &[char], seq2: &[char], gap_chars: &[char]) -> usize {
    seq1.iter()
        .zip(seq2.iter())
        .filter(|(a, b)| {
            let a_gap = gap_chars.contains(a);
            let b_gap = gap_chars.contains(b);
            if a_gap && b_gap {
                // Both gaps = match (don't count as mismatch)
                false
            } else {
                // One gap + one residue = mismatch
                // Two different residues = mismatch
                !a.eq_ignore_ascii_case(b)
            }
        })
        .count()
}

/// Compute condensed distance matrix for all sequence pairs.
/// Returns distances in row-major condensed form for kodama.
pub fn compute_distance_matrix(sequences: &[Vec<char>], gap_chars: &[char]) -> Vec<f64> {
    let n = sequences.len();
    let mut distances = Vec::with_capacity(n * (n - 1) / 2);

    for i in 0..n {
        for j in (i + 1)..n {
            let dist = hamming_distance(&sequences[i], &sequences[j], gap_chars);
            distances.push(dist as f64);
        }
    }
    distances
}

/// Perform hierarchical clustering and return sequence indices in dendrogram order.
/// Uses UPGMA (average linkage) for balanced trees.
#[allow(dead_code)]
pub fn cluster_sequences(sequences: &[Vec<char>], gap_chars: &[char]) -> Vec<usize> {
    cluster_sequences_with_tree(sequences, gap_chars).order
}

/// Perform hierarchical clustering and return both order and tree visualization.
pub fn cluster_sequences_with_tree(sequences: &[Vec<char>], gap_chars: &[char]) -> ClusterResult {
    let n = sequences.len();
    if n <= 1 {
        return ClusterResult {
            order: (0..n).collect(),
            tree_lines: if n == 1 {
                vec!["─".to_string()]
            } else {
                vec![]
            },
            tree_width: if n == 1 { 1 } else { 0 },
            group_order: None,
            collapsed_tree_lines: None,
        };
    }

    let mut distances = compute_distance_matrix(sequences, gap_chars);
    let dendrogram = linkage(&mut distances, n, Method::Average);

    // Extract leaf order from dendrogram (depth-first traversal)
    let order = dendrogram_order(&dendrogram, n);

    // Build tree visualization
    let (tree_lines, tree_width) = build_tree_chars(&dendrogram, n, &order);

    ClusterResult {
        order,
        tree_lines,
        tree_width,
        group_order: None,
        collapsed_tree_lines: None,
    }
}

/// Perform hierarchical clustering using precomputed collapse groups.
/// This clusters only representative sequences, then expands the result.
/// Much faster when there are many identical sequences.
pub fn cluster_sequences_with_collapse(
    sequences: &[Vec<char>],
    gap_chars: &[char],
    collapse_groups: &[(usize, Vec<usize>)],
) -> ClusterResult {
    let n = sequences.len();
    let num_unique = collapse_groups.len();

    // If no duplicates or trivial case, use standard clustering
    // but still produce group_order so collapse+cluster works correctly
    if num_unique == n || n <= 1 {
        let mut result = cluster_sequences_with_tree(sequences, gap_chars);
        // Map each sequence index back to its group index
        // When all sequences are unique, group i contains sequence collapse_groups[i].0
        // So we need: for each position in order, find which group that sequence belongs to
        let mut seq_to_group = vec![0usize; n];
        for (group_idx, (rep, _)) in collapse_groups.iter().enumerate() {
            seq_to_group[*rep] = group_idx;
        }
        result.group_order = Some(result.order.iter().map(|&seq_idx| seq_to_group[seq_idx]).collect());
        result.collapsed_tree_lines = Some(result.tree_lines.clone());
        return result;
    }

    // Edge case: only one unique sequence (all identical)
    if num_unique == 1 {
        return ClusterResult {
            order: collapse_groups[0].1.clone(),
            tree_lines: vec!["─".to_string(); n],
            tree_width: 1,
            group_order: Some(vec![0]), // Only one group at position 0
            collapsed_tree_lines: Some(vec!["─".to_string()]), // One group = one line
        };
    }

    // Extract representative sequences
    let rep_indices: Vec<usize> = collapse_groups.iter().map(|(rep, _)| *rep).collect();
    let rep_sequences: Vec<Vec<char>> = rep_indices
        .iter()
        .map(|&idx| sequences[idx].clone())
        .collect();

    // Cluster only the representatives
    let mut distances = compute_distance_matrix(&rep_sequences, gap_chars);
    let dendrogram = linkage(&mut distances, num_unique, Method::Average);

    // Get order of representatives
    let rep_order = dendrogram_order(&dendrogram, num_unique);

    // Build tree for representatives
    let (rep_tree_lines, tree_width) = build_tree_chars(&dendrogram, num_unique, &rep_order);

    // Expand order: for each representative in order, include all its members
    let mut order = Vec::with_capacity(n);
    let mut tree_lines = Vec::with_capacity(n);

    // Build collapsed tree lines in display order (one per group)
    let mut collapsed_tree_lines = Vec::with_capacity(num_unique);

    for &rep_idx in &rep_order {
        let (_, members) = &collapse_groups[rep_idx];
        let tree_line = &rep_tree_lines[rep_order.iter().position(|&x| x == rep_idx).unwrap()];

        // Add to collapsed tree (one line per group)
        collapsed_tree_lines.push(tree_line.clone());

        // Expand for full tree (one line per member)
        for &member in members {
            order.push(member);
            tree_lines.push(tree_line.clone());
        }
    }

    ClusterResult {
        order,
        tree_lines,
        tree_width,
        group_order: Some(rep_order),
        collapsed_tree_lines: Some(collapsed_tree_lines),
    }
}

/// Extract leaf ordering from dendrogram via depth-first traversal.
/// This places similar sequences adjacent to each other.
fn dendrogram_order(dend: &kodama::Dendrogram<f64>, n: usize) -> Vec<usize> {
    let mut order = Vec::with_capacity(n);
    let steps = dend.steps();

    if steps.is_empty() {
        return (0..n).collect();
    }

    // The dendrogram has n-1 steps, each merging two clusters.
    // Cluster indices 0..n are original sequences.
    // Cluster indices n..2n-1 are merged clusters (step i creates cluster n+i).

    // Do a depth-first traversal starting from the root (last merged cluster).
    let root = n + steps.len() - 1;
    traverse_cluster(root, n, steps, &mut order);

    order
}

/// Recursively traverse a cluster to collect leaf indices in order.
fn traverse_cluster(cluster: usize, n: usize, steps: &[kodama::Step<f64>], order: &mut Vec<usize>) {
    if cluster < n {
        // This is a leaf (original sequence)
        order.push(cluster);
    } else {
        // This is a merged cluster - traverse both children
        let step_idx = cluster - n;
        let step = &steps[step_idx];
        traverse_cluster(step.cluster1, n, steps, order);
        traverse_cluster(step.cluster2, n, steps, order);
    }
}

/// Build dendrogram representation showing tree topology.
/// Uses bracket-style characters: ─┬┘│ to show which sequences group together.
/// Returns (tree_lines, tree_width).
fn build_tree_chars(
    dend: &kodama::Dendrogram<f64>,
    n: usize,
    order: &[usize],
) -> (Vec<String>, usize) {
    let steps = dend.steps();

    const MAX_TREE_WIDTH: usize = 16;

    if steps.is_empty() || n <= 1 {
        return (vec!["─".to_string(); n], 1);
    }

    // Map from original sequence index to display row
    let mut orig_to_row = vec![0usize; n];
    for (row, &orig) in order.iter().enumerate() {
        orig_to_row[orig] = row;
    }

    // For each node (leaf or internal), track its row span in display order
    // Leaves span a single row, internal nodes span from min to max of children
    let mut node_row_min = vec![0usize; 2 * n - 1];
    let mut node_row_max = vec![0usize; 2 * n - 1];

    // Initialize leaves
    for orig in 0..n {
        let row = orig_to_row[orig];
        node_row_min[orig] = row;
        node_row_max[orig] = row;
    }

    // Compute internal node spans (steps create nodes n, n+1, n+2, ...)
    for (i, step) in steps.iter().enumerate() {
        let node_id = n + i;
        node_row_min[node_id] = node_row_min[step.cluster1].min(node_row_min[step.cluster2]);
        node_row_max[node_id] = node_row_max[step.cluster1].max(node_row_max[step.cluster2]);
    }

    // Assign columns to internal nodes using recursive traversal
    // Each node gets the next available column when we "close" it
    let mut node_col = vec![0usize; 2 * n - 1];
    let mut next_col = 0usize;

    // Process nodes in order of their creation (merge order)
    // This ensures parent nodes get columns after their children
    for (i, _step) in steps.iter().enumerate() {
        let node_id = n + i;
        node_col[node_id] = next_col;
        next_col += 1;
    }

    let tree_width = next_col.clamp(1, MAX_TREE_WIDTH);

    // Scale columns if we exceed max width
    if next_col > MAX_TREE_WIDTH {
        for col in node_col.iter_mut().skip(n) {
            *col = (*col * (MAX_TREE_WIDTH - 1)) / (next_col - 1).max(1);
        }
    }

    // Build tree lines
    let mut tree_lines = Vec::with_capacity(n);

    for row in 0..n {
        let mut chars = vec![' '; tree_width];

        // For each internal node, draw its contribution to this row
        for (i, _step) in steps.iter().enumerate() {
            let node_id = n + i;
            let col = node_col[node_id];
            if col >= tree_width {
                continue;
            }

            let row_min = node_row_min[node_id];
            let row_max = node_row_max[node_id];

            if row < row_min || row > row_max {
                // Outside this node's span
                continue;
            }

            if row == row_min {
                // Top of bracket
                chars[col] = '┬';
            } else if row == row_max {
                // Bottom of bracket
                chars[col] = '┘';
            } else {
                // Middle - vertical line (don't overwrite existing)
                if chars[col] == ' ' {
                    chars[col] = '│';
                }
            }
        }

        // Fill horizontal lines from left to first bracket character
        let fill_to = chars
            .iter()
            .position(|&ch| ch == '│' || ch == '┬' || ch == '┘')
            .unwrap_or(tree_width);

        for ch in chars.iter_mut().take(fill_to) {
            if *ch == ' ' {
                *ch = '─';
            }
        }

        tree_lines.push(chars.into_iter().collect());
    }

    (tree_lines, tree_width)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hamming_distance_identical() {
        let seq1: Vec<char> = "ACGU".chars().collect();
        let seq2: Vec<char> = "ACGU".chars().collect();
        let gaps = vec!['-', '.'];
        assert_eq!(hamming_distance(&seq1, &seq2, &gaps), 0);
    }

    #[test]
    fn test_hamming_distance_different() {
        let seq1: Vec<char> = "ACGU".chars().collect();
        let seq2: Vec<char> = "UGCA".chars().collect();
        let gaps = vec!['-', '.'];
        assert_eq!(hamming_distance(&seq1, &seq2, &gaps), 4);
    }

    #[test]
    fn test_hamming_distance_with_gaps() {
        let seq1: Vec<char> = "AC-U".chars().collect();
        let seq2: Vec<char> = "AC-U".chars().collect();
        let gaps = vec!['-', '.'];
        assert_eq!(hamming_distance(&seq1, &seq2, &gaps), 0);

        let seq3: Vec<char> = "ACGU".chars().collect();
        assert_eq!(hamming_distance(&seq1, &seq3, &gaps), 1); // gap vs G
    }

    #[test]
    fn test_hamming_distance_case_insensitive() {
        let seq1: Vec<char> = "acgu".chars().collect();
        let seq2: Vec<char> = "ACGU".chars().collect();
        let gaps = vec!['-', '.'];
        assert_eq!(hamming_distance(&seq1, &seq2, &gaps), 0);
    }

    #[test]
    fn test_cluster_single_sequence() {
        let sequences = vec!["ACGU".chars().collect()];
        let gaps = vec!['-', '.'];
        let order = cluster_sequences(&sequences, &gaps);
        assert_eq!(order, vec![0]);
    }

    #[test]
    fn test_cluster_two_sequences() {
        let sequences = vec!["ACGU".chars().collect(), "ACGU".chars().collect()];
        let gaps = vec!['-', '.'];
        let order = cluster_sequences(&sequences, &gaps);
        assert_eq!(order.len(), 2);
        assert!(order.contains(&0));
        assert!(order.contains(&1));
    }

    #[test]
    fn test_cluster_groups_similar() {
        // Sequences 0 and 1 are identical, sequence 2 is different
        let sequences = vec![
            "AAAA".chars().collect(),
            "AAAA".chars().collect(),
            "UUUU".chars().collect(),
        ];
        let gaps = vec!['-', '.'];
        let order = cluster_sequences(&sequences, &gaps);

        // Check that 0 and 1 are adjacent in the order
        let pos0 = order.iter().position(|&x| x == 0).unwrap();
        let pos1 = order.iter().position(|&x| x == 1).unwrap();
        assert!(
            (pos0 as i32 - pos1 as i32).abs() == 1,
            "Similar sequences should be adjacent"
        );
    }

    #[test]
    fn test_tree_rendering() {
        // Test with 4 sequences: 0,1 similar, 2,3 similar
        let sequences = vec![
            "AAAA".chars().collect(),
            "AAAG".chars().collect(),
            "UUUU".chars().collect(),
            "UUUG".chars().collect(),
        ];
        let gaps = vec!['-', '.'];
        let result = cluster_sequences_with_tree(&sequences, &gaps);

        // Check we got 4 tree lines
        assert_eq!(result.tree_lines.len(), 4);
        // Check tree has expected width
        assert!(result.tree_width >= 1, "Tree should have some width");
        // Check each line contains expected dendrogram characters
        for line in &result.tree_lines {
            assert!(
                line.chars().all(|c| "─┬┘│ ".contains(c)),
                "Tree line '{}' contains unexpected characters",
                line
            );
        }
    }

    #[test]
    fn test_tree_rendering_single() {
        let sequences = vec!["ACGU".chars().collect()];
        let gaps = vec!['-', '.'];
        let result = cluster_sequences_with_tree(&sequences, &gaps);

        assert_eq!(result.tree_lines.len(), 1);
        assert_eq!(result.tree_width, 1);
        assert_eq!(result.tree_lines[0], "─");
    }

    #[test]
    fn test_cluster_with_collapse_groups() {
        // Test clustering with precomputed collapse groups
        // Sequences: A, A, B, A, C (indices 0-4, where 0,1,3 are identical "A")
        let sequences: Vec<Vec<char>> = vec![
            "AAAA".chars().collect(), // 0 - A
            "AAAA".chars().collect(), // 1 - A (duplicate)
            "CCCC".chars().collect(), // 2 - B
            "AAAA".chars().collect(), // 3 - A (duplicate)
            "UUUU".chars().collect(), // 4 - C
        ];
        let gaps = vec!['-', '.'];

        // Collapse groups: (representative, all_members)
        let collapse_groups = vec![
            (0, vec![0, 1, 3]), // A appears 3 times
            (2, vec![2]),       // B appears once
            (4, vec![4]),       // C appears once
        ];

        let result = cluster_sequences_with_collapse(&sequences, &gaps, &collapse_groups);

        // Should have all 5 sequences in order
        assert_eq!(result.order.len(), 5);
        assert_eq!(result.tree_lines.len(), 5);

        // All A sequences (0, 1, 3) should be adjacent
        let pos0 = result.order.iter().position(|&x| x == 0).unwrap();
        let pos1 = result.order.iter().position(|&x| x == 1).unwrap();
        let pos3 = result.order.iter().position(|&x| x == 3).unwrap();

        // Check they're consecutive
        let a_positions = vec![pos0, pos1, pos3];
        let min_pos = *a_positions.iter().min().unwrap();
        let max_pos = *a_positions.iter().max().unwrap();
        assert_eq!(
            max_pos - min_pos,
            2,
            "All A sequences should be consecutive"
        );
    }

    #[test]
    fn test_cluster_with_collapse_all_identical() {
        // Edge case: all sequences identical
        let sequences: Vec<Vec<char>> = vec![
            "AAAA".chars().collect(),
            "AAAA".chars().collect(),
            "AAAA".chars().collect(),
        ];
        let gaps = vec!['-', '.'];
        let collapse_groups = vec![(0, vec![0, 1, 2])];

        let result = cluster_sequences_with_collapse(&sequences, &gaps, &collapse_groups);

        assert_eq!(result.order.len(), 3);
        assert_eq!(result.tree_lines.len(), 3);
    }

    #[test]
    fn test_cluster_changes_order() {
        // Sequences arranged so clustering MUST change order:
        // 0 and 2 are similar (AAAA vs AAAG), 1 and 3 are similar (UUUU vs UUUG)
        // Original order [0, 1, 2, 3] should become something like [0, 2, 1, 3] or [1, 3, 0, 2]
        let sequences = vec![
            "AAAA".chars().collect(), // 0 - similar to 2
            "UUUU".chars().collect(), // 1 - similar to 3
            "AAAG".chars().collect(), // 2 - similar to 0
            "UUUG".chars().collect(), // 3 - similar to 1
        ];
        let gaps = vec!['-', '.'];
        let order = cluster_sequences(&sequences, &gaps);

        // Order should NOT be [0,1,2,3] - similar sequences should be adjacent
        assert_ne!(
            order,
            vec![0, 1, 2, 3],
            "Clustering should change order to group similar sequences"
        );

        // Verify similar sequences are adjacent:
        // 0 and 2 should be adjacent (both start with AAA)
        let pos0 = order.iter().position(|&x| x == 0).unwrap();
        let pos2 = order.iter().position(|&x| x == 2).unwrap();
        assert!(
            (pos0 as i32 - pos2 as i32).abs() == 1,
            "Sequences 0 and 2 (both AAAX) should be adjacent, got positions {} and {}",
            pos0,
            pos2
        );

        // 1 and 3 should be adjacent (both start with UUU)
        let pos1 = order.iter().position(|&x| x == 1).unwrap();
        let pos3 = order.iter().position(|&x| x == 3).unwrap();
        assert!(
            (pos1 as i32 - pos3 as i32).abs() == 1,
            "Sequences 1 and 3 (both UUUX) should be adjacent, got positions {} and {}",
            pos1,
            pos3
        );
    }

    #[test]
    fn test_cluster_protein_sequences() {
        // Test with protein-like sequences to ensure clustering works for non-RNA
        let sequences = vec![
            "MKTL".chars().collect(), // 0 - similar to 2
            "WFGH".chars().collect(), // 1 - similar to 3
            "MKTV".chars().collect(), // 2 - similar to 0
            "WFGI".chars().collect(), // 3 - similar to 1
        ];
        let gaps = vec!['-', '.'];
        let order = cluster_sequences(&sequences, &gaps);

        // Similar sequences should be adjacent
        let pos0 = order.iter().position(|&x| x == 0).unwrap();
        let pos2 = order.iter().position(|&x| x == 2).unwrap();
        assert!(
            (pos0 as i32 - pos2 as i32).abs() == 1,
            "Protein sequences 0 and 2 (both MKT*) should be adjacent"
        );

        let pos1 = order.iter().position(|&x| x == 1).unwrap();
        let pos3 = order.iter().position(|&x| x == 3).unwrap();
        assert!(
            (pos1 as i32 - pos3 as i32).abs() == 1,
            "Protein sequences 1 and 3 (both WFG*) should be adjacent"
        );
    }

    #[test]
    fn test_cluster_all_unique_sequences() {
        // Test when all sequences are unique (common for protein alignments)
        // This tests the code path where num_unique == n
        let sequences = vec![
            "AAAA".chars().collect(),
            "CCCC".chars().collect(),
            "GGGG".chars().collect(),
            "UUUU".chars().collect(),
        ];
        let gaps = vec!['-', '.'];

        // Create collapse groups where each sequence is its own group
        let collapse_groups = vec![(0, vec![0]), (1, vec![1]), (2, vec![2]), (3, vec![3])];

        let result = cluster_sequences_with_collapse(&sequences, &gaps, &collapse_groups);

        // Should still produce a valid ordering with all 4 sequences
        assert_eq!(result.order.len(), 4);
        assert!(result.order.contains(&0));
        assert!(result.order.contains(&1));
        assert!(result.order.contains(&2));
        assert!(result.order.contains(&3));
    }
}
