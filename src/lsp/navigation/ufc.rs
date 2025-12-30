// UFC (Uniform Function Call) method resolution helpers

use crate::lsp::document_store::DocumentStore;
use crate::lsp::type_inference::infer_receiver_type_from_store;
use crate::lsp::types::UfcMethodInfo;
use crate::well_known::well_known;
use lsp_types::*;

/// Find UFC method call at the given position (e.g., receiver.method())
pub fn find_ufc_method_at_position(content: &str, position: Position) -> Option<UfcMethodInfo> {
    let lines: Vec<&str> = content.lines().collect();
    if position.line as usize >= lines.len() {
        return None;
    }

    let line = lines[position.line as usize];
    let char_pos = position.character as usize;
    let chars: Vec<char> = line.chars().collect();

    // Check if we're after a dot (UFC method call)
    if char_pos > 0 {
        let mut dot_pos = None;
        for i in (0..char_pos).rev() {
            if chars[i] == '.' {
                dot_pos = Some(i);
                break;
            } else if chars[i] == ' ' || chars[i] == '(' || chars[i] == ')' {
                break;
            }
        }

        if let Some(dot) = dot_pos {
            // Extract the method name after the dot
            let mut method_end = dot + 1;
            while method_end < chars.len()
                && (chars[method_end].is_alphanumeric() || chars[method_end] == '_')
            {
                method_end += 1;
            }
            let method_name: String = chars[(dot + 1)..method_end].iter().collect();

            // Extract the object/receiver before the dot
            let mut obj_start = dot;
            let mut paren_depth = 0;
            while obj_start > 0 {
                obj_start -= 1;
                match chars[obj_start] {
                    ')' => paren_depth += 1,
                    '(' => {
                        if paren_depth > 0 {
                            paren_depth -= 1;
                        } else {
                            break;
                        }
                    }
                    ' ' | '\t' | '\n' | '=' | '{' | '[' | ',' | ';' if paren_depth == 0 => {
                        obj_start += 1;
                        break;
                    }
                    _ => {}
                }
            }

            let receiver: String = chars[obj_start..dot].iter().collect();
            return Some(UfcMethodInfo {
                receiver: receiver.trim().to_string(),
                method_name,
            });
        }
    }
    None
}

/// Parse a generic type like HashMap<K, V> into ("HashMap", ["K", "V"])
fn parse_generic_type(type_str: &str) -> (String, Vec<String>) {
    if let Some(angle_pos) = type_str.find('<') {
        let base = type_str[..angle_pos].to_string();
        let params_end = type_str.rfind('>').unwrap_or(type_str.len());
        let params_str = &type_str[angle_pos + 1..params_end];

        let params = if params_str.is_empty() {
            Vec::new()
        } else {
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

            if !current.is_empty() {
                params.push(current.trim().to_string());
            }
            params
        };
        (base, params)
    } else {
        (type_str.to_string(), Vec::new())
    }
}

/// Find a method in a stdlib file
fn find_stdlib_location(
    stdlib_path: &str,
    method_name: &str,
    receiver_type: Option<&str>,
    store: &DocumentStore,
) -> Option<Location> {
    for (uri, doc) in &store.documents {
        if uri.path().contains(stdlib_path) {
            // Try exact method name first
            if let Some(symbol) = doc.symbols.get(method_name) {
                return Some(Location {
                    uri: uri.clone(),
                    range: symbol.range.clone(),
                });
            }
            
            // Try Type.method format (e.g., "String.push")
            if let Some(recv_type) = receiver_type {
                let qualified_name = format!("{}.{}", recv_type, method_name);
                if let Some(symbol) = doc.symbols.get(&qualified_name) {
                    return Some(Location {
                        uri: uri.clone(),
                        range: symbol.range.clone(),
                    });
                }
            }
        }
    }
    None
}

/// Resolve a UFC method call to its definition location
pub fn resolve_ufc_method(method_info: &UfcMethodInfo, store: &DocumentStore) -> Option<Location> {
    let receiver_type = infer_receiver_type_from_store(&method_info.receiver, store);

    // First, try to find the method in stdlib_symbols
    if let Some(symbol) = store.stdlib_symbols.get(&method_info.method_name) {
        if let Some(def_uri) = &symbol.definition_uri {
            return Some(Location {
                uri: def_uri.clone(),
                range: symbol.range.clone(),
            });
        }
    }

    // Extract base type and generic parameters
    let (base_type, _generic_params) = if let Some(ref typ) = receiver_type {
        parse_generic_type(typ)
    } else {
        (String::new(), Vec::new())
    };

    // Fallback: Try to find method by searching stdlib symbols with receiver type prefix
    if !base_type.is_empty() {
        let prefixed_name = format!("{}_{}", base_type.to_lowercase(), method_info.method_name);
        if let Some(symbol) = store.stdlib_symbols.get(&prefixed_name) {
            if let Some(def_uri) = &symbol.definition_uri {
                return Some(Location {
                    uri: def_uri.clone(),
                    range: symbol.range.clone(),
                });
            }
        }
    }

    let wk = well_known();

    let stdlib_location = match base_type.as_str() {
        t if wk.is_result(t) => {
            find_stdlib_location("core/result.zen", &method_info.method_name, Some(wk.result_name()), store)
        }
        t if wk.is_option(t) => {
            find_stdlib_location("core/option.zen", &method_info.method_name, Some(wk.option_name()), store)
        }
        "String" | "StaticString" | "str" => {
            find_stdlib_location("string.zen", &method_info.method_name, Some("String"), store)
        }
        "HashMap" => {
            find_stdlib_location("collections/hashmap.zen", &method_info.method_name, Some("HashMap"), store)
        }
        "DynVec" => {
            find_stdlib_location("vec.zen", &method_info.method_name, Some("DynVec"), store)
        }
        "Vec" => {
            find_stdlib_location("vec.zen", &method_info.method_name, Some("Vec"), store)
        }
        "Array" => {
            find_stdlib_location("collections/array.zen", &method_info.method_name, Some("Array"), store)
        }
        "Allocator" | "GPA" => {
            find_stdlib_location("memory/allocator.zen", &method_info.method_name, Some("Allocator"), store)
                .or_else(|| find_stdlib_location("memory/gpa.zen", &method_info.method_name, Some("GPA"), store))
        }
        _ => None,
    };

    if stdlib_location.is_some() {
        return stdlib_location;
    }

    if method_info.method_name == "loop" || method_info.method_name == "iter" {
        return find_stdlib_location("iterator.zen", &method_info.method_name, None, store);
    }

    // Enhanced UFC function search - any function can be called as a method
    for (uri, doc) in &store.documents {
        if let Some(symbol) = doc.symbols.get(&method_info.method_name) {
            if matches!(
                symbol.kind,
                lsp_types::SymbolKind::FUNCTION | lsp_types::SymbolKind::METHOD
            ) {
                if let Some(detail) = &symbol.detail {
                    if let Some(params_start) = detail.find('(') {
                        if let Some(params_end) = detail.find(')') {
                            let params = &detail[params_start + 1..params_end];
                            if !params.is_empty() {
                                if let Some(first_param) = params.split(',').next() {
                                    if let Some(receiver_type) = &receiver_type {
                                        if first_param.contains(receiver_type) {
                                            return Some(Location {
                                                uri: uri.clone(),
                                                range: symbol.range.clone(),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                return Some(Location {
                    uri: uri.clone(),
                    range: symbol.range.clone(),
                });
            }
        }

        // Search for functions that might be UFC-callable
        for (name, symbol) in &doc.symbols {
            if name == &method_info.method_name
                && matches!(symbol.kind, lsp_types::SymbolKind::FUNCTION)
            {
                return Some(Location {
                    uri: uri.clone(),
                    range: symbol.range.clone(),
                });
            }

            if let Some(receiver_type) = &receiver_type {
                let prefixed_name = format!(
                    "{}_{}",
                    receiver_type.to_lowercase(),
                    method_info.method_name
                );
                if name == &prefixed_name && matches!(symbol.kind, lsp_types::SymbolKind::FUNCTION)
                {
                    return Some(Location {
                        uri: uri.clone(),
                        range: symbol.range.clone(),
                    });
                }
            }
        }
    }

    None
}
