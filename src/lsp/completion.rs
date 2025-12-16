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
use super::stdlib_resolver::StdlibResolver;

// ============================================================================
// PUBLIC COMPLETION HANDLER
// ============================================================================

/// Main completion request handler
pub fn handle_completion(req: Request, store: &std::sync::Arc<std::sync::Mutex<DocumentStore>>) -> Response {
    let store_guard = match store.lock() {
        Ok(guard) => guard,
        Err(_) => {
            // Return an empty completion response rather than panicking on poisoned lock
            let empty = CompletionResponse::Array(Vec::new());
            return Response {
                id: req.id,
                result: Some(serde_json::to_value(empty).unwrap_or(Value::Null)),
                error: None,
            };
        }
    };
    let store = &*store_guard;
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
                    // Provide struct field completions first (most relevant)
                    let mut completions = get_struct_field_completions(&receiver_type, store);
                    
                    // Only add UFC methods if we didn't find struct fields (to avoid stdlib clutter)
                    // Or if receiver_type is not a struct name (starts with uppercase)
                    if completions.is_empty() || !receiver_type.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                        completions.extend(get_ufc_method_completions(&receiver_type, store));
                    }

                    let response = CompletionResponse::Array(completions);
                    return Response {
                        id: req.id,
                        result: Some(serde_json::to_value(response).unwrap_or(Value::Null)),
                        error: None,
                    };
                }
                ZenCompletionContext::ModulePath { base } => {
                    // Provide module path completions (e.g., @std.io, @std.types)
                    let completions = get_module_path_completions(&base, store);

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

    // Add primitive types (including StaticString - always available)
    for ty in ["i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "usize", "f32", "f64", "bool", "StaticString", "void"] {
        completions.push(CompletionItem {
            label: ty.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(format!("{} - Built-in primitive type", ty)),
            documentation: Some(Documentation::String(format!("Built-in primitive type `{}`. Always available, no import needed.", ty))),
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

    // Check if we're completing after @std. (module path completion)
    if char_pos > 5 {
        let before_cursor = &line[..char_pos.min(line.len())];
        if before_cursor.ends_with("@std.") || before_cursor.contains("@std.") {
            // Check if we're right after @std.
            if let Some(std_pos) = before_cursor.rfind("@std.") {
                let after_std = &before_cursor[std_pos + 5..];
                // If there's no dot after @std., we're completing module names
                if !after_std.contains('.') {
                    return Some(ZenCompletionContext::ModulePath {
                        base: "@std".to_string(),
                    });
                }
            }
        }
    }

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
    
    // First check workspace symbols (faster lookup)
    if let Some(symbol) = store.workspace_symbols.get(receiver) {
        if let Some(type_info) = &symbol.type_info {
            return Some(format_type(type_info));
        }
    }
    
    // Then check document symbols
    for doc in store.documents.values().take(MAX_DOCS_FOR_TYPE_INFERENCE) {
        if let Some(symbol) = doc.symbols.get(receiver) {
            if let Some(type_info) = &symbol.type_info {
                return Some(format_type(type_info));
            }
            // Enhanced detail parsing for better type inference
            if let Some(detail) = &symbol.detail {
                // Parse variable type annotations: "name: Type"
                if let Some(colon_pos) = detail.find(':') {
                    let type_part = detail[colon_pos + 1..].trim();
                    // Extract type name (stop at space, comma, or end)
                    let type_end = type_part.find(' ')
                        .or_else(|| type_part.find(','))
                        .or_else(|| type_part.find('='))
                        .unwrap_or(type_part.len());
                    let type_name = type_part[..type_end].trim();
                    if !type_name.is_empty() {
                        // Check if it's a struct name (starts with uppercase)
                        if type_name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                            return Some(type_name.to_string());
                        }
                    }
                }
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
                // Check for collection types with generics - use simple string matching instead of regex
                for type_name in ["HashMap<", "DynVec<", "Vec<", "Array<", "Option<", "Result<"] {
                    if detail.contains(type_name) {
                        return Some(type_name.trim_end_matches('<').to_string());
                    }
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

    // Enhanced pattern matching for function calls and constructors - optimized: use string matching instead of regex
    let receiver_trim = receiver.trim();
    if receiver_trim.starts_with("HashMap(") || receiver_trim.starts_with("HashMap<") {
        return Some("HashMap".to_string());
    }
    if receiver_trim.starts_with("DynVec(") || receiver_trim.starts_with("DynVec<") {
        return Some("DynVec".to_string());
    }
    if receiver_trim.starts_with("Vec(") || receiver_trim.starts_with("Vec<") {
        return Some("Vec".to_string());
    }
    if receiver_trim.starts_with("Array(") || receiver_trim.starts_with("Array<") {
        return Some("Array".to_string());
    }
    if receiver_trim.starts_with("Some(") {
        return Some("Option".to_string());
    }
    if receiver_trim == "None" {
        return Some("Option".to_string());
    }
    if receiver_trim.starts_with("Ok(") || receiver_trim.starts_with("Err(") {
        return Some("Result".to_string());
    }
    if receiver_trim.starts_with("Result.") {
        return Some("Result".to_string());
    }
    if receiver_trim.starts_with("Option.") {
        return Some("Option".to_string());
    }
    if receiver_trim.starts_with("get_default_allocator()") {
        return Some("Allocator".to_string());
    }
    if receiver_trim.starts_with('[') && receiver_trim.contains(';') && receiver_trim.ends_with(']') {
        return Some("Array".to_string());
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
// MODULE PATH COMPLETIONS
// ============================================================================

fn get_module_path_completions(base: &str, store: &DocumentStore) -> Vec<CompletionItem> {
    let mut completions = Vec::new();
    
    if base == "@std" {
        // Get actual modules from stdlib resolver (dynamic based on file structure)
        let available_modules = store.stdlib_resolver.list_modules();
        
        for module_name in available_modules {
            completions.push(CompletionItem {
                label: format!("{}.{}", base, module_name),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some(format!("Import from {} module", module_name)),
                documentation: Some(Documentation::String(format!("Standard library module: {}", module_name))),
                ..Default::default()
            });
        }
        
        // Fallback to hardcoded list if resolver found nothing (for backwards compatibility)
        if completions.is_empty() {
            // Provide comprehensive stdlib module completions
            // Organized by category for better UX
            let modules = vec![
            // Core modules
            ("io", "IO module - console I/O operations (println, read, etc.)"),
            ("types", "Types module - StaticString and other type definitions"),
            ("core", "Core module - Option, Result, and fundamental types"),
            
            // Data structures
            ("collections", "Collections module - HashMap, List, Queue, Set, Stack"),
            
            // System & utilities
            ("fs", "File system module - file operations"),
            ("net", "Network module - networking functionality"),
            ("sys", "System module - system-level operations"),
            ("process", "Process module - process management"),
            ("env", "Environment module - environment variables"),
            ("path", "Path module - path manipulation"),
            
            // Math & algorithms
            ("math", "Math module - mathematical functions"),
            ("algorithm", "Algorithm module - common algorithms"),
            ("random", "Random module - random number generation"),
            
            // Text & encoding
            ("string", "String module - string operations"),
            ("text", "Text module - text processing utilities"),
            ("regex", "Regex module - regular expressions"),
            ("encoding", "Encoding module - encoding/decoding"),
            
            // Data formats
            ("json", "JSON module - JSON parsing and generation"),
            ("url", "URL module - URL parsing and manipulation"),
            
            // Time & date
            ("time", "Time module - time operations"),
            ("datetime", "DateTime module - date and time handling"),
            
            // Concurrency & memory
            ("memory_unified", "Memory module - unified memory management"),
            ("memory_virtual", "Memory module - virtual memory"),
            ("allocator_async", "Async allocator module"),
            ("concurrent_unified", "Concurrency module - unified concurrency primitives"),
            
            // Testing & development
            ("testing", "Testing module - test framework"),
            ("assert", "Assert module - assertion utilities"),
            ("log", "Log module - logging functionality"),
            
            // Build & meta
            ("build", "Build module - build system"),
            ("build_enhanced", "Enhanced build module"),
            ("package", "Package module - package management"),
            ("meta", "Meta module - metaprogramming"),
            
            // Other
            ("error", "Error module - error handling"),
            ("iterator", "Iterator module - iterator utilities"),
            ("behaviors", "Behaviors module - behavioral patterns"),
            ("crypto", "Crypto module - cryptographic functions"),
            ("http", "HTTP module - HTTP client/server"),
            ("ffi", "FFI module - foreign function interface"),
            ("utils", "Utils module - utility functions"),
            ];
            
            for (name, desc) in modules {
                completions.push(CompletionItem {
                    label: format!("{}.{}", base, name),
                    kind: Some(CompletionItemKind::MODULE),
                    detail: Some(desc.to_string()),
                    documentation: Some(Documentation::String(format!("Import from {} module", name))),
                    ..Default::default()
                });
            }
        }
    } else if base.starts_with("@std.") {
        // Handle nested paths like @std.collections -> show hashmap, list, etc.
        let submodule = base.strip_prefix("@std.").unwrap_or("");
        let _submodule_path = format!("@std.{}", submodule);
        
        // Try to resolve the submodule to see what's inside  
        // Note: resolve_module_path returns the file path, but we need the directory
        // For now, construct the directory path manually
        let submodule_dir = store.stdlib_resolver.stdlib_root.join(submodule.replace('.', "/"));
        if submodule_dir.exists() && submodule_dir.is_dir() {
            let dir_path = &submodule_dir;
            if dir_path.is_dir() {
                // Scan directory for available modules/files
                let mut submodules = Vec::new();
                StdlibResolver::scan_directory(&dir_path, &mut submodules, submodule);
                
                for submod in submodules {
                    completions.push(CompletionItem {
                        label: format!("{}.{}", base, submod),
                        kind: Some(CompletionItemKind::MODULE),
                        detail: Some(format!("Nested module: {}", submod)),
                        documentation: None,
                        ..Default::default()
                    });
                }
            }
        }
    }
    
    completions
}

// ============================================================================
// STRUCT FIELD COMPLETIONS
// ============================================================================

fn get_struct_field_completions(receiver_type: &str, store: &DocumentStore) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    
    // Extract struct name from type string (handle cases like "Person", "Person<...>", etc.)
    let struct_name = receiver_type.split('<').next().unwrap_or(receiver_type).trim();
    
    // Find struct definition in documents
    use super::hover::find_struct_definition_in_documents;
    if let Some(struct_def) = find_struct_definition_in_documents(struct_name, &store.documents) {
        // Add field completions
        for field in &struct_def.fields {
            items.push(CompletionItem {
                label: field.name.clone(),
                kind: Some(CompletionItemKind::FIELD),
                detail: Some(format!("{}: {}", field.name, format_type(&field.type_))),
                documentation: Some(Documentation::String(format!("Field of `{}` struct", struct_name))),
                ..Default::default()
            });
        }
    }
    
    items
}

// ============================================================================
// UFC METHOD COMPLETIONS
// ============================================================================

fn get_ufc_method_completions(receiver_type: &str, store: &DocumentStore) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    // Query compiler integration for actual methods
    for sig in store.compiler.get_methods_for_type(receiver_type) {
        let mut item = CompletionItem::default();
        item.label = sig.name.clone();
        item.kind = Some(CompletionItemKind::METHOD);
        item.detail = Some(format!("({}) -> {}",
            sig.params
                .iter()
                .map(|(n, t)| format!("{}: {}", n, super::utils::format_type(t)))
                .collect::<Vec<_>>()
                .join(", "),
            super::utils::format_type(&sig.return_type)
        ));
        items.push(item);
    }

    items
}
