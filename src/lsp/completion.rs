// Completion functionality for Zen Language Server
// Provides code completion with type-aware method suggestions

use lsp_server::{Request, Response};
use lsp_types::*;
use serde_json::Value;
use std::collections::HashMap;

// Import from other LSP modules
use super::types::{SymbolInfo, ZenCompletionContext};
use super::document_store::DocumentStore;
use super::utils::{symbol_kind_to_completion_kind, format_type};

// ============================================================================
// PUBLIC COMPLETION HANDLER
// ============================================================================

/// Main completion request handler
pub fn handle_completion(req: Request, store: &DocumentStore) -> Response {
    let params: CompletionParams = match serde_json::from_value(req.params) {
        Ok(p) => p,
        Err(_) => {
            return Response {
                id: req.id,
                result: Some(Value::Null),
                error: None,
            };
        }
    };

    // Check if we're completing after a dot (UFC method call)
    if let Some(doc) = store.documents.get(&params.text_document_position.text_document.uri) {
        let position = params.text_document_position.position;

        if let Some(context) = get_completion_context(&doc.content, position, store) {
            match context {
                ZenCompletionContext::UfcMethod { receiver_type } => {
                    // Provide UFC method completions
                    let completions = get_ufc_method_completions(&receiver_type);

                    let response = CompletionResponse::Array(completions);
                    return Response {
                        id: req.id,
                        result: Some(serde_json::to_value(response).unwrap_or(Value::Null)),
                        error: None,
                    };
                }
                ZenCompletionContext::General => {
                    // Fall through to provide general completions
                }
            }
        }
    }

    // Provide general completions
    let mut completions = vec![
        // Keywords
        CompletionItem {
            label: "main".to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("main = () i32 { ... }".to_string()),
            documentation: Some(Documentation::String("Entry point function".to_string())),
            ..Default::default()
        },
        CompletionItem {
            label: "loop".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("loop() { ... }".to_string()),
            documentation: Some(Documentation::String("Infinite loop with break statement".to_string())),
            ..Default::default()
        },
        CompletionItem {
            label: "return".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("return value".to_string()),
            documentation: Some(Documentation::String("Return from function".to_string())),
            ..Default::default()
        },
        CompletionItem {
            label: "break".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("break".to_string()),
            documentation: Some(Documentation::String("Break from loop".to_string())),
            ..Default::default()
        },
        CompletionItem {
            label: "continue".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("continue".to_string()),
            documentation: Some(Documentation::String("Continue to next iteration".to_string())),
            ..Default::default()
        },
        // Common types
        CompletionItem {
            label: "Option".to_string(),
            kind: Some(CompletionItemKind::ENUM),
            detail: Some("Option<T>".to_string()),
            documentation: Some(Documentation::String("Optional value type".to_string())),
            ..Default::default()
        },
        CompletionItem {
            label: "Result".to_string(),
            kind: Some(CompletionItemKind::ENUM),
            detail: Some("Result<T, E>".to_string()),
            documentation: Some(Documentation::String("Result type for error handling".to_string())),
            ..Default::default()
        },
        CompletionItem {
            label: "Some".to_string(),
            kind: Some(CompletionItemKind::ENUM_MEMBER),
            detail: Some("Some(value)".to_string()),
            documentation: Some(Documentation::String("Option variant with value".to_string())),
            ..Default::default()
        },
        CompletionItem {
            label: "None".to_string(),
            kind: Some(CompletionItemKind::ENUM_MEMBER),
            detail: Some("None".to_string()),
            documentation: Some(Documentation::String("Option variant without value".to_string())),
            ..Default::default()
        },
        CompletionItem {
            label: "Ok".to_string(),
            kind: Some(CompletionItemKind::ENUM_MEMBER),
            detail: Some("Ok(value)".to_string()),
            documentation: Some(Documentation::String("Success variant of Result".to_string())),
            ..Default::default()
        },
        CompletionItem {
            label: "Err".to_string(),
            kind: Some(CompletionItemKind::ENUM_MEMBER),
            detail: Some("Err(error)".to_string()),
            documentation: Some(Documentation::String("Error variant of Result".to_string())),
            ..Default::default()
        },
        // Collections
        CompletionItem {
            label: "Vec".to_string(),
            kind: Some(CompletionItemKind::STRUCT),
            detail: Some("Vec<T, size>".to_string()),
            documentation: Some(Documentation::String("Fixed-size vector (stack allocated)".to_string())),
            ..Default::default()
        },
        CompletionItem {
            label: "DynVec".to_string(),
            kind: Some(CompletionItemKind::STRUCT),
            detail: Some("DynVec<T>".to_string()),
            documentation: Some(Documentation::String("Dynamic vector (requires allocator)".to_string())),
            ..Default::default()
        },
        CompletionItem {
            label: "HashMap".to_string(),
            kind: Some(CompletionItemKind::STRUCT),
            detail: Some("HashMap<K, V>".to_string()),
            documentation: Some(Documentation::String("Hash map (requires allocator)".to_string())),
            ..Default::default()
        },
        // Module imports
        CompletionItem {
            label: "@std".to_string(),
            kind: Some(CompletionItemKind::MODULE),
            detail: Some("Standard library".to_string()),
            documentation: Some(Documentation::String("Import standard library modules".to_string())),
            ..Default::default()
        },
        CompletionItem {
            label: "@this".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Current module reference".to_string()),
            documentation: Some(Documentation::String("Reference to current module".to_string())),
            ..Default::default()
        },
    ];

    // Add primitive types
    for ty in ["i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64", "bool"] {
        completions.push(CompletionItem {
            label: ty.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(format!("{} type", ty)),
            documentation: None,
            ..Default::default()
        });
    }

    // Add document symbols (functions, structs, enums defined in current file)
    if let Some(doc) = store.documents.get(&params.text_document_position.text_document.uri) {
        for (name, symbol) in &doc.symbols {
            completions.push(CompletionItem {
                label: name.clone(),
                kind: Some(symbol_kind_to_completion_kind(symbol.kind)),
                detail: symbol.detail.clone(),
                documentation: None,
                ..Default::default()
            });
        }
    }

    // Add stdlib symbols (functions and types from standard library)
    for (name, symbol) in &store.stdlib_symbols {
        completions.push(CompletionItem {
            label: name.clone(),
            kind: Some(symbol_kind_to_completion_kind(symbol.kind)),
            detail: symbol.detail.clone(),
            documentation: None,
            ..Default::default()
        });
    }

    // Add workspace symbols (from other files in the project)
    // Limit to prevent overwhelming the user
    let mut workspace_count = 0;
    const MAX_WORKSPACE_COMPLETIONS: usize = 50;

    for (name, symbol) in &store.workspace_symbols {
        if workspace_count >= MAX_WORKSPACE_COMPLETIONS {
            break;
        }

        // Only add if not already in completions (avoid duplicates)
        if !completions.iter().any(|c| c.label == *name) {
            completions.push(CompletionItem {
                label: name.clone(),
                kind: Some(symbol_kind_to_completion_kind(symbol.kind)),
                detail: symbol.detail.clone(),
                documentation: None,
                ..Default::default()
            });
            workspace_count += 1;
        }
    }

    let response = CompletionResponse::Array(completions);

    Response {
        id: req.id,
        result: Some(serde_json::to_value(response).unwrap_or(Value::Null)),
        error: None,
    }
}

// ============================================================================
// CONTEXT DETECTION
// ============================================================================

fn get_completion_context(content: &str, position: Position, store: &DocumentStore) -> Option<ZenCompletionContext> {
    let lines: Vec<&str> = content.lines().collect();
    if position.line as usize >= lines.len() {
        return Some(ZenCompletionContext::General);
    }

    let line = lines[position.line as usize];
    let char_pos = position.character as usize;

    // Check if we're after a dot
    if char_pos > 0 && line.chars().nth(char_pos - 1) == Some('.') {
        // Extract the receiver expression before the dot
        let chars: Vec<char> = line.chars().collect();
        let mut start = if char_pos > 1 { char_pos - 2 } else { 0 };
        let mut paren_depth = 0;

        // Find the start of the receiver expression
        while start > 0 {
            match chars[start] {
                ')' => paren_depth += 1,
                '(' => {
                    if paren_depth > 0 {
                        paren_depth -= 1;
                    } else {
                        break;
                    }
                }
                ' ' | '\t' | '=' | '{' | '[' | ',' | ';' if paren_depth == 0 => {
                    start += 1;
                    break;
                }
                _ => {}
            }
            start -= 1;
        }

        let receiver: String = chars[start..(char_pos - 1)].iter().collect();
        let receiver = receiver.trim();

        // Try to infer the receiver type
        let receiver_type = infer_receiver_type(receiver, store)
            .unwrap_or_else(|| "unknown".to_string());

        return Some(ZenCompletionContext::UfcMethod {
            receiver_type,
        });
    }

    Some(ZenCompletionContext::General)
}

// ============================================================================
// TYPE INFERENCE
// ============================================================================

fn parse_generic_type(type_str: &str) -> (String, Vec<String>) {
    // Parse a generic type like HashMap<K, V> into ("HashMap", ["K", "V"])
    if let Some(angle_pos) = type_str.find('<') {
        let base = type_str[..angle_pos].to_string();
        let params_end = type_str.rfind('>').unwrap_or(type_str.len());
        let params_str = &type_str[angle_pos + 1..params_end];

        let params = if params_str.is_empty() {
            Vec::new()
        } else {
            // Handle nested generics by tracking bracket depth
            let mut params = Vec::new();
            let mut current = String::new();
            let mut depth = 0;

            for ch in params_str.chars() {
                match ch {
                    '<' => {
                        depth += 1;
                        current.push(ch);
                    }
                    '>' => {
                        depth -= 1;
                        current.push(ch);
                    }
                    ',' if depth == 0 => {
                        params.push(current.trim().to_string());
                        current.clear();
                    }
                    _ => current.push(ch),
                }
            }

            if !current.trim().is_empty() {
                params.push(current.trim().to_string());
            }

            params
        };

        (base, params)
    } else {
        (type_str.to_string(), Vec::new())
    }
}

pub fn infer_variable_type(
    content: &str,
    var_name: &str,
    local_symbols: &HashMap<String, SymbolInfo>,
    stdlib_symbols: &HashMap<String, SymbolInfo>,
    workspace_symbols: &HashMap<String, SymbolInfo>
) -> Option<String> {
    // Look for variable assignment: var_name = function_call() or var_name: Type = ...
    let lines: Vec<&str> = content.lines().collect();

    // Limit to first 1000 lines to prevent performance issues
    let max_lines = lines.len().min(1000);
    for line in lines.iter().take(max_lines) {
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

fn infer_receiver_type(receiver: &str, store: &DocumentStore) -> Option<String> {
    // Enhanced type inference for UFC method resolution with nested generic support

    // Check if receiver is a string literal
    if receiver.starts_with('"') || receiver.starts_with("'") {
        return Some("String".to_string());
    }

    // Check for numeric literals
    if receiver.chars().all(|c| c.is_numeric() || c == '.' || c == '-') {
        if receiver.contains('.') {
            return Some("f64".to_string());
        } else {
            return Some("i32".to_string());
        }
    }

    // Check if receiver is a known variable in symbols (limit search)
    const MAX_DOCS_FOR_TYPE_INFERENCE: usize = 20;
    for doc in store.documents.values().take(MAX_DOCS_FOR_TYPE_INFERENCE) {
        if let Some(symbol) = doc.symbols.get(receiver) {
            if let Some(type_info) = &symbol.type_info {
                return Some(format_type(type_info));
            }
            // Enhanced detail parsing for better type inference
            if let Some(detail) = &symbol.detail {
                // Parse function return types
                if detail.contains(" = ") && detail.contains(")") {
                    if let Some(return_type) = detail.split(" = ").nth(1) {
                        if let Some(ret) = return_type.split(')').nth(1).map(|s| s.trim()) {
                            if !ret.is_empty() && ret != "void" {
                                return Some(ret.to_string());
                            }
                        }
                    }
                }
                // Check for collection types with generics
                if let Some(cap) = regex::Regex::new(r"(HashMap|DynVec|Vec|Array|Option|Result)<[^>]+>")
                    .ok()?.captures(detail) {
                    return Some(cap[1].to_string());
                }
                // Fallback to simple contains checks
                for type_name in ["HashMap", "DynVec", "Vec", "Array", "Option", "Result", "String", "StaticString"] {
                    if detail.contains(type_name) {
                        return Some(type_name.to_string());
                    }
                }
            }
        }
    }

    // Enhanced pattern matching for function calls and constructors
    let patterns = [
        (r"HashMap\s*\(", "HashMap"),
        (r"DynVec\s*\(", "DynVec"),
        (r"Vec\s*[<(]", "Vec"),
        (r"Array\s*\(", "Array"),
        (r"Some\s*\(", "Option"),
        (r"None", "Option"),
        (r"Ok\s*\(", "Result"),
        (r"Err\s*\(", "Result"),
        (r"Result\.", "Result"),
        (r"Option\.", "Option"),
        (r"get_default_allocator\s*\(\)", "Allocator"),
        (r"\[\s*\d+\s*;\s*\d+\s*\]", "Array"), // Fixed array syntax
    ];

    for (pattern, type_name) in patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if re.is_match(receiver) {
                return Some(type_name.to_string());
            }
        }
    }

    // Enhanced support for method call chains (e.g., foo.bar().baz())
    if receiver.contains('.') && receiver.contains('(') {
        return infer_chained_method_type(receiver, store);
    }

    None
}

fn infer_chained_method_type(receiver: &str, store: &DocumentStore) -> Option<String> {
    // Parse method chains from left to right, tracking type through each call
    let mut current_type: Option<String> = None;
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut paren_depth = 0;
    let mut in_string = false;

    // Split by dots, respecting parentheses and strings
    for ch in receiver.chars() {
        match ch {
            '"' => {
                in_string = !in_string;
                current.push(ch);
            }
            '(' if !in_string => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' if !in_string => {
                paren_depth -= 1;
                current.push(ch);
            }
            '.' if !in_string && paren_depth == 0 => {
                if !current.is_empty() {
                    parts.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }

    // Process each part of the chain
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            // First part - infer its base type
            current_type = infer_base_expression_type(part, store);
        } else if let Some(ref curr_type) = current_type {
            // Subsequent parts are method calls on the current type
            if let Some(method_name) = part.split('(').next() {
                current_type = get_method_return_type(curr_type, method_name);
            }
        }
    }

    current_type
}

fn infer_base_expression_type(expr: &str, store: &DocumentStore) -> Option<String> {
    // Infer type of base expression (before any method calls)
    let expr = expr.trim();

    // String literal
    if expr.starts_with('"') {
        return Some("String".to_string());
    }

    // Numeric literal
    if expr.parse::<i64>().is_ok() {
        return Some("i32".to_string());
    }
    if expr.parse::<f64>().is_ok() {
        return Some("f64".to_string());
    }

    // Constructor calls
    if let Some(type_name) = expr.split('(').next() {
        match type_name {
            "HashMap" => return Some("HashMap".to_string()),
            "DynVec" => return Some("DynVec".to_string()),
            "Vec" => return Some("Vec".to_string()),
            "Array" => return Some("Array".to_string()),
            "Some" => return Some("Option".to_string()),
            "None" => return Some("Option".to_string()),
            "Ok" => return Some("Result".to_string()),
            "Err" => return Some("Result".to_string()),
            "get_default_allocator" => return Some("Allocator".to_string()),
            _ => {
                // Check if it's a variable (limit search for performance)
                const MAX_DOCS_SEARCH: usize = 20;
                for doc in store.documents.values().take(MAX_DOCS_SEARCH) {
                    if let Some(symbol) = doc.symbols.get(type_name) {
                        if let Some(type_info) = &symbol.type_info {
                            let type_str = format_type(type_info);
                            if let Some(base) = type_str.split('<').next() {
                                return Some(base.to_string());
                            }
                            return Some(type_str);
                        }
                    }
                }
            }
        }
    }

    // Fixed array syntax [size; type]
    if expr.starts_with('[') && expr.contains(';') {
        return Some("Array".to_string());
    }

    None
}

fn get_method_return_type(receiver_type: &str, method_name: &str) -> Option<String> {
    // Comprehensive method return type mapping
    match receiver_type {
        "String" | "StaticString" | "str" => match method_name {
            "to_string" | "to_upper" | "to_lower" | "trim" | "concat" |
            "replace" | "substr" | "reverse" | "repeat" => Some("String".to_string()),
            "to_i32" | "to_i64" | "to_f64" => Some("Option".to_string()),
            "split" => Some("Array".to_string()),
            "len" | "index_of" => Some("i32".to_string()),
            "char_at" => Some("Option".to_string()),
            "contains" | "starts_with" | "ends_with" | "is_empty" => Some("bool".to_string()),
            "to_bytes" => Some("Array".to_string()),
            _ => None,
        },
        "HashMap" => match method_name {
            "get" | "remove" => Some("Option".to_string()),
            "keys" | "values" => Some("Array".to_string()),
            "len" | "capacity" => Some("i32".to_string()),
            "contains_key" | "is_empty" => Some("bool".to_string()),
            "insert" => Some("Option".to_string()), // Returns old value if any
            _ => None,
        },
        "DynVec" | "Vec" => match method_name {
            "get" | "pop" | "first" | "last" => Some("Option".to_string()),
            "len" | "capacity" => Some("i32".to_string()),
            "is_empty" | "is_full" | "contains" => Some("bool".to_string()),
            _ => None,
        },
        "Array" => match method_name {
            "get" | "pop" | "first" | "last" => Some("Option".to_string()),
            "len" => Some("i32".to_string()),
            "is_empty" | "contains" => Some("bool".to_string()),
            "slice" => Some("Array".to_string()),
            _ => None,
        },
        "Option" => match method_name {
            "is_some" | "is_none" => Some("bool".to_string()),
            "unwrap" | "unwrap_or" | "expect" => Some("T".to_string()), // Would need generics tracking
            "map" => Some("Option".to_string()),
            "and" | "or" => Some("Option".to_string()),
            _ => None,
        },
        "Result" => match method_name {
            "is_ok" | "is_err" => Some("bool".to_string()),
            "unwrap" | "raise" | "expect" | "unwrap_or" => Some("T".to_string()),
            "map" => Some("Result".to_string()),
            "map_err" => Some("Result".to_string()),
            _ => None,
        },
        "Allocator" => match method_name {
            "alloc" => Some("*mut u8".to_string()),
            "clone" => Some("Allocator".to_string()),
            _ => None,
        },
        _ => None,
    }
}

pub fn find_stdlib_location(stdlib_path: &str, method_name: &str, store: &DocumentStore) -> Option<Location> {
    // Try to find the method in the stdlib file
    // First check if we have the stdlib file open
    for (uri, doc) in &store.documents {
        if uri.path().contains(stdlib_path) {
            // Look for the method in this file's symbols
            if let Some(symbol) = doc.symbols.get(method_name) {
                return Some(Location {
                    uri: uri.clone(),
                    range: symbol.range.clone(),
                });
            }
        }
    }

    // If not found in open documents, we could potentially open and parse the stdlib file
    // For now, return None to indicate it's a built-in method
    None
}

// ============================================================================
// UFC METHOD COMPLETIONS
// ============================================================================

fn get_ufc_method_completions(_receiver_type: &str) -> Vec<CompletionItem> {
    // TODO: Query real method definitions from compiler/AST instead of hardcoded data
    // For now, return empty until we implement proper method introspection
    Vec::new()
}
