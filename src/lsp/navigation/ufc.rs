// UFC (Uniform Function Call) method resolution helpers

use super::super::document_store::DocumentStore;
use super::super::types::UfcMethodInfo;
use super::super::utils::format_type;
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

/// Infer the type of a receiver expression for UFC method resolution
fn infer_receiver_type(receiver: &str, store: &DocumentStore) -> Option<String> {
    // Check if receiver is a string literal
    if receiver.starts_with('"') || receiver.starts_with("'") {
        return Some("String".to_string());
    }

    // Check for numeric literals
    if receiver
        .chars()
        .all(|c| c.is_numeric() || c == '.' || c == '-')
    {
        if receiver.contains('.') {
            return Some("f64".to_string());
        } else {
            return Some("i32".to_string());
        }
    }

    // Check if receiver is a known variable in symbols
    const MAX_DOCS_FOR_TYPE_INFERENCE: usize = 20;
    for doc in store.documents.values().take(MAX_DOCS_FOR_TYPE_INFERENCE) {
        if let Some(symbol) = doc.symbols.get(receiver) {
            if let Some(type_info) = &symbol.type_info {
                return Some(format_type(type_info));
            }
            if let Some(detail) = &symbol.detail {
                if detail.contains(" = ") && detail.contains(")") {
                    if let Some(return_type) = detail.split(" = ").nth(1) {
                        if let Some(ret) = return_type.split(')').nth(1).map(|s| s.trim()) {
                            if !ret.is_empty() && ret != "void" {
                                return Some(ret.to_string());
                            }
                        }
                    }
                }
                for type_name in [
                    "HashMap<", "DynVec<", "Vec<", "Array<", "Option<", "Result<",
                ] {
                    if detail.contains(type_name) {
                        return Some(type_name.trim_end_matches('<').to_string());
                    }
                }
                let wk = well_known();
                for type_name in [
                    "HashMap",
                    "DynVec",
                    "Vec",
                    "Array",
                    wk.option_name(),
                    wk.result_name(),
                    "String",
                    "StaticString",
                ] {
                    if detail.contains(type_name) {
                        return Some(type_name.to_string());
                    }
                }
            }
        }
    }

    // Pattern matching for function calls and constructors
    let wk = well_known();
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
        return Some(wk.option_name().to_string());
    }
    if receiver_trim == wk.none_name() {
        return Some(wk.option_name().to_string());
    }
    if receiver_trim.starts_with("Ok(") || receiver_trim.starts_with("Err(") {
        return Some(wk.result_name().to_string());
    }
    if receiver_trim.starts_with("Result.") {
        return Some(wk.result_name().to_string());
    }
    if receiver_trim.starts_with("Option.") {
        return Some(wk.option_name().to_string());
    }
    if receiver_trim.starts_with("get_default_allocator()") {
        return Some("Allocator".to_string());
    }
    if receiver_trim.starts_with('[') && receiver_trim.contains(';') && receiver_trim.ends_with(']')
    {
        return Some("Array".to_string());
    }

    None
}

/// Find a method in a stdlib file
fn find_stdlib_location(
    stdlib_path: &str,
    method_name: &str,
    store: &DocumentStore,
) -> Option<Location> {
    for (uri, doc) in &store.documents {
        if uri.path().contains(stdlib_path) {
            if let Some(symbol) = doc.symbols.get(method_name) {
                return Some(Location {
                    uri: uri.clone(),
                    range: symbol.range.clone(),
                });
            }
        }
    }
    None
}

/// Resolve a UFC method call to its definition location
pub fn resolve_ufc_method(method_info: &UfcMethodInfo, store: &DocumentStore) -> Option<Location> {
    let receiver_type = infer_receiver_type(&method_info.receiver, store);

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

    // Legacy fallback: Hardcoded method lists
    let wk = well_known();
    if wk.is_result(&base_type) {
        let result_methods = [
            "raise",
            "is_ok",
            "is_err",
            "map",
            "map_err",
            "unwrap",
            "unwrap_or",
            "expect",
            "unwrap_err",
            "and_then",
            "or_else",
        ];
        if result_methods.contains(&method_info.method_name.as_str()) {
            return find_stdlib_location("core/result.zen", &method_info.method_name, store);
        }
    }
    if wk.is_option(&base_type) {
        let option_methods = [
            "is_some",
            "is_none",
            "unwrap",
            "unwrap_or",
            "map",
            "or",
            "and",
            "expect",
            "and_then",
            "or_else",
            "filter",
        ];
        if option_methods.contains(&method_info.method_name.as_str()) {
            return find_stdlib_location("core/option.zen", &method_info.method_name, store);
        }
    }
    match base_type.as_str() {
        "String" | "StaticString" | "str" => {
            let string_methods = [
                "len",
                "to_i32",
                "to_i64",
                "to_f64",
                "to_upper",
                "to_lower",
                "trim",
                "split",
                "substr",
                "char_at",
                "contains",
                "starts_with",
                "ends_with",
                "index_of",
                "replace",
                "concat",
                "repeat",
                "reverse",
                "strip_prefix",
                "strip_suffix",
                "to_bytes",
                "from_bytes",
            ];
            if string_methods.contains(&method_info.method_name.as_str()) {
                return find_stdlib_location("string.zen", &method_info.method_name, store);
            }
        }
        "HashMap" => {
            let hashmap_methods = [
                "insert",
                "get",
                "remove",
                "contains_key",
                "keys",
                "values",
                "len",
                "clear",
                "is_empty",
                "iter",
                "drain",
                "extend",
                "merge",
            ];
            if hashmap_methods.contains(&method_info.method_name.as_str()) {
                return find_stdlib_location(
                    "collections/hashmap.zen",
                    &method_info.method_name,
                    store,
                );
            }
        }
        "DynVec" => {
            let dynvec_methods = [
                "push", "pop", "get", "set", "len", "clear", "capacity", "insert", "remove",
                "is_empty", "resize", "extend", "drain", "first", "last", "sort", "reverse",
                "contains",
            ];
            if dynvec_methods.contains(&method_info.method_name.as_str()) {
                return find_stdlib_location("vec.zen", &method_info.method_name, store);
            }
        }
        "Vec" => {
            let vec_methods = [
                "push", "get", "set", "len", "clear", "capacity", "is_full", "is_empty",
            ];
            if vec_methods.contains(&method_info.method_name.as_str()) {
                return find_stdlib_location("vec.zen", &method_info.method_name, store);
            }
        }
        "Array" => {
            let array_methods = [
                "len", "get", "set", "push", "pop", "first", "last", "slice", "contains", "find",
                "sort", "reverse",
            ];
            if array_methods.contains(&method_info.method_name.as_str()) {
                return find_stdlib_location(
                    "collections/array.zen",
                    &method_info.method_name,
                    store,
                );
            }
        }
        "Allocator" => {
            let allocator_methods = ["alloc", "dealloc", "realloc", "clone"];
            if allocator_methods.contains(&method_info.method_name.as_str()) {
                return find_stdlib_location("memory_unified.zen", &method_info.method_name, store);
            }
        }
        _ => {}
    }

    // Check for generic iterator/collection methods
    if method_info.method_name == "loop" || method_info.method_name == "iter" {
        return find_stdlib_location("iterator.zen", &method_info.method_name, store);
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
