//! Undo/redo history.

use crate::stockholm::Alignment;

/// A snapshot of the alignment state for undo/redo.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub alignment: Alignment,
    pub cursor_row: usize,
    pub cursor_col: usize,
}

/// Undo/redo history manager.
#[derive(Debug, Default)]
pub struct History {
    /// Undo stack.
    undo_stack: Vec<Snapshot>,
    /// Redo stack.
    redo_stack: Vec<Snapshot>,
    /// Maximum history size.
    max_size: usize,
}

impl History {
    /// Create a new history with default max size.
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_size: 100,
        }
    }

    /// Create a new history with a specific max size.
    #[allow(dead_code)] // API for configurable history size
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_size,
        }
    }

    /// Save a snapshot before making changes.
    pub fn save(&mut self, alignment: &Alignment, cursor_row: usize, cursor_col: usize) {
        // Clear redo stack when making new changes
        self.redo_stack.clear();

        // Add snapshot to undo stack
        self.undo_stack.push(Snapshot {
            alignment: alignment.clone(),
            cursor_row,
            cursor_col,
        });

        // Trim if exceeds max size
        while self.undo_stack.len() > self.max_size {
            self.undo_stack.remove(0);
        }
    }

    /// Undo the last change, returning the previous state.
    pub fn undo(&mut self, current: &Alignment, cursor_row: usize, cursor_col: usize) -> Option<Snapshot> {
        if let Some(snapshot) = self.undo_stack.pop() {
            // Save current state to redo stack
            self.redo_stack.push(Snapshot {
                alignment: current.clone(),
                cursor_row,
                cursor_col,
            });
            Some(snapshot)
        } else {
            None
        }
    }

    /// Redo the last undone change.
    pub fn redo(&mut self, current: &Alignment, cursor_row: usize, cursor_col: usize) -> Option<Snapshot> {
        if let Some(snapshot) = self.redo_stack.pop() {
            // Save current state to undo stack
            self.undo_stack.push(Snapshot {
                alignment: current.clone(),
                cursor_row,
                cursor_col,
            });
            Some(snapshot)
        } else {
            None
        }
    }

    /// Check if undo is available.
    #[allow(dead_code)] // API for status bar display
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available.
    #[allow(dead_code)] // API for status bar display
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Get the number of undo steps available.
    #[allow(dead_code)] // API for status display
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get the number of redo steps available.
    #[allow(dead_code)] // API for status display
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stockholm::Sequence;
    use std::rc::Rc;

    fn make_alignment(data: &str) -> Alignment {
        let mut alignment = Alignment::new();
        alignment.sequences.push(Rc::new(Sequence::new("seq1", data)));
        alignment
    }

    #[test]
    fn test_undo_redo() {
        let mut history = History::new();

        let state1 = make_alignment("ACGU");
        let state2 = make_alignment("ACGU.");
        let state3 = make_alignment("ACGU..");

        // Save state1
        history.save(&state1, 0, 0);

        // Save state2
        history.save(&state2, 0, 1);

        // Undo to state2
        let snapshot = history.undo(&state3, 0, 2);
        assert!(snapshot.is_some());
        assert_eq!(snapshot.unwrap().alignment.sequences[0].data(), "ACGU.");

        // Undo to state1
        let snapshot = history.undo(&state2, 0, 1);
        assert!(snapshot.is_some());
        assert_eq!(snapshot.unwrap().alignment.sequences[0].data(), "ACGU");

        // Redo to state2
        let snapshot = history.redo(&state1, 0, 0);
        assert!(snapshot.is_some());
        assert_eq!(snapshot.unwrap().alignment.sequences[0].data(), "ACGU.");
    }

    #[test]
    fn test_redo_cleared_on_new_change() {
        let mut history = History::new();

        let state1 = make_alignment("ACGU");
        let state2 = make_alignment("ACGU.");

        history.save(&state1, 0, 0);
        history.undo(&state2, 0, 1);
        assert!(history.can_redo());

        // Make new change
        history.save(&state2, 0, 1);
        assert!(!history.can_redo());
    }
}
