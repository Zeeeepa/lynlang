// Type Inference Utilities for LSP
// Extracted from ZenLanguageServer for better code organization

use lsp_types::*;
use std::collections::HashMap;
use super::types::{Document, SymbolInfo};
use super::utils::format_type;

pub fn parse_generic_type(type_str: &str) -> (String, Vec<String>) {
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

pub fn infer_receiver_type(receiver: &str, documents: &HashMap<Url, Document>) -> Option<String> {
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
    for doc in documents.values().take(MAX_DOCS_FOR_TYPE_INFERENCE) {
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
        return infer_chained_method_type(receiver, documents);
    }

    None
}

pub fn infer_chained_method_type(receiver: &str, documents: &HashMap<Url, Document>) -> Option<String> {
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
            current_type = infer_base_expression_type(part, documents);
        } else if let Some(ref curr_type) = current_type {
            // Subsequent parts are method calls on the current type
            if let Some(method_name) = part.split('(').next() {
                current_type = get_method_return_type(curr_type, method_name);
            }
        }
    }

    current_type
}

pub fn infer_base_expression_type(expr: &str, documents: &HashMap<Url, Document>) -> Option<String> {
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
                for doc in documents.values().take(MAX_DOCS_SEARCH) {
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

pub fn get_method_return_type(receiver_type: &str, method_name: &str) -> Option<String> {
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

pub fn find_stdlib_location(stdlib_path: &str, method_name: &str, documents: &HashMap<Url, Document>) -> Option<Location> {
    // Try to find the method in the stdlib file
    // First check if we have the stdlib file open
    for (uri, doc) in documents {
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

// Wrapper functions that work with DocumentStore
use super::document_store::DocumentStore;

pub fn infer_receiver_type_from_store(receiver: &str, store: &DocumentStore) -> Option<String> {
    infer_receiver_type(receiver, &store.documents)
}

pub fn infer_chained_method_type_from_store(receiver: &str, store: &DocumentStore) -> Option<String> {
    infer_chained_method_type(receiver, &store.documents)
}

pub fn infer_base_expression_type_from_store(expr: &str, store: &DocumentStore) -> Option<String> {
    infer_base_expression_type(expr, &store.documents)
}

// Additional parsing and extraction functions
use crate::ast::AstType;

pub fn extract_generic_types(ast_type: &AstType) -> (Option<String>, Option<String>) {
    match ast_type {
        // Result<T, E>
        AstType::Generic { name, type_args } if name == "Result" && type_args.len() == 2 => {
            let ok_type = format_type(&type_args[0]);
            let err_type = format_type(&type_args[1]);
            (Some(ok_type), Some(err_type))
        }
        // Option<T>
        AstType::Generic { name, type_args } if name == "Option" && type_args.len() == 1 => {
            let inner_type = format_type(&type_args[0]);
            (Some(inner_type), None)
        }
        _ => (None, None)
    }
}

pub fn parse_function_from_source(content: &str, func_name: &str) -> (Option<String>, Option<String>) {
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
                            return (Some(parts[0].trim().to_string()), Some(parts[1].trim().to_string()));
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

pub fn parse_return_type_generics(signature: &str) -> (Option<String>, Option<String>) {
    // Parse function signature to extract Result<T, E> or Option<T>
    // Example: "divide = (a: f64, b: f64) Result<f64, StaticString>"

    // Find the return type (after the closing paren)
    if let Some(paren_pos) = signature.rfind(')') {
        let after_paren = signature[paren_pos+1..].trim();

        // Check for Result<T, E>
        if after_paren.starts_with("Result<") {
            if let Some(start) = after_paren.find('<') {
                if let Some(end) = after_paren.rfind('>') {
                    let generics = &after_paren[start+1..end];

                    // Smart split by comma - handle nested generics
                    let parts = split_generic_args(generics);
                    if parts.len() == 2 {
                        return (Some(parts[0].to_string()), Some(parts[1].to_string()));
                    } else if parts.len() == 1 {
                        // Single type param (shouldn't happen for Result)
                        return (Some(parts[0].to_string()), Some("unknown".to_string()));
                    }
                }
            }
        }
        // Check for Option<T>
        else if after_paren.starts_with("Option<") {
            if let Some(start) = after_paren.find('<') {
                if let Some(end) = after_paren.rfind('>') {
                    let inner_type = after_paren[start+1..end].trim();
                    return (Some(inner_type.to_string()), None);
                }
            }
        }
    }

    (None, None)
}

pub fn split_generic_args(args: &str) -> Vec<String> {
    // Split generic arguments by comma, respecting nested <>
    let mut result = Vec::new();
    let mut current = String::new();
    let mut depth = 0;

    for ch in args.chars() {
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
                result.push(current.trim().to_string());
                current.clear();
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.trim().is_empty() {
        result.push(current.trim().to_string());
    }

    result
}

pub fn infer_function_return_types(
    func_name: &str,
    all_docs: &HashMap<Url, Document>
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
