// Document highlight handler

use lsp_server::{Request, Response};
use lsp_types::*;
use serde_json::Value;
use super::super::document_store::DocumentStore;
use super::utils::{find_symbol_at_position, is_word_boundary_char};

/// Find all occurrences of a symbol in a document for highlighting
fn find_symbol_occurrences(content: &str, symbol_name: &str) -> Vec<DocumentHighlight> {
    let mut highlights = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (line_idx, line) in lines.iter().enumerate() {
        let mut start_pos = 0;
        while let Some(pos) = line[start_pos..].find(symbol_name) {
            let actual_pos = start_pos + pos;
            let is_word_start = actual_pos == 0 || is_word_boundary_char(line, actual_pos.saturating_sub(1));
            let end_pos = actual_pos + symbol_name.len();
            let is_word_end = end_pos >= line.len() || is_word_boundary_char(line, end_pos);

            if is_word_start && is_word_end {
                highlights.push(DocumentHighlight {
                    range: Range {
                        start: Position { line: line_idx as u32, character: actual_pos as u32 },
                        end: Position { line: line_idx as u32, character: end_pos as u32 },
                    },
                    kind: Some(DocumentHighlightKind::TEXT),
                });
            }
            start_pos = actual_pos + 1;
        }
    }
    highlights
}

/// Handle textDocument/documentHighlight requests
pub fn handle_document_highlight(req: Request, store: &std::sync::Arc<std::sync::Mutex<DocumentStore>>) -> Response {
    let params: DocumentHighlightParams = match serde_json::from_value(req.params) {
        Ok(p) => p,
        Err(_) => {
            return Response {
                id: req.id,
                result: Some(Value::Null),
                error: None,
            };
        }
    };

    let store = match store.lock() {
        Ok(s) => s,
        Err(_) => {
            return Response { id: req.id, result: Some(Value::Null), error: None };
        }
    };
    
    if let Some(doc) = store.documents.get(&params.text_document_position_params.text_document.uri) {
        let position = params.text_document_position_params.position;
        if let Some(symbol_name) = find_symbol_at_position(&doc.content, position) {
            let highlights = find_symbol_occurrences(&doc.content, &symbol_name);
            return Response {
                id: req.id,
                result: Some(serde_json::to_value(highlights).unwrap_or(Value::Null)),
                error: None,
            };
        }
    }

    Response {
        id: req.id,
        result: Some(Value::Null),
        error: None,
    }
}

