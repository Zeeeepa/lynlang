// Hover Module for Zen LSP
// Handles textDocument/hover requests

mod structs;
mod expressions;
mod format_string;
mod builtins;
mod patterns;
mod imports;
mod inference;
mod response;

use lsp_server::{Request, Response};
use lsp_types::*;
use serde_json::Value;

use super::types::*;
use super::utils::{format_type, format_symbol_kind};
use super::document_store::DocumentStore;
use super::navigation::find_symbol_at_position;
use super::navigation::find_symbol_definition_in_content;
use super::compiler_integration::CompilerIntegration;
use crate::ast::{AstType, Program};

pub use structs::*;
pub use expressions::*;
pub use format_string::*;
pub use inference::infer_variable_type;

// Re-export main handler
pub use handler::handle_hover;

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
            
            // Find the symbol at the cursor position FIRST (needed for format string check)
            if let Some(symbol_name) = find_symbol_at_position(&doc.content, position) {
                let request_id = req.id.clone();
                
                // PRIORITY 1: Check if we're hovering on a format string field access (e.g., ${person.name})
                // This MUST happen before string literal check, otherwise we'll show "String literal" for expressions inside ${...}
                if let Some(format_hover) = format_string::get_format_string_field_hover(&doc.content, position, &symbol_name, &doc.symbols, &store) {
                    return create_hover_response_from_string(request_id.clone(), format_hover);
                }
                
                // Check if we're hovering on a method call (e.g., io.println)
                if let Some(response) = handle_method_call_hover(&doc.content, position, &symbol_name, &store, request_id.clone()) {
                    return response;
                }
                
                // Check if we're hovering over a pattern match variable
                if let Some(pattern_hover) = patterns::get_pattern_match_hover(&doc.content, position, &symbol_name, &doc.symbols, &store.stdlib_symbols, &store.documents) {
                    return create_hover_response_from_string(request_id.clone(), pattern_hover);
                }

                // Check if we're hovering over an enum variant definition
                if let Some(enum_hover) = patterns::get_enum_variant_hover(&doc.content, position, &symbol_name) {
                    return create_hover_response_from_string(request_id.clone(), enum_hover);
                }

                // Check for symbol info in current document
                if let Some(response) = handle_symbol_hover(&symbol_name, &doc, &store, position, request_id.clone()) {
                    return response;
                }

                // Check stdlib symbols
                if let Some(symbol_info) = store.stdlib_symbols.get(&symbol_name) {
                    return response::create_hover_response(request_id.clone(), symbol_info, None);
                }

                // Check other open documents - limit search for performance
                if let Some(response) = handle_other_documents_hover(&symbol_name, &store, request_id.clone()) {
                    return response;
                }

                // Check for imports (e.g., { io } = @std)
                if let Some(response) = handle_import_hover(&symbol_name, &doc.content, position, &store, request_id.clone()) {
                    return response;
                }

                // Check for built-in types FIRST (before fallback text search)
                let builtin_text = builtins::get_builtin_hover_text(&symbol_name);
                if builtin_text != "Zen language element" {
                    return create_hover_response_from_string(request_id.clone(), builtin_text.to_string());
                }

                // Try to infer type from variable assignment in the current file
                if let Some(response) = handle_inferred_type_hover(&symbol_name, &doc, &store, request_id.clone()) {
                    return response;
                }

                // Find definition location with context
                if let Some(response) = handle_definition_hover(&symbol_name, &doc.content, &params.text_document_position_params.text_document.uri, request_id.clone()) {
                    return response;
                }

                // Try to parse and infer type using compiler integration
                if let Some(response) = handle_expression_type_inference(&symbol_name, &doc, &store, position, request_id.clone()) {
                    return response;
                }

                // If we found a symbol but couldn't provide useful hover info, show debug info
                let debug_info = format!(
                    "**Symbol:** `{}`\n\n**Status:** No hover information available\n\n*Checked:* symbols, stdlib, imports, builtins, type inference, compiler",
                    symbol_name
                );
                return create_hover_response_from_string(request_id, debug_info);
            }
            // If no symbol found at position (empty line, whitespace, etc.), return null
        }

        Response {
            id: req.id,
            result: Some(Value::Null),
            error: None,
        }
    }

    fn handle_method_call_hover(
        content: &str,
        position: Position,
        symbol_name: &str,
        store: &DocumentStore,
        request_id: lsp_server::RequestId,
    ) -> Option<Response> {
        let line = content
            .lines()
            .nth(position.line as usize)
            .unwrap_or("");
        let char_pos = position.character as usize;
        
        if let Some(dot_pos) = line[..char_pos.min(line.len())].rfind('.') {
            let before_dot = &line[..dot_pos];
            let after_dot = &line[dot_pos + 1..];
            
            if let Some(method_start) = after_dot.find(|c: char| c.is_alphanumeric() || c == '_') {
                let method_name = after_dot[method_start..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect::<String>();
                    
                if method_name == symbol_name {
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
                        return Some(response::create_hover_response(request_id, symbol_info, None));
                    }
                    
                    // Also check just the method name in stdlib
                    if let Some(symbol_info) = store.stdlib_symbols.get(&method_name) {
                        return Some(response::create_hover_response(request_id, symbol_info, None));
                    }
                }
            }
        }
        None
    }

    fn handle_symbol_hover(
        symbol_name: &str,
        doc: &Document,
        store: &DocumentStore,
        position: Position,
        request_id: lsp_server::RequestId,
    ) -> Option<Response> {
        if let Some(symbol_info) = doc.symbols.get(symbol_name) {
            let mut hover_content = Vec::with_capacity(4);

            if let Some(detail) = &symbol_info.detail {
                hover_content.push(format!("```zen\n{}\n```", detail));
            }

            if let Some(doc) = &symbol_info.documentation {
                hover_content.push(doc.clone());
            }

            // Add type information - if it's a struct type, show the struct definition with fields
            if let Some(type_info) = &symbol_info.type_info {
                if let AstType::Struct { name, .. } = type_info {
                    if let Some(struct_def) = structs::find_struct_definition(name, doc, store) {
                        hover_content.push(format!("```zen\n{}\n```", structs::format_struct_definition(&struct_def)));
                    } else {
                        hover_content.push(format!("**Type:** `{}`", format_type(type_info)));
                    }
                } else {
                    hover_content.push(format!("**Type:** `{}`", format_type(type_info)));
                }
            } else if symbol_info.kind == SymbolKind::STRUCT {
                if let Some(struct_def) = structs::find_struct_definition(symbol_name, doc, store) {
                    hover_content.clear();
                    hover_content.push(format!("```zen\n{}\n```", structs::format_struct_definition(&struct_def)));
                }
            }

            hover_content.push(format!("**Kind:** {}", format_symbol_kind(symbol_info.kind)));

            let contents = HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: hover_content.join("\n\n"),
            });

            return Some(Response {
                id: request_id,
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
            });
        }
        None
    }

    fn handle_other_documents_hover(
        symbol_name: &str,
        store: &DocumentStore,
        request_id: lsp_server::RequestId,
    ) -> Option<Response> {
        const MAX_DOCS_HOVER_SEARCH: usize = 10;
        for (_uri, other_doc) in store.documents.iter().take(MAX_DOCS_HOVER_SEARCH) {
            if let Some(symbol_info) = other_doc.symbols.get(symbol_name) {
                let mut hover_content = Vec::with_capacity(3);

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

                return Some(Response {
                    id: request_id,
                    result: Some(serde_json::to_value(Hover {
                        contents,
                        range: None,
                    }).unwrap_or(Value::Null)),
                    error: None,
                });
            }
        }
        None
    }

    fn handle_import_hover(
        symbol_name: &str,
        content: &str,
        position: Position,
        store: &DocumentStore,
        request_id: lsp_server::RequestId,
    ) -> Option<Response> {
        if let Some(import_info) = imports::find_import_info(content, symbol_name, position) {
            let mut hover_content = Vec::with_capacity(4);
            hover_content.push(format!("```zen\n{}\n```", import_info.import_line));
            hover_content.push(format!("**Imported from:** `{}`", import_info.source));
            
            if let Some(symbol_info) = store.stdlib_symbols.get(symbol_name) {
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
                hover_content.push("**Type:** Module/Namespace".to_string());
            }
            
            return Some(create_hover_response_from_string(request_id, hover_content.join("\n\n")));
        }
        None
    }

    fn handle_inferred_type_hover(
        symbol_name: &str,
        doc: &Document,
        store: &DocumentStore,
        request_id: lsp_server::RequestId,
    ) -> Option<Response> {
        if let Some(inferred_type) = inference::infer_variable_type(&doc.content, symbol_name, &doc.symbols, &store.stdlib_symbols, &store.workspace_symbols, Some(&store.documents)) {
            let enhanced_type = if let Some(struct_name) = structs::extract_struct_name_from_type(&inferred_type) {
                if let Some(struct_def) = structs::find_struct_definition(&struct_name, doc, store) {
                    format!("```zen\n{}\n```", structs::format_struct_definition(&struct_def))
                } else {
                    inferred_type
                }
            } else {
                inferred_type
            };
            
            return Some(create_hover_response_from_string(request_id, enhanced_type));
        }
        None
    }

    fn handle_definition_hover(
        symbol_name: &str,
        content: &str,
        uri: &lsp_types::Url,
        request_id: lsp_server::RequestId,
    ) -> Option<Response> {
        if let Some(range) = find_symbol_definition_in_content(content, symbol_name) {
            let line_text = content
                .lines()
                .nth(range.start.line as usize)
                .map(|l| l.trim())
                .unwrap_or("");
            let file_name = uri
                .path_segments()
                .and_then(|s| s.last())
                .unwrap_or("unknown");
            
            let mut hover_content = Vec::with_capacity(3);
            hover_content.push(format!("```zen\n{}\n```", line_text));
            hover_content.push(format!("**Location:** `{}:{}`", file_name, range.start.line + 1));
            
            if let Some(type_str) = inference::extract_type_from_line(line_text) {
                hover_content.push(format!("**Type:** `{}`", type_str));
            }
            
            return Some(create_hover_response_from_string(request_id, hover_content.join("\n\n")));
        }
        None
    }

    fn handle_expression_type_inference(
        symbol_name: &str,
        doc: &Document,
        _store: &DocumentStore,
        position: Position,
        request_id: lsp_server::RequestId,
    ) -> Option<Response> {
        // Try to parse the symbol as an expression and infer its type using compiler
        let line = doc.content
            .lines()
            .nth(position.line as usize)
            .unwrap_or("");
        
        let char_pos = position.character as usize;
        
        // Check if we're inside a string literal (but NOT inside ${...})
        if !is_inside_format_expression(line, char_pos) {
            if let Some(string_literal) = extract_string_literal_from_line(line, char_pos) {
                return Some(create_hover_response_from_string(
                    request_id,
                    format!("```zen\n{}\n```\n\n**Type:** `StaticString`\n\n**Literal:** String literal", string_literal)
                ));
            }
        }
        
        // Try to find the full expression containing this symbol
        let trimmed_symbol = symbol_name.trim();
        
        // Check if it's a string literal (with quotes)
        if trimmed_symbol.starts_with('"') && trimmed_symbol.ends_with('"') {
            return Some(create_hover_response_from_string(
                request_id,
                format!("```zen\n{}\n```\n\n**Type:** `StaticString`\n\n**Literal:** String literal", trimmed_symbol)
            ));
        }
        
        // Check if it's a number
        if trimmed_symbol.parse::<i64>().is_ok() {
            return Some(create_hover_response_from_string(
                request_id,
                format!("```zen\n{}\n```\n\n**Type:** `i32`\n\n**Literal:** Integer literal", trimmed_symbol)
            ));
        }
        
        if trimmed_symbol.parse::<f64>().is_ok() && trimmed_symbol.contains('.') {
            return Some(create_hover_response_from_string(
                request_id,
                format!("```zen\n{}\n```\n\n**Type:** `f64`\n\n**Literal:** Floating point literal", trimmed_symbol)
            ));
        }
        
        // Check if it's a boolean
        if trimmed_symbol == "true" || trimmed_symbol == "false" {
            return Some(create_hover_response_from_string(
                request_id,
                format!("```zen\n{}\n```\n\n**Type:** `bool`\n\n**Literal:** Boolean literal", trimmed_symbol)
            ));
        }
        
        // Try to parse as expression and use compiler integration
        if let Some(ast) = &doc.ast {
            let program = Program {
                declarations: ast.clone(),
                statements: vec![],
            };
            
            let mut compiler = CompilerIntegration::new();
            
            // Try parsing the symbol as an expression
            if let Ok(expr_type) = compiler.infer_expression_type_from_string(&program, symbol_name) {
                return Some(create_hover_response_from_string(
                    request_id,
                    format!("```zen\n{}\n```\n\n**Type:** `{}`\n\n*Inferred using compiler*", symbol_name, format_type(&expr_type))
                ));
            }
            
            // Try to find the expression in context
            if let Some(expr_str) = extract_expression_from_line(line, char_pos, symbol_name) {
                if let Ok(expr_type) = compiler.infer_expression_type_from_string(&program, &expr_str) {
                    return Some(create_hover_response_from_string(
                        request_id,
                        format!("```zen\n{}\n```\n\n**Type:** `{}`\n\n*Inferred using compiler*", expr_str, format_type(&expr_type))
                    ));
                }
            }
        }
        
        None
    }
    
    /// Check if cursor is inside a format expression ${...}
    fn is_inside_format_expression(line: &str, char_pos: usize) -> bool {
        // Find the nearest ${ before the cursor
        let mut search_pos = 0;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 100;
        
        while iterations < MAX_ITERATIONS {
            iterations += 1;
            
            let search_range = search_pos..char_pos.min(line.len());
            if search_range.is_empty() {
                break;
            }
            
            let dollar_pos = if let Some(pos) = line[search_range].rfind('$') {
                search_pos + pos
            } else {
                break;
            };
            
            // Check if it's escaped
            if dollar_pos > 0 && line.as_bytes()[dollar_pos - 1] == b'\\' {
                search_pos = dollar_pos.saturating_sub(1);
                continue;
            }
            
            // Check if it's ${...
            if dollar_pos + 1 < line.len() && line.as_bytes()[dollar_pos + 1] == b'{' {
                // Find the closing }
                if let Some(close_brace) = line[dollar_pos + 2..].find('}') {
                    let expr_end = dollar_pos + 2 + close_brace;
                    // Check if cursor is between ${ and }
                    if char_pos > dollar_pos && char_pos <= expr_end + 1 {
                        return true;
                    }
                } else {
                    // No closing brace found, but we're inside ${...
                    if char_pos > dollar_pos {
                        return true;
                    }
                }
            }
            
            if dollar_pos == 0 {
                break;
            }
            search_pos = dollar_pos.saturating_sub(1);
        }
        
        false
    }
    
    /// Extract string literal if cursor is inside one (public for use in handler)
    fn extract_string_literal_from_line(line: &str, char_pos: usize) -> Option<String> {
        let mut in_string = false;
        let mut string_start = 0;
        let mut escape_next = false;
        
        for (i, ch) in line.char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }
            
            if ch == '\\' {
                escape_next = true;
                continue;
            }
            
            if ch == '"' {
                if !in_string {
                    // Start of string
                    in_string = true;
                    string_start = i;
                } else {
                    // End of string
                    // Include boundaries: if cursor is at or between the quotes
                    if char_pos >= string_start && char_pos <= i {
                        // We're inside this string literal (including on the quotes)
                        return Some(line[string_start..=i].to_string());
                    }
                    in_string = false;
                }
            }
        }
        
        // If we're still in a string at the end, check if cursor is in it
        if in_string && char_pos >= string_start {
            return Some(line[string_start..].to_string());
        }
        
        None
    }
    
    fn extract_expression_from_line(_line: &str, _char_pos: usize, symbol_name: &str) -> Option<String> {
        // Try to extract a valid expression containing the symbol
        // If the symbol itself looks like a literal, return it
        if symbol_name.starts_with('"') && symbol_name.ends_with('"') {
            return Some(symbol_name.to_string());
        }
        
        // For now, just return the symbol itself - the parser will handle it
        Some(symbol_name.to_string())
    }

    fn create_hover_response_from_string(id: lsp_server::RequestId, content: String) -> Response {
        let contents = HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: content,
        });

        Response {
            id,
            result: Some(serde_json::to_value(Hover {
                contents,
                range: None,
            }).unwrap_or(Value::Null)),
            error: None,
        }
    }
}
