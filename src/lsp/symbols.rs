// Symbol-related functionality for the LSP server
// Handles document symbols, inlay hints, and code lens features

use lsp_server::{Request, Response};
use lsp_types::*;
use serde_json::Value;

use crate::ast::{Declaration, Expression, Statement};
use super::document_store::DocumentStore;
use super::types::Document;

// ============================================================================
// DOCUMENT SYMBOLS
// ============================================================================

pub fn handle_document_symbols(req: Request, store: &DocumentStore) -> Response {
    let params: DocumentSymbolParams = match serde_json::from_value(req.params) {
        Ok(p) => p,
        Err(_) => {
            return Response {
                id: req.id,
                result: Some(Value::Null),
                error: None,
            };
        }
    };

    if let Some(doc) = store.documents.get(&params.text_document.uri) {
        let symbols: Vec<DocumentSymbol> = doc.symbols.values().map(|sym| {
            DocumentSymbol {
                name: sym.name.clone(),
                detail: sym.detail.clone(),
                kind: sym.kind,
                tags: None,
                #[allow(deprecated)]
                deprecated: None,
                range: sym.range,
                selection_range: sym.range,
                children: None,
            }
        }).collect();

        return Response {
            id: req.id,
            result: Some(serde_json::to_value(symbols).unwrap_or(Value::Null)),
            error: None,
        };
    }

    Response {
        id: req.id,
        result: Some(Value::Null),
        error: None,
    }
}

// ============================================================================
// INLAY HINTS
// ============================================================================

pub fn handle_inlay_hints(req: Request, store: &DocumentStore) -> Response {
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

    let mut hints = Vec::new();

    // Parse the AST and extract variable declarations with position tracking
    if let Some(ast) = &doc.ast {
        for decl in ast {
            if let Declaration::Function(func) = decl {
                collect_hints_from_statements(&func.body, &doc.content, doc, store, &mut hints);
            }
        }
    }

    Response {
        id: req.id,
        result: serde_json::to_value(hints).ok(),
        error: None,
    }
}

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
    let lines: Vec<&str> = content.lines().collect();

    for (line_num, line) in lines.iter().enumerate() {
        // Look for Zen variable declaration patterns:
        // "var_name = " or "var_name := " or "var_name ::= " or "var_name : Type ="

        // First, check if line contains the variable name
        if let Some(name_pos) = line.find(var_name) {
            // Check if this is actually a variable declaration
            // Must be at start of line or after whitespace
            let before_ok = name_pos == 0 ||
                line.chars().nth(name_pos - 1).map_or(true, |c| c.is_whitespace());

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
    let lines: Vec<&str> = content.lines().collect();

    for (line_num, line) in lines.iter().enumerate() {
        // Look for function call pattern: "func_name("
        if let Some(func_pos) = line.find(func_name) {
            // Check if this is actually a function call (followed by '(')
            let after_func = &line[func_pos + func_name.len()..].trim_start();
            if after_func.starts_with('(') {
                // Find the opening paren position
                let paren_pos = func_pos + func_name.len() + (line[func_pos + func_name.len()..].find('(').unwrap_or(0));

                // Find the nth argument by counting commas
                let mut current_arg = 0;
                let mut depth = 0;
                let mut i = paren_pos + 1;

                while i < line.len() {
                    let c = line.chars().nth(i).unwrap_or('\0');

                    if c == '(' {
                        depth += 1;
                    } else if c == ')' {
                        if depth == 0 {
                            break;
                        }
                        depth -= 1;
                    } else if c == ',' && depth == 0 {
                        current_arg += 1;
                    }

                    if current_arg == arg_index && !c.is_whitespace() && c != '(' {
                        return Some(Position {
                            line: line_num as u32,
                            character: i as u32,
                        });
                    }

                    i += 1;
                }

                // If we're looking for arg 0, return position right after opening paren
                if arg_index == 0 && current_arg == 0 {
                    let mut j = paren_pos + 1;
                    while j < line.len() {
                        let c = line.chars().nth(j).unwrap_or('\0');
                        if !c.is_whitespace() {
                            return Some(Position {
                                line: line_num as u32,
                                character: j as u32,
                            });
                        }
                        j += 1;
                    }
                }
            }
        }
    }

    None
}

// ============================================================================
// CODE LENS
// ============================================================================

pub fn handle_code_lens(req: Request, store: &DocumentStore) -> Response {
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
