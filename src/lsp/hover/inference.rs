use lsp_types::Url;
use std::collections::HashMap;

use crate::lsp::types::*;
use crate::lsp::utils::format_type;
use super::structs;
use crate::stdlib_types::stdlib_types;

pub fn infer_variable_type_enhanced(
    content: &str,
    var_name: &str,
    local_symbols: &HashMap<String, SymbolInfo>,
    stdlib_symbols: &HashMap<String, SymbolInfo>,
    workspace_symbols: &HashMap<String, SymbolInfo>,
    documents: Option<&HashMap<Url, Document>>,
) -> Option<String> {
    let var_eq = format!("{} =", var_name);
    let var_mut = format!("{} ::=", var_name);
    let var_mut2 = format!("{}::=", var_name);
    
    for line in content.lines() {
        let trimmed = line.trim();
        
        if !trimmed.starts_with(var_name) {
            continue;
        }
        
        let is_mutable = trimmed.starts_with(&var_mut) || trimmed.starts_with(&var_mut2);
        let is_immutable = trimmed.starts_with(&var_eq);
        
        if !is_mutable && !is_immutable {
            continue;
        }
        
        let mutability = if is_mutable { "::" } else { "" };
        
        if let Some(eq_pos) = line.find('=') {
            let before_eq = &line[..eq_pos];
            if let Some(colon_pos) = before_eq.rfind(':') {
                let after_colon = before_eq[colon_pos + 1..].trim();
                let after_colon = after_colon.trim_start_matches(':').trim();
                if !after_colon.is_empty() && !after_colon.starts_with('=') {
                    return Some(format!(
                        "```zen\n{}{}: {} = ...\n```\n\n**Type:** `{}`",
                        var_name, mutability, after_colon, after_colon
                    ));
                }
            }
            
            let rhs = line[eq_pos + 1..].trim();
            
            if let Some((type_str, value_info)) = infer_type_from_rhs_fast(rhs, stdlib_symbols) {
                let value_part = value_info
                    .map(|v| format!("\n\n**Value:** `{}` *(compile-time)*", v))
                    .unwrap_or_default();
                
                return Some(format!(
                    "```zen\n{}{}: {} = {}\n```\n\n**Type:** `{}`{}",
                    var_name, mutability, type_str, rhs, type_str, value_part
                ));
            }
        }
    }
    
    None
}

fn infer_type_from_rhs_fast(rhs: &str, stdlib_symbols: &HashMap<String, SymbolInfo>) -> Option<(String, Option<String>)> {
    let rhs = rhs.trim();
    
    if rhs.starts_with('"') {
        return Some(("StaticString".to_string(), Some(rhs.to_string())));
    }
    
    if rhs == "true" || rhs == "false" {
        return Some(("bool".to_string(), Some(rhs.to_string())));
    }
    
    if let Ok(val) = rhs.parse::<i64>() {
        let type_str = if val >= i32::MIN as i64 && val <= i32::MAX as i64 { "i32" } else { "i64" };
        return Some((type_str.to_string(), Some(rhs.to_string())));
    }
    
    if rhs.contains('.') && !rhs.contains('(') && rhs.parse::<f64>().is_ok() {
        return Some(("f64".to_string(), Some(rhs.to_string())));
    }
    
    if let Some(paren_pos) = rhs.find('(') {
        let call_expr = rhs[..paren_pos].trim();
        
        if let Some(dot_pos) = call_expr.rfind('.') {
            let receiver = call_expr[..dot_pos].trim();
            let method = call_expr[dot_pos + 1..].trim();
            
            if receiver.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                if method == "new" || method == "init" || method == "create" || method == "default" {
                    return Some((receiver.to_string(), None));
                }
            }
            
            if let Some(ret_type) = stdlib_types().get_function_return_type(receiver, method) {
                return Some((format_type(ret_type), None));
            }
            
            if let Some(ret_type) = stdlib_types().get_method_return_type(receiver, method) {
                return Some((format_type(ret_type), None));
            }
            
            let full_name = format!("{}.{}", receiver, method);
            if let Some(info) = stdlib_symbols.get(&full_name) {
                if let Some(detail) = &info.detail {
                    if let Some(ret) = extract_return_type_simple(detail) {
                        return Some((ret, None));
                    }
                }
            }
        }
        
        if call_expr.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            return Some((call_expr.to_string(), None));
        }
    }
    
    if let Some(brace_pos) = rhs.find('{') {
        let type_name = rhs[..brace_pos].trim();
        if type_name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            return Some((type_name.to_string(), None));
        }
    }
    
    None
}

fn extract_return_type_simple(sig: &str) -> Option<String> {
    sig.rfind(')').map(|pos| {
        let ret = sig[pos + 1..].trim();
        ret.split_whitespace().next().map(|s| s.to_string())
    }).flatten()
}

fn infer_type_from_rhs(
    rhs: &str,
    _var_name: &str,
    local_symbols: &HashMap<String, SymbolInfo>,
    stdlib_symbols: &HashMap<String, SymbolInfo>,
    workspace_symbols: &HashMap<String, SymbolInfo>,
    documents: Option<&HashMap<Url, Document>>,
) -> Option<(String, Option<String>)> {
    let rhs = rhs.trim();
    
    if rhs.starts_with('"') || rhs.starts_with('\'') {
        return Some(("StaticString".to_string(), Some(rhs.to_string())));
    }
    
    if let Ok(val) = rhs.parse::<i64>() {
        if val >= i32::MIN as i64 && val <= i32::MAX as i64 {
            return Some(("i32".to_string(), Some(val.to_string())));
        }
        return Some(("i64".to_string(), Some(val.to_string())));
    }
    
    if rhs.contains('.') && !rhs.contains('(') {
        if rhs.parse::<f64>().is_ok() {
            return Some(("f64".to_string(), Some(rhs.to_string())));
        }
    }
    
    if rhs == "true" || rhs == "false" {
        return Some(("bool".to_string(), Some(rhs.to_string())));
    }
    
    if let Some(paren_pos) = rhs.find('(') {
        let call_expr = rhs[..paren_pos].trim();
        
        if let Some(dot_pos) = call_expr.rfind('.') {
            let receiver = call_expr[..dot_pos].trim();
            let method = call_expr[dot_pos + 1..].trim();
            
            if receiver.chars().next().is_some_and(|c| c.is_uppercase()) && method == "new" {
                return Some((receiver.to_string(), None));
            }
            
            if let Some(return_type) = stdlib_types().get_method_return_type(receiver, method) {
                return Some((format_type(return_type), None));
            }
            
            if let Some(return_type) = stdlib_types().get_function_return_type(receiver, method) {
                return Some((format_type(return_type), None));
            }
            
            let full_name = format!("{}.{}", receiver, method);
            if let Some(info) = stdlib_symbols.get(&full_name) {
                if let Some(detail) = &info.detail {
                    if let Some(ret_type) = extract_return_type(detail) {
                        return Some((ret_type, None));
                    }
                }
            }
        }
        
        if let Some(info) = local_symbols.get(call_expr)
            .or_else(|| stdlib_symbols.get(call_expr))
            .or_else(|| workspace_symbols.get(call_expr))
        {
            if let Some(type_info) = &info.type_info {
                return Some((format_type(type_info), None));
            }
            if let Some(detail) = &info.detail {
                if let Some(ret_type) = extract_return_type(detail) {
                    return Some((ret_type, None));
                }
            }
        }
        
        if call_expr.chars().next().is_some_and(|c| c.is_uppercase()) {
            return Some((call_expr.to_string(), None));
        }
    }
    
    if let Some(brace_pos) = rhs.find('{') {
        let type_name = rhs[..brace_pos].trim();
        if type_name.chars().next().is_some_and(|c| c.is_uppercase()) {
            if let Some(docs) = documents {
                if let Some(struct_def) = structs::find_struct_definition_in_documents(type_name, docs) {
                    return Some((type_name.to_string(), None));
                }
            }
            return Some((type_name.to_string(), None));
        }
    }
    
    None
}

fn extract_return_type(signature: &str) -> Option<String> {
    if let Some(close_paren) = signature.rfind(')') {
        let return_part = signature[close_paren + 1..].trim();
        let return_part = return_part.trim_start_matches('{').trim();
        if !return_part.is_empty() && return_part != "void" {
            return Some(return_part.split_whitespace().next()?.to_string());
        }
    }
    None
}

/// Extract type from a definition line
pub fn extract_type_from_line(line: &str) -> Option<String> {
    // Look for type annotations: name: Type or name: Type =
    if let Some(colon_pos) = line.find(':') {
        let after_colon = &line[colon_pos + 1..];
        // Extract type (stop at =, {, or end)
        let type_end = after_colon
            .find('=')
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

/// Infer variable type from assignment
pub fn infer_variable_type(
    content: &str,
    var_name: &str,
    local_symbols: &HashMap<String, SymbolInfo>,
    stdlib_symbols: &HashMap<String, SymbolInfo>,
    workspace_symbols: &HashMap<String, SymbolInfo>,
    documents: Option<&HashMap<Url, Document>>,
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
                            return Some(format!(
                                "```zen\n{}: {}\n```\n\n**Type:** `{}`",
                                var_name, type_str, type_str
                            ));
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
                    if let Some(func_info) = local_symbols
                        .get(func_name)
                        .or_else(|| stdlib_symbols.get(func_name))
                        .or_else(|| workspace_symbols.get(func_name))
                    {
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
                    if type_name.chars().next().is_some_and(|c| c.is_uppercase()) {
                        // Try to find the struct definition to show its fields
                        let type_display = if let Some(docs) = documents {
                            if let Some(struct_def) =
                                structs::find_struct_definition_in_documents(type_name, docs)
                            {
                                format!(
                                    "```zen\n{}\n```",
                                    structs::format_struct_definition(&struct_def)
                                )
                            } else {
                                format!("```zen\n{} = {} {{ ... }}\n```\n\n**Type:** `{}`\n\n**Inferred from:** constructor", var_name, type_name, type_name)
                            }
                        } else {
                            format!("```zen\n{} = {} {{ ... }}\n```\n\n**Type:** `{}`\n\n**Inferred from:** constructor", var_name, type_name, type_name)
                        };
                        return Some(type_display);
                    }
                }
                if let Some(paren_pos) = rhs.find('(') {
                    let call_expr = rhs[..paren_pos].trim();
                    
                    if let Some(dot_pos) = call_expr.find('.') {
                        let receiver = &call_expr[..dot_pos];
                        let method = &call_expr[dot_pos + 1..];
                        
                        if let Some(return_type) = stdlib_types().get_method_return_type(receiver, method) {
                            let type_str = format_type(return_type);
                            return Some(format!(
                                "```zen\n{}: {}\n```\n\n**Type:** `{}`\n\n**Inferred from:** `{}.{}()` return type",
                                var_name, type_str, type_str, receiver, method
                            ));
                        }
                        
                        if let Some(return_type) = stdlib_types().get_function_return_type(receiver, method) {
                            let type_str = format_type(return_type);
                            return Some(format!(
                                "```zen\n{}: {}\n```\n\n**Type:** `{}`\n\n**Inferred from:** `{}.{}()` return type",
                                var_name, type_str, type_str, receiver, method
                            ));
                        }
                    }
                    
                    let type_name = call_expr;
                    if type_name.chars().next().is_some_and(|c| c.is_uppercase()) {
                        let type_display = if let Some(docs) = documents {
                            if let Some(struct_def) =
                                structs::find_struct_definition_in_documents(type_name, docs)
                            {
                                format!(
                                    "```zen\n{}\n```",
                                    structs::format_struct_definition(&struct_def)
                                )
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
