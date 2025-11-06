// Code Lens Module for Zen LSP
// Handles textDocument/codeLens requests

use lsp_server::Request;
use lsp_server::Response;
use lsp_types::*;
use serde_json::Value;
use std::sync::{Arc, Mutex};

use crate::ast::Declaration;

use super::document_store::DocumentStore;
use super::types::Document;

// ============================================================================
// PUBLIC HANDLER FUNCTION
// ============================================================================

pub fn handle_code_lens(req: Request, store: &Arc<Mutex<DocumentStore>>) -> Response {
    let params: CodeLensParams = match serde_json::from_value(req.params) {
        Ok(p) => p,
        Err(_) => {
            return Response {
                id: req.id,
                result: Some(Value::Null),
                error: None,
            };
        }
    };

    let store = store.lock().unwrap();
    let doc = match store.documents.get(&params.text_document.uri) {
        Some(d) => d,
        None => {
            return Response {
                id: req.id,
                result: Some(Value::Null),
                error: None,
            };
        }
    };

    let mut lenses = Vec::new();

    // Find test functions and add "Run Test" code lens
    if let Some(ast) = &doc.ast {
        for decl in ast.iter() {
            if let Declaration::Function(func) = decl {
                let func_name = &func.name;

                // Check if this is a test function (starts with test_ or ends with _test)
                if func_name.starts_with("test_") || func_name.ends_with("_test") || func_name.contains("_test_") {
                    // Find the line number of this function
                    let line_num = find_function_line(&doc.content, func_name);

                    if let Some(line) = line_num {
                        lenses.push(CodeLens {
                            range: Range {
                                start: Position {
                                    line: line as u32,
                                    character: 0,
                                },
                                end: Position {
                                    line: line as u32,
                                    character: 0,
                                },
                            },
                            command: Some(Command {
                                title: "▶ Run Test".to_string(),
                                command: "zen.runTest".to_string(),
                                arguments: Some(vec![
                                    serde_json::to_value(&params.text_document.uri).unwrap(),
                                    serde_json::to_value(func_name).unwrap(),
                                ]),
                            }),
                            data: None,
                        });
                    }
                }
            }
        }
    }

    Response {
        id: req.id,
        result: serde_json::to_value(lenses).ok(),
        error: None,
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn find_function_line(content: &str, func_name: &str) -> Option<usize> {
    let lines: Vec<&str> = content.lines().collect();
    for (line_num, line) in lines.iter().enumerate() {
        // Look for function definition: "func_name = "
        if line.contains(func_name) && line.contains("=") && line.contains("(") {
            // Verify this is a function definition, not just usage
            if let Some(eq_pos) = line.find('=') {
                if let Some(name_start) = line.find(func_name) {
                    // Check if function name comes before '=' and there's '(' after
                    if name_start < eq_pos && line[eq_pos..].contains('(') {
                        return Some(line_num);
                    }
                }
            }
        }
    }
    None
}

