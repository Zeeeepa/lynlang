use lsp_server::{Request, Response, ResponseError, ErrorCode};
use lsp_types::*;
use serde_json::Value;
use std::sync::{Arc, Mutex};

use super::document_store::DocumentStore;

pub fn handle_formatting(req: Request, store: Arc<Mutex<DocumentStore>>) -> Response {
    let params: DocumentFormattingParams = match serde_json::from_value(req.params) {
        Ok(p) => p,
        Err(_) => {
            return Response {
                id: req.id,
                result: Some(Value::Null),
                error: Some(ResponseError {
                    code: ErrorCode::InvalidParams as i32,
                    message: "Invalid parameters".to_string(),
                    data: None,
                }),
            }
        }
    };

    let store = match store.lock() { Ok(s) => s, Err(_) => { return Response { id: req.id, result: Some(serde_json::to_value(Vec::<TextEdit>::new()).unwrap_or(serde_json::Value::Null)), error: None }; } };
    let doc = match store.documents.get(&params.text_document.uri) {
        Some(doc) => doc,
        None => {
            return Response {
                id: req.id,
                result: Some(Value::Null),
                error: None,
            }
        }
    };

    // Format the document
    let formatted = format_document(&doc.content);

    // Create a single edit that replaces the entire document
    let line_count = doc.content.lines().count();
    let last_line_len = doc.content.lines().last().map(|l| l.len()).unwrap_or(0);

    let edit = TextEdit {
        range: Range {
            start: Position::new(0, 0),
            end: Position::new(line_count as u32, last_line_len as u32),
        },
        new_text: formatted,
    };

    Response {
        id: req.id,
        result: Some(serde_json::to_value(vec![edit]).unwrap_or(Value::Null)),
        error: None,
    }
}

fn format_document(content: &str) -> String {
    let mut formatted = String::new();
    let mut indent_level: usize = 0;
    let indent_str = "    "; // 4 spaces

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            formatted.push('\n');
            continue;
        }

        // Decrease indent for closing braces
        if trimmed.starts_with('}') || trimmed.starts_with(']') {
            indent_level = indent_level.saturating_sub(1);
        }

        // Add indentation
        for _ in 0..indent_level {
            formatted.push_str(indent_str);
        }

        formatted.push_str(trimmed);
        formatted.push('\n');

        // Increase indent after opening braces
        if trimmed.ends_with('{') || trimmed.ends_with('[') {
            indent_level += 1;
        }
    }

    formatted
}
