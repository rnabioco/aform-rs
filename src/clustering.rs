//! Sequence clustering using hierarchical agglomerative clustering.
//!
//! Uses Hamming distance and UPGMA (average linkage) to group similar sequences.
//! Identical sequences are collapsed before clustering to reduce O(n²) distance computation.

use kodama::{Method, linkage};
use rayon::prelude::*;

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

/// Build a 256-entry lookup table marking which ASCII bytes are gap characters.
/// Alignments are ASCII, so a byte LUT gives O(1) gap tests.
pub fn build_gap_lut(gap_chars: &[char]) -> [bool; 256] {
    let mut lut = [false; 256];
    for &c in gap_chars {
        if c.is_ascii() {
            lut[c as usize] = true;
        }
    }
    lut
}

/// Compute Hamming distance between two byte sequences (count mismatches).
/// Comparison is case-insensitive. Both gaps count as a match; one gap and one
/// residue counts as a mismatch; two differing residues count as a mismatch.
pub fn hamming_distance(seq1: &[u8], seq2: &[u8], gap_lut: &[bool; 256]) -> usize {
    seq1.iter()
        .zip(seq2.iter())
        .filter(|&(&a, &b)| {
            let a_gap = gap_lut[a as usize];
            let b_gap = gap_lut[b as usize];
            if a_gap && b_gap {
                // Both gaps = match (don't count as mismatch)
                false
            } else {
                // One gap + one residue = mismatch
                // Two different residues = mismatch (case-insensitive)
                !a.eq_ignore_ascii_case(&b)
            }
        })
        .count()
}

/// Compute condensed distance matrix for all sequence pairs.
/// Returns distances in row-major condensed form for kodama:
/// for i in 0..n, for j in i+1..n. Parallelized across rows with rayon while
/// preserving that exact ordering.
pub fn compute_distance_matrix(sequences: &[Vec<u8>], gap_lut: &[bool; 256]) -> Vec<f64> {
    let n = sequences.len();

    // Each row i contributes distances to j = i+1..n, in order.
    let rows: Vec<Vec<f64>> = (0..n)
        .into_par_iter()
        .map(|i| {
            (i + 1..n)
                .map(|j| hamming_distance(&sequences[i], &sequences[j], gap_lut) as f64)
                .collect::<Vec<f64>>()
        })
        .collect();

    // Flatten in row order to reproduce the original nested-loop layout exactly.
    let mut distances = Vec::with_capacity(n * (n - 1) / 2);
    for row in rows {
        distances.extend(row);
    }
    distances
}

/// Perform hierarchical clustering and return sequence indices in dendrogram order.
/// Uses UPGMA (average linkage) for balanced trees.
#[allow(dead_code)]
pub fn cluster_sequences(sequences: &[Vec<u8>], gap_lut: &[bool; 256]) -> Vec<usize> {
    cluster_sequences_with_tree(sequences, gap_lut).order
}

/// Perform hierarchical clustering and return both order and tree visualization.
pub fn cluster_sequences_with_tree(sequences: &[Vec<u8>], gap_lut: &[bool; 256]) -> ClusterResult {
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

    let mut distances = compute_distance_matrix(sequences, gap_lut);
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
    sequences: &[Vec<u8>],
    gap_lut: &[bool; 256],
    collapse_groups: &[(usize, Vec<usize>)],
) -> ClusterResult {
    let n = sequences.len();
    let num_unique = collapse_groups.len();

    // If no duplicates or trivial case, use standard clustering
    // but still produce group_order so collapse+cluster works correctly
    if num_unique == n || n <= 1 {
        let mut result = cluster_sequences_with_tree(sequences, gap_lut);
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
        // collapsed_tree mirrors the (possibly empty) per-row tree.
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
    let rep_sequences: Vec<Vec<u8>> = rep_indices
        .iter()
        .map(|&idx| sequences[idx].clone())
        .collect();

    // Cluster only the representatives
    let mut distances = compute_distance_matrix(&rep_sequences, gap_lut);
    let dendrogram = linkage(&mut distances, num_unique, Method::Average);

    // Get order of representatives
    let rep_order = dendrogram_order(&dendrogram, num_unique);

    // Build tree for representatives (one line per group / representative).
    let (rep_tree_lines, tree_width) = build_tree_chars(&dendrogram, num_unique, &rep_order);

    // Expand order: for each representative in order, include all its members.
    let mut order = Vec::with_capacity(n);
    let mut tree_lines = Vec::with_capacity(n);

    // Build collapsed tree lines in display order (one per group)
    let mut collapsed_tree_lines = Vec::with_capacity(num_unique);

    for (display_pos, &rep_idx) in rep_order.iter().enumerate() {
        let (_, members) = &collapse_groups[rep_idx];

        // rep_tree_lines is indexed by display position (row) already.
        let tree_line = rep_tree_lines.get(display_pos).cloned().unwrap_or_default();

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

/// Merge a newly-drawn box-drawing direction into an existing grid cell so that
/// segments meeting at a cell pick a visually-correct junction character.
///
/// `up`/`down`/`left`/`right` describe the connections the new segment adds. The
/// existing cell's own connections are decoded, unioned with the new ones, and the
/// matching box-drawing glyph is returned.
fn merge_cell(existing: char, up: bool, down: bool, left: bool, right: bool) -> char {
    // Decode existing connections. Half-line glyphs must round-trip so that a
    // single-direction stub (e.g. the top of a vertical, which connects only
    // downward) is not mistaken for a full line when another segment joins it.
    let (mut eu, mut ed, mut el, mut er) = match existing {
        '╵' => (true, false, false, false),
        '╷' => (false, true, false, false),
        '╴' => (false, false, true, false),
        '╶' => (false, false, false, true),
        '─' => (false, false, true, true),
        '│' => (true, true, false, false),
        '┌' => (false, true, false, true),
        '┐' => (false, true, true, false),
        '└' => (true, false, false, true),
        '┘' => (true, false, true, false),
        '├' => (true, true, false, true),
        '┤' => (true, true, true, false),
        '┬' => (false, true, true, true),
        '┴' => (true, false, true, true),
        '┼' => (true, true, true, true),
        _ => (false, false, false, false),
    };
    eu |= up;
    ed |= down;
    el |= left;
    er |= right;

    match (eu, ed, el, er) {
        (false, false, false, false) => ' ',
        // Single directions (half lines) — kept distinct so corners can form.
        (true, false, false, false) => '╵',
        (false, true, false, false) => '╷',
        (false, false, true, false) => '╴',
        (false, false, false, true) => '╶',
        // Straight lines.
        (false, false, true, true) => '─',
        (true, true, false, false) => '│',
        // Corners.
        (false, true, false, true) => '┌',
        (false, true, true, false) => '┐',
        (true, false, false, true) => '└',
        (true, false, true, false) => '┘',
        // Tees and cross.
        (true, true, false, true) => '├',
        (true, true, true, false) => '┤',
        (false, true, true, true) => '┬',
        (true, false, true, true) => '┴',
        (true, true, true, true) => '┼',
    }
}

/// Merge a direction set into a grid cell.
fn draw(
    grid: &mut [Vec<char>],
    row: usize,
    col: usize,
    up: bool,
    down: bool,
    left: bool,
    right: bool,
) {
    let cur = grid[row][col];
    grid[row][col] = merge_cell(cur, up, down, left, right);
}

/// Build a rotated (left-to-right) dendrogram, scipy style.
///
/// Columns encode merge height (dissimilarity): leaves sit at column 0 on the left,
/// the root merge sits at the far right. Rows are the display order. Returns
/// (tree_lines, tree_width).
fn build_tree_chars(
    dend: &kodama::Dendrogram<f64>,
    n: usize,
    order: &[usize],
) -> (Vec<String>, usize) {
    let steps = dend.steps();

    if steps.is_empty() || n <= 1 {
        return (vec!["─".to_string(); n], 1);
    }

    // Map from original sequence index to display row.
    let mut orig_to_row = vec![0usize; n];
    for (row, &orig) in order.iter().enumerate() {
        orig_to_row[orig] = row;
    }

    // Choose an adaptive tree width, capped so wide dendrograms stay readable.
    let tree_width = 32.min((n - 1).max(1));

    let num_nodes = 2 * n - 1;
    let mut conn_row = vec![0usize; num_nodes];
    let mut node_col = vec![0usize; num_nodes];

    // Leaves: connection row is their display row, column 0.
    for orig in 0..n {
        conn_row[orig] = orig_to_row[orig];
        node_col[orig] = 0;
    }

    // Max dissimilarity is at the last (root) step since merges are monotonic.
    let max_diss = steps.last().map(|s| s.dissimilarity).unwrap_or(0.0);

    // Internal nodes: connection row is the (rounded) midpoint of children's rows;
    // column scales with dissimilarity so higher merges sit further right.
    let mut parent = vec![usize::MAX; num_nodes];
    for (i, step) in steps.iter().enumerate() {
        let node_id = n + i;
        let c1 = step.cluster1;
        let c2 = step.cluster2;
        parent[c1] = node_id;
        parent[c2] = node_id;
        conn_row[node_id] = (conn_row[c1] + conn_row[c2]).div_ceil(2);
        let col = if max_diss > 0.0 {
            ((step.dissimilarity / max_diss) * (tree_width - 1) as f64).round() as usize
        } else {
            tree_width - 1
        };
        node_col[node_id] = col.min(tree_width - 1);
    }

    // A leaf pair ("cherry") spans two adjacent rows, so its join can only sit on
    // one of them — no cell exists between. Point it toward its parent (top if the
    // parent is above, bottom otherwise) so the outgoing branch reads as a straight
    // line instead of always dog-legging from the bottom. Cherries never nest, and
    // their span is fixed (both children are leaves), so this can't strand a branch.
    for (i, step) in steps.iter().enumerate() {
        let (c1, c2) = (step.cluster1, step.cluster2);
        if c1 < n && c2 < n {
            let node_id = n + i;
            let top = conn_row[c1].min(conn_row[c2]);
            let bottom = conn_row[c1].max(conn_row[c2]);
            let p = parent[node_id];
            conn_row[node_id] = if p != usize::MAX && conn_row[p] <= top {
                top
            } else {
                bottom
            };
        }
    }

    // Grid of box-drawing characters.
    let mut grid = vec![vec![' '; tree_width]; n];

    for (i, step) in steps.iter().enumerate() {
        let node_id = n + i;
        let col = node_col[node_id];
        let c1 = step.cluster1;
        let c2 = step.cluster2;
        let r1 = conn_row[c1];
        let r2 = conn_row[c2];
        let (top, bottom) = if r1 <= r2 { (r1, r2) } else { (r2, r1) };

        // Vertical segment at this node's column spanning its two children rows.
        for r in top..=bottom {
            let up = r > top;
            let down = r < bottom;
            draw(&mut grid, r, col, up, down, false, false);
        }

        // Horizontal run at each child's connection row from the child's column to
        // this node's column. A child's column is always <= this node's (merges are
        // monotonic in height). When strictly less, draw the connector; on a tie the
        // child already sits on this column's vertical spine, so drawing anything
        // (a leftward stub) would dead-end into empty space.
        for &(child, child_row) in &[(c1, r1), (c2, r2)] {
            let child_col = node_col[child];
            if child_col < col {
                for cc in child_col..=col {
                    let left = cc > child_col;
                    let right = cc < col;
                    draw(&mut grid, child_row, cc, false, false, left, right);
                }
            }
        }
    }

    // Extend each leaf to the left edge. Only draw leftward: the rightward link is
    // the horizontal connector's job, and adding `right` here would dead-end into
    // empty space for a leaf whose merge sits in column 0.
    for &row in orig_to_row.iter() {
        draw(&mut grid, row, 0, false, false, true, false);
    }

    // Soften the L-corners to rounded glyphs for a nicer dendrogram. Junction
    // merging above uses the sharp forms; round only at the end. T-junctions and
    // crosses have no rounded Unicode variants, so they stay square.
    let tree_lines: Vec<String> = grid
        .into_iter()
        .map(|r| r.into_iter().map(round_corner).collect())
        .collect();

    (tree_lines, tree_width)
}

/// Map the four sharp box-drawing corners to their rounded Unicode equivalents.
fn round_corner(c: char) -> char {
    match c {
        '┌' => '╭',
        '┐' => '╮',
        '┘' => '╯',
        '└' => '╰',
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a byte sequence from a string literal.
    fn seq(s: &str) -> Vec<u8> {
        s.as_bytes().to_vec()
    }

    /// Helper: standard gap LUT for '-' and '.'.
    fn gaps() -> [bool; 256] {
        build_gap_lut(&['-', '.'])
    }

    fn decode(c: char) -> (bool, bool, bool, bool) {
        match c {
            '╵' => (true, false, false, false),
            '╷' => (false, true, false, false),
            '╴' => (false, false, true, false),
            '╶' => (false, false, false, true),
            '─' => (false, false, true, true),
            '│' => (true, true, false, false),
            '┌' | '╭' => (false, true, false, true),
            '┐' | '╮' => (false, true, true, false),
            '└' | '╰' => (true, false, false, true),
            '┘' | '╯' => (true, false, true, false),
            '├' => (true, true, false, true),
            '┤' => (true, true, true, false),
            '┬' => (false, true, true, true),
            '┴' => (true, false, true, true),
            '┼' => (true, true, true, true),
            _ => (false, false, false, false),
        }
    }

    /// Count grid cells whose connection points at a neighbor that does not connect
    /// back (a "dead-end" stub). Leftward connections at column 0 reach the display
    /// edge and are allowed.
    fn count_dead_ends(tree_lines: &[String], width: usize) -> usize {
        let grid: Vec<Vec<char>> = tree_lines.iter().map(|l| l.chars().collect()).collect();
        let h = grid.len();
        let mut dead = 0;
        for row in 0..h {
            for col in 0..width {
                let (u, d, l, ri) = decode(grid[row][col]);
                if ri && (col + 1 >= width || !decode(grid[row][col + 1]).2) {
                    dead += 1;
                }
                if l && col > 0 && !decode(grid[row][col - 1]).3 {
                    dead += 1;
                }
                if u && (row == 0 || !decode(grid[row - 1][col]).1) {
                    dead += 1;
                }
                if d && (row + 1 >= h || !decode(grid[row + 1][col]).0) {
                    dead += 1;
                }
            }
        }
        dead
    }

    #[test]
    fn test_dendrogram_has_no_dead_ends() {
        // Regression: every drawn segment must connect to a neighbor — no orphan
        // "starter branch" stubs. Exercised on the real r-scape alignment, whose
        // dissimilarity ties previously produced spurious ┤ / ┬ stubs.
        let path = std::path::Path::new("examples/r-scape/RF00005.sto");
        let Ok(aln) = crate::stockholm::parser::parse_file(path) else {
            return; // example not present in this environment; skip
        };
        let seqs: Vec<Vec<u8>> = aln
            .sequences
            .iter()
            .map(|s| s.data().into_bytes())
            .collect();
        let r = cluster_sequences_with_tree(&seqs, &gaps());
        assert_eq!(
            count_dead_ends(&r.tree_lines, r.tree_width),
            0,
            "dendrogram contains dead-end stubs"
        );
    }

    #[test]
    fn test_hamming_distance_identical() {
        let seq1 = seq("ACGU");
        let seq2 = seq("ACGU");
        assert_eq!(hamming_distance(&seq1, &seq2, &gaps()), 0);
    }

    #[test]
    fn test_hamming_distance_different() {
        let seq1 = seq("ACGU");
        let seq2 = seq("UGCA");
        assert_eq!(hamming_distance(&seq1, &seq2, &gaps()), 4);
    }

    #[test]
    fn test_hamming_distance_with_gaps() {
        let seq1 = seq("AC-U");
        let seq2 = seq("AC-U");
        assert_eq!(hamming_distance(&seq1, &seq2, &gaps()), 0);

        let seq3 = seq("ACGU");
        assert_eq!(hamming_distance(&seq1, &seq3, &gaps()), 1); // gap vs G
    }

    #[test]
    fn test_hamming_distance_case_insensitive() {
        let seq1 = seq("acgu");
        let seq2 = seq("ACGU");
        assert_eq!(hamming_distance(&seq1, &seq2, &gaps()), 0);
    }

    #[test]
    fn test_cluster_single_sequence() {
        let sequences = vec![seq("ACGU")];
        let order = cluster_sequences(&sequences, &gaps());
        assert_eq!(order, vec![0]);
    }

    #[test]
    fn test_cluster_two_sequences() {
        let sequences = vec![seq("ACGU"), seq("ACGU")];
        let order = cluster_sequences(&sequences, &gaps());
        assert_eq!(order.len(), 2);
        assert!(order.contains(&0));
        assert!(order.contains(&1));
    }

    #[test]
    fn test_cluster_groups_similar() {
        // Sequences 0 and 1 are identical, sequence 2 is different
        let sequences = vec![seq("AAAA"), seq("AAAA"), seq("UUUU")];
        let order = cluster_sequences(&sequences, &gaps());

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
        let sequences = vec![seq("AAAA"), seq("AAAG"), seq("UUUU"), seq("UUUG")];
        let result = cluster_sequences_with_tree(&sequences, &gaps());

        // Check we got 4 tree lines
        assert_eq!(result.tree_lines.len(), 4);
        // Check tree has expected width
        assert!(result.tree_width >= 1, "Tree should have some width");
        // Check each line contains expected dendrogram characters
        for line in &result.tree_lines {
            assert!(
                line.chars().all(|c| "─│┌┐└┘├┤┬┴┼╭╮╯╰╷╵╴╶ ".contains(c)),
                "Tree line '{}' contains unexpected characters",
                line
            );
        }
    }

    #[test]
    fn test_tree_column_reflects_merge_height() {
        // Four sequences forming two tight pairs that merge at low height, with the
        // two pairs joined at the root at high height. The pair-merge columns must be
        // strictly less than the root-merge column (column reflects merge height).
        let sequences = vec![
            seq("AAAAAAAA"),
            seq("AAAAAAAG"), // pair with 0
            seq("UUUUUUUU"),
            seq("UUUUUUUG"), // pair with 2
        ];
        let gap_lut = gaps();
        let n = sequences.len();

        let mut distances = compute_distance_matrix(&sequences, &gap_lut);
        let dend = linkage(&mut distances, n, Method::Average);
        let steps = dend.steps();

        // Three steps: two pair merges (low diss) then the root merge (max diss).
        let max_diss = steps.last().unwrap().dissimilarity;
        assert!(max_diss > 0.0);
        let tree_width = 32.min((n - 1).max(1));

        let col_of = |diss: f64| ((diss / max_diss) * (tree_width - 1) as f64).round() as usize;

        // The last step is the root; the first two are the tight pair merges.
        let root_col = col_of(steps[2].dissimilarity);
        let pair0_col = col_of(steps[0].dissimilarity);
        let pair1_col = col_of(steps[1].dissimilarity);

        assert!(
            pair0_col < root_col,
            "pair merge column {pair0_col} should be < root column {root_col}"
        );
        assert!(
            pair1_col < root_col,
            "pair merge column {pair1_col} should be < root column {root_col}"
        );

        // And the rendered tree uses only box-drawing characters.
        let result = cluster_sequences_with_tree(&sequences, &gap_lut);
        for line in &result.tree_lines {
            assert!(
                line.chars().all(|c| "─│┌┐└┘├┤┬┴┼╭╮╯╰╷╵╴╶ ".contains(c)),
                "Tree line '{}' contains unexpected characters",
                line
            );
        }
    }

    #[test]
    fn test_tree_rendering_single() {
        let sequences = vec![seq("ACGU")];
        let result = cluster_sequences_with_tree(&sequences, &gaps());

        assert_eq!(result.tree_lines.len(), 1);
        assert_eq!(result.tree_width, 1);
        assert_eq!(result.tree_lines[0], "─");
    }

    #[test]
    fn test_cluster_with_collapse_groups() {
        // Test clustering with precomputed collapse groups
        // Sequences: A, A, B, A, C (indices 0-4, where 0,1,3 are identical "A")
        let sequences: Vec<Vec<u8>> = vec![
            seq("AAAA"), // 0 - A
            seq("AAAA"), // 1 - A (duplicate)
            seq("CCCC"), // 2 - B
            seq("AAAA"), // 3 - A (duplicate)
            seq("UUUU"), // 4 - C
        ];

        // Collapse groups: (representative, all_members)
        let collapse_groups = vec![
            (0, vec![0, 1, 3]), // A appears 3 times
            (2, vec![2]),       // B appears once
            (4, vec![4]),       // C appears once
        ];

        let result = cluster_sequences_with_collapse(&sequences, &gaps(), &collapse_groups);

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
        let sequences: Vec<Vec<u8>> = vec![seq("AAAA"), seq("AAAA"), seq("AAAA")];
        let collapse_groups = vec![(0, vec![0, 1, 2])];

        let result = cluster_sequences_with_collapse(&sequences, &gaps(), &collapse_groups);

        assert_eq!(result.order.len(), 3);
        assert_eq!(result.tree_lines.len(), 3);
    }

    #[test]
    fn test_cluster_changes_order() {
        // Sequences arranged so clustering MUST change order:
        // 0 and 2 are similar (AAAA vs AAAG), 1 and 3 are similar (UUUU vs UUUG)
        // Original order [0, 1, 2, 3] should become something like [0, 2, 1, 3] or [1, 3, 0, 2]
        let sequences = vec![
            seq("AAAA"), // 0 - similar to 2
            seq("UUUU"), // 1 - similar to 3
            seq("AAAG"), // 2 - similar to 0
            seq("UUUG"), // 3 - similar to 1
        ];
        let order = cluster_sequences(&sequences, &gaps());

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
            seq("MKTL"), // 0 - similar to 2
            seq("WFGH"), // 1 - similar to 3
            seq("MKTV"), // 2 - similar to 0
            seq("WFGI"), // 3 - similar to 1
        ];
        let order = cluster_sequences(&sequences, &gaps());

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
        let sequences = vec![seq("AAAA"), seq("CCCC"), seq("GGGG"), seq("UUUU")];

        // Create collapse groups where each sequence is its own group
        let collapse_groups = vec![(0, vec![0]), (1, vec![1]), (2, vec![2]), (3, vec![3])];

        let result = cluster_sequences_with_collapse(&sequences, &gaps(), &collapse_groups);

        // Should still produce a valid ordering with all 4 sequences
        assert_eq!(result.order.len(), 4);
        assert!(result.order.contains(&0));
        assert!(result.order.contains(&1));
        assert!(result.order.contains(&2));
        assert!(result.order.contains(&3));
    }

    #[test]
    fn test_large_alignment_still_builds_tree() {
        // Regression: large alignments must still produce a full, non-empty tree
        // (one line per row) so the dendrogram renders for real-world sizes.
        let n = 200;
        let sequences: Vec<Vec<u8>> = (0..n)
            .map(|i| format!("{:08b}", i % 256).into_bytes())
            .collect();
        let collapse_groups: Vec<(usize, Vec<usize>)> = (0..n).map(|i| (i, vec![i])).collect();

        let result = cluster_sequences_with_collapse(&sequences, &gaps(), &collapse_groups);

        assert_eq!(result.order.len(), n);
        assert_eq!(
            result.tree_lines.len(),
            n,
            "tree must have one line per row"
        );
        assert!(
            result.tree_lines.iter().all(|l| !l.is_empty()),
            "no tree line should be empty for a large alignment"
        );
        assert!(result.tree_width >= 1);
    }
}
