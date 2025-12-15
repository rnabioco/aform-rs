//! Sequence clustering using hierarchical agglomerative clustering.
//!
//! Uses Hamming distance and UPGMA (average linkage) to group similar sequences.

use kodama::{linkage, Method};

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
pub fn cluster_sequences(sequences: &[Vec<char>], gap_chars: &[char]) -> Vec<usize> {
    let n = sequences.len();
    if n <= 1 {
        return (0..n).collect();
    }

    let mut distances = compute_distance_matrix(sequences, gap_chars);
    let dendrogram = linkage(&mut distances, n, Method::Average);

    // Extract leaf order from dendrogram (depth-first traversal)
    dendrogram_order(&dendrogram, n)
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
        let sequences = vec![
            "ACGU".chars().collect(),
            "ACGU".chars().collect(),
        ];
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
        assert!((pos0 as i32 - pos1 as i32).abs() == 1, "Similar sequences should be adjacent");
    }
}
