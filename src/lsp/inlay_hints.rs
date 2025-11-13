// Inlay Hints Module for Zen LSP
// Handles textDocument/inlayHint requests

use lsp_server::Request;
use lsp_server::Response;
use lsp_types::*;
use serde_json::Value;
use std::sync::{Arc, Mutex};

use crate::ast::{Declaration, Expression, Statement};

use super::document_store::DocumentStore;
use super::types::Document;

// ============================================================================
// PUBLIC HANDLER FUNCTION
// ============================================================================

pub fn handle_inlay_hints(req: Request, store: &Arc<Mutex<DocumentStore>>) -> Response {
    let params: InlayHintParams = match serde_json::from_value(req.params) {
        Ok(p) => p,
        Err(_) => {
            return Response {
                id: req.id,
                result: Some(Value::Null),
                error: None,
            };
        }
    };

    let store = match store.lock() { Ok(s) => s, Err(_) => { return Response { id: req.id, result: Some(serde_json::to_value(Vec::<InlayHint>::new()).unwrap_or(serde_json::Value::Null)), error: None }; } };
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

    let mut hints = Vec::with_capacity(20); // Pre-allocate for common case

    // Parse the AST and extract variable declarations with position tracking
    if let Some(ast) = &doc.ast {
        for decl in ast {
            if let Declaration::Function(func) = decl {
                collect_hints_from_statements(&func.body, &doc.content, doc, &store, &mut hints);
            }
        }
    }

    Response {
        id: req.id,
        result: serde_json::to_value(hints).ok(),
        error: None,
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn collect_hints_from_statements(
    statements: &[Statement],
    content: &str,
    doc: &Document,
    store: &DocumentStore,
    hints: &mut Vec<InlayHint>
) {
    for stmt in statements {
        match stmt {
            Statement::VariableDeclaration { name, type_, initializer, .. } => {
                // Only add hints for variables without explicit type annotations
                if type_.is_none() {
                    if let Some(init) = initializer {
                        if let Some(inferred_type) = infer_expression_type(init, doc, store) {
                            // Find the position of this variable declaration in the source
                            if let Some(position) = find_variable_position(content, name) {
                                hints.push(InlayHint {
                                    position,
                                    label: InlayHintLabel::String(format!(": {}", inferred_type)),
                                    kind: Some(InlayHintKind::TYPE),
                                    text_edits: None,
                                    tooltip: None,
                                    padding_left: None,
                                    padding_right: None,
                                    data: None,
                                });
                            }
                        }

                        // Also collect parameter hints from function calls in initializer
                        collect_param_hints_from_expression(init, content, doc, store, hints);
                    }
                }
            }
            Statement::Expression(expr) => {
                // Collect parameter hints from standalone expressions
                collect_param_hints_from_expression(expr, content, doc, store, hints);
            }
            Statement::Return(expr) => {
                // Collect parameter hints from return expressions
                collect_param_hints_from_expression(expr, content, doc, store, hints);
            }
            Statement::Loop { body, .. } => {
                collect_hints_from_statements(body, content, doc, store, hints);
            }
            _ => {}
        }
    }
}

fn find_variable_position(content: &str, var_name: &str) -> Option<Position> {
    // Optimized: iterate lines directly without collecting
    for (line_num, line) in content.lines().enumerate() {
        // Look for Zen variable declaration patterns:
        // "var_name = " or "var_name := " or "var_name ::= " or "var_name : Type ="

        // First, check if line contains the variable name
        if let Some(name_pos) = line.find(var_name) {
            // Check if this is actually a variable declaration
            // Must be at start of line or after whitespace - optimized: use byte indexing
            let before_ok = name_pos == 0 || {
                line.as_bytes()
                    .get(name_pos.saturating_sub(1))
                    .map(|&b| b < 128 && (b as char).is_whitespace())
                    .unwrap_or(true)
            };

            if before_ok {
                // Check what comes after the variable name
                let after_name = &line[name_pos + var_name.len()..].trim_start();

                // Check for Zen assignment patterns
                if after_name.starts_with("=") ||
                   after_name.starts_with(":=") ||
                   after_name.starts_with("::=") ||
                   after_name.starts_with(":") {
                    // Position after the variable name (where type hint should go)
                    let char_pos = name_pos + var_name.len();
                    return Some(Position {
                        line: line_num as u32,
                        character: char_pos as u32,
                    });
                }
            }
        }
    }

    None
}

fn infer_expression_type(expr: &Expression, doc: &Document, store: &DocumentStore) -> Option<String> {
    match expr {
        Expression::Integer32(_) => Some("i32".to_string()),
        Expression::Integer64(_) => Some("i64".to_string()),
        Expression::Float32(_) => Some("f32".to_string()),
        Expression::Float64(_) => Some("f64".to_string()),
        Expression::String(_) => Some("StaticString".to_string()),
        Expression::Boolean(_) => Some("bool".to_string()),
        Expression::BinaryOp { left, right, .. } => {
            // Simple type inference for binary operations
            let left_type = infer_expression_type(left, doc, store)?;
            let right_type = infer_expression_type(right, doc, store)?;

            if left_type == "f64" || right_type == "f64" {
                Some("f64".to_string())
            } else if left_type == "i64" || right_type == "i64" {
                Some("i64".to_string())
            } else {
                Some("i32".to_string())
            }
        }
        Expression::FunctionCall { name, .. } => {
            // Look up function return type from document symbols
            if let Some(symbol) = doc.symbols.get(name) {
                // Extract return type from function signature like "add = (a: i32, b: i32) i32"
                if let Some(detail) = &symbol.detail {
                    return extract_return_type_from_signature(detail);
                }
            }

            // Check stdlib and workspace symbols
            if let Some(symbol) = store.stdlib_symbols.get(name) {
                if let Some(detail) = &symbol.detail {
                    return extract_return_type_from_signature(detail);
                }
            }
            if let Some(symbol) = store.workspace_symbols.get(name) {
                if let Some(detail) = &symbol.detail {
                    return extract_return_type_from_signature(detail);
                }
            }
            None
        }
        Expression::StructLiteral { name, .. } => {
            // Struct literals have the type of the struct
            Some(name.clone())
        }
        Expression::ArrayLiteral(elements) => {
            // Infer array element type from first element
            if let Some(first) = elements.first() {
                if let Some(elem_type) = infer_expression_type(first, doc, store) {
                    return Some(format!("[{}]", elem_type));
                }
            }
            None
        }
        Expression::Identifier(var_name) => {
            // Look up variable type from document symbols
            if let Some(symbol) = doc.symbols.get(var_name) {
                if let Some(detail) = &symbol.detail {
                    return Some(detail.clone());
                }
            }
            None
        }
        _ => None,
    }
}

fn extract_return_type_from_signature(signature: &str) -> Option<String> {
    // Parse signature like "function_name = (params) ReturnType"
    // Find the closing paren, then take everything after it
    if let Some(close_paren) = signature.rfind(')') {
        let return_type = signature[close_paren + 1..].trim();
        if !return_type.is_empty() {
            return Some(return_type.to_string());
        }
    }
    None
}

fn collect_param_hints_from_expression(
    expr: &Expression,
    content: &str,
    doc: &Document,
    store: &DocumentStore,
    hints: &mut Vec<InlayHint>
) {
    match expr {
        Expression::FunctionCall { name, args } => {
            // Look up function signature to get parameter names
            if let Some(param_names) = get_function_param_names(name, doc, store) {
                // Add parameter name hints for each argument (if we have enough names)
                for (idx, arg) in args.iter().enumerate() {
                    if let Some(param_name) = param_names.get(idx) {
                        // Find the position of this argument in the source
                        if let Some(arg_pos) = find_function_arg_position(content, name, idx) {
                            hints.push(InlayHint {
                                position: arg_pos,
                                label: InlayHintLabel::String(format!("{}: ", param_name)),
                                kind: Some(InlayHintKind::PARAMETER),
                                text_edits: None,
                                tooltip: None,
                                padding_left: None,
                                padding_right: Some(true),
                                data: None,
                            });
                        }
                    }

                    // Recursively collect from nested function calls
                    collect_param_hints_from_expression(arg, content, doc, store, hints);
                }
            }
        }
        Expression::BinaryOp { left, right, .. } => {
            collect_param_hints_from_expression(left, content, doc, store, hints);
            collect_param_hints_from_expression(right, content, doc, store, hints);
        }
        Expression::QuestionMatch { scrutinee, arms } => {
            collect_param_hints_from_expression(scrutinee, content, doc, store, hints);
            for arm in arms {
                collect_param_hints_from_expression(&arm.body, content, doc, store, hints);
            }
        }
        Expression::StructLiteral { fields, .. } => {
            for (_, field_expr) in fields {
                collect_param_hints_from_expression(field_expr, content, doc, store, hints);
            }
        }
        Expression::ArrayLiteral(elements) => {
            for elem in elements {
                collect_param_hints_from_expression(elem, content, doc, store, hints);
            }
        }
        _ => {}
    }
}

fn get_function_param_names(func_name: &str, doc: &Document, store: &DocumentStore) -> Option<Vec<String>> {
    // First, try to find function in the document's AST
    if let Some(ast) = &doc.ast {
        for decl in ast {
            if let Declaration::Function(func) = decl {
                if &func.name == func_name {
                    return Some(func.args.iter().map(|(name, _)| name.clone()).collect());
                }
            }
        }
    }

    // Check stdlib symbols
    if let Some(symbol) = store.stdlib_symbols.get(func_name) {
        if let Some(detail) = &symbol.detail {
            return extract_param_names_from_signature(detail);
        }
    }

    // Check workspace symbols
    if let Some(symbol) = store.workspace_symbols.get(func_name) {
        if let Some(detail) = &symbol.detail {
            return extract_param_names_from_signature(detail);
        }
    }

    None
}

fn extract_param_names_from_signature(signature: &str) -> Option<Vec<String>> {
    // Parse signature like "function_name = (a: i32, b: i32) ReturnType"
    let open_paren = signature.find('(')?;
    let close_paren = signature.find(')')?;

    let params_str = &signature[open_paren + 1..close_paren];
    if params_str.trim().is_empty() {
        return Some(vec![]);
    }

    let param_names: Vec<String> = params_str
        .split(',')
        .filter_map(|param| {
            // Each param is like "name: Type"
            let parts: Vec<&str> = param.trim().split(':').collect();
            if !parts.is_empty() {
                Some(parts[0].trim().to_string())
            } else {
                None
            }
        })
        .collect();

    Some(param_names)
}

fn find_function_arg_position(content: &str, func_name: &str, arg_index: usize) -> Option<Position> {
    // Optimized: iterate lines directly without collecting
    for (line_num, line) in content.lines().enumerate() {
        // Look for function call pattern: "func_name("
        if let Some(func_pos) = line.find(func_name) {
            // Check if this is actually a function call (followed by '(')
            let after_func = &line[func_pos + func_name.len()..].trim_start();
            if after_func.starts_with('(') {
                // Find the opening paren position
                let paren_pos = func_pos + func_name.len() + (line[func_pos + func_name.len()..].find('(').unwrap_or(0));

                // Find the nth argument by counting commas - optimized: use byte indexing
                let mut current_arg = 0;
                let mut depth = 0;
                let mut i = paren_pos + 1;
                let bytes = line.as_bytes();

                while i < line.len() {
                    // Optimized: use byte indexing for ASCII characters
                    let c = if let Some(&b) = bytes.get(i) {
                        if b < 128 {
                            b as char
                        } else {
                            // Fallback to char iteration for non-ASCII
                            line.chars().nth(i).unwrap_or('\0')
                        }
                    } else {
                        break;
                    };

                    if c == '(' {
                        depth += 1;
                    } else if c == ')' {
                        if depth == 0 {
                            break;
                        }
                        depth -= 1;
                    } else if c == ',' && depth == 0 {
                        if current_arg == arg_index {
                            // Found the position of the argument
                            return Some(Position {
                                line: line_num as u32,
                                character: (i + 1) as u32, // After the comma
                            });
                        }
                        current_arg += 1;
                    }

                    i += 1;
                }

                // If we're looking for the first argument and there are no commas before it
                if arg_index == 0 && current_arg == 0 {
                    return Some(Position {
                        line: line_num as u32,
                        character: (paren_pos + 1) as u32,
                    });
                }
            }
        }
    }

    None
}

