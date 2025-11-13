// Signature Help Module for Zen LSP
// Handles textDocument/signatureHelp requests

use lsp_server::Request;
use lsp_server::Response;
use lsp_types::*;
use serde_json::Value;
use std::sync::{Arc, Mutex};

use super::document_store::DocumentStore;
use super::types::SymbolInfo;

// ============================================================================
// PUBLIC HANDLER FUNCTION
// ============================================================================

pub fn handle_signature_help(req: Request, store: &Arc<Mutex<DocumentStore>>) -> Response {
    let params: SignatureHelpParams = match serde_json::from_value(req.params) {
        Ok(p) => p,
        Err(_) => {
            return Response {
                id: req.id,
                result: Some(Value::Null),
                error: None,
            };
        }
    };

    let store = match store.lock() { Ok(s) => s, Err(_) => {
        let empty = SignatureHelp { signatures: vec![], active_signature: None, active_parameter: None };
        return Response { id: req.id, result: Some(serde_json::to_value(empty).unwrap_or(serde_json::Value::Null)), error: None };
    } };
    let doc = match store.documents.get(&params.text_document_position_params.text_document.uri) {
        Some(d) => d,
        None => {
            return Response {
                id: req.id,
                result: Some(Value::Null),
                error: None,
            };
        }
    };

    // Find function call at cursor position
    let position = params.text_document_position_params.position;
    let function_call = find_function_call_at_position(&doc.content, position);

    eprintln!("[LSP] Signature help at {}:{} - function_call: {:?}",
        position.line, position.character, function_call);

    let signature_help = match function_call {
        Some((function_name, active_param)) => {
            eprintln!("[LSP] Looking for function '{}' in {} doc symbols",
                function_name, doc.symbols.len());

            // Look up function in symbols (document, stdlib, workspace)
            let mut signature_info = None;

            // Check document symbols first (highest priority)
            if let Some(symbol) = doc.symbols.get(&function_name) {
                eprintln!("[LSP] Found '{}' in document symbols", function_name);
                signature_info = Some(create_signature_info(symbol));
            }

            // Check stdlib symbols if not found
            if signature_info.is_none() {
                if let Some(symbol) = store.stdlib_symbols.get(&function_name) {
                    signature_info = Some(create_signature_info(symbol));
                }
            }

            // Check workspace symbols if not found
            if signature_info.is_none() {
                if let Some(symbol) = store.workspace_symbols.get(&function_name) {
                    signature_info = Some(create_signature_info(symbol));
                }
            }

            match signature_info {
                Some(sig_info) => SignatureHelp {
                    signatures: vec![sig_info],
                    active_signature: Some(0),
                    active_parameter: Some(active_param as u32),
                },
                None => SignatureHelp {
                    signatures: vec![],
                    active_signature: None,
                    active_parameter: None,
                },
            }
        }
        None => SignatureHelp {
            signatures: vec![],
            active_signature: None,
            active_parameter: None,
        },
    };

    Response {
        id: req.id,
        result: serde_json::to_value(signature_help).ok(),
        error: None,
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn find_function_call_at_position(content: &str, position: Position) -> Option<(String, usize)> {
    let lines: Vec<&str> = content.lines().collect();
    if position.line as usize >= lines.len() {
        return None;
    }

    // Build context string from current line and potential previous lines for multi-line calls
    let mut context = String::new();
    let mut line_offset = 0;

    // Look back up to 5 lines for multi-line function calls
    let start_line = if position.line >= 5 { position.line - 5 } else { 0 };
    for i in start_line..=position.line {
        if i as usize >= lines.len() {
            break;
        }
        if i == position.line {
            line_offset = context.len();
        }
        context.push_str(lines[i as usize]);
        context.push(' ');
    }

    let cursor_pos = line_offset + position.character as usize;
    let cursor_pos = cursor_pos.min(context.len());

    // Find the function call - look backwards from cursor for opening paren
    let mut paren_count = 0;
    let mut current_pos = cursor_pos;

    // Move to the nearest opening paren
    while current_pos > 0 {
        let ch = context.chars().nth(current_pos - 1)?;
        if ch == ')' {
            paren_count += 1;
        } else if ch == '(' {
            if paren_count == 0 {
                break;
            }
            paren_count -= 1;
        }
        current_pos -= 1;
    }

    if current_pos == 0 {
        return None; // No opening paren found
    }

    // Extract function name before the opening paren
    let before_paren = &context[..current_pos - 1];
    let function_name = before_paren
        .split(|c: char| c.is_whitespace() || c == '=' || c == ',' || c == ';' || c == '{' || c == '(')
        .last()?
        .trim()
        .split('.')
        .last()?
        .to_string();

    // Count parameters by counting commas at paren_depth = 0
    let inside_parens = &context[current_pos..cursor_pos];
    let mut active_param = 0;
    let mut depth = 0;

    for ch in inside_parens.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => active_param += 1,
            _ => {}
        }
    }

    Some((function_name, active_param))
}

fn create_signature_info(symbol: &SymbolInfo) -> SignatureInformation {
    // Extract function signature from symbol detail
    let label = symbol.detail.clone().unwrap_or_else(|| {
        format!("{}(...)", symbol.name)
    });

    // Parse parameters from the function signature
    let parameters = parse_function_parameters(&label);

    SignatureInformation {
        label,
        documentation: symbol.documentation.as_ref().map(|doc| {
            Documentation::String(doc.clone())
        }),
        parameters: if parameters.is_empty() {
            None
        } else {
            Some(parameters)
        },
        active_parameter: None,
    }
}

fn parse_function_parameters(signature: &str) -> Vec<ParameterInformation> {
    // Parse signature like "function_name = (param1: Type1, param2: Type2) ReturnType"
    let mut parameters = Vec::new();

    // Find the parameter section between ( and )
    if let Some(start) = signature.find('(') {
        if let Some(end) = signature[start..].find(')') {
            let params_str = &signature[start + 1..start + end];

            // Split by commas (simple for now, could be enhanced for nested types)
            for param in params_str.split(',') {
                let param = param.trim();
                if !param.is_empty() {
                    parameters.push(ParameterInformation {
                        label: lsp_types::ParameterLabel::Simple(param.to_string()),
                        documentation: None,
                    });
                }
            }
        }
    }

    parameters
}

