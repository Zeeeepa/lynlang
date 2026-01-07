// Type Inference Utilities for LSP
// Extracted from ZenLanguageServer for better code organization

use super::document_store::DocumentStore;
use super::types::Document;
use super::utils::format_type;
use crate::well_known::well_known;
use lsp_types::*;
use std::collections::HashMap;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Split generic arguments by comma, respecting nested <> brackets
fn split_generic_args(args: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut depth = 0;

    for ch in args.chars() {
        match ch {
            '<' => { depth += 1; current.push(ch); }
            '>' => { depth -= 1; current.push(ch); }
            ',' if depth == 0 => {
                if !current.trim().is_empty() {
                    result.push(current.trim().to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        result.push(current.trim().to_string());
    }
    result
}

/// Infer type from common constructor/literal prefixes
fn infer_type_from_prefix(expr: &str) -> Option<String> {
    let wk = well_known();
    let trimmed = expr.trim();

    // String literals
    if trimmed.starts_with('"') || trimmed.starts_with('\'') {
        return Some("String".to_string());
    }

    // Constructor patterns (Type(...) or Type<...>)
    let type_patterns = [
        ("HashMap(", "HashMap"), ("HashMap<", "HashMap"),
        ("DynVec(", "DynVec"), ("DynVec<", "DynVec"),
        ("Vec(", "Vec"), ("Vec<", "Vec"),
        ("Array(", "Array"), ("Array<", "Array"),
        ("Some(", wk.option_name()), ("Ok(", wk.result_name()), ("Err(", wk.result_name()),
        ("Result.", wk.result_name()), ("Option.", wk.option_name()),
        ("get_default_allocator()", "Allocator"),
    ];

    for (prefix, type_name) in type_patterns {
        if trimmed.starts_with(prefix) {
            return Some(type_name.to_string());
        }
    }

    // None variant
    if trimmed == wk.none_name() {
        return Some(wk.option_name().to_string());
    }

    // Array literal [size; type]
    if trimmed.starts_with('[') && trimmed.contains(';') && trimmed.ends_with(']') {
        return Some("Array".to_string());
    }

    None
}

// ============================================================================
// MAIN FUNCTIONS
// ============================================================================

pub fn parse_generic_type(type_str: &str) -> (String, Vec<String>) {
    // First, try parsing using the real Parser
    if let Ok(ast) = (|| {
        let lexer = crate::lexer::Lexer::new(type_str);
        let mut parser = crate::parser::Parser::new(lexer);
        parser.parse_type()
    })() {
        match ast {
            crate::ast::AstType::Generic { name, type_args } => {
                let args = type_args.iter().map(super::utils::format_type).collect();
                return (name, args);
            }
            other => return (super::utils::format_type(&other), Vec::new()),
        }
    }

    // Fallback: manual parsing using helper
    if let Some(angle_pos) = type_str.find('<') {
        let base = type_str[..angle_pos].to_string();
        let params_end = type_str.rfind('>').unwrap_or(type_str.len());
        let params = split_generic_args(&type_str[angle_pos + 1..params_end]);
        (base, params)
    } else {
        (type_str.to_string(), Vec::new())
    }
}

pub fn infer_receiver_type(receiver: &str, documents: &HashMap<Url, Document>) -> Option<String> {
    // Check common patterns first using helper
    if let Some(t) = infer_type_from_prefix(receiver) {
        return Some(t);
    }

    // Check for numeric literals
    let trimmed = receiver.trim();
    if trimmed.chars().all(|c| c.is_numeric() || c == '.' || c == '-') && !trimmed.is_empty() {
        return Some(if trimmed.contains('.') { "f64" } else { "i32" }.to_string());
    }

    // Check if receiver is a known variable in symbols (limit search)
    for doc in documents.values().take(crate::lsp::search_limits::TYPE_INFERENCE_SEARCH) {
        if let Some(symbol) = doc.symbols.get(receiver) {
            if let Some(type_info) = &symbol.type_info {
                return Some(format_type(type_info));
            }
            if let Some(detail) = &symbol.detail {
                if let Some(t) = extract_type_from_detail(detail) {
                    return Some(t);
                }
            }
        }
    }

    None
}

/// Extract type from symbol detail string
fn extract_type_from_detail(detail: &str) -> Option<String> {
    // Parse function return types: "name = (...) ReturnType"
    if detail.contains(" = ") && detail.contains(')') {
        if let Some(return_type) = detail.split(" = ").nth(1) {
            if let Some(ret) = return_type.split(')').nth(1).map(|s| s.trim()) {
                if !ret.is_empty() && ret != "void" {
                    let (base, _) = parse_generic_type(ret);
                    return Some(base);
                }
            }
        }
    }

    // Try type after ':'
    if let Some(colon_pos) = detail.find(':') {
        let after = detail[colon_pos + 1..].trim();
        let (base, _) = parse_generic_type(after);
        if !base.is_empty() {
            return Some(base);
        }
    }

    // Try type after ')'
    if let Some(close_paren) = detail.rfind(')') {
        let after = detail[close_paren + 1..].trim();
        if !after.is_empty() && after != "void" {
            let (base, _) = parse_generic_type(after);
            if !base.is_empty() {
                return Some(base);
            }
        }
    }

    // Fallback: check for known type names
    let wk = well_known();
    let known_types = ["HashMap", "DynVec", "Vec", "Array", wk.option_name(), wk.result_name(), "String", "StaticString"];
    for type_name in known_types {
        if detail.contains(type_name) {
            return Some(type_name.to_string());
        }
    }

    None
}

pub fn infer_chained_method_type(
    receiver: &str,
    documents: &HashMap<Url, Document>,
) -> Option<String> {
    // Parse method chains from left to right, tracking type through each call
    let mut current_type: Option<String> = None;
    let parts = split_method_chain(receiver);

    // Process each part of the chain
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            // First part - infer its base type
            current_type = infer_base_expression_type(part, documents);
        } else if let Some(ref curr_type) = current_type {
            // Subsequent parts are method calls on the current type
            if let Some(method_name) = part.split('(').next() {
                current_type = legacy_get_method_return_type(curr_type, method_name);
            }
        }
    }

    current_type
}

pub fn infer_base_expression_type(
    expr: &str,
    documents: &HashMap<Url, Document>,
) -> Option<String> {
    let expr = expr.trim();

    // Check common patterns first using helper
    if let Some(t) = infer_type_from_prefix(expr) {
        return Some(t);
    }

    // Numeric literals
    if expr.parse::<i64>().is_ok() {
        return Some("i32".to_string());
    }
    if expr.parse::<f64>().is_ok() {
        return Some("f64".to_string());
    }

    // Check if it's a known variable
    if let Some(type_name) = expr.split('(').next() {
        for doc in documents.values().take(crate::lsp::search_limits::TYPE_INFERENCE_SEARCH) {
            if let Some(symbol) = doc.symbols.get(type_name) {
                if let Some(type_info) = &symbol.type_info {
                    let type_str = format_type(type_info);
                    return Some(type_str.split('<').next().unwrap_or(&type_str).to_string());
                }
            }
        }
    }

    None
}

fn legacy_get_method_return_type(receiver_type: &str, method_name: &str) -> Option<String> {
    let wk = well_known();
    let option_type = wk.option_name().to_string();
    let result_type = wk.result_name().to_string();
    
    // Comprehensive method return type mapping
    match receiver_type {
        "String" | "StaticString" | "str" => match method_name {
            "to_string" | "to_upper" | "to_lower" | "trim" | "concat" | "replace" | "substr"
            | "reverse" | "repeat" => Some("String".to_string()),
            "to_i32" | "to_i64" | "to_f64" => Some(option_type.clone()),
            "split" => Some("Array".to_string()),
            "len" | "index_of" => Some("i32".to_string()),
            "char_at" => Some(option_type.clone()),
            "contains" | "starts_with" | "ends_with" | "is_empty" => Some("bool".to_string()),
            "to_bytes" => Some("Array".to_string()),
            _ => None,
        },
        "HashMap" => match method_name {
            "get" | "remove" => Some(option_type.clone()),
            "keys" | "values" => Some("Array".to_string()),
            "len" | "capacity" => Some("i32".to_string()),
            "contains_key" | "is_empty" => Some("bool".to_string()),
            "insert" => Some(option_type.clone()), // Returns old value if any
            _ => None,
        },
        "DynVec" | "Vec" => match method_name {
            "get" | "pop" | "first" | "last" => Some(option_type.clone()),
            "len" | "capacity" => Some("i32".to_string()),
            "is_empty" | "is_full" | "contains" => Some("bool".to_string()),
            _ => None,
        },
        "Array" => match method_name {
            "get" | "pop" | "first" | "last" => Some(option_type.clone()),
            "len" => Some("i32".to_string()),
            "is_empty" | "contains" => Some("bool".to_string()),
            "slice" => Some("Array".to_string()),
            _ => None,
        },
        rt if wk.is_option(rt) => match method_name {
            "is_some" | "is_none" => Some("bool".to_string()),
            "unwrap" | "unwrap_or" | "expect" => Some("T".to_string()), // Would need generics tracking
            "map" => Some(option_type.clone()),
            "and" | "or" => Some(option_type.clone()),
            _ => None,
        },
        rt if wk.is_result(rt) => match method_name {
            "is_ok" | "is_err" => Some("bool".to_string()),
            "unwrap" | "raise" | "expect" | "unwrap_or" => Some("T".to_string()),
            "map" => Some(result_type.clone()),
            "map_err" => Some(result_type.clone()),
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

pub fn get_method_return_type(
    receiver_type: &str,
    method_name: &str,
    store: &DocumentStore,
) -> Option<String> {
    // Prefer compiler integration data
    if let Some(ast_type) = store
        .compiler
        .get_method_return_type(receiver_type, method_name)
    {
        return Some(format_type(&ast_type));
    }

    // Fallback: use symbol tables if compiler doesn't know this method yet
    if let Some(symbol) = store
        .stdlib_symbols
        .get(method_name)
        .or_else(|| store.workspace_symbols.get(method_name))
    {
        if let Some(type_info) = &symbol.type_info {
            return Some(format_type(type_info));
        }

        if let Some(detail) = &symbol.detail {
            if let Some(return_type) = extract_return_type_from_detail(detail) {
                return Some(return_type);
            }
        }
    }

    // Final fallback: legacy hardcoded mappings
    legacy_get_method_return_type(receiver_type, method_name)
}

fn extract_return_type_from_detail(detail: &str) -> Option<String> {
    if let Some(close_paren) = detail.rfind(')') {
        let after = detail[close_paren + 1..].trim();
        if !after.is_empty() && after != "void" {
            // Strip trailing block braces if present (e.g., "String {" -> "String")
            let cleaned = after.trim_end_matches('{').trim();
            if !cleaned.is_empty() {
                return Some(cleaned.to_string());
            }
        }
    }
    None
}

fn split_method_chain(receiver: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut paren_depth: usize = 0;
    let mut in_string = false;

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
                if paren_depth > 0 {
                    paren_depth -= 1;
                }
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

    parts
}

// Wrapper functions that work with DocumentStore

pub fn find_self_type_in_enclosing_function(content: &str, position: Position) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let current_line = position.line as usize;

    for line_idx in (0..=current_line).rev() {
        let line = lines.get(line_idx)?;

        if let Some(self_pos) = line.find("self:") {
            let after_self = &line[self_pos + 5..];
            let type_end = after_self
                .find(|c| c == ',' || c == ')')
                .unwrap_or(after_self.len());
            let self_type = after_self[..type_end].trim();
            if !self_type.is_empty() {
                return Some(self_type.to_string());
            }
        }
    }
    None
}

pub fn infer_receiver_type_with_context(
    receiver: &str,
    content: Option<&str>,
    position: Option<Position>,
    store: &DocumentStore,
) -> Option<String> {
    if receiver == "self" {
        if let (Some(content), Some(pos)) = (content, position) {
            if let Some(self_type) = find_self_type_in_enclosing_function(content, pos) {
                return Some(self_type);
            }
        }
    }

    if receiver.contains('.') && receiver.contains('(') {
        if let Some(ty) = infer_chained_method_type_from_store(receiver, store) {
            return Some(ty);
        }
    }

    infer_receiver_type(receiver, &store.documents)
}

pub fn infer_receiver_type_from_store(receiver: &str, store: &DocumentStore) -> Option<String> {
    infer_receiver_type_with_context(receiver, None, None, store)
}

pub fn infer_chained_method_type_from_store(
    receiver: &str,
    store: &DocumentStore,
) -> Option<String> {
    let mut current_type: Option<String> = None;
    let parts = split_method_chain(receiver);

    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            current_type = infer_base_expression_type_from_store(part, store);
        } else if let Some(ref curr_type) = current_type {
            if let Some(method_name) = part.split('(').next() {
                let method_name = method_name.trim();
                current_type = get_method_return_type(curr_type, method_name, store);
            }
        }
    }

    current_type
}

pub fn infer_base_expression_type_from_store(expr: &str, store: &DocumentStore) -> Option<String> {
    infer_base_expression_type(expr, &store.documents)
}

// Additional parsing and extraction functions
use crate::ast::AstType;

pub fn extract_generic_types(ast_type: &AstType) -> (Option<String>, Option<String>) {
    let wk = well_known();
    match ast_type {
        // Result<T, E>
        AstType::Generic { name, type_args } if wk.is_result(name) && type_args.len() == 2 => {
            let ok_type = format_type(&type_args[0]);
            let err_type = format_type(&type_args[1]);
            (Some(ok_type), Some(err_type))
        }
        // Option<T>
        AstType::Generic { name, type_args } if wk.is_option(name) && type_args.len() == 1 => {
            let inner_type = format_type(&type_args[0]);
            (Some(inner_type), None)
        }
        _ => (None, None),
    }
}

pub fn parse_function_from_source(
    content: &str,
    func_name: &str,
) -> (Option<String>, Option<String>) {
    // Find the function definition line
    // Example: "divide = (a: f64, b: f64) Result<f64, StaticString> {"

    for line in content.lines() {
        if line.contains(&format!("{} =", func_name)) || line.contains(&format!("{}=", func_name)) {
            // Found the function definition
            if let Some(result_pos) = line.find("Result<") {
                // Extract Result<T, E> by finding the matching >
                let after_result = &line[result_pos..];
                if let Some(start) = after_result.find('<') {
                    // Find the matching closing >
                    let mut depth = 0;
                    let mut end_pos = start;
                    for (i, ch) in after_result[start..].chars().enumerate() {
                        match ch {
                            '<' => depth += 1,
                            '>' => {
                                depth -= 1;
                                if depth == 0 {
                                    end_pos = start + i;
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }

                    if end_pos > start {
                        let generics = &after_result[start + 1..end_pos];
                        let parts = split_generic_args(generics);
                        if parts.len() >= 2 {
                            return (
                                Some(parts[0].trim().to_string()),
                                Some(parts[1].trim().to_string()),
                            );
                        } else if parts.len() == 1 {
                            // Only one part found - maybe parsing error
                            return (Some(parts[0].trim().to_string()), Some("E".to_string()));
                        }
                    }
                }
            } else if let Some(option_pos) = line.find("Option<") {
                // Extract Option<T>
                if let Some(start) = line[option_pos..].find('<') {
                    if let Some(end) = line[option_pos..].rfind('>') {
                        let inner = line[option_pos + start + 1..option_pos + end].trim();
                        return (Some(inner.to_string()), None);
                    }
                }
            }
        }
    }

    (None, None)
}

pub fn infer_function_return_types(
    func_name: &str,
    all_docs: &HashMap<Url, Document>,
) -> (Option<String>, Option<String>) {
    use crate::ast::Declaration;

    // Try AST first (most accurate)
    for (_uri, doc) in all_docs {
        if let Some(ast) = &doc.ast {
            for decl in ast {
                if let Declaration::Function(func) = decl {
                    if func.name == func_name {
                        // Found it! Extract types from the return type
                        let result = extract_generic_types(&func.return_type);
                        if result.0.is_some() {
                            return result;
                        }
                    }
                }
            }
        }
    }

    // Fallback: parse from source code if AST unavailable (e.g., parse errors)
    for (_uri, doc) in all_docs {
        let result = parse_function_from_source(&doc.content, func_name);
        if result.0.is_some() && result.1.is_some() {
            return result;
        }
    }

    (None, None)
}
