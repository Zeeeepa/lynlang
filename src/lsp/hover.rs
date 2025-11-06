// Hover Module for Zen LSP
// Handles textDocument/hover requests

use lsp_server::{Request, Response};
use lsp_types::*;
use serde_json::Value;
use std::collections::HashMap;

use super::types::*;
use super::utils::{format_type, format_symbol_kind};
use super::document_store::DocumentStore;
use super::navigation::find_symbol_at_position;

// ============================================================================
// PUBLIC HANDLER FUNCTION
// ============================================================================

/// Handle textDocument/hover requests
pub fn handle_hover(req: Request, store: &std::sync::Arc<std::sync::Mutex<DocumentStore>>) -> Response {
    let params: HoverParams = match serde_json::from_value(req.params) {
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
    if let Some(doc) = store.documents.get(&params.text_document_position_params.text_document.uri) {
        let position = params.text_document_position_params.position;

        // Find the symbol at the cursor position
        if let Some(symbol_name) = find_symbol_at_position(&doc.content, position) {
            // Check if we're hovering on a method call (e.g., io.println)
            let line = doc.content.lines().nth(position.line as usize).unwrap_or("");
            let char_pos = position.character as usize;
            if let Some(dot_pos) = line[..char_pos.min(line.len())].rfind('.') {
                // We're hovering on a method call - extract receiver.method
                let before_dot = &line[..dot_pos];
                let after_dot = &line[dot_pos + 1..];
                if let Some(method_start) = after_dot.find(|c: char| c.is_alphanumeric() || c == '_') {
                    let method_name = after_dot[method_start..]
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect::<String>();
                    if method_name == symbol_name {
                        // Extract receiver name
                        let receiver = before_dot
                            .chars()
                            .rev()
                            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '.')
                            .collect::<String>()
                            .chars()
                            .rev()
                            .collect::<String>()
                            .trim()
                            .to_string();
                        let full_method = format!("{}.{}", receiver, method_name);
                        
                        // Check stdlib for the full method path
                        if let Some(symbol_info) = store.stdlib_symbols.get(&full_method) {
                            return create_hover_response(req.id, symbol_info, None);
                        }
                        
                        // Also check just the method name in stdlib
                        if let Some(symbol_info) = store.stdlib_symbols.get(&method_name) {
                            return create_hover_response(req.id, symbol_info, None);
                        }
                    }
                }
            }
            
            // Check if we're hovering over a pattern match variable
            if let Some(pattern_hover) = get_pattern_match_hover(&doc.content, position, &symbol_name, &doc.symbols, &store.stdlib_symbols, &store.documents) {
                let contents = HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: pattern_hover,
                });
                return Response {
                    id: req.id,
                    result: Some(serde_json::to_value(Hover {
                        contents,
                        range: None,
                    }).unwrap_or(Value::Null)),
                    error: None,
                };
            }

            // Check if we're hovering over an enum variant definition
            if let Some(enum_hover) = get_enum_variant_hover(&doc.content, position, &symbol_name) {
                let contents = HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: enum_hover,
                });
                return Response {
                    id: req.id,
                    result: Some(serde_json::to_value(Hover {
                        contents,
                        range: None,
                    }).unwrap_or(Value::Null)),
                    error: None,
                };
            }

            // Check for symbol info in current document
            if let Some(symbol_info) = doc.symbols.get(&symbol_name) {
                let mut hover_content = Vec::new();

                // Add type signature
                if let Some(detail) = &symbol_info.detail {
                    hover_content.push(format!("```zen\n{}\n```", detail));
                }

                // Add documentation
                if let Some(doc) = &symbol_info.documentation {
                    hover_content.push(doc.clone());
                }

                // Add type information
                if let Some(type_info) = &symbol_info.type_info {
                    hover_content.push(format!("**Type:** `{}`", format_type(type_info)));
                }

                // Add symbol kind
                hover_content.push(format!("**Kind:** {}", format_symbol_kind(symbol_info.kind)));

                let contents = HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: hover_content.join("\n\n"),
                });

                return Response {
                    id: req.id,
                    result: Some(serde_json::to_value(Hover {
                        contents,
                        range: Some(Range {
                            start: Position {
                                line: position.line,
                                character: position.character.saturating_sub(symbol_name.len() as u32),
                            },
                            end: Position {
                                line: position.line,
                                character: position.character + symbol_name.len() as u32,
                            },
                        }),
                    }).unwrap_or(Value::Null)),
                    error: None,
                };
            }

            // Check stdlib symbols
            if let Some(symbol_info) = store.stdlib_symbols.get(&symbol_name) {
                return create_hover_response(req.id, symbol_info, None);
            }

            // Check other open documents
            for (_uri, other_doc) in &store.documents {
                if let Some(symbol_info) = other_doc.symbols.get(&symbol_name) {
                    let mut hover_content = Vec::new();

                    if let Some(detail) = &symbol_info.detail {
                        hover_content.push(format!("```zen\n{}\n```", detail));
                    }

                    if let Some(type_info) = &symbol_info.type_info {
                        hover_content.push(format!("**Type:** `{}`", format_type(type_info)));
                    }

                    let contents = HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: hover_content.join("\n\n"),
                    });

                    return Response {
                        id: req.id,
                        result: Some(serde_json::to_value(Hover {
                            contents,
                            range: None,
                        }).unwrap_or(Value::Null)),
                        error: None,
                    };
                }
            }

            // Try to infer type from variable assignment in the current file
            if let Some(inferred_type) = infer_variable_type(&doc.content, &symbol_name, &doc.symbols, &store.stdlib_symbols, &store.workspace_symbols) {
                let contents = HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: inferred_type,
                });
                return Response {
                    id: req.id,
                    result: Some(serde_json::to_value(Hover {
                        contents,
                        range: None,
                    }).unwrap_or(Value::Null)),
                    error: None,
                };
            }

            // Provide hover for built-in types and keywords
            let hover_text = get_builtin_hover_text(&symbol_name);
            let contents = HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: hover_text.to_string(),
            });

            return Response {
                id: req.id,
                result: Some(serde_json::to_value(Hover {
                    contents,
                    range: None,
                }).unwrap_or(Value::Null)),
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

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn create_hover_response(id: lsp_server::RequestId, symbol_info: &SymbolInfo, range: Option<Range>) -> Response {
    let mut hover_content = Vec::new();
    if let Some(detail) = &symbol_info.detail {
        hover_content.push(format!("```zen\n{}\n```", detail));
    }
    if let Some(doc) = &symbol_info.documentation {
        hover_content.push(doc.clone());
    }
    if let Some(type_info) = &symbol_info.type_info {
        hover_content.push(format!("**Type:** `{}`", format_type(type_info)));
    }
    hover_content.push("**Source:** Standard Library".to_string());
    
    let contents = HoverContents::Markup(MarkupContent {
        kind: MarkupKind::Markdown,
        value: hover_content.join("\n\n"),
    });
    
    Response {
        id,
        result: Some(serde_json::to_value(Hover {
            contents,
            range,
        }).unwrap_or(Value::Null)),
        error: None,
    }
}

fn get_builtin_hover_text(symbol_name: &str) -> &'static str {
    match symbol_name {
        // Primitive integer types
        "i8" => "```zen\ni8\n```\n\n**Signed 8-bit integer**\n- Range: -128 to 127\n- Size: 1 byte",
        "i16" => "```zen\ni16\n```\n\n**Signed 16-bit integer**\n- Range: -32,768 to 32,767\n- Size: 2 bytes",
        "i32" => "```zen\ni32\n```\n\n**Signed 32-bit integer**\n- Range: -2,147,483,648 to 2,147,483,647\n- Size: 4 bytes",
        "i64" => "```zen\ni64\n```\n\n**Signed 64-bit integer**\n- Range: -9,223,372,036,854,775,808 to 9,223,372,036,854,775,807\n- Size: 8 bytes",
        "u8" => "```zen\nu8\n```\n\n**Unsigned 8-bit integer**\n- Range: 0 to 255\n- Size: 1 byte",
        "u16" => "```zen\nu16\n```\n\n**Unsigned 16-bit integer**\n- Range: 0 to 65,535\n- Size: 2 bytes",
        "u32" => "```zen\nu32\n```\n\n**Unsigned 32-bit integer**\n- Range: 0 to 4,294,967,295\n- Size: 4 bytes",
        "u64" => "```zen\nu64\n```\n\n**Unsigned 64-bit integer**\n- Range: 0 to 18,446,744,073,709,551,615\n- Size: 8 bytes",
        "usize" => "```zen\nusize\n```\n\n**Pointer-sized unsigned integer**\n- Size: Platform dependent (4 or 8 bytes)\n- Used for array indexing and memory offsets",

        // Floating point types
        "f32" => "```zen\nf32\n```\n\n**32-bit floating point**\n- Precision: ~7 decimal digits\n- Size: 4 bytes\n- IEEE 754 single precision",
        "f64" => "```zen\nf64\n```\n\n**64-bit floating point**\n- Precision: ~15 decimal digits\n- Size: 8 bytes\n- IEEE 754 double precision",

        // Boolean
        "bool" => "```zen\nbool\n```\n\n**Boolean type**\n- Values: `true` or `false`\n- Size: 1 byte",

        // Void
        "void" => "```zen\nvoid\n```\n\n**Void type**\n- Represents the absence of a value\n- Used as return type for functions with no return",

        // Option and Result
        "Option" => "```zen\nOption<T>:\n    Some: T,\n    None\n```\n\n**Optional value type**\n- Represents a value that may or may not exist\n- No null/nil in Zen!",
        "Result" => "```zen\nResult<T, E>:\n    Ok: T,\n    Err: E\n```\n\n**Result type for error handling**\n- Represents success (Ok) or failure (Err)\n- Use `.raise()` for error propagation",

        // Collections
        "HashMap" => "```zen\nHashMap<K, V>\n```\n\n**Hash map collection**\n- Key-value storage with O(1) average lookup\n- Requires allocator",
        "DynVec" => "```zen\nDynVec<T>\n```\n\n**Dynamic vector**\n- Growable array\n- Requires allocator",
        "Vec" => "```zen\nVec<T, size>\n```\n\n**Fixed-size vector**\n- Stack-allocated\n- Compile-time size\n- No allocator needed",
        "Array" => "```zen\nArray<T>\n```\n\n**Dynamic array**\n- Requires allocator",

        // String types
        "String" => "```zen\nString\n```\n\n**Dynamic string type**\n- Mutable, heap-allocated\n- Requires allocator",
        "StaticString" => "```zen\nStaticString\n```\n\n**Static string type**\n- Immutable, compile-time\n- No allocator needed",

        // Keywords
        "loop" => "```zen\nloop() { ... }\nloop((handle) { ... })\n(range).loop((i) { ... })\n```\n\n**Loop construct**\n- Internal state management\n- Can provide control handle or iteration values",
        "return" => "```zen\nreturn expr\n```\n\n**Return statement**\n- Returns a value from a function",
        "break" => "```zen\nbreak\n```\n\n**Break statement**\n- Exits the current loop",
        "continue" => "```zen\ncontinue\n```\n\n**Continue statement**\n- Skips to the next loop iteration",

        // Error handling
        "raise" => "```zen\nexpr.raise()\n```\n\n**Error propagation**\n- Unwraps Result<T, E> or returns Err early\n- Equivalent to Rust's `?` operator",

        _ => "Zen language element",
    }
}

fn get_pattern_match_hover(
    _content: &str,
    _position: Position,
    _symbol_name: &str,
    _doc_symbols: &HashMap<String, SymbolInfo>,
    _stdlib_symbols: &HashMap<String, SymbolInfo>,
    _documents: &HashMap<lsp_types::Url, super::types::Document>,
) -> Option<String> {
    // TODO: Implement pattern match hover
    None
}

fn get_enum_variant_hover(_content: &str, _position: Position, _symbol_name: &str) -> Option<String> {
    // TODO: Implement enum variant hover
    None
}

fn infer_variable_type(
    content: &str,
    var_name: &str,
    local_symbols: &HashMap<String, SymbolInfo>,
    stdlib_symbols: &HashMap<String, SymbolInfo>,
    workspace_symbols: &HashMap<String, SymbolInfo>
) -> Option<String> {
    // Look for variable assignment: var_name = function_call() or var_name: Type = ...
    let lines: Vec<&str> = content.lines().collect();

    for line in lines {
        // Pattern: var_name = function_call()
        if line.contains(&format!("{} =", var_name)) || line.contains(&format!("{}=", var_name)) {
            // Check for type annotation first: var_name: Type = ...
            if let Some(colon_pos) = line.find(':') {
                if let Some(eq_pos) = line.find('=') {
                    if colon_pos < eq_pos {
                        // Extract type between : and =
                        let type_str = line[colon_pos + 1..eq_pos].trim();
                        if !type_str.is_empty() {
                            return Some(format!("```zen\n{}: {}\n```\n\n**Type:** `{}`", var_name, type_str, type_str));
                        }
                    }
                }
            }

            // Try to find function call and infer return type
            if let Some(eq_pos) = line.find('=') {
                let rhs = &line[eq_pos + 1..].trim();

                // Check if it's a function call
                if let Some(paren_pos) = rhs.find('(') {
                    let func_name = rhs[..paren_pos].trim();

                    // Look up function in symbols
                    if let Some(func_info) = local_symbols.get(func_name)
                        .or_else(|| stdlib_symbols.get(func_name))
                        .or_else(|| workspace_symbols.get(func_name)) {

                        if let Some(type_info) = &func_info.type_info {
                            let type_str = format_type(type_info);
                            return Some(format!(
                                "```zen\n{} = {}()\n```\n\n**Type:** `{}`\n\n**Inferred from:** `{}` function return type",
                                var_name, func_name, type_str, func_name
                            ));
                        }

                        // Try to parse from detail string
                        if let Some(detail) = &func_info.detail {
                            // Parse "func_name = (args) return_type"
                            if let Some(arrow_or_paren_close) = detail.rfind(')') {
                                let return_part = detail[arrow_or_paren_close + 1..].trim();
                                if !return_part.is_empty() && return_part != "void" {
                                    return Some(format!(
                                        "```zen\n{} = {}()\n```\n\n**Type:** `{}`\n\n**Inferred from:** `{}` function return type",
                                        var_name, func_name, return_part, func_name
                                    ));
                                }
                            }
                        }
                    }
                }

                // Check for constructor calls (Type { ... } or Type(...))
                if let Some(brace_pos) = rhs.find('{') {
                    let type_name = rhs[..brace_pos].trim();
                    if !type_name.is_empty() && type_name.chars().next().unwrap().is_uppercase() {
                        return Some(format!(
                            "```zen\n{} = {} {{ ... }}\n```\n\n**Type:** `{}`\n\n**Inferred from:** constructor",
                            var_name, type_name, type_name
                        ));
                    }
                }

                // Check for literals
                let trimmed = rhs.trim();
                if trimmed.starts_with('"') || trimmed.starts_with('\'') {
                    return Some(format!(
                        "```zen\n{} = {}\n```\n\n**Type:** `StaticString`",
                        var_name, trimmed
                    ));
                }
                if trimmed.parse::<i32>().is_ok() {
                    return Some(format!(
                        "```zen\n{} = {}\n```\n\n**Type:** `i32`",
                        var_name, trimmed
                    ));
                }
                if trimmed.parse::<f64>().is_ok() && trimmed.contains('.') {
                    return Some(format!(
                        "```zen\n{} = {}\n```\n\n**Type:** `f64`",
                        var_name, trimmed
                    ));
                }
                if trimmed == "true" || trimmed == "false" {
                    return Some(format!(
                        "```zen\n{} = {}\n```\n\n**Type:** `bool`",
                        var_name, trimmed
                    ));
                }
            }
        }
    }

    None
}

