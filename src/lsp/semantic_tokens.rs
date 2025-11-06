// Semantic Tokens Module for Zen LSP
// Handles textDocument/semanticTokens/full requests

use lsp_server::{Request, Response, ResponseError, ErrorCode};
use lsp_types::*;
use serde_json::Value;

use super::document_store::DocumentStore;

// ============================================================================
// PUBLIC HANDLER FUNCTION
// ============================================================================

/// Handle textDocument/semanticTokens/full requests
pub fn handle_semantic_tokens(req: Request, store: &std::sync::Arc<std::sync::Mutex<DocumentStore>>) -> Response {
    let params: SemanticTokensParams = match serde_json::from_value(req.params) {
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

    let store = store.lock().unwrap();
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

    // Generate semantic tokens for the document
    let tokens = generate_semantic_tokens(&doc.content);

    let result = SemanticTokens {
        result_id: None,
        data: tokens,
    };

    Response {
        id: req.id,
        result: Some(serde_json::to_value(result).unwrap_or(Value::Null)),
        error: None,
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn generate_semantic_tokens(content: &str) -> Vec<SemanticToken> {
    let mut tokens = Vec::new();

    // Token type indices (must match the legend in server capabilities)
    const TYPE_NAMESPACE: u32 = 0;
    const TYPE_TYPE: u32 = 1;
    const TYPE_CLASS: u32 = 2;
    const TYPE_ENUM: u32 = 3;
    const TYPE_INTERFACE: u32 = 4;
    const TYPE_STRUCT: u32 = 5;
    const TYPE_TYPE_PARAM: u32 = 6;
    const TYPE_PARAMETER: u32 = 7;
    const TYPE_VARIABLE: u32 = 8;
    const TYPE_PROPERTY: u32 = 9;
    const TYPE_ENUM_MEMBER: u32 = 10;
    const TYPE_EVENT: u32 = 11;
    const TYPE_FUNCTION: u32 = 12;
    const TYPE_METHOD: u32 = 13;
    const TYPE_MACRO: u32 = 14;
    const TYPE_KEYWORD: u32 = 15;
    const TYPE_MODIFIER: u32 = 16;
    const TYPE_COMMENT: u32 = 17;
    const TYPE_STRING: u32 = 18;
    const TYPE_NUMBER: u32 = 19;
    const TYPE_REGEXP: u32 = 20;
    const TYPE_OPERATOR: u32 = 21;

    // Token modifiers (can be combined)
    const MOD_DECLARATION: u32 = 0b1;
    const MOD_DEFINITION: u32 = 0b10;
    const MOD_READONLY: u32 = 0b100;
    const MOD_STATIC: u32 = 0b1000;
    const MOD_DEPRECATED: u32 = 0b10000;
    const MOD_ABSTRACT: u32 = 0b100000;
    const MOD_ASYNC: u32 = 0b1000000;
    const MOD_MODIFICATION: u32 = 0b10000000;
    const MOD_DOCUMENTATION: u32 = 0b100000000;
    const MOD_DEFAULT_LIBRARY: u32 = 0b1000000000;

    let mut prev_line = 0;
    let mut prev_start = 0;
    let lines: Vec<&str> = content.lines().collect();

    // Track context for better token classification
    let mut in_function = false;
    let mut _in_struct = false;
    let mut _in_allocator_context = false;

    for (line_idx, line) in lines.iter().enumerate() {
        let mut char_idx = 0;
        let mut chars = line.chars().peekable();

        while let Some(ch) = chars.next() {
            let start = char_idx;
            char_idx += ch.len_utf8();

            // Skip whitespace
            if ch.is_whitespace() {
                continue;
            }

            // Comments
            if ch == '/' && chars.peek() == Some(&'/') {
                // Single-line comment
                let length = line.len() - start;
                let delta_line = line_idx as u32 - prev_line;
                let delta_start = if delta_line == 0 {
                    start as u32 - prev_start
                } else {
                    start as u32
                };

                tokens.push(SemanticToken {
                    delta_line,
                    delta_start,
                    length: length as u32,
                    token_type: TYPE_COMMENT,
                    token_modifiers_bitset: 0,
                });

                prev_line = line_idx as u32;
                prev_start = start as u32;
                break; // Rest of line is comment
            }

            // String literals
            if ch == '"' {
                let mut string_len = 1;
                let mut escaped = false;
                while let Some(&next_ch) = chars.peek() {
                    chars.next();
                    char_idx += next_ch.len_utf8();
                    string_len += next_ch.len_utf8();

                    if escaped {
                        escaped = false;
                    } else if next_ch == '\\' {
                        escaped = true;
                    } else if next_ch == '"' {
                        break;
                    }
                }

                let delta_line = line_idx as u32 - prev_line;
                let delta_start = if delta_line == 0 {
                    start as u32 - prev_start
                } else {
                    start as u32
                };

                tokens.push(SemanticToken {
                    delta_line,
                    delta_start,
                    length: string_len as u32,
                    token_type: TYPE_STRING,
                    token_modifiers_bitset: 0,
                });

                prev_line = line_idx as u32;
                prev_start = start as u32;
                continue;
            }

            // Numbers
            if ch.is_numeric() {
                let mut num_len = ch.len_utf8();
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_numeric() || next_ch == '.' || next_ch == '_' {
                        chars.next();
                        char_idx += next_ch.len_utf8();
                        num_len += next_ch.len_utf8();
                    } else {
                        break;
                    }
                }

                let delta_line = line_idx as u32 - prev_line;
                let delta_start = if delta_line == 0 {
                    start as u32 - prev_start
                } else {
                    start as u32
                };

                tokens.push(SemanticToken {
                    delta_line,
                    delta_start,
                    length: num_len as u32,
                    token_type: TYPE_NUMBER,
                    token_modifiers_bitset: 0,
                });

                prev_line = line_idx as u32;
                prev_start = start as u32;
                continue;
            }

            // Identifiers and keywords
            if ch.is_alphabetic() || ch == '_' {
                let mut word = String::from(ch);
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_alphanumeric() || next_ch == '_' {
                        chars.next();
                        char_idx += next_ch.len_utf8();
                        word.push(next_ch);
                    } else {
                        break;
                    }
                }

                // Check if this is a UFC method call by looking ahead for '.'
                let is_ufc_call = chars.peek() == Some(&'.');

                // Check if this is allocator-related
                let is_allocator_related = word.contains("allocator") || word == "alloc" || word == "dealloc";

                let (token_type, modifiers) = match word.as_str() {
                    // Keywords
                    "fn" => {
                        in_function = true;
                        (TYPE_KEYWORD, 0)
                    }
                    "struct" => {
                        _in_struct = true;
                        (TYPE_KEYWORD, 0)
                    }
                    "enum" => (TYPE_KEYWORD, 0),
                    "let" | "mut" | "const" => (TYPE_KEYWORD, 0),
                    "if" | "else" | "match" | "while" | "for" | "loop" | "break" | "continue" => (TYPE_KEYWORD, 0),
                    "return" => (TYPE_KEYWORD, 0),
                    "raise" => (TYPE_KEYWORD, MOD_ASYNC), // Special highlighting for error propagation
                    "import" | "export" | "pub" => (TYPE_KEYWORD, 0),
                    "true" | "false" | "null" => (TYPE_KEYWORD, 0),

                    // Built-in types
                    "i8" | "i16" | "i32" | "i64" | "i128" |
                    "u8" | "u16" | "u32" | "u64" | "u128" |
                    "f32" | "f64" | "bool" | "void" => (TYPE_TYPE, MOD_DEFAULT_LIBRARY),

                    // Zen-specific types
                    "String" | "StaticString" => (TYPE_TYPE, MOD_DEFAULT_LIBRARY),
                    "Option" | "Result" => (TYPE_ENUM, MOD_DEFAULT_LIBRARY),
                    "HashMap" | "DynVec" | "Vec" | "Array" | "HashSet" => {
                        _in_allocator_context = true; // These types need allocators
                        (TYPE_CLASS, MOD_DEFAULT_LIBRARY)
                    }
                    "Allocator" => (TYPE_INTERFACE, MOD_DEFAULT_LIBRARY | MOD_ABSTRACT),

                    // Allocator-related functions (highlight specially)
                    "get_default_allocator" => (TYPE_FUNCTION, MOD_DEFAULT_LIBRARY | MOD_STATIC),

                    // Enum variants
                    "Some" | "None" | "Ok" | "Err" => (TYPE_ENUM_MEMBER, 0),

                    // Function names (when we know we're after 'fn')
                    _ if in_function && prev_line == line_idx as u32 => {
                        in_function = false;
                        (TYPE_FUNCTION, MOD_DECLARATION | MOD_DEFINITION)
                    }

                    // Allocator-related identifiers get special highlighting
                    _ if is_allocator_related => (TYPE_INTERFACE, MOD_ABSTRACT),

                    // UFC calls get method highlighting
                    _ if is_ufc_call => (TYPE_VARIABLE, 0), // Will be followed by method

                    // Default to variable
                    _ => (TYPE_VARIABLE, 0),
                };

                let delta_line = line_idx as u32 - prev_line;
                let delta_start = if delta_line == 0 {
                    start as u32 - prev_start
                } else {
                    start as u32
                };

                tokens.push(SemanticToken {
                    delta_line,
                    delta_start,
                    length: word.len() as u32,
                    token_type,
                    token_modifiers_bitset: modifiers,
                });

                prev_line = line_idx as u32;
                prev_start = start as u32;
                continue;
            }

            // Handle dot operator specially for UFC method calls
            if ch == '.' {
                // Mark the dot operator
                let delta_line = line_idx as u32 - prev_line;
                let delta_start = if delta_line == 0 {
                    start as u32 - prev_start
                } else {
                    start as u32
                };

                tokens.push(SemanticToken {
                    delta_line,
                    delta_start,
                    length: 1,
                    token_type: TYPE_OPERATOR,
                    token_modifiers_bitset: 0,
                });

                prev_line = line_idx as u32;
                prev_start = start as u32;

                // Look ahead for method name after dot
                if let Some(&next_ch) = chars.peek() {
                    if next_ch.is_alphabetic() || next_ch == '_' {
                        // Skip the dot we just processed
                        let method_start = char_idx;
                        let mut method_name = String::new();

                        while let Some(&ch) = chars.peek() {
                            if ch.is_alphanumeric() || ch == '_' {
                                chars.next();
                                char_idx += ch.len_utf8();
                                method_name.push(ch);
                            } else {
                                break;
                            }
                        }

                        // Add the method name token with special highlighting
                        let delta_line = 0;  // Same line as dot
                        let delta_start = method_start as u32 - prev_start;

                        // Determine if this is a special method
                        let is_allocator_method = method_name.contains("alloc");
                        let is_error_method = method_name == "raise";

                        let (token_type, modifiers) = if is_error_method {
                            (TYPE_METHOD, MOD_ASYNC) // Special for error propagation
                        } else if is_allocator_method {
                            (TYPE_METHOD, MOD_ABSTRACT) // Special for allocator methods
                        } else {
                            (TYPE_METHOD, 0) // Regular UFC method
                        };

                        tokens.push(SemanticToken {
                            delta_line,
                            delta_start,
                            length: method_name.len() as u32,
                            token_type,
                            token_modifiers_bitset: modifiers,
                        });

                        prev_line = line_idx as u32;
                        prev_start = method_start as u32;
                    }
                }
                continue;
            }

            // Other operators
            if "+-*/%&|^!<>=".contains(ch) {
                let delta_line = line_idx as u32 - prev_line;
                let delta_start = if delta_line == 0 {
                    start as u32 - prev_start
                } else {
                    start as u32
                };

                tokens.push(SemanticToken {
                    delta_line,
                    delta_start,
                    length: 1,
                    token_type: TYPE_OPERATOR,
                    token_modifiers_bitset: 0,
                });

                prev_line = line_idx as u32;
                prev_start = start as u32;
            }
        }
    }

    tokens
}

