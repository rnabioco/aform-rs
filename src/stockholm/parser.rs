//! Stockholm format parser.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::rc::Rc;
use thiserror::Error;

use super::types::*;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Invalid Stockholm header")]
    InvalidHeader,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Unexpected end of file")]
    UnexpectedEof,
    #[allow(dead_code)] // Error variant for detailed error messages
    #[error("Invalid line format: {0}")]
    InvalidLine(String),
    #[error("Inconsistent sequence lengths")]
    InconsistentLengths,
}

/// Parse a Stockholm format alignment from a reader.
pub fn parse<R: Read>(reader: R) -> Result<Alignment, ParseError> {
    let buf_reader = BufReader::new(reader);
    let mut lines = buf_reader.lines();

    // Check header
    let header = lines.next().ok_or(ParseError::UnexpectedEof)??;
    if !header.starts_with("# STOCKHOLM") {
        return Err(ParseError::InvalidHeader);
    }

    let mut alignment = Alignment::new();

    // For blocked format: accumulate sequence data across blocks
    let mut seq_data: HashMap<String, String> = HashMap::new();
    let mut seq_order: Vec<String> = Vec::new();

    // For blocked residue annotations
    let mut gr_data: HashMap<(String, String), String> = HashMap::new();
    let mut gc_data: HashMap<String, String> = HashMap::new();

    for line_result in lines {
        let line = line_result?;

        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        // End of alignment
        if line.starts_with("//") {
            break;
        }

        // Comment line (not annotation)
        if line.starts_with('#') && !line.starts_with("#=") {
            continue;
        }

        // File annotation: #=GF tag value
        if line.starts_with("#=GF") {
            if let Some(rest) = line.strip_prefix("#=GF") {
                let parts: Vec<&str> = rest.trim().splitn(2, char::is_whitespace).collect();
                if parts.len() >= 2 {
                    alignment.file_annotations.push(FileAnnotation {
                        tag: parts[0].to_string(),
                        value: parts[1].trim().to_string(),
                    });
                } else if parts.len() == 1 {
                    alignment.file_annotations.push(FileAnnotation {
                        tag: parts[0].to_string(),
                        value: String::new(),
                    });
                }
            }
            continue;
        }

        // Sequence annotation: #=GS seqid tag value
        if line.starts_with("#=GS") {
            if let Some(rest) = line.strip_prefix("#=GS") {
                let parts: Vec<&str> = rest.trim().splitn(3, char::is_whitespace).collect();
                if parts.len() >= 3 {
                    let seqid = parts[0].to_string();
                    let tag = parts[1].to_string();
                    let value = parts[2].trim().to_string();
                    alignment
                        .sequence_annotations
                        .entry(seqid)
                        .or_default()
                        .push(SequenceAnnotation { tag, value });
                }
            }
            continue;
        }

        // Column annotation: #=GC tag data
        if line.starts_with("#=GC") {
            if let Some(rest) = line.strip_prefix("#=GC") {
                let parts: Vec<&str> = rest.trim().splitn(2, char::is_whitespace).collect();
                if parts.len() >= 2 {
                    let tag = parts[0].to_string();
                    let data = parts[1].trim().to_string();
                    // Accumulate for blocked format
                    gc_data
                        .entry(tag)
                        .and_modify(|s| s.push_str(&data))
                        .or_insert(data);
                }
            }
            continue;
        }

        // Residue annotation: #=GR seqid tag data
        if line.starts_with("#=GR") {
            if let Some(rest) = line.strip_prefix("#=GR") {
                let parts: Vec<&str> = rest.trim().splitn(3, char::is_whitespace).collect();
                if parts.len() >= 3 {
                    let seqid = parts[0].to_string();
                    let tag = parts[1].to_string();
                    let data = parts[2].trim().to_string();
                    // Accumulate for blocked format
                    gr_data
                        .entry((seqid, tag))
                        .and_modify(|s| s.push_str(&data))
                        .or_insert(data);
                }
            }
            continue;
        }

        // Sequence line: seqid data
        let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
        if parts.len() >= 2 {
            let seqid = parts[0].to_string();
            let data = parts[1].trim().replace(' ', ""); // Remove any internal spaces

            if !seq_data.contains_key(&seqid) {
                seq_order.push(seqid.clone());
            }
            seq_data
                .entry(seqid)
                .and_modify(|s| s.push_str(&data))
                .or_insert(data);
        }
    }

    // Build sequences in order
    for seqid in seq_order {
        if let Some(data) = seq_data.remove(&seqid) {
            alignment.sequences.push(Rc::new(Sequence::new(seqid, data)));
        }
    }

    // Build column annotations
    for (tag, data) in gc_data {
        alignment.column_annotations.push(ColumnAnnotation { tag, data });
    }

    // Build residue annotations
    for ((seqid, tag), data) in gr_data {
        alignment
            .residue_annotations
            .entry(seqid)
            .or_default()
            .push(ResidueAnnotation { tag, data });
    }

    // Validate lengths
    if !alignment.is_valid() {
        return Err(ParseError::InconsistentLengths);
    }

    Ok(alignment)
}

/// Parse a Stockholm alignment from a string.
#[allow(dead_code)] // API convenience function
pub fn parse_str(s: &str) -> Result<Alignment, ParseError> {
    parse(s.as_bytes())
}

/// Parse a Stockholm alignment from a file path.
pub fn parse_file(path: &std::path::Path) -> Result<Alignment, ParseError> {
    let file = std::fs::File::open(path)?;
    parse(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_ALIGNMENT: &str = r#"# STOCKHOLM 1.0
#=GF AC RF00001
#=GF ID 5S_rRNA
seq1/1-10    ACGU..ACGU
seq2/1-10    ACGU..ACGU
#=GC SS_cons <<<<..>>>>
//
"#;

    #[test]
    fn test_parse_simple() {
        let alignment = parse_str(SIMPLE_ALIGNMENT).unwrap();
        assert_eq!(alignment.sequences.len(), 2);
        assert_eq!(alignment.sequences[0].id, "seq1/1-10");
        assert_eq!(alignment.sequences[0].data(), "ACGU..ACGU");
        assert_eq!(alignment.width(), 10);
        assert_eq!(alignment.ss_cons(), Some("<<<<..>>>>"));
    }

    #[test]
    fn test_parse_file_annotations() {
        let alignment = parse_str(SIMPLE_ALIGNMENT).unwrap();
        assert_eq!(alignment.file_annotations.len(), 2);
        assert_eq!(alignment.file_annotations[0].tag, "AC");
        assert_eq!(alignment.file_annotations[0].value, "RF00001");
    }

    const BLOCKED_ALIGNMENT: &str = r#"# STOCKHOLM 1.0
seq1    ACGU
seq2    ACGU
#=GC SS_cons <<>>

seq1    WXYZ
seq2    WXYZ
#=GC SS_cons <<>>
//
"#;

    #[test]
    fn test_parse_blocked() {
        let alignment = parse_str(BLOCKED_ALIGNMENT).unwrap();
        assert_eq!(alignment.sequences.len(), 2);
        assert_eq!(alignment.sequences[0].data(), "ACGUWXYZ");
        assert_eq!(alignment.width(), 8);
    }

    #[test]
    fn test_invalid_header() {
        let result = parse_str("not a stockholm file\n//\n");
        assert!(matches!(result, Err(ParseError::InvalidHeader)));
    }

    const R2R_ALIGNMENT: &str = r#"# STOCKHOLM 1.0
#=GF R2R var_hairpin [ ]
#=GF R2R var_backbone_range 1 2
martian        CAGGGAAACCUGAUUUUAGGA
venusian       CGU.UUCG.ACGUA...AGGA
#=GC SS_cons   <<<<....>>>>.........
#=GC R2R_LABEL ...[....]...1...2T...
//
"#;

    #[test]
    fn test_parse_r2r() {
        let alignment = parse_str(R2R_ALIGNMENT).unwrap();
        assert_eq!(alignment.sequences.len(), 2);
        assert_eq!(alignment.sequences[0].id, "martian");

        // Check SS_cons is parsed
        assert_eq!(alignment.ss_cons(), Some("<<<<....>>>>........."));

        // Check R2R_LABEL is captured as a column annotation
        let r2r_label = alignment.column_annotations.iter()
            .find(|a| a.tag == "R2R_LABEL");
        assert!(r2r_label.is_some());
        assert_eq!(r2r_label.unwrap().data, "...[....]...1...2T...");

        // Check R2R #=GF commands are captured
        let r2r_commands: Vec<_> = alignment.file_annotations.iter()
            .filter(|a| a.tag == "R2R")
            .collect();
        assert_eq!(r2r_commands.len(), 2);
    }
}
