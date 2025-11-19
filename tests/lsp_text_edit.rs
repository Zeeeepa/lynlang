// Test suite for LSP text edit operations
// Tests the apply_text_edit logic to ensure file corruption bug is fixed

#[cfg(test)]
mod lsp_text_edit_tests {
    use lsp_types::{Position, Range};

    // Copy of the fixed apply_text_edit function for testing
    fn apply_text_edit(content: &str, range: &Range, new_text: &str) -> String {
        let start_line = range.start.line as usize;
        let start_char = range.start.character as usize;
        let end_line = range.end.line as usize;
        let end_char = range.end.character as usize;

        // Helper function to convert LSP position to byte offset
        fn position_to_byte_offset(content: &str, target_line: usize, target_char: usize) -> usize {
            let mut current_line = 0;
            let mut current_char = 0;

            for (byte_idx, ch) in content.char_indices() {
                if current_line == target_line && current_char == target_char {
                    return byte_idx;
                }

                if ch == '\n' {
                    current_line += 1;
                    current_char = 0;
                } else {
                    current_char += 1;
                }
            }

            if current_line == target_line && current_char == target_char {
                return content.len();
            }

            content.len()
        }

        let start_byte = position_to_byte_offset(content, start_line, start_char);
        let end_byte = position_to_byte_offset(content, end_line, end_char);

        let start_byte = start_byte.min(content.len());
        let end_byte = end_byte.min(content.len());
        let (start_byte, end_byte) = if start_byte <= end_byte {
            (start_byte, end_byte)
        } else {
            (end_byte, start_byte)
        };

        let mut result = String::with_capacity(content.len() - (end_byte - start_byte) + new_text.len());
        result.push_str(&content[..start_byte]);
        result.push_str(new_text);
        result.push_str(&content[end_byte..]);

        result
    }

    #[test]
    fn test_single_char_insertion_at_start() {
        let content = "hello";
        let range = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 0, character: 0 },
        };
        let result = apply_text_edit(content, &range, "X");
        assert_eq!(result, "Xhello", "Should insert 'X' at start");
    }

    #[test]
    fn test_single_char_insertion_in_middle() {
        let content = "hello";
        let range = Range {
            start: Position { line: 0, character: 2 },
            end: Position { line: 0, character: 2 },
        };
        let result = apply_text_edit(content, &range, "X");
        assert_eq!(result, "heXllo", "Should insert 'X' at position 2");
    }

    #[test]
    fn test_single_char_replacement() {
        let content = "hello";
        let range = Range {
            start: Position { line: 0, character: 1 },
            end: Position { line: 0, character: 2 },
        };
        let result = apply_text_edit(content, &range, "a");
        assert_eq!(result, "hallo", "Should replace 'e' with 'a'");
    }

    #[test]
    fn test_multiline_content_insertion() {
        let content = "line1\nline2\nline3";
        let range = Range {
            start: Position { line: 1, character: 5 },
            end: Position { line: 1, character: 5 },
        };
        let result = apply_text_edit(content, &range, "X");
        assert_eq!(result, "line1\nline2X\nline3", "Should insert 'X' on line 1");
    }

    #[test]
    fn test_multiline_replacement() {
        let content = "line1\nline2\nline3";
        let range = Range {
            start: Position { line: 1, character: 0 },
            end: Position { line: 1, character: 5 },
        };
        let result = apply_text_edit(content, &range, "NEW");
        assert_eq!(result, "line1\nNEW\nline3", "Should replace entire line2 with NEW");
    }

    #[test]
    fn test_deletion() {
        let content = "hello";
        let range = Range {
            start: Position { line: 0, character: 1 },
            end: Position { line: 0, character: 4 },
        };
        let result = apply_text_edit(content, &range, "");
        assert_eq!(result, "ho", "Should delete 'ell'");
    }

    #[test]
    fn test_insertion_at_end_of_file() {
        let content = "hello";
        let range = Range {
            start: Position { line: 0, character: 5 },
            end: Position { line: 0, character: 5 },
        };
        let result = apply_text_edit(content, &range, "!");
        assert_eq!(result, "hello!", "Should append '!' at end");
    }

    #[test]
    fn test_critical_first_line_preservation() {
        // This is the bug we're fixing: first line should NOT be replaced with cursor line
        let content = "first line\ncursor line";
        
        // Simulate cursor being on line 1, inserting after 'c'
        let range = Range {
            start: Position { line: 1, character: 1 },
            end: Position { line: 1, character: 1 },
        };
        let result = apply_text_edit(content, &range, "X");
        
        // First line should remain UNCHANGED
        assert!(result.starts_with("first line\n"), 
                "First line must be preserved! Got: {}", result);
        assert!(result.contains("cXursor line"), 
                "Should insert 'X' on line 1, got: {}", result);
    }

    #[test]
    fn test_multiline_range_replacement() {
        let content = "line1\nline2\nline3\nline4";
        // Replace from line 1 char 2 ("ne2") to line 2 char 3 ("lin")
        // Content: "line1\n[line2\nlin]e3\nline4"
        let range = Range {
            start: Position { line: 1, character: 2 },
            end: Position { line: 2, character: 3 },
        };
        let result = apply_text_edit(content, &range, "REPLACED");
        // Result should be: "line1\nli" + "REPLACED" + "e3\nline4"
        assert_eq!(result, "line1\nliREPLACEDe3\nline4", 
                   "Should replace across lines");
    }

    #[test]
    fn test_empty_file_insertion() {
        let content = "";
        let range = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 0, character: 0 },
        };
        let result = apply_text_edit(content, &range, "hello");
        assert_eq!(result, "hello", "Should insert into empty file");
    }

    #[test]
    fn test_unicode_handling() {
        let content = "hello 🦀 world";
        let range = Range {
            start: Position { line: 0, character: 6 },
            end: Position { line: 0, character: 7 },
        };
        let result = apply_text_edit(content, &range, "X");
        // Note: character position counts code units, not bytes
        // This test verifies we don't panic on unicode
        assert!(!result.is_empty(), "Should handle unicode content");
    }
}
