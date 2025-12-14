//! Stockholm format writer.

use std::io::{Result, Write};

use super::types::*;

/// Write a Stockholm format alignment to a writer.
pub fn write<W: Write>(alignment: &Alignment, mut writer: W) -> Result<()> {
    // Header
    writeln!(writer, "# STOCKHOLM 1.0")?;

    // File annotations (#=GF)
    for ann in &alignment.file_annotations {
        writeln!(writer, "#=GF {} {}", ann.tag, ann.value)?;
    }

    if !alignment.file_annotations.is_empty() {
        writeln!(writer)?;
    }

    // Calculate padding for alignment
    let max_id_len = alignment.max_id_len();
    let padding = max_id_len.max(10);

    // Sequence annotations (#=GS) - group by sequence
    for seq in &alignment.sequences {
        if let Some(annotations) = alignment.sequence_annotations.get(&seq.id) {
            for ann in annotations {
                writeln!(writer, "#=GS {:padding$} {} {}", seq.id, ann.tag, ann.value)?;
            }
        }
    }

    if !alignment.sequence_annotations.is_empty() {
        writeln!(writer)?;
    }

    // Sequences and their residue annotations (#=GR)
    for seq in &alignment.sequences {
        writeln!(writer, "{:padding$} {}", seq.id, seq.data())?;

        // Per-residue annotations for this sequence
        if let Some(annotations) = alignment.residue_annotations.get(&seq.id) {
            for ann in annotations {
                writeln!(writer, "#=GR {:padding$} {} {}", seq.id, ann.tag, ann.data)?;
            }
        }
    }

    // Column annotations (#=GC)
    for ann in &alignment.column_annotations {
        writeln!(writer, "#=GC {:padding$} {}", ann.tag, ann.data)?;
    }

    // Terminator
    writeln!(writer, "//")?;

    Ok(())
}

/// Write a Stockholm alignment to a string.
#[allow(dead_code)] // API convenience function
pub fn write_string(alignment: &Alignment) -> Result<String> {
    let mut buffer = Vec::new();
    write(alignment, &mut buffer)?;
    Ok(String::from_utf8_lossy(&buffer).to_string())
}

/// Write a Stockholm alignment to a file.
pub fn write_file(alignment: &Alignment, path: &std::path::Path) -> Result<()> {
    let file = std::fs::File::create(path)?;
    write(alignment, file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stockholm::parser;
    use std::rc::Rc;

    #[test]
    fn test_roundtrip() {
        let input = r#"# STOCKHOLM 1.0
#=GF AC RF00001
#=GF ID 5S_rRNA

seq1/1-10  ACGU..ACGU
seq2/1-10  ACGU..ACGU
#=GC SS_cons   <<<<..>>>>
//
"#;

        let alignment = parser::parse_str(input).unwrap();
        let output = write_string(&alignment).unwrap();

        // Parse the output again
        let reparsed = parser::parse_str(&output).unwrap();

        assert_eq!(alignment.sequences.len(), reparsed.sequences.len());
        assert_eq!(alignment.sequences[0].data(), reparsed.sequences[0].data());
        assert_eq!(alignment.ss_cons(), reparsed.ss_cons());
    }

    #[test]
    fn test_write_simple() {
        let mut alignment = Alignment::new();
        alignment.sequences.push(Rc::new(Sequence::new("seq1", "ACGU")));
        alignment.sequences.push(Rc::new(Sequence::new("seq2", "ACGU")));
        alignment.column_annotations.push(ColumnAnnotation {
            tag: "SS_cons".to_string(),
            data: "<><>".to_string(),
        });

        let output = write_string(&alignment).unwrap();
        assert!(output.contains("# STOCKHOLM 1.0"));
        assert!(output.contains("seq1"));
        assert!(output.contains("ACGU"));
        assert!(output.contains("#=GC SS_cons"));
        assert!(output.contains("//"));
    }
}
