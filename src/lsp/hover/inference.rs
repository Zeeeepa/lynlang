use lsp_types::Url;
use std::collections::HashMap;

use crate::lsp::types::*;
use crate::lsp::utils::format_type;
use super::structs;
use crate::stdlib_types::stdlib_types;

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
