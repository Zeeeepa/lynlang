// Hover Module for Zen LSP
// Handles textDocument/hover requests

mod structs;
mod expressions;
mod format_string;

use lsp_server::{Request, Response};
use lsp_types::*;
use serde_json::Value;
use std::collections::HashMap;

use super::types::*;
use super::utils::{format_type, format_symbol_kind};
use super::document_store::DocumentStore;
use super::navigation::find_symbol_at_position;
use super::navigation::find_symbol_definition_in_content;
use crate::ast::AstType;

pub use structs::*;
pub use expressions::*;
pub use format_string::*;

// Re-export main handler and helper functions
pub use handler::{handle_hover, infer_variable_type};

mod handler {
    use super::*;

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

        let store = match store.lock() {
            Ok(s) => s,
            Err(_) => {
                return Response { id: req.id, result: Some(Value::Null), error: None };
            }
        };
        if let Some(doc) = store.documents.get(&params.text_document_position_params.text_document.uri) {
            let position = params.text_document_position_params.position;

            // Find the symbol at the cursor position
            if let Some(symbol_name) = find_symbol_at_position(&doc.content, position) {
                // Check if we're hovering on a method call (e.g., io.println)
                // Optimized: get line directly without creating full iterator
                let line = doc.content
                    .lines()
                    .nth(position.line as usize)
                    .unwrap_or("");
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
                            // Extract receiver name - optimized: find last valid identifier
                            let receiver = before_dot
                                .trim_end()
                                .rsplit(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
                                .next()
                                .unwrap_or("")
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

                // Check if we're hovering on a format string field access (e.g., ${person.name})
                if let Some(format_hover) = format_string::get_format_string_field_hover(&doc.content, position, &symbol_name, &doc.symbols, &store) {
                    let contents = HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format_hover,
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
                    let mut hover_content = Vec::with_capacity(4); // Pre-allocate for common case

                    // Add type signature
                    if let Some(detail) = &symbol_info.detail {
                        hover_content.push(format!("```zen\n{}\n```", detail));
                    }

                    // Add documentation
                    if let Some(doc) = &symbol_info.documentation {
                        hover_content.push(doc.clone());
                    }

                    // Add type information - if it's a struct type, show the struct definition with fields
                    if let Some(type_info) = &symbol_info.type_info {
                        if let AstType::Struct { name, .. } = type_info {
                            // Try to find the struct definition and show its fields
                            if let Some(struct_def) = structs::find_struct_definition(name, &doc, &store) {
                                hover_content.push(format!("```zen\n{}\n```", structs::format_struct_definition(&struct_def)));
                            } else {
                                hover_content.push(format!("**Type:** `{}`", format_type(type_info)));
                            }
                        } else {
                            hover_content.push(format!("**Type:** `{}`", format_type(type_info)));
                        }
                    } else if symbol_info.kind == SymbolKind::STRUCT {
                        // If it's a struct symbol but no type_info, try to find the struct definition
                        if let Some(struct_def) = structs::find_struct_definition(&symbol_name, &doc, &store) {
                            hover_content.clear(); // Clear the generic detail
                            hover_content.push(format!("```zen\n{}\n```", structs::format_struct_definition(&struct_def)));
                        }
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

                // Check other open documents - limit search for performance
                const MAX_DOCS_HOVER_SEARCH: usize = 10;
                for (_uri, other_doc) in store.documents.iter().take(MAX_DOCS_HOVER_SEARCH) {
                    if let Some(symbol_info) = other_doc.symbols.get(&symbol_name) {
                        let mut hover_content = Vec::with_capacity(3); // Pre-allocate

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

                // Check for imports (e.g., { io } = @std)
                if let Some(import_info) = find_import_info(&doc.content, &symbol_name, position) {
                    let mut hover_content = Vec::with_capacity(4); // Pre-allocate
                    hover_content.push(format!("```zen\n{}\n```", import_info.import_line));
                    hover_content.push(format!("**Imported from:** `{}`", import_info.source));
                    
                    // Try to find the actual definition in stdlib
                    if let Some(symbol_info) = store.stdlib_symbols.get(&symbol_name) {
                        if let Some(detail) = &symbol_info.detail {
                            hover_content.insert(0, format!("```zen\n{}\n```", detail));
                        }
                        if let Some(def_uri) = &symbol_info.definition_uri {
                            if let Some(path) = def_uri.to_file_path().ok() {
                                hover_content.push(format!("**Definition:** `{}`", path.display()));
                            }
                        }
                        if let Some(type_info) = &symbol_info.type_info {
                            hover_content.push(format!("**Type:** `{}`", format_type(type_info)));
                        }
                    } else {
                        // If not in stdlib, it might be a module namespace
                        hover_content.push("**Type:** Module/Namespace".to_string());
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

                // Check for built-in types FIRST (before fallback text search)
                let builtin_text = get_builtin_hover_text(&symbol_name);
                if builtin_text != "Zen language element" {
                    let contents = HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: builtin_text.to_string(),
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

                // Try to infer type from variable assignment in the current file
                // Limit search to prevent performance issues
                if let Some(inferred_type) = infer_variable_type(&doc.content, &symbol_name, &doc.symbols, &store.stdlib_symbols, &store.workspace_symbols, Some(&store.documents)) {
                    // Check if the inferred type is a struct and enhance the hover
                    let enhanced_type = if let Some(struct_name) = structs::extract_struct_name_from_type(&inferred_type) {
                        if let Some(struct_def) = structs::find_struct_definition(&struct_name, &doc, &store) {
                            format!("```zen\n{}\n```", structs::format_struct_definition(&struct_def))
                        } else {
                            inferred_type
                        }
                    } else {
                        inferred_type
                    };
                    
                    let contents = HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: enhanced_type,
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

                // Find definition location with context
                if let Some(range) = find_symbol_definition_in_content(&doc.content, &symbol_name) {
                    // Optimized: get line directly without collecting all lines
                    let line_text = doc.content
                        .lines()
                        .nth(range.start.line as usize)
                        .map(|l| l.trim())
                        .unwrap_or("");
                    let file_name = params.text_document_position_params.text_document.uri
                        .path_segments()
                        .and_then(|s| s.last())
                        .unwrap_or("unknown");
                    
                    let mut hover_content = Vec::with_capacity(3); // Pre-allocate
                    hover_content.push(format!("```zen\n{}\n```", line_text));
                    hover_content.push(format!("**Location:** `{}:{}`", file_name, range.start.line + 1));
                    
                    // Try to extract more context
                    if let Some(type_str) = extract_type_from_line(line_text) {
                        hover_content.push(format!("**Type:** `{}`", type_str));
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

                // Final fallback
                let contents = HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Zen language element".to_string(),
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

    fn create_hover_response(id: lsp_server::RequestId, symbol_info: &SymbolInfo, range: Option<Range>) -> Response {
        let mut hover_content = Vec::with_capacity(5); // Pre-allocate for common case
        if let Some(detail) = &symbol_info.detail {
            hover_content.push(format!("```zen\n{}\n```", detail));
        }
        if let Some(doc) = &symbol_info.documentation {
            hover_content.push(doc.clone());
        }
        if let Some(type_info) = &symbol_info.type_info {
            hover_content.push(format!("**Type:** `{}`", format_type(type_info)));
        }
        
        // Add location information
        if let Some(def_uri) = &symbol_info.definition_uri {
            if let Ok(path) = def_uri.to_file_path() {
                hover_content.push(format!("**Definition:** `{}:{}`", 
                    path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown"),
                    symbol_info.range.start.line + 1));
            } else {
                hover_content.push("**Source:** Standard Library".to_string());
            }
        } else {
            hover_content.push("**Source:** Standard Library".to_string());
        }
        
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

    pub fn get_builtin_hover_text(symbol_name: &str) -> &'static str {
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

    pub fn get_pattern_match_hover(
        content: &str,
        position: Position,
        symbol_name: &str,
        _local_symbols: &HashMap<String, SymbolInfo>,
        _stdlib_symbols: &HashMap<String, SymbolInfo>,
        all_docs: &HashMap<lsp_types::Url, super::super::types::Document>,
    ) -> Option<String> {
        use super::super::type_inference::infer_function_return_types;
        
        let lines: Vec<&str> = content.lines().collect();
        if position.line as usize >= lines.len() {
            return None;
        }

        let current_line = lines[position.line as usize];

        // Check if we're in a pattern match arm (contains '|' and '{')
        if !current_line.contains('|') {
            return None;
        }

        // Find the scrutinee by looking backwards for 'variable ?'
        let mut scrutinee_name = None;
        let mut scrutinee_line = None;
        for i in (0..=position.line).rev() {
            let line = lines[i as usize].trim();
            if line.contains('?') && !line.starts_with("//") {
                // Found the pattern match - extract variable name
                if let Some(q_pos) = line.find('?') {
                    let before_q = line[..q_pos].trim();
                    // Get the last word before '?'
                    if let Some(var) = before_q.split_whitespace().last() {
                        scrutinee_name = Some(var.to_string());
                        scrutinee_line = Some(i);
                        break;
                    }
                }
            }
            // Don't search too far back
            if position.line - i > 10 {
                break;
            }
        }

        if let Some(scrutinee) = scrutinee_name {
            // Try to infer the type of the scrutinee by looking at its definition
            if let Some(scrutinee_line_num) = scrutinee_line {
                // Look backwards from scrutinee for its definition
                for i in (0..scrutinee_line_num).rev() {
                    let line = lines[i as usize];
                    // Check if this line defines the scrutinee
                    if line.contains(&format!("{} =", scrutinee)) {
                        // Try to infer type from the assignment
                        // Example: result = divide(10.0, 2.0)
                        if let Some(eq_pos) = line.find('=') {
                            let rhs = line[eq_pos+1..].trim();

                            // Check if it's a function call
                            if let Some(paren_pos) = rhs.find('(') {
                                let func_name = rhs[..paren_pos].trim();

                                // Try to find the function definition and extract its return type
                                let (concrete_ok_type, concrete_err_type) = infer_function_return_types(func_name, all_docs);

                                // Now determine what pattern variable we're hovering over
                                // Example: | Ok(val) or | Err(msg)
                                let pattern_arm = current_line.trim();

                                if pattern_arm.contains(&format!("Ok({}", symbol_name)) || pattern_arm.contains(&format!("Ok({})", symbol_name)) {
                                    // This is the Ok variant - extract the success type
                                    let type_display = concrete_ok_type.clone().unwrap_or_else(|| "T".to_string());
                                    let full_result_type = if let (Some(ok), Some(err)) = (&concrete_ok_type, &concrete_err_type) {
                                        format!("Result<{}, {}>", ok, err)
                                    } else {
                                        "Result<T, E>".to_string()
                                    };

                                    return Some(format!(
                                        "```zen\n{}: {}\n```\n\n**Pattern match variable**\n\nExtracted from `{}` (assigned from `{}()`)\n\nThis is the success value from the `Ok` variant.",
                                        symbol_name,
                                        type_display,
                                        full_result_type,
                                        func_name
                                    ));
                                } else if pattern_arm.contains(&format!("Err({}", symbol_name)) || pattern_arm.contains(&format!("Err({})", symbol_name)) {
                                    // This is the Err variant - extract the error type
                                    let type_display = concrete_err_type.clone().unwrap_or_else(|| "E".to_string());
                                    let full_result_type = if let (Some(ok), Some(err)) = (&concrete_ok_type, &concrete_err_type) {
                                        format!("Result<{}, {}>", ok, err)
                                    } else {
                                        "Result<T, E>".to_string()
                                    };

                                    return Some(format!(
                                        "```zen\n{}: {}\n```\n\n**Pattern match variable**\n\nExtracted from `{}` (assigned from `{}()`)\n\nThis is the error value from the `Err` variant.",
                                        symbol_name,
                                        type_display,
                                        full_result_type,
                                        func_name
                                    ));
                                } else if pattern_arm.contains(&format!("Some({}", symbol_name)) || pattern_arm.contains(&format!("Some({})", symbol_name)) {
                                    // This is Option.Some
                                    let inner_type = concrete_ok_type.clone().unwrap_or_else(|| "T".to_string());
                                    return Some(format!(
                                        "```zen\n{}: {}\n```\n\n**Pattern match variable**\n\nExtracted from `Option<{}>` (assigned from `{}()`)\n\nThis is the value from the `Some` variant.",
                                        symbol_name,
                                        inner_type,
                                        inner_type,
                                        func_name
                                    ));
                                }
                            }
                        }
                    }
                    // Don't search too far back
                    if scrutinee_line_num - i > 20 {
                        break;
                    }
                }
            }
        }

        None
    }

    pub fn get_enum_variant_hover(content: &str, position: Position, symbol_name: &str) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();
        if position.line as usize >= lines.len() {
            return None;
        }

        let current_line = lines[position.line as usize];

        // Check if this line is an enum variant (has ':' after the symbol)
        if !current_line.contains(&format!("{}:", symbol_name)) && !current_line.contains(&format!("{},", symbol_name)) {
            return None;
        }

        // Find the enum name by looking backwards
        let mut enum_name = None;
        for i in (0..position.line).rev() {
            let line = lines[i as usize].trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            // Check if this line is an enum definition (identifier followed by ':' at end)
            if line.ends_with(':') && !line.contains("::") {
                let name = line.trim_end_matches(':').trim();
                if name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '<' || c == '>' || c == ',') {
                    enum_name = Some(name.to_string());
                    break;
                }
            }
            // If we hit something that's not a variant, stop
            if !line.contains(':') && !line.contains(',') {
                break;
            }
        }

        if let Some(enum_name) = enum_name {
            // Extract variant payload info from current line
            let payload_info = if current_line.contains('{') {
                // Struct-like payload
                let start = current_line.find('{')?;
                let end = current_line.rfind('}')?;
                let fields = &current_line[start+1..end];
                format!(" with fields: `{}`", fields.trim())
            } else if current_line.contains(": ") {
                // Type payload
                let parts: Vec<&str> = current_line.split(':').collect();
                if parts.len() >= 2 {
                    let type_part = parts[1].trim().trim_end_matches(',');
                    format!(" of type `{}`", type_part)
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            Some(format!(
                "```zen\n{}\n    {}...\n```\n\n**Enum variant** `{}`{}\n\nPart of enum `{}`",
                enum_name,
                symbol_name,
                symbol_name,
                payload_info,
                enum_name.split('<').next().unwrap_or(&enum_name)
            ))
        } else {
            None
        }
    }

    // Helper to find import information for a symbol
    struct ImportInfo {
        import_line: String,
        source: String,
    }

    fn find_import_info(content: &str, symbol_name: &str, position: Position) -> Option<ImportInfo> {
        let lines: Vec<&str> = content.lines().collect();
        
        // Search backwards from current position for import statements
        let start_line = (position.line as usize).min(lines.len().saturating_sub(1));
        for i in (0..=start_line).rev() {
            let line = lines[i].trim();
            
            // Look for pattern: { symbol_name } = @std or { symbol_name, ... } = @std
            if line.starts_with('{') && line.contains('}') && line.contains('=') {
                // Extract the import pattern
                if let Some(brace_end) = line.find('}') {
                    let import_part = &line[1..brace_end];
                    let imports: Vec<&str> = import_part.split(',').map(|s| s.trim()).collect();
                    
                    if imports.contains(&symbol_name) {
                        // Extract the source (after =)
                        if let Some(eq_pos) = line.find('=') {
                            let source = line[eq_pos + 1..].trim();
                            return Some(ImportInfo {
                                import_line: line.to_string(),
                                source: source.to_string(),
                            });
                        }
                    }
                }
            }
        }
        
        None
    }

    // Extract type from a definition line
    fn extract_type_from_line(line: &str) -> Option<String> {
        // Look for type annotations: name: Type or name: Type =
        if let Some(colon_pos) = line.find(':') {
            let after_colon = &line[colon_pos + 1..];
            // Extract type (stop at =, {, or end)
            let type_end = after_colon.find('=')
                .or_else(|| after_colon.find('{'))
                .or_else(|| after_colon.find('('))
                .unwrap_or(after_colon.len());
            let type_str = after_colon[..type_end].trim();
            if !type_str.is_empty() {
                return Some(type_str.to_string());
            }
        }
        None
    }

    pub fn infer_variable_type(
        content: &str,
        var_name: &str,
        local_symbols: &HashMap<String, SymbolInfo>,
        stdlib_symbols: &HashMap<String, SymbolInfo>,
        workspace_symbols: &HashMap<String, SymbolInfo>,
        documents: Option<&HashMap<lsp_types::Url, Document>>,
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
                            // Try to find the struct definition to show its fields
                            let type_display = if let Some(docs) = documents {
                                if let Some(struct_def) = structs::find_struct_definition_in_documents(type_name, docs) {
                                    format!("```zen\n{}\n```", structs::format_struct_definition(&struct_def))
                                } else {
                                    format!("```zen\n{} = {} {{ ... }}\n```\n\n**Type:** `{}`\n\n**Inferred from:** constructor", var_name, type_name, type_name)
                                }
                            } else {
                                format!("```zen\n{} = {} {{ ... }}\n```\n\n**Type:** `{}`\n\n**Inferred from:** constructor", var_name, type_name, type_name)
                            };
                            return Some(type_display);
                        }
                    }
                    // Also check for constructor calls with parentheses: Type(...)
                    if let Some(paren_pos) = rhs.find('(') {
                        let type_name = rhs[..paren_pos].trim();
                        if !type_name.is_empty() && type_name.chars().next().unwrap().is_uppercase() {
                            // Try to find the struct definition to show its fields
                            let type_display = if let Some(docs) = documents {
                                if let Some(struct_def) = structs::find_struct_definition_in_documents(type_name, docs) {
                                    format!("```zen\n{}\n```", structs::format_struct_definition(&struct_def))
                                } else {
                                    format!("```zen\n{} = {}()\n```\n\n**Type:** `{}`\n\n**Inferred from:** constructor", var_name, type_name, type_name)
                                }
                            } else {
                                format!("```zen\n{} = {}()\n```\n\n**Type:** `{}`\n\n**Inferred from:** constructor", var_name, type_name, type_name)
                            };
                            return Some(type_display);
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
}

