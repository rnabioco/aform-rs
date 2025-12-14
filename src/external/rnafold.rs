//! ViennaRNA integration (RNAfold, RNAalifold).
//!
//! See: https://github.com/ViennaRNA/ViennaRNA

use std::io::Write;
use std::process::{Command, Stdio};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum RnaFoldError {
    #[error("RNAfold not found in PATH")]
    NotFound,
    #[error("RNAfold execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Failed to parse RNAfold output")]
    ParseError,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result of a folding operation.
#[derive(Debug, Clone)]
pub struct FoldResult {
    /// Secondary structure in dot-bracket notation.
    pub structure: String,
    /// Minimum free energy (kcal/mol).
    pub mfe: Option<f64>,
}

/// Check if RNAfold is available.
pub fn rnafold_available() -> bool {
    Command::new("RNAfold")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Check if RNAalifold is available.
pub fn rnaalifold_available() -> bool {
    Command::new("RNAalifold")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Fold a single sequence using RNAfold.
///
/// The sequence should be ungapped (gaps will be removed).
pub fn fold_sequence(sequence: &str, name: &str) -> Result<FoldResult, RnaFoldError> {
    // Remove gaps from sequence
    let clean_seq: String = sequence
        .chars()
        .filter(|c| c.is_alphabetic())
        .collect();

    if clean_seq.is_empty() {
        return Err(RnaFoldError::ParseError);
    }

    // Create FASTA input
    let fasta = format!(">{name}\n{clean_seq}\n");

    // Run RNAfold
    let mut child = Command::new("RNAfold")
        .arg("--noPS") // Don't generate PostScript
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| RnaFoldError::NotFound)?;

    // Write input
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(fasta.as_bytes())?;
    }

    // Get output
    let output = child.wait_with_output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RnaFoldError::ExecutionFailed(stderr.to_string()));
    }

    // Parse output
    parse_rnafold_output(&String::from_utf8_lossy(&output.stdout))
}

/// Parse RNAfold output.
///
/// Output format:
/// ```text
/// >name
/// SEQUENCE
/// STRUCTURE (MFE)
/// ```
fn parse_rnafold_output(output: &str) -> Result<FoldResult, RnaFoldError> {
    let lines: Vec<&str> = output.lines().collect();

    // Find the structure line (contains dots/brackets and energy)
    for line in lines.iter().rev() {
        let trimmed = line.trim();
        if trimmed.contains('(') && trimmed.contains(')') && trimmed.contains('.') {
            // Try to parse structure and energy
            // Format: "...(((...)))... (-5.60)"
            if let Some((structure_part, energy_part)) = trimmed.rsplit_once(' ') {
                let structure = structure_part.trim().to_string();

                // Parse energy from "(X.XX)" format
                let mfe = energy_part
                    .trim_start_matches('(')
                    .trim_end_matches(')')
                    .parse::<f64>()
                    .ok();

                return Ok(FoldResult { structure, mfe });
            }
            // No energy, just structure
            return Ok(FoldResult {
                structure: trimmed.to_string(),
                mfe: None,
            });
        }
    }

    Err(RnaFoldError::ParseError)
}

/// Fold an alignment using RNAalifold.
///
/// Takes sequences in Stockholm-like format (aligned, with gaps).
pub fn fold_alignment(
    sequences: &[(String, String)], // (id, aligned_sequence)
) -> Result<FoldResult, RnaFoldError> {
    if sequences.is_empty() {
        return Err(RnaFoldError::ParseError);
    }

    // Create ClustalW-like input (RNAalifold default format)
    let mut clustal = String::from("CLUSTAL W\n\n");

    for (id, seq) in sequences {
        // Truncate long IDs
        let short_id = if id.len() > 30 {
            &id[..30]
        } else {
            id.as_str()
        };
        clustal.push_str(&format!("{short_id:<30} {seq}\n"));
    }

    // Run RNAalifold
    let mut child = Command::new("RNAalifold")
        .arg("--noPS")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| RnaFoldError::NotFound)?;

    // Write input
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(clustal.as_bytes())?;
    }

    // Get output
    let output = child.wait_with_output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RnaFoldError::ExecutionFailed(stderr.to_string()));
    }

    // Parse output (similar to RNAfold)
    parse_rnaalifold_output(&String::from_utf8_lossy(&output.stdout))
}

/// Parse RNAalifold output.
fn parse_rnaalifold_output(output: &str) -> Result<FoldResult, RnaFoldError> {
    let lines: Vec<&str> = output.lines().collect();

    // RNAalifold output format:
    // consensus sequence
    // structure (energy = X.XX kcal/mol)
    for line in &lines {
        let trimmed = line.trim();

        // Look for structure line with energy
        if (trimmed.starts_with('.') || trimmed.starts_with('(') || trimmed.starts_with('<'))
            && (trimmed.contains('(') || trimmed.contains('<'))
        {
            // Try to extract structure and energy
            if let Some(paren_pos) = trimmed.rfind(" (") {
                let structure = trimmed[..paren_pos].trim().to_string();
                let energy_str = &trimmed[paren_pos..];

                // Parse energy
                let mfe = energy_str
                    .split_whitespace()
                    .find(|s| s.starts_with('-') || s.parse::<f64>().is_ok())
                    .and_then(|s| s.parse::<f64>().ok());

                return Ok(FoldResult { structure, mfe });
            }
            return Ok(FoldResult {
                structure: trimmed.to_string(),
                mfe: None,
            });
        }
    }

    Err(RnaFoldError::ParseError)
}

/// Expand a gapped structure to match gapped sequence.
///
/// RNAfold returns structure for ungapped sequence, but we need it aligned.
pub fn expand_structure_to_alignment(
    ungapped_structure: &str,
    aligned_sequence: &str,
    gap_chars: &[char],
) -> String {
    let mut result = String::new();
    let mut struct_iter = ungapped_structure.chars();

    for ch in aligned_sequence.chars() {
        if gap_chars.contains(&ch) {
            result.push('.');
        } else if let Some(struct_ch) = struct_iter.next() {
            result.push(struct_ch);
        } else {
            result.push('.');
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rnafold_output() {
        let output = ">test\nACGUACGU\n........ (-0.50)\n";
        let result = parse_rnafold_output(output).unwrap();
        assert_eq!(result.structure, "........");
        assert!((result.mfe.unwrap() - (-0.50)).abs() < 0.01);
    }

    #[test]
    fn test_expand_structure() {
        let structure = "<<>>";
        let sequence = "A..CG..U";
        let gap_chars = ['.', '-'];

        let expanded = expand_structure_to_alignment(structure, sequence, &gap_chars);
        // Structure maps to non-gap positions: A='<', C='<', G='>', U='>'
        assert_eq!(expanded, "<..<>..>");
    }

    #[test]
    fn test_availability() {
        // Just test that the function doesn't panic
        let _ = rnafold_available();
        let _ = rnaalifold_available();
    }
}
