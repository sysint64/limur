use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use cosmic_text::Edit;

#[derive(Default, Clone, PartialEq)]
pub struct TextEditHistoryManager {
    pub(crate) entries: VecDeque<TextEditDelta>,
    pub(crate) cursor: usize,
    pub(crate) max_entries: usize,
    pub(crate) last_insert_time: Option<Instant>,
    pub(crate) coalesce_threshold: Duration,
    pub(crate) coalesce_enabled: bool,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum TextDeletionDirection {
    Forward,  // Delete
    Backward, // Backspace
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextEditDelta {
    Insert {
        cursor_before: cosmic_text::Cursor,
        cursor_after: cosmic_text::Cursor,
        text: String,
    },
    Delete {
        start: cosmic_text::Cursor,
        end: cosmic_text::Cursor,
        deleted_text: String,
        direction: TextDeletionDirection,
    },
    Replace {
        range_before: (cosmic_text::Cursor, cosmic_text::Cursor),
        range_after: (cosmic_text::Cursor, cosmic_text::Cursor),
        text_before: String,
        text_after: String,
    },
}

// fn cursor_to_point(cursor: &cosmic_text::Cursor) -> tree_sitter::Point {
//     tree_sitter::Point {
//         row: cursor.line,
//         column: cursor.index,
//     }
// }

// fn cursor_to_byte(cursor: &cosmic_text::Cursor, editor: &cosmic_text::Editor) -> usize {
//     editor.with_buffer(|buffer| {
//         let mut byte_offset = 0;

//         // Add bytes from all previous lines (including newlines)
//         for line_idx in 0..cursor.line {
//             if let Some(line) = buffer.lines.get(line_idx) {
//                 byte_offset += line.text().len() + 1; // +1 for '\n'
//             }
//         }

//         // Add bytes within current line up to cursor position
//         if let Some(line) = buffer.lines.get(cursor.line) {
//             let text = line.text();
//             let index = cursor.index.min(text.len());
//             byte_offset += text[..index].len();
//         }

//         byte_offset
//     })
// }

impl TextEditDelta {
    pub fn apply_to_buffer(&self, buffer: &mut cosmic_text::Buffer) {
        let mut editor = cosmic_text::Editor::new(buffer);
        self.apply(&mut editor);
    }

    pub fn undo_to_buffer(&self, buffer: &mut cosmic_text::Buffer) {
        let mut editor = cosmic_text::Editor::new(buffer);
        self.undo(&mut editor);
    }

    pub fn apply(&self, editor: &mut cosmic_text::Editor) {
        editor.set_selection(cosmic_text::Selection::None);

        match self {
            TextEditDelta::Insert {
                cursor_before,
                cursor_after,
                text,
                ..
            } => {
                editor.set_cursor(*cursor_before);
                editor.insert_string(text, None);

                debug_assert!(*cursor_after == editor.cursor());
            }
            TextEditDelta::Delete { start, end, .. } => {
                let (start, end) = normalize_range(*start, *end);

                editor.delete_range(start, end);
                editor.set_cursor(start);
            }
            TextEditDelta::Replace {
                range_before: (start, end),
                text_after,
                ..
            } => {
                let (start, end) = normalize_range(*start, *end);

                editor.delete_range(start, end);
                editor.set_cursor(start);
                editor.insert_string(text_after, None);
            }
        }
    }

    pub fn undo(&self, editor: &mut cosmic_text::Editor) {
        editor.set_selection(cosmic_text::Selection::None);

        match self {
            TextEditDelta::Insert {
                cursor_before,
                cursor_after,
                ..
            } => {
                editor.delete_range(*cursor_before, *cursor_after);
                editor.set_cursor(*cursor_before);
            }
            TextEditDelta::Delete {
                start,
                end,
                deleted_text,
                direction,
                ..
            } => {
                let (start, end) = if start > end {
                    (end, start)
                } else {
                    (start, end)
                };

                editor.set_cursor(*start);
                editor.insert_string(deleted_text, None);

                match direction {
                    TextDeletionDirection::Forward => editor.set_cursor(*start),
                    TextDeletionDirection::Backward => editor.set_cursor(*end),
                }
            }
            TextEditDelta::Replace {
                range_after: (start, end),
                text_before,
                ..
            } => {
                let (start, end) = if start > end {
                    (end, start)
                } else {
                    (start, end)
                };

                // Deletes the current (replaced) text
                editor.delete_range(*start, *end);
                editor.set_cursor(*start);
                editor.insert_string(text_before, None);
            }
        }
    }

    // pub fn to_input_edit(&self, editor: &cosmic_text::Editor) -> tree_sitter::InputEdit {
    //     match self {
    //         TextEditDelta::Insert {
    //             cursor_before,
    //             cursor_after,
    //             text,
    //         } => {
    //             let start_byte = cursor_to_byte(cursor_before, editor);
    //             let new_end_byte = start_byte + text.len();

    //             tree_sitter::InputEdit {
    //                 start_byte,
    //                 old_end_byte: start_byte, // Zero-width before insert
    //                 new_end_byte,
    //                 start_position: cursor_to_point(cursor_before),
    //                 old_end_position: cursor_to_point(cursor_before),
    //                 new_end_position: cursor_to_point(cursor_after),
    //             }
    //         }

    //         TextEditDelta::Delete {
    //             start,
    //             end,
    //             deleted_text,
    //             ..
    //         } => {
    //             let (start, end) = normalize_range(*start, *end);
    //             let start_byte = cursor_to_byte(&start, editor);
    //             let old_end_byte = start_byte + deleted_text.len();

    //             tree_sitter::InputEdit {
    //                 start_byte,
    //                 old_end_byte,
    //                 new_end_byte: start_byte, // Zero-width after delete
    //                 start_position: cursor_to_point(&start),
    //                 old_end_position: cursor_to_point(&end),
    //                 new_end_position: cursor_to_point(&start),
    //             }
    //         }

    //         TextEditDelta::Replace {
    //             range_before: (start, end),
    //             range_after: (_, end_after),
    //             text_before,
    //             text_after,
    //         } => {
    //             let (start, end) = normalize_range(*start, *end);
    //             let start_byte = cursor_to_byte(&start, editor);
    //             let old_end_byte = start_byte + text_before.len();
    //             let new_end_byte = start_byte + text_after.len();

    //             tree_sitter::InputEdit {
    //                 start_byte,
    //                 old_end_byte,
    //                 new_end_byte,
    //                 start_position: cursor_to_point(&start),
    //                 old_end_position: cursor_to_point(&end),
    //                 new_end_position: cursor_to_point(end_after),
    //             }
    //         }
    //     }
    // }

    // pub fn to_undo_input_edit(&self, editor: &cosmic_text::Editor) -> tree_sitter::InputEdit {
    //     match self {
    //         TextEditDelta::Insert {
    //             cursor_before,
    //             cursor_after,
    //             text,
    //         } => {
    //             // Undo insert = delete the inserted text
    //             let start_byte = cursor_to_byte(cursor_before, editor);
    //             let old_end_byte = start_byte + text.len();

    //             tree_sitter::InputEdit {
    //                 start_byte,
    //                 old_end_byte,
    //                 new_end_byte: start_byte, // Delete makes it zero-width
    //                 start_position: cursor_to_point(cursor_before),
    //                 old_end_position: cursor_to_point(cursor_after),
    //                 new_end_position: cursor_to_point(cursor_before),
    //             }
    //         }

    //         TextEditDelta::Delete {
    //             start,
    //             end,
    //             deleted_text,
    //             ..
    //         } => {
    //             // Undo delete = insert the deleted text back
    //             let (start, _) = normalize_range(*start, *end);
    //             let start_byte = cursor_to_byte(&start, editor);
    //             let new_end_byte = start_byte + deleted_text.len();

    //             tree_sitter::InputEdit {
    //                 start_byte,
    //                 old_end_byte: start_byte, // Was zero-width
    //                 new_end_byte,
    //                 start_position: cursor_to_point(&start),
    //                 old_end_position: cursor_to_point(&start),
    //                 new_end_position: cursor_to_point(end), // Back to original end
    //             }
    //         }

    //         TextEditDelta::Replace {
    //             range_after: (start_after, end_after),
    //             range_before: (_, end_before),
    //             text_before,
    //             text_after,
    //         } => {
    //             // Undo replace = replace back with original text
    //             let (start_after, _) = normalize_range(*start_after, *end_after);
    //             let start_byte = cursor_to_byte(&start_after, editor);
    //             let old_end_byte = start_byte + text_after.len();
    //             let new_end_byte = start_byte + text_before.len();

    //             tree_sitter::InputEdit {
    //                 start_byte,
    //                 old_end_byte,
    //                 new_end_byte,
    //                 start_position: cursor_to_point(&start_after),
    //                 old_end_position: cursor_to_point(end_after),
    //                 new_end_position: cursor_to_point(end_before),
    //             }
    //         }
    //     }
    // }

    fn try_coalesce(&mut self, delta: &TextEditDelta) -> bool {
        match self {
            TextEditDelta::Insert {
                cursor_before,
                cursor_after,
                text,
                ..
            } => match delta {
                TextEditDelta::Insert {
                    cursor_before: next_cursor_before,
                    cursor_after: next_cursor_after,
                    text: next_text,
                    ..
                } => {
                    if *cursor_after == *next_cursor_before {
                        *self = TextEditDelta::Insert {
                            cursor_before: *cursor_before,
                            cursor_after: *next_cursor_after,
                            text: text.to_owned() + next_text,
                        };

                        true
                    } else {
                        false
                    }
                }
                _ => false,
            },
            TextEditDelta::Delete {
                start,
                end,
                deleted_text,
                direction,
            } => match delta {
                TextEditDelta::Delete {
                    start: next_start,
                    end: next_end,
                    deleted_text: next_deleted_text,
                    direction: next_direction,
                } => {
                    let (start, end) = normalize_range(*start, *end);
                    let (next_start, next_end) = normalize_range(*next_start, *next_end);

                    if start.line == next_start.line
                        && end.line == next_end.line
                        && start.line == end.line
                    {
                        if direction == next_direction {
                            match direction {
                                TextDeletionDirection::Forward => {
                                    if start.index == next_start.index {
                                        *self = TextEditDelta::Delete {
                                            start,
                                            end: cosmic_text::Cursor::new_with_affinity(
                                                start.line,
                                                start.index
                                                    + (deleted_text.len()
                                                        + next_deleted_text.len()),
                                                next_end.affinity,
                                            ),
                                            deleted_text: deleted_text.clone() + next_deleted_text,
                                            direction: *direction,
                                        };

                                        true
                                    } else {
                                        false
                                    }
                                }
                                TextDeletionDirection::Backward => {
                                    if start.index == next_end.index {
                                        *self = TextEditDelta::Delete {
                                            start: next_start,
                                            end,
                                            deleted_text: next_deleted_text.clone() + deleted_text,
                                            direction: *next_direction,
                                        };

                                        true
                                    } else {
                                        false
                                    }
                                }
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                _ => false,
            },
            // No coalesce for replace
            TextEditDelta::Replace { .. } => false,
        }
    }
}

#[inline]
fn normalize_range(
    start: cosmic_text::Cursor,
    end: cosmic_text::Cursor,
) -> (cosmic_text::Cursor, cosmic_text::Cursor) {
    if start > end {
        (end, start)
    } else {
        (start, end)
    }
}

impl TextEditHistoryManager {
    pub fn new(max_entries: usize, coalesce_enabled: bool) -> Self {
        Self {
            entries: VecDeque::new(),
            cursor: 0,
            max_entries,
            last_insert_time: None,
            coalesce_threshold: Duration::from_millis(300),
            coalesce_enabled,
        }
    }

    pub fn push(&mut self, delta: TextEditDelta) {
        // If cursor is not at the end of history, remove all history after cursor
        if self.cursor != self.entries.len() {
            self.entries.truncate(self.cursor);
        }

        // If this is a character insert that happened quickly after the last one,
        // try to coalesce with the previous delta
        if let Some(last_time) = self.last_insert_time
            && last_time.elapsed() < self.coalesce_threshold
            && self.coalesce_enabled
            && let Some(last_delta) = self.entries.back_mut()
            && last_delta.try_coalesce(&delta)
        {
            self.last_insert_time = Some(Instant::now());
            return; // Successfully coalesced
        }

        if self.coalesce_enabled {
            match delta {
                TextEditDelta::Insert { .. } => {
                    self.last_insert_time = Some(Instant::now());
                }
                TextEditDelta::Delete { .. } => {
                    self.last_insert_time = Some(Instant::now());
                }
                TextEditDelta::Replace { .. } => {
                    // No coalesce for replace
                    self.last_insert_time = None;
                }
            }
        }

        self.entries.push_back(delta);

        while self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }

        self.cursor = self.entries.len();
    }

    pub fn clear(&mut self) {
        self.cursor = 0;
        self.entries.clear();
    }

    pub fn undo(&mut self, editor: &mut cosmic_text::Editor) -> Option<&TextEditDelta> {
        if self.cursor == 0 {
            return None;
        }

        self.cursor -= 1;

        let delta = &self.entries[self.cursor];
        delta.undo(editor);

        Some(delta)
    }

    pub fn redo(&mut self, editor: &mut cosmic_text::Editor) -> Option<&TextEditDelta> {
        if self.cursor >= self.entries.len() {
            return None;
        }

        let delta = &self.entries[self.cursor];
        delta.apply(editor);

        self.cursor += 1;

        Some(delta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmic_text::{Cursor, Editor, FontSystem};

    fn get_editor_text(editor: &Editor) -> String {
        let mut full_text = String::new();
        let cursor = editor.cursor();

        editor.with_buffer(|buffer| {
            for (i, line) in buffer.lines.iter().enumerate() {
                if cursor.line == i {
                    let mut bytes = vec![];

                    for (i, byte) in line.text().bytes().enumerate() {
                        if cursor.index == i {
                            bytes.push(b'|');
                        }

                        bytes.push(byte);
                    }

                    if cursor.index == line.text().len() {
                        bytes.push(b'|');
                    }

                    full_text.push_str(std::str::from_utf8(&bytes).unwrap());
                } else {
                    full_text.push_str(line.text());
                }
                full_text.push('\n');
            }
        });

        if full_text.ends_with('\n') {
            full_text.pop();
        }

        full_text
    }

    // Helper function to create a test editor with initial text
    fn create_editor_with_text(text: &str) -> Editor<'_> {
        let mut font_system = FontSystem::new();
        let mut editor = Editor::new(cosmic_text::Buffer::new(
            &mut font_system,
            cosmic_text::Metrics::new(14.0, 16.0),
        ));
        if !text.is_empty() {
            editor.insert_string(text, None);
            editor.set_cursor(Cursor::new(0, 0)); // Reset cursor to start
        }
        editor
    }

    #[test]
    fn test_basic_push_and_undo_redo() {
        // Start with editor in the final state after all operations
        let mut editor = create_editor_with_text("hello world");
        let mut history = TextEditHistoryManager::new(10, false);

        // Create and push an insert delta
        let delta = TextEditDelta::Insert {
            cursor_before: Cursor::new(0, 5),
            cursor_after: Cursor::new(0, 11),
            text: " world".to_string(),
        };

        history.push(delta);

        // Initially, cursor should be at the end (1 entry)
        assert_eq!(history.cursor, 1);
        assert_eq!(history.entries.len(), 1);

        // Redo should do nothing as we at the end of the history
        history.redo(&mut editor);
        assert_eq!(get_editor_text(&editor), "|hello world");
        assert_eq!(history.cursor, 1); // Should stay at 1 after redo

        // Undo should revert the change
        history.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "hello|");
        assert_eq!(history.cursor, 0);

        // Redo again should reapply
        history.redo(&mut editor);
        assert_eq!(get_editor_text(&editor), "hello world|");
        assert_eq!(history.cursor, 1);
    }

    #[test]
    fn test_multiple_operations() {
        // Start with editor in the final state after all operations
        let mut editor = create_editor_with_text(" case");
        let mut history = TextEditHistoryManager::new(10, false);

        // First operation: Insert " case"
        let insert_delta = TextEditDelta::Insert {
            cursor_before: Cursor::new(0, 4),
            cursor_after: Cursor::new(0, 9),
            text: " case".to_string(),
        };
        history.push(insert_delta);

        // Second operation: Delete "test"
        let delete_delta = TextEditDelta::Delete {
            start: Cursor::new(0, 0),
            end: Cursor::new(0, 4),
            deleted_text: "test".to_string(),
            direction: TextDeletionDirection::Backward,
        };
        history.push(delete_delta);

        assert_eq!(history.entries.len(), 2);
        assert_eq!(history.cursor, 2);

        // Undo both operations step by step
        history.undo(&mut editor); // Undo delete operation
        assert_eq!(get_editor_text(&editor), "test| case");
        assert_eq!(history.cursor, 1);

        history.undo(&mut editor); // Undo insert operation
        assert_eq!(get_editor_text(&editor), "test|");
        assert_eq!(history.cursor, 0);

        // Redo both operations
        history.redo(&mut editor); // Redo insert
        assert_eq!(get_editor_text(&editor), "test case|");
        assert_eq!(history.cursor, 1);

        history.redo(&mut editor); // Redo delete
        assert_eq!(get_editor_text(&editor), "| case");
        assert_eq!(history.cursor, 2);
    }

    #[test]
    fn test_history_truncation_on_new_operation() {
        // Start with editor in the final state after all operations
        let mut editor = create_editor_with_text("start 1 2 3");
        let mut history = TextEditHistoryManager::new(10, false);

        // Add three operations that represent how we got to this state
        history.push(TextEditDelta::Insert {
            cursor_before: Cursor::new(0, 5),
            cursor_after: Cursor::new(0, 7),
            text: " 1".to_string(),
        });

        history.push(TextEditDelta::Insert {
            cursor_before: Cursor::new(0, 7),
            cursor_after: Cursor::new(0, 9),
            text: " 2".to_string(),
        });

        history.push(TextEditDelta::Insert {
            cursor_before: Cursor::new(0, 9),
            cursor_after: Cursor::new(0, 11),
            text: " 3".to_string(),
        });

        assert_eq!(history.entries.len(), 3);
        assert_eq!(history.cursor, 3);

        // Undo twice to go back to "start 1"
        history.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "start 1 2|");
        history.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "start 1|");
        assert_eq!(history.cursor, 1);

        // Add a new operation - this should truncate history after cursor
        history.push(TextEditDelta::Insert {
            cursor_before: Cursor::new(0, 7),
            cursor_after: Cursor::new(0, 9),
            text: " X".to_string(),
        });

        let mut editor = create_editor_with_text("start 1 X");

        // History should now only have 2 entries (original + new one)
        assert_eq!(history.entries.len(), 2);
        assert_eq!(history.cursor, 2);

        // Verify we can't redo the old operations (cursor is at the end)
        history.redo(&mut editor);
        assert_eq!(get_editor_text(&editor), "|start 1 X"); // No change
        assert_eq!(history.cursor, 2);

        history.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "start 1|");
        assert_eq!(history.cursor, 1);

        history.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "start|");
        assert_eq!(history.cursor, 0);

        history.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "start|"); // No change
        assert_eq!(history.cursor, 0);
    }

    #[test]
    fn test_max_entries_limit() {
        let mut editor = create_editor_with_text("test 0 1 2 3 4");
        let mut history = TextEditHistoryManager::new(3, false); // Small limit

        // Add more operations than the limit
        for i in 0..5 {
            history.push(TextEditDelta::Insert {
                cursor_before: Cursor::new(0, 4 + i * 2),
                cursor_after: Cursor::new(0, 4 + i * 2 + 2),
                text: format!(" {}", i),
            });
        }

        // Should only keep the last 3 operations
        assert_eq!(history.entries.len(), 3);
        assert_eq!(history.cursor, 3);

        // Should be able to undo 3 times
        history.undo(&mut editor);
        history.undo(&mut editor);
        history.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "test 0 1|");
        assert_eq!(history.cursor, 0);

        // Fourth undo should have no effect
        history.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "test 0 1|");
        assert_eq!(history.cursor, 0);
    }

    #[test]
    fn test_clear_history() {
        let mut editor = create_editor_with_text(" case");
        let mut history = TextEditHistoryManager::new(10, false);

        // Add some operations
        history.push(TextEditDelta::Insert {
            cursor_before: Cursor::new(0, 4),
            cursor_after: Cursor::new(0, 9),
            text: " case".to_string(),
        });

        history.push(TextEditDelta::Delete {
            start: Cursor::new(0, 0),
            end: Cursor::new(0, 4),
            deleted_text: "test".to_string(),
            direction: TextDeletionDirection::Backward,
        });

        assert_eq!(history.entries.len(), 2);
        assert_eq!(history.cursor, 2);

        // Clear history
        history.clear();

        assert_eq!(history.entries.len(), 0);
        assert_eq!(history.cursor, 0);

        // Undo/redo should have no effect after clear
        history.undo(&mut editor);
        history.redo(&mut editor);
        assert_eq!(history.cursor, 0);
    }

    #[test]
    fn test_undo_redo_bounds_checking() {
        let mut editor = create_editor_with_text("test case");
        let mut history = TextEditHistoryManager::new(10, false);

        // Test undo on empty history
        history.undo(&mut editor);
        assert_eq!(history.cursor, 0);

        // Test redo on empty history
        history.redo(&mut editor);
        assert_eq!(history.cursor, 0);

        // Add one operation
        history.push(TextEditDelta::Insert {
            cursor_before: Cursor::new(0, 4),
            cursor_after: Cursor::new(0, 9),
            text: " case".to_string(),
        });

        // Test multiple redos at the end
        history.redo(&mut editor);
        history.redo(&mut editor); // Should have no effect
        history.redo(&mut editor); // Should have no effect
        assert_eq!(history.cursor, 1);

        // Test multiple undos at the beginning
        history.undo(&mut editor);
        history.undo(&mut editor); // Should have no effect
        history.undo(&mut editor); // Should have no effect
        assert_eq!(history.cursor, 0);
    }

    #[test]
    fn test_complex_undo_redo_sequence() {
        let mut editor = create_editor_with_text("hello Rust!");
        let mut history = TextEditHistoryManager::new(10, false);

        // Add multiple operations
        let operations = vec![
            TextEditDelta::Insert {
                cursor_before: Cursor::new(0, 5),
                cursor_after: Cursor::new(0, 11),
                text: " world".to_string(),
            },
            TextEditDelta::Insert {
                cursor_before: Cursor::new(0, 11),
                cursor_after: Cursor::new(0, 12),
                text: "!".to_string(),
            },
            TextEditDelta::Replace {
                range_before: (Cursor::new(0, 6), Cursor::new(0, 11)),
                range_after: (Cursor::new(0, 6), Cursor::new(0, 10)),
                text_before: "world".to_string(),
                text_after: "Rust".to_string(),
            },
        ];

        for op in operations {
            history.push(op);
        }

        assert_eq!(history.entries.len(), 3);
        assert_eq!(history.cursor, 3);

        // Test complex sequence
        history.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "hello world|!");
        assert_eq!(history.cursor, 2);

        history.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "hello world|");
        assert_eq!(history.cursor, 1);

        history.redo(&mut editor);
        assert_eq!(get_editor_text(&editor), "hello world!|");
        assert_eq!(history.cursor, 2);

        history.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "hello world|");
        assert_eq!(history.cursor, 1);

        history.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "hello|");
        assert_eq!(history.cursor, 0);

        history.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "hello|");
        assert_eq!(history.cursor, 0);

        history.redo(&mut editor);
        assert_eq!(get_editor_text(&editor), "hello world|");
        assert_eq!(history.cursor, 1);

        history.redo(&mut editor);
        assert_eq!(get_editor_text(&editor), "hello world!|");
        assert_eq!(history.cursor, 2);

        history.redo(&mut editor);
        assert_eq!(get_editor_text(&editor), "hello Rust|!");
        assert_eq!(history.cursor, 3);

        // Should not be able to redo further
        history.redo(&mut editor);
        assert_eq!(get_editor_text(&editor), "hello Rust|!");
        assert_eq!(history.cursor, 3);
    }

    #[test]
    fn test_cursor_position_tracking() {
        let mut editor = create_editor_with_text("hello world");
        let mut history = TextEditHistoryManager::new(10, false);

        // Verify initial state
        assert_eq!(history.cursor, 0);
        assert_eq!(history.entries.len(), 0);

        // Push one operation
        history.push(TextEditDelta::Insert {
            cursor_before: Cursor::new(0, 0),
            cursor_after: Cursor::new(0, 5),
            text: "hello".to_string(),
        });

        assert_eq!(history.cursor, 1);
        assert_eq!(history.entries.len(), 1);

        // Undo moves cursor back
        history.undo(&mut editor);
        assert_eq!(history.cursor, 0);

        // Redo moves cursor forward
        history.redo(&mut editor);
        assert_eq!(history.cursor, 1);

        // Push another operation
        history.push(TextEditDelta::Insert {
            cursor_before: Cursor::new(0, 5),
            cursor_after: Cursor::new(0, 11),
            text: " world".to_string(),
        });

        assert_eq!(history.cursor, 2);
        assert_eq!(history.entries.len(), 2);
    }

    #[test]
    fn test_insert_single_character() {
        let mut editor = create_editor_with_text("hello world");
        editor.set_cursor(Cursor::new(0, 5)); // Position after "hello"

        let delta = TextEditDelta::Insert {
            cursor_before: Cursor::new(0, 5),
            cursor_after: Cursor::new(0, 6),
            text: ",".to_string(),
        };

        // Test redo (apply the insert)
        delta.apply(&mut editor);
        assert_eq!(get_editor_text(&editor), "hello,| world");

        // Test undo (remove the insert)
        delta.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "hello| world");
    }

    #[test]
    fn test_insert_multiple_characters() {
        let mut editor = create_editor_with_text("hello world");
        editor.set_cursor(Cursor::new(0, 6)); // Position after "hello "

        let delta = TextEditDelta::Insert {
            cursor_before: Cursor::new(0, 6),
            cursor_after: Cursor::new(0, 16), // After inserting "beautiful "
            text: "beautiful ".to_string(),
        };

        // Test redo
        delta.apply(&mut editor);
        assert_eq!(get_editor_text(&editor), "hello beautiful |world");

        // Test undo
        delta.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "hello |world");
    }

    #[test]
    fn test_insert_at_beginning() {
        let mut editor = create_editor_with_text("world");
        editor.set_cursor(Cursor::new(0, 0)); // At the very beginning

        let delta = TextEditDelta::Insert {
            cursor_before: Cursor::new(0, 0),
            cursor_after: Cursor::new(0, 6), // After inserting "Hello "
            text: "Hello ".to_string(),
        };

        // Test redo
        delta.apply(&mut editor);
        assert_eq!(get_editor_text(&editor), "Hello |world");

        // Test undo
        delta.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "|world");
    }

    #[test]
    fn test_insert_at_end() {
        let mut editor = create_editor_with_text("Hello");
        editor.set_cursor(Cursor::new(0, 5)); // At the end

        let delta = TextEditDelta::Insert {
            cursor_before: Cursor::new(0, 5),
            cursor_after: Cursor::new(0, 11), // After inserting " world"
            text: " world".to_string(),
        };

        // Test redo
        delta.apply(&mut editor);
        assert_eq!(get_editor_text(&editor), "Hello world|");

        // Test undo
        delta.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "Hello|");
    }

    #[test]
    fn test_delete_single_character() {
        let mut editor = create_editor_with_text("hello, world");

        let delta = TextEditDelta::Delete {
            start: Cursor::new(0, 5), // The comma position
            end: Cursor::new(0, 6),   // After the comma
            deleted_text: ",".to_string(),
            direction: TextDeletionDirection::Backward,
        };

        // Test redo (apply the delete)
        delta.apply(&mut editor);
        assert_eq!(get_editor_text(&editor), "hello| world");

        // Test undo (restore the deleted character)
        delta.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "hello,| world");
    }

    #[test]
    fn test_delete_multiple_characters() {
        let mut editor = create_editor_with_text("hello beautiful world");

        let delta = TextEditDelta::Delete {
            start: Cursor::new(0, 6), // After "hello "
            end: Cursor::new(0, 16),  // Before "world"
            deleted_text: "beautiful ".to_string(),
            direction: TextDeletionDirection::Backward,
        };

        // Test redo
        delta.apply(&mut editor);
        assert_eq!(get_editor_text(&editor), "hello |world");

        // Test undo
        delta.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "hello beautiful |world");
    }

    #[test]
    fn test_delete_multiple_characters_forward() {
        let mut editor = create_editor_with_text("hello beautiful world");

        let delta = TextEditDelta::Delete {
            start: Cursor::new(0, 6), // After "hello "
            end: Cursor::new(0, 16),  // Before "world"
            deleted_text: "beautiful ".to_string(),
            direction: TextDeletionDirection::Forward,
        };

        // Test redo
        delta.apply(&mut editor);
        assert_eq!(get_editor_text(&editor), "hello |world");

        // Test undo
        delta.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "hello |beautiful world");
    }

    #[test]
    fn test_delete_from_beginning() {
        let mut editor = create_editor_with_text("Hello world");

        let delta = TextEditDelta::Delete {
            start: Cursor::new(0, 0), // From beginning
            end: Cursor::new(0, 6),   // After "Hello "
            deleted_text: "Hello ".to_string(),
            direction: TextDeletionDirection::Backward,
        };

        // Test redo
        delta.apply(&mut editor);
        assert_eq!(get_editor_text(&editor), "|world");

        // Test undo
        delta.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "Hello |world");
    }

    #[test]
    fn test_delete_to_end() {
        let mut editor = create_editor_with_text("Hello world");

        let delta = TextEditDelta::Delete {
            start: Cursor::new(0, 5), // After "Hello"
            end: Cursor::new(0, 11),  // To the end
            deleted_text: " world".to_string(),
            direction: TextDeletionDirection::Backward,
        };

        // Test redo
        delta.apply(&mut editor);
        assert_eq!(get_editor_text(&editor), "Hello|");

        // Test undo
        delta.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "Hello world|");
    }

    #[test]
    fn test_replace_single_word() {
        let mut editor = create_editor_with_text("Hello world");

        let delta = TextEditDelta::Replace {
            range_before: (Cursor::new(0, 6), Cursor::new(0, 11)), // "world"
            range_after: (Cursor::new(0, 6), Cursor::new(0, 10)),  // "Rust" (shorter)
            text_before: "world".to_string(),
            text_after: "Rust".to_string(),
        };

        // Test redo (apply the replace)
        delta.apply(&mut editor);
        assert_eq!(get_editor_text(&editor), "Hello Rust|");

        // Test undo (restore original text)
        delta.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "Hello world|");
    }

    #[test]
    fn test_replace_with_longer_text() {
        let mut editor = create_editor_with_text("Hello world");

        let delta = TextEditDelta::Replace {
            range_before: (Cursor::new(0, 6), Cursor::new(0, 11)), // "world"
            range_after: (Cursor::new(0, 6), Cursor::new(0, 22)),  // "amazing universe"
            text_before: "world".to_string(),
            text_after: "amazing universe".to_string(),
        };

        // Test redo
        delta.apply(&mut editor);
        assert_eq!(get_editor_text(&editor), "Hello amazing universe|");

        // Test undo
        delta.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "Hello world|");
    }

    #[test]
    fn test_replace_with_shorter_text() {
        let mut editor = create_editor_with_text("Hello amazing universe");

        let delta = TextEditDelta::Replace {
            range_before: (Cursor::new(0, 6), Cursor::new(0, 22)), // "amazing universe"
            range_after: (Cursor::new(0, 6), Cursor::new(0, 11)),  // "world"
            text_before: "amazing universe".to_string(),
            text_after: "world".to_string(),
        };

        // Test redo
        delta.apply(&mut editor);
        assert_eq!(get_editor_text(&editor), "Hello world|");

        // Test undo
        delta.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "Hello amazing universe|");
    }

    #[test]
    fn test_replace_entire_text() {
        let mut editor = create_editor_with_text("Old text");

        let delta = TextEditDelta::Replace {
            range_before: (Cursor::new(0, 0), Cursor::new(0, 8)), // Entire "Old text"
            range_after: (Cursor::new(0, 0), Cursor::new(0, 8)),  // Entire "New text"
            text_before: "Old text".to_string(),
            text_after: "New text".to_string(),
        };

        // Test redo
        delta.apply(&mut editor);
        assert_eq!(get_editor_text(&editor), "New text|");

        // Test undo
        delta.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "Old text|");
    }

    #[test]
    fn test_replace_empty_with_text() {
        let mut editor = create_editor_with_text("Hello ");

        let delta = TextEditDelta::Replace {
            range_before: (Cursor::new(0, 6), Cursor::new(0, 6)), // Empty selection
            range_after: (Cursor::new(0, 6), Cursor::new(0, 11)), // "world"
            text_before: "".to_string(),
            text_after: "world".to_string(),
        };

        // Test redo
        delta.apply(&mut editor);
        assert_eq!(get_editor_text(&editor), "Hello world|");

        // Test undo
        delta.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "Hello |");
    }

    #[test]
    fn test_replace_text_with_empty() {
        let mut editor = create_editor_with_text("Hello world");

        let delta = TextEditDelta::Replace {
            range_before: (Cursor::new(0, 5), Cursor::new(0, 11)), // " world"
            range_after: (Cursor::new(0, 5), Cursor::new(0, 5)),   // Empty
            text_before: " world".to_string(),
            text_after: "".to_string(),
        };

        // Test redo
        delta.apply(&mut editor);
        assert_eq!(get_editor_text(&editor), "Hello|");

        // Test undo
        delta.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "Hello world|");
    }

    #[test]
    fn test_sequential_operations() {
        let mut editor = create_editor_with_text("test");

        // First: Insert " case"
        let insert_delta = TextEditDelta::Insert {
            cursor_before: Cursor::new(0, 4),
            cursor_after: Cursor::new(0, 9),
            text: " case".to_string(),
        };

        // Second: Delete "test"
        let delete_delta = TextEditDelta::Delete {
            start: Cursor::new(0, 0),
            end: Cursor::new(0, 4),
            deleted_text: "test".to_string(),
            direction: TextDeletionDirection::Backward,
        };

        // Apply operations
        insert_delta.apply(&mut editor);
        assert_eq!(get_editor_text(&editor), "test case|");

        delete_delta.apply(&mut editor);
        assert_eq!(get_editor_text(&editor), "| case");

        // Undo operations in reverse order
        delete_delta.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "test| case");

        insert_delta.undo(&mut editor);
        assert_eq!(get_editor_text(&editor), "test|");
    }

    #[test]
    fn test_multiple_undo_redo_cycles() {
        let mut editor = create_editor_with_text("hello");

        let delta = TextEditDelta::Insert {
            cursor_before: Cursor::new(0, 5),
            cursor_after: Cursor::new(0, 11),
            text: " world".to_string(),
        };

        // Apply and undo multiple times
        for _ in 0..3 {
            delta.apply(&mut editor);
            assert_eq!(get_editor_text(&editor), "hello world|");

            delta.undo(&mut editor);
            assert_eq!(get_editor_text(&editor), "hello|");
        }
    }
}
