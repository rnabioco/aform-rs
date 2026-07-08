//! Sequence clustering using hierarchical agglomerative clustering.
//!
//! Uses Hamming distance and UPGMA (average linkage) to group similar sequences.
//! Identical sequences are collapsed before clustering to reduce O(n²) distance computation.

use kodama::{Method, linkage};

/// Maximum width (in characters) of the rendered dendrogram column.
const MAX_TREE_WIDTH: usize = 16;

/// Layout mode for the dendrogram column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TreeLayout {
    /// Topology only: horizontal position = tree depth. Compact and uniform.
    #[default]
    Cladogram,
    /// Distance-scaled: horizontal position ∝ merge dissimilarity (branch length).
    Phylogram,
}

// Connection-direction bits for the box-drawing bitmask grid.
const UP: u8 = 1;
const DOWN: u8 = 2;
const LEFT: u8 = 4;
const RIGHT: u8 = 8;

/// Translate a set of connection directions into a rounded box-drawing glyph.
fn mask_to_glyph(mask: u8) -> char {
    const H: u8 = LEFT | RIGHT;
    const V: u8 = UP | DOWN;
    const DR: u8 = DOWN | RIGHT;
    const DL: u8 = DOWN | LEFT;
    const UR: u8 = UP | RIGHT;
    const UL: u8 = UP | LEFT;
    const VR: u8 = V | RIGHT;
    const VL: u8 = V | LEFT;
    const HD: u8 = H | DOWN;
    const HU: u8 = H | UP;
    const CROSS: u8 = V | H;
    match mask {
        LEFT | RIGHT | H => '─',
        UP | DOWN | V => '│',
        DR => '╭',
        DL => '╮',
        UR => '╰',
        UL => '╯',
        VR => '├',
        VL => '┤',
        HD => '┬',
        HU => '┴',
        CROSS => '┼',
        _ => ' ',
    }
}

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
    cluster_sequences_with_tree(sequences, gap_chars, TreeLayout::Cladogram).order
}

/// Perform hierarchical clustering and return both order and tree visualization.
pub fn cluster_sequences_with_tree(
    sequences: &[Vec<char>],
    gap_chars: &[char],
    layout: TreeLayout,
) -> ClusterResult {
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

    // Each sequence occupies exactly one output row at its display position.
    let mut leaf_span = vec![(0usize, 0usize); n];
    for (row, &leaf) in order.iter().enumerate() {
        leaf_span[leaf] = (row, row);
    }

    let (tree_lines, tree_width) = build_tree_chars(&dendrogram, n, &leaf_span, n, layout);

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
    layout: TreeLayout,
) -> ClusterResult {
    let n = sequences.len();
    let num_unique = collapse_groups.len();

    // If no duplicates or trivial case, use standard clustering
    // but still produce group_order so collapse+cluster works correctly
    if num_unique == n || n <= 1 {
        let mut result = cluster_sequences_with_tree(sequences, gap_chars, layout);
        // Map each sequence index back to its group index
        // When all sequences are unique, group i contains sequence collapse_groups[i].0
        // So we need: for each position in order, find which group that sequence belongs to
        let mut seq_to_group = vec![0usize; n];
        for (group_idx, (rep, _)) in collapse_groups.iter().enumerate() {
            seq_to_group[*rep] = group_idx;
        }
        result.group_order = Some(
            result
                .order
                .iter()
                .map(|&seq_idx| seq_to_group[seq_idx])
                .collect(),
        );
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

    // Expand rep order into full sequence order, tracking the row block each
    // group occupies so the full tree can be drawn correctly across all n rows.
    let mut order = Vec::with_capacity(n);
    let mut rep_span = vec![(0usize, 0usize); num_unique];
    let mut row = 0usize;
    for &rep_slot in &rep_order {
        let (_, members) = &collapse_groups[rep_slot];
        let start = row;
        for &member in members {
            order.push(member);
            row += 1;
        }
        rep_span[rep_slot] = (start, row - 1);
    }

    // Full tree: each group is one leaf spanning its block of member rows.
    let (tree_lines, tree_width) = build_tree_chars(&dendrogram, num_unique, &rep_span, n, layout);

    // Collapsed tree: each group is one leaf on a single row, in display order.
    let mut group_span = vec![(0usize, 0usize); num_unique];
    for (grow, &rep_slot) in rep_order.iter().enumerate() {
        group_span[rep_slot] = (grow, grow);
    }
    let (collapsed_tree_lines, _) =
        build_tree_chars(&dendrogram, num_unique, &group_span, num_unique, layout);

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

/// Render a dendrogram as box-drawing lines, one string per output row.
///
/// `m` is the number of clustered leaves (whole sequences, or collapse
/// representatives). `leaf_row_span[leaf_id]` is the `(first_row, last_row)` block
/// each leaf occupies in the final `total_rows`-high output — a single row for the
/// un-collapsed case, a k-row block for a collapse group. Leaves sit at column 0
/// (adjacent to the alignment) and the root is at the rightmost column.
///
/// The layout assigns every node an anchor row (where its horizontal arm sits) and
/// a column, then paints connections onto a direction-bitmask grid so lines join
/// with correct corners/tees. Returns `(lines, width)`.
fn build_tree_chars(
    dend: &kodama::Dendrogram<f64>,
    m: usize,
    leaf_row_span: &[(usize, usize)],
    total_rows: usize,
    layout: TreeLayout,
) -> (Vec<String>, usize) {
    let steps = dend.steps();

    if m <= 1 || steps.is_empty() {
        return (vec!["─".to_string(); total_rows], 1);
    }

    let num_nodes = 2 * m - 1;
    let root = num_nodes - 1;

    // Anchor row and column for every node (leaves 0..m, internal m..num_nodes).
    let mut anchor = vec![0usize; num_nodes];
    let mut col = vec![0usize; num_nodes];
    for leaf in 0..m {
        let (lo, hi) = leaf_row_span[leaf];
        anchor[leaf] = (lo + hi) / 2;
        col[leaf] = 0;
    }

    // Phylogram scales by branch length; the root merge is the largest dissimilarity.
    let root_dissim = steps.last().map(|s| s.dissimilarity).unwrap_or(0.0);
    let phylo = matches!(layout, TreeLayout::Phylogram) && root_dissim > 0.0;

    for (i, step) in steps.iter().enumerate() {
        let node = m + i;
        anchor[node] = (anchor[step.cluster1] + anchor[step.cluster2]) / 2;
        let child_max = col[step.cluster1].max(col[step.cluster2]);
        col[node] = if phylo {
            let scaled =
                ((step.dissimilarity / root_dissim) * (MAX_TREE_WIDTH - 1) as f64).round() as usize;
            scaled.max(1).max(child_max)
        } else {
            child_max + 1
        };
    }

    // Compress columns to fit MAX_TREE_WIDTH, keeping leaves at column 0.
    let raw_max = col.iter().copied().max().unwrap_or(0);
    if raw_max >= MAX_TREE_WIDTH {
        for c in col.iter_mut().skip(m) {
            *c = *c * (MAX_TREE_WIDTH - 1) / raw_max;
        }
        // Restore parent ≥ children after integer rounding.
        for (i, step) in steps.iter().enumerate() {
            let node = m + i;
            col[node] = col[node].max(col[step.cluster1]).max(col[step.cluster2]);
        }
    }

    let width = col.iter().copied().max().unwrap_or(0) + 1;

    // Paint connections onto a bitmask grid, then translate each cell to a glyph.
    let mut grid = vec![0u8; total_rows * width];
    let idx = |r: usize, c: usize| r * width + c;

    for (i, step) in steps.iter().enumerate() {
        let node = m + i;
        let c = col[node];
        let (a1, a2) = (anchor[step.cluster1], anchor[step.cluster2]);

        // Horizontal arm from each child's column rightward to this node's column.
        for &child in &[step.cluster1, step.cluster2] {
            let ra = anchor[child];
            for x in col[child]..c {
                grid[idx(ra, x)] |= RIGHT;
                grid[idx(ra, x + 1)] |= LEFT;
            }
        }

        // Vertical bar spanning the two child anchor rows.
        for r in a1.min(a2)..a1.max(a2) {
            grid[idx(r, c)] |= DOWN;
            grid[idx(r + 1, c)] |= UP;
        }

        // Outgoing arm toward the parent (the root has none).
        if node != root {
            grid[idx(anchor[node], c)] |= RIGHT;
        }
    }

    let tree_lines = (0..total_rows)
        .map(|r| (0..width).map(|c| mask_to_glyph(grid[idx(r, c)])).collect())
        .collect();

    (tree_lines, width)
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
        let result = cluster_sequences_with_tree(&sequences, &gaps, TreeLayout::Cladogram);

        // Check we got 4 tree lines
        assert_eq!(result.tree_lines.len(), 4);
        // Check tree has expected width
        assert!(result.tree_width >= 1, "Tree should have some width");
        // Check each line contains only box-drawing dendrogram characters
        for line in &result.tree_lines {
            assert!(
                line.chars().all(|c| "─│╭╮╰╯├┤┬┴┼ ".contains(c)),
                "Tree line '{}' contains unexpected characters",
                line
            );
        }
        // Every leaf row must connect to the tree at column 0.
        for line in &result.tree_lines {
            assert_eq!(
                line.chars().next(),
                Some('─'),
                "Leaf row '{}' should start with a horizontal connector",
                line
            );
        }
    }

    #[test]
    fn test_tree_rendering_single() {
        let sequences = vec!["ACGU".chars().collect()];
        let gaps = vec!['-', '.'];
        let result = cluster_sequences_with_tree(&sequences, &gaps, TreeLayout::Cladogram);

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

        let result = cluster_sequences_with_collapse(
            &sequences,
            &gaps,
            &collapse_groups,
            TreeLayout::Cladogram,
        );

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

        let result = cluster_sequences_with_collapse(
            &sequences,
            &gaps,
            &collapse_groups,
            TreeLayout::Cladogram,
        );

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

        let result = cluster_sequences_with_collapse(
            &sequences,
            &gaps,
            &collapse_groups,
            TreeLayout::Cladogram,
        );

        // Should still produce a valid ordering with all 4 sequences
        assert_eq!(result.order.len(), 4);
        assert!(result.order.contains(&0));
        assert!(result.order.contains(&1));
        assert!(result.order.contains(&2));
        assert!(result.order.contains(&3));
    }
}
