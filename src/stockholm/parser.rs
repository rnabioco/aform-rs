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

/// Accumulates the lines of a single alignment record (between `# STOCKHOLM`
/// headers / `//` terminators) and builds an [`Alignment`] from them.
#[derive(Default)]
struct RecordBuilder {
    alignment: Alignment,
    /// Blocked format: accumulated sequence data across blocks.
    seq_data: HashMap<String, String>,
    seq_order: Vec<String>,
    /// Blocked residue annotations.
    gr_data: HashMap<(String, String), String>,
    gc_data: HashMap<String, String>,
    /// Whether any content line has been seen for this record.
    has_content: bool,
}

impl RecordBuilder {
    fn new() -> Self {
        Self::default()
    }

    /// Returns true if any sequence or annotation line has been processed.
    fn has_content(&self) -> bool {
        self.has_content
    }

    /// Process a single (non-empty, non-terminator) line of a record.
    fn process_line(&mut self, line: &str) {
        // Comment line (not annotation) - includes the `# STOCKHOLM` header.
        if line.starts_with('#') && !line.starts_with("#=") {
            return;
        }

        self.has_content = true;

        // File annotation: #=GF tag value
        if let Some(rest) = line.strip_prefix("#=GF") {
            let parts: Vec<&str> = rest.trim().splitn(2, char::is_whitespace).collect();
            if parts.len() >= 2 {
                self.alignment.file_annotations.push(FileAnnotation {
                    tag: parts[0].to_string(),
                    value: parts[1].trim().to_string(),
                });
            } else if parts.len() == 1 {
                self.alignment.file_annotations.push(FileAnnotation {
                    tag: parts[0].to_string(),
                    value: String::new(),
                });
            }
            return;
        }

        // Sequence annotation: #=GS seqid tag value
        if let Some(rest) = line.strip_prefix("#=GS") {
            let parts: Vec<&str> = rest.trim().splitn(3, char::is_whitespace).collect();
            if parts.len() >= 3 {
                let seqid = parts[0].to_string();
                let tag = parts[1].to_string();
                let value = parts[2].trim().to_string();
                self.alignment
                    .sequence_annotations
                    .entry(seqid)
                    .or_default()
                    .push(SequenceAnnotation { tag, value });
            }
            return;
        }

        // Column annotation: #=GC tag data
        if let Some(rest) = line.strip_prefix("#=GC") {
            let parts: Vec<&str> = rest.trim().splitn(2, char::is_whitespace).collect();
            if parts.len() >= 2 {
                let tag = parts[0].to_string();
                let data = parts[1].trim().to_string();
                self.gc_data
                    .entry(tag)
                    .and_modify(|s| s.push_str(&data))
                    .or_insert(data);
            }
            return;
        }

        // Residue annotation: #=GR seqid tag data
        if let Some(rest) = line.strip_prefix("#=GR") {
            let parts: Vec<&str> = rest.trim().splitn(3, char::is_whitespace).collect();
            if parts.len() >= 3 {
                let seqid = parts[0].to_string();
                let tag = parts[1].to_string();
                let data = parts[2].trim().to_string();
                self.gr_data
                    .entry((seqid, tag))
                    .and_modify(|s| s.push_str(&data))
                    .or_insert(data);
            }
            return;
        }

        // Sequence line: seqid data
        let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
        if parts.len() >= 2 {
            let seqid = parts[0].to_string();
            let data = parts[1].trim().replace(' ', ""); // Remove any internal spaces

            if !self.seq_data.contains_key(&seqid) {
                self.seq_order.push(seqid.clone());
            }
            self.seq_data
                .entry(seqid)
                .and_modify(|s| s.push_str(&data))
                .or_insert(data);
        }
    }

    /// Finalize the accumulated lines into an [`Alignment`].
    fn finish(mut self) -> Result<Alignment, ParseError> {
        // Build sequences in order
        for seqid in std::mem::take(&mut self.seq_order) {
            if let Some(data) = self.seq_data.remove(&seqid) {
                self.alignment
                    .sequences
                    .push(Rc::new(Sequence::new(seqid, data)));
            }
        }

        // Build column annotations
        for (tag, data) in std::mem::take(&mut self.gc_data) {
            self.alignment
                .column_annotations
                .push(ColumnAnnotation { tag, data });
        }

        // Build residue annotations
        for ((seqid, tag), data) in std::mem::take(&mut self.gr_data) {
            self.alignment
                .residue_annotations
                .entry(seqid)
                .or_default()
                .push(ResidueAnnotation { tag, data });
        }

        // Validate lengths
        if !self.alignment.is_valid() {
            return Err(ParseError::InconsistentLengths);
        }

        Ok(self.alignment)
    }
}

/// Parse the first alignment from a Stockholm format reader.
///
/// A Stockholm file may contain several alignments; this returns only the
/// first. Use [`parse_all`] to read every alignment.
pub fn parse<R: Read>(reader: R) -> Result<Alignment, ParseError> {
    let mut alignments = parse_all(reader)?;
    if alignments.is_empty() {
        return Err(ParseError::UnexpectedEof);
    }
    Ok(alignments.remove(0))
}

/// Parse every alignment contained in a Stockholm format reader.
///
/// Stockholm files can concatenate multiple alignments, each introduced by a
/// `# STOCKHOLM` header and terminated by `//`. Returns one [`Alignment`] per
/// record, in file order. Empty records are skipped.
pub fn parse_all<R: Read>(reader: R) -> Result<Vec<Alignment>, ParseError> {
    let buf_reader = BufReader::new(reader);
    let mut lines = buf_reader.lines();

    // Check header
    let header = lines.next().ok_or(ParseError::UnexpectedEof)??;
    if !header.starts_with("# STOCKHOLM") {
        return Err(ParseError::InvalidHeader);
    }

    let mut alignments = Vec::new();
    let mut builder = RecordBuilder::new();

    for line_result in lines {
        let line = line_result?;

        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        // End of an alignment record
        if line.starts_with("//") {
            if builder.has_content() {
                alignments.push(builder.finish()?);
            }
            builder = RecordBuilder::new();
            continue;
        }

        builder.process_line(&line);
    }

    // Handle a trailing record with no closing `//`.
    if builder.has_content() {
        alignments.push(builder.finish()?);
    }

    Ok(alignments)
}

/// Parse a Stockholm alignment from a string.
#[allow(dead_code)] // API convenience function
pub fn parse_str(s: &str) -> Result<Alignment, ParseError> {
    parse(s.as_bytes())
}

/// Open a Stockholm file, transparently decompressing `.gz` files, and pass the
/// reader to `f`.
fn with_reader<T>(
    path: &std::path::Path,
    f: impl FnOnce(Box<dyn Read>) -> Result<T, ParseError>,
) -> Result<T, ParseError> {
    use flate2::read::GzDecoder;

    let file = std::fs::File::open(path)?;

    // Check if file is gzip-compressed by extension
    let is_gzip = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("gz"));

    if is_gzip {
        f(Box::new(GzDecoder::new(file)))
    } else {
        f(Box::new(file))
    }
}

/// Parse the first alignment from a Stockholm file path.
/// Automatically handles gzip-compressed files (.gz extension).
#[allow(dead_code)] // retained for single-alignment callers
pub fn parse_file(path: &std::path::Path) -> Result<Alignment, ParseError> {
    with_reader(path, parse)
}

/// Parse every alignment from a Stockholm file path.
/// Automatically handles gzip-compressed files (.gz extension).
pub fn parse_all_file(path: &std::path::Path) -> Result<Vec<Alignment>, ParseError> {
    with_reader(path, parse_all)
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
        let r2r_label = alignment
            .column_annotations
            .iter()
            .find(|a| a.tag == "R2R_LABEL");
        assert!(r2r_label.is_some());
        assert_eq!(r2r_label.unwrap().data, "...[....]...1...2T...");

        // Check R2R #=GF commands are captured
        let r2r_commands: Vec<_> = alignment
            .file_annotations
            .iter()
            .filter(|a| a.tag == "R2R")
            .collect();
        assert_eq!(r2r_commands.len(), 2);
    }

    const MULTI_ALIGNMENT: &str = r#"# STOCKHOLM 1.0
#=GF ID first
seqA ACGU
seqB ACGU
//
# STOCKHOLM 1.0
#=GF ID second
seqC GGGCCC
seqD GGG..C
#=GC SS_cons <<<>>>
//
# STOCKHOLM 1.0
#=GF ID third
seqE UU
//
"#;

    #[test]
    fn test_parse_all_multiple() {
        let alignments = parse_all(MULTI_ALIGNMENT.as_bytes()).unwrap();
        assert_eq!(alignments.len(), 3);

        assert_eq!(alignments[0].get_file_annotation("ID"), Some("first"));
        assert_eq!(alignments[0].sequences.len(), 2);
        assert_eq!(alignments[0].width(), 4);

        assert_eq!(alignments[1].get_file_annotation("ID"), Some("second"));
        assert_eq!(alignments[1].width(), 6);
        assert_eq!(alignments[1].ss_cons(), Some("<<<>>>"));

        assert_eq!(alignments[2].get_file_annotation("ID"), Some("third"));
        assert_eq!(alignments[2].sequences.len(), 1);
    }

    #[test]
    fn test_parse_returns_first() {
        // The single-alignment `parse` should yield only the first record.
        let alignment = parse_str(MULTI_ALIGNMENT).unwrap();
        assert_eq!(alignment.get_file_annotation("ID"), Some("first"));
    }

    #[test]
    fn test_parse_all_single() {
        let alignments = parse_all(SIMPLE_ALIGNMENT.as_bytes()).unwrap();
        assert_eq!(alignments.len(), 1);
        assert_eq!(alignments[0].sequences.len(), 2);
    }

    #[test]
    fn test_parse_all_skips_empty_records() {
        // A stray `//` or empty header block should not produce empty alignments.
        let input = "# STOCKHOLM 1.0\n//\n# STOCKHOLM 1.0\nseqA ACGU\n//\n";
        let alignments = parse_all(input.as_bytes()).unwrap();
        assert_eq!(alignments.len(), 1);
        assert_eq!(alignments[0].sequences[0].id, "seqA");
    }
}
