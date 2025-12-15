//! Editor commands for alignment manipulation.

use std::rc::Rc;

use crate::app::App;
use crate::stockholm::ShiftDirection;

impl App {
    /// Insert a gap at the cursor position in the current sequence.
    pub fn insert_gap(&mut self) {
        self.save_undo_state();

        // Translate display row to actual sequence index (for clustering support)
        let actual_row = self.display_to_actual_row(self.cursor_row);

        if let Some(seq_rc) = self.alignment.sequences.get_mut(actual_row) {
            let seq = Rc::make_mut(seq_rc);
            seq.insert_gap(self.cursor_col, self.gap_char);

            // Also update associated #=GR annotations
            if let Some(annotations) = self.alignment.residue_annotations.get_mut(&seq.id) {
                for ann in annotations {
                    if self.cursor_col <= ann.data.len() {
                        ann.data.insert(self.cursor_col, self.gap_char);
                    }
                }
            }
        }

        self.mark_modified();
        self.cursor_right();
    }

    /// Delete a gap at the cursor position in the current sequence.
    pub fn delete_gap(&mut self) -> bool {
        if !self.is_current_gap() {
            self.set_status("Not a gap character");
            return false;
        }

        self.save_undo_state();

        // Translate display row to actual sequence index (for clustering support)
        let actual_row = self.display_to_actual_row(self.cursor_row);

        let seq_id = self
            .alignment
            .sequences
            .get(actual_row)
            .map(|s| s.id.clone());

        if let Some(seq_rc) = self.alignment.sequences.get_mut(actual_row) {
            let seq = Rc::make_mut(seq_rc);
            if seq.delete_gap(self.cursor_col, &self.gap_chars) {
                // Also update associated #=GR annotations
                if let Some(id) = seq_id
                    && let Some(annotations) = self.alignment.residue_annotations.get_mut(&id)
                {
                    for ann in annotations {
                        if self.cursor_col < ann.data.len() {
                            ann.data.remove(self.cursor_col);
                        }
                    }
                }
                self.mark_modified();
                return true;
            }
        }

        false
    }

    /// Insert a gap column at the cursor position.
    pub fn insert_gap_column(&mut self) {
        self.save_undo_state();
        self.alignment
            .insert_gap_column(self.cursor_col, self.gap_char);
        self.mark_modified();
        self.update_structure_cache();
    }

    /// Delete a gap column at the cursor position.
    pub fn delete_gap_column(&mut self) -> bool {
        if self
            .alignment
            .delete_gap_column(self.cursor_col, &self.gap_chars)
        {
            self.save_undo_state();
            self.mark_modified();
            self.clamp_cursor();
            self.update_structure_cache();
            true
        } else {
            self.set_status("Column contains non-gap characters");
            false
        }
    }

    /// Internal shift without undo/status - consolidated implementation.
    fn shift_sequence_internal(&mut self, direction: ShiftDirection) -> bool {
        // Translate display row to actual sequence index (for clustering support)
        let actual_row = self.display_to_actual_row(self.cursor_row);

        let seq_id = self
            .alignment
            .sequences
            .get(actual_row)
            .map(|s| s.id.clone());

        if let Some(seq_rc) = self.alignment.sequences.get_mut(actual_row) {
            let seq = Rc::make_mut(seq_rc);
            if seq.shift(self.cursor_col, direction, &self.gap_chars) {
                // Also shift associated #=GR annotations
                if let Some(id) = seq_id
                    && let Some(annotations) = self.alignment.residue_annotations.get_mut(&id)
                {
                    for ann in annotations {
                        let mut temp = crate::stockholm::Sequence::new("temp", ann.data.clone());
                        temp.shift(self.cursor_col, direction, &self.gap_chars);
                        ann.data = temp.data();
                    }
                }
                return true;
            }
        }
        false
    }

    /// Shift current sequence in the given direction with undo support.
    fn shift_sequence_with_undo(&mut self, direction: ShiftDirection) -> bool {
        self.save_undo_state();
        if self.shift_sequence_internal(direction) {
            self.mark_modified();
            true
        } else {
            let dir_str = match direction {
                ShiftDirection::Left => "left",
                ShiftDirection::Right => "right",
            };
            self.set_status(format!("Cannot shift {dir_str} (no gap found)"));
            false
        }
    }

    /// Shift current sequence left.
    pub fn shift_sequence_left(&mut self) -> bool {
        self.shift_sequence_with_undo(ShiftDirection::Left)
    }

    /// Shift current sequence right.
    pub fn shift_sequence_right(&mut self) -> bool {
        self.shift_sequence_with_undo(ShiftDirection::Right)
    }

    /// Throw sequence in the given direction (shift as far as possible).
    fn throw_sequence(&mut self, direction: ShiftDirection) {
        self.save_undo_state();
        let mut shifted = false;
        while self.shift_sequence_internal(direction) {
            shifted = true;
        }
        if shifted {
            self.mark_modified();
        } else {
            let dir_str = match direction {
                ShiftDirection::Left => "left",
                ShiftDirection::Right => "right",
            };
            self.set_status(format!("Cannot throw {dir_str} (no gaps found)"));
        }
    }

    /// Throw sequence left (shift as far as possible).
    pub fn throw_sequence_left(&mut self) {
        self.throw_sequence(ShiftDirection::Left);
    }

    /// Throw sequence right (shift as far as possible).
    pub fn throw_sequence_right(&mut self) {
        self.throw_sequence(ShiftDirection::Right);
    }

    /// Undo the last action.
    pub fn undo(&mut self) {
        if let Some(snapshot) = self
            .history
            .undo(&self.alignment, self.cursor_row, self.cursor_col)
        {
            self.alignment = snapshot.alignment;
            self.cursor_row = snapshot.cursor_row;
            self.cursor_col = snapshot.cursor_col;
            self.modified = true; // Still modified from original save
            self.update_structure_cache();
            self.set_status("Undo");
        } else {
            self.set_status("Nothing to undo");
        }
    }

    /// Redo the last undone action.
    pub fn redo(&mut self) {
        if let Some(snapshot) = self
            .history
            .redo(&self.alignment, self.cursor_row, self.cursor_col)
        {
            self.alignment = snapshot.alignment;
            self.cursor_row = snapshot.cursor_row;
            self.cursor_col = snapshot.cursor_col;
            self.modified = true;
            self.update_structure_cache();
            self.set_status("Redo");
        } else {
            self.set_status("Nothing to redo");
        }
    }

    /// Save current state for undo.
    fn save_undo_state(&mut self) {
        self.history
            .save(&self.alignment, self.cursor_row, self.cursor_col);
    }

    /// Delete the current sequence.
    pub fn delete_sequence(&mut self) {
        if self.alignment.sequences.is_empty() {
            return;
        }

        self.save_undo_state();

        // Translate display row to actual sequence index (for clustering support)
        let actual_row = self.display_to_actual_row(self.cursor_row);

        let seq_id = self.alignment.sequences[actual_row].id.clone();
        self.alignment.sequences.remove(actual_row);

        // Remove associated annotations
        self.alignment.sequence_annotations.remove(&seq_id);
        self.alignment.residue_annotations.remove(&seq_id);

        self.mark_modified();
        self.clamp_cursor();

        // Recompute clustering if active (indices become stale after deletion)
        if self.cluster_order.is_some() {
            self.precompute_collapse_groups(); // Refresh group indices first
            self.cluster_sequences();
        }
    }

    /// Convert alignment to uppercase.
    pub fn uppercase_alignment(&mut self) {
        self.save_undo_state();
        for seq in &mut self.alignment.sequences {
            Rc::make_mut(seq).make_uppercase();
        }
        self.mark_modified();
    }

    /// Convert alignment to lowercase.
    pub fn lowercase_alignment(&mut self) {
        self.save_undo_state();
        for seq in &mut self.alignment.sequences {
            Rc::make_mut(seq).make_lowercase();
        }
        self.mark_modified();
    }

    /// Convert T to U in all sequences.
    pub fn convert_t_to_u(&mut self) {
        self.save_undo_state();
        for seq in &mut self.alignment.sequences {
            let seq = Rc::make_mut(seq);
            seq.replace_char('T', 'U');
            seq.replace_char('t', 'u');
        }
        self.mark_modified();
    }

    /// Convert U to T in all sequences.
    pub fn convert_u_to_t(&mut self) {
        self.save_undo_state();
        for seq in &mut self.alignment.sequences {
            let seq = Rc::make_mut(seq);
            seq.replace_char('U', 'T');
            seq.replace_char('u', 't');
        }
        self.mark_modified();
    }

    /// Trim leading gap-only columns from the alignment.
    pub fn trim_left(&mut self) {
        self.save_undo_state();
        let removed = self.alignment.trim_left(&self.gap_chars);
        if removed > 0 {
            self.mark_modified();
            self.clamp_cursor();
            self.update_structure_cache();
            self.set_status(format!("Trimmed {removed} columns from left"));
        } else {
            self.set_status("No gap-only columns on left");
        }
    }

    /// Trim trailing gap-only columns from the alignment.
    pub fn trim_right(&mut self) {
        self.save_undo_state();
        let removed = self.alignment.trim_right(&self.gap_chars);
        if removed > 0 {
            self.mark_modified();
            self.clamp_cursor();
            self.update_structure_cache();
            self.set_status(format!("Trimmed {removed} columns from right"));
        } else {
            self.set_status("No gap-only columns on right");
        }
    }

    /// Trim both leading and trailing gap-only columns.
    pub fn trim(&mut self) {
        self.save_undo_state();
        let left = self.alignment.trim_left(&self.gap_chars);
        let right = self.alignment.trim_right(&self.gap_chars);
        let total = left + right;
        if total > 0 {
            self.mark_modified();
            self.clamp_cursor();
            self.update_structure_cache();
            self.set_status(format!(
                "Trimmed {total} columns ({left} left, {right} right)"
            ));
        } else {
            self.set_status("No gap-only columns to trim");
        }
    }
}
