//! Sequence clustering using hierarchical agglomerative clustering.
//!
//! Uses Hamming distance and UPGMA (average linkage) to group similar sequences.

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

/// Information about a node in the dendrogram for tree rendering.
#[derive(Debug, Clone)]
struct NodeInfo {
    /// First row in display order that belongs to this subtree.
    row_min: usize,
    /// Last row in display order that belongs to this subtree.
    row_max: usize,
    /// Depth from leaves (leaves = 0, root = max).
    depth: usize,
}

/// Build ASCII tree representation from dendrogram.
/// Returns (tree_lines, tree_width).
fn build_tree_chars(
    dend: &kodama::Dendrogram<f64>,
    n: usize,
    order: &[usize],
) -> (Vec<String>, usize) {
    let steps = dend.steps();
    if steps.is_empty() || n <= 1 {
        return (vec!["─".to_string(); n], 1);
    }

    // Create a mapping from original sequence index to display row
    let mut orig_to_row = vec![0; n];
    for (row, &orig) in order.iter().enumerate() {
        orig_to_row[orig] = row;
    }

    // Compute NodeInfo for all nodes (leaves + internal)
    // Node IDs: 0..n are leaves, n..2n-1 are internal nodes (from merge steps)
    let mut node_info = vec![
        NodeInfo {
            row_min: 0,
            row_max: 0,
            depth: 0
        };
        2 * n - 1
    ];

    // Initialize leaves
    for orig in 0..n {
        let row = orig_to_row[orig];
        node_info[orig] = NodeInfo {
            row_min: row,
            row_max: row,
            depth: 0,
        };
    }

    // Compute internal nodes bottom-up (steps are in merge order)
    for (i, step) in steps.iter().enumerate() {
        let node_id = n + i;
        let c1 = &node_info[step.cluster1];
        let c2 = &node_info[step.cluster2];
        node_info[node_id] = NodeInfo {
            row_min: c1.row_min.min(c2.row_min),
            row_max: c1.row_max.max(c2.row_max),
            depth: c1.depth.max(c2.depth) + 1,
        };
    }

    // Find max depth (tree width in columns)
    let max_depth = node_info.iter().map(|n| n.depth).max().unwrap_or(0);

    // For each internal node, we need to track which column it occupies.
    // Use depth-based column: depth 1 nodes at column 0, depth 2 at column 1, etc.
    // But multiple nodes can have the same depth, so we assign unique columns.

    // Sort internal nodes by depth, then by row_min to ensure consistent ordering
    let mut internal_nodes: Vec<(usize, &NodeInfo)> =
        (n..2 * n - 1).map(|id| (id, &node_info[id])).collect();
    internal_nodes.sort_by_key(|(_, info)| (info.depth, info.row_min));

    // Assign columns based on depth
    let mut node_columns = vec![0usize; 2 * n - 1];
    for (node_id, info) in &internal_nodes {
        node_columns[*node_id] = info.depth - 1; // depth 1 -> column 0
    }

    // Build tree lines for each row
    let tree_width = max_depth;
    let mut tree_lines = Vec::with_capacity(n);

    for row in 0..n {
        let mut chars: Vec<char> = vec![' '; tree_width];

        // First char is always horizontal line connecting to tree
        if tree_width > 0 {
            chars[0] = '─';
        }

        // For each internal node, determine what character to draw at its column
        for &(node_id, info) in &internal_nodes {
            let col = node_columns[node_id];
            if col >= tree_width {
                continue;
            }

            if row < info.row_min || row > info.row_max {
                // This node doesn't span this row - draw horizontal line if needed
                // (continuation from left)
                if chars[col] == ' ' && col > 0 && chars[col - 1] != ' ' {
                    // Continue horizontal line through
                    chars[col] = '─';
                }
            } else if row == info.row_min && row == info.row_max {
                // Single-row span (shouldn't happen for internal nodes, but handle it)
                chars[col] = '─';
            } else if row == info.row_min {
                // Top of this node's span
                chars[col] = '┬';
            } else if row == info.row_max {
                // Bottom of this node's span
                chars[col] = '┘';
            } else {
                // Middle of span - vertical line
                chars[col] = '│';
            }
        }

        // Fill horizontal lines from left until we hit a vertical element or end
        let mut fill = true;
        for ch in &mut chars {
            if *ch == ' ' && fill {
                *ch = '─';
            } else if *ch == '│' || *ch == '┘' {
                fill = false;
            } else if *ch == '┬' {
                // After a ┬, continue filling to the right
                fill = true;
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
        // Check tree has expected width (depth = 2 for this tree)
        assert!(result.tree_width >= 1, "Tree should have some width");
        // Check each line contains expected characters
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
        assert_eq!(result.tree_lines[0], "─");
    }
}
