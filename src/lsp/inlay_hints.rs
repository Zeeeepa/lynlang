use lsp_server::{Request, Response};
use lsp_types::*;
use serde_json::Value;
use std::sync::{Arc, Mutex};

use crate::ast::{AstType, Declaration, Expression, Statement};
use crate::stdlib_types::stdlib_types;

use super::document_store::DocumentStore;
use super::types::Document;
use super::utils::format_type;

pub fn handle_inlay_hints(req: Request, store: &Arc<Mutex<DocumentStore>>) -> Response {
    let params: InlayHintParams = match serde_json::from_value(req.params) {
        Ok(p) => p,
        Err(_) => return empty_response(req.id),
    };

    let store = match store.lock() {
        Ok(s) => s,
        Err(_) => return empty_response(req.id),
    };

    let doc = match store.documents.get(&params.text_document.uri) {
        Some(d) => d,
        None => return empty_response(req.id),
    };

    let mut hints = Vec::new();
    
    collect_hints_from_content(&doc.content, &mut hints);

    if let Some(ast) = &doc.ast {
        for decl in ast {
            if let Declaration::Function(func) = decl {
                collect_hints_from_statements(&func.body, &doc.content, doc, &store, &mut hints);
            }
        }
    }

    Response {
        id: req.id,
        result: serde_json::to_value(hints).ok(),
        error: None,
    }
}

fn empty_response(id: lsp_server::RequestId) -> Response {
    Response {
        id,
        result: Some(serde_json::to_value(Vec::<InlayHint>::new()).unwrap_or(Value::Null)),
        error: None,
    }
}

fn collect_hints_from_content(content: &str, hints: &mut Vec<InlayHint>) {
    let mut seen: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();
    
    for (line_num, line) in content.lines().enumerate() {
        if let Some(hint) = try_create_hint_for_line(line, line_num as u32, &mut seen) {
            hints.push(hint);
        }
    }
}

fn try_create_hint_for_line(
    line: &str, 
    line_num: u32,
    seen: &mut std::collections::HashSet<(u32, u32)>,
) -> Option<InlayHint> {
    let trimmed = line.trim();
    
    if trimmed.is_empty() 
        || trimmed.starts_with("//")
        || trimmed.starts_with('{')
        || trimmed.starts_with('}')
        || trimmed.starts_with('|')
        || trimmed.contains("= (")
        || trimmed.contains("= @")
    {
        return None;
    }

    let (_var_name, var_end_pos, has_explicit_type, rhs) = parse_var_decl(line)?;
    
    if has_explicit_type {
        return None;
    }

    let key = (line_num, var_end_pos);
    if seen.contains(&key) {
        return None;
    }

    let inferred = infer_type(&rhs)?;
    seen.insert(key);

    Some(InlayHint {
        position: Position { line: line_num, character: var_end_pos },
        label: InlayHintLabel::String(format!(": {}", inferred)),
        kind: Some(InlayHintKind::TYPE),
        text_edits: None,
        tooltip: None,
        padding_left: None,
        padding_right: None,
        data: None,
    })
}

fn parse_var_decl(line: &str) -> Option<(String, u32, bool, String)> {
    let trimmed = line.trim();
    
    if trimmed.is_empty() || trimmed.starts_with("//") {
        return None;
    }
    
    let (eq_pos, eq_char_count) = if let Some(pos) = trimmed.find(" ::= ") {
        (pos, 5)
    } else if let Some(pos) = trimmed.find("::=") {
        (pos, 3)
    } else if let Some(pos) = trimmed.find(" = ") {
        (pos, 3)
    } else {
        return None;
    };
    
    let before_eq = &trimmed[..eq_pos];
    let after_eq = &trimmed[eq_pos + eq_char_count..];

    let has_explicit_type = if before_eq.contains("::") {
        let parts: Vec<&str> = before_eq.splitn(2, "::").collect();
        parts.len() == 2 && !parts[1].trim().is_empty()
    } else if before_eq.contains(':') {
        let colon_pos = before_eq.find(':')?;
        let after_colon = before_eq[colon_pos + 1..].trim();
        !after_colon.is_empty()
    } else {
        false
    };

    let var_name = if before_eq.contains("::") {
        before_eq.split("::").next()?.trim()
    } else if before_eq.contains(':') {
        before_eq.split(':').next()?.trim()
    } else {
        before_eq.trim()
    };

    if var_name.is_empty() 
        || var_name.contains('(')
        || var_name.contains(')')
        || var_name.contains('.')
        || var_name.contains(' ')
        || var_name.chars().next()?.is_uppercase()
    {
        return None;
    }

    let var_start = line.find(var_name)?;
    let var_end_pos = (var_start + var_name.len()) as u32;

    Some((var_name.to_string(), var_end_pos, has_explicit_type, after_eq.trim().to_string()))
}

fn infer_type(rhs: &str) -> Option<String> {
    let rhs = rhs.trim();
    
    if rhs.starts_with('"') || rhs.starts_with('\'') {
        return Some("StaticString".to_string());
    }
    
    if rhs == "true" || rhs == "false" {
        return Some("bool".to_string());
    }
    
    if let Ok(v) = rhs.parse::<i64>() {
        return Some(if v >= i32::MIN as i64 && v <= i32::MAX as i64 { "i32" } else { "i64" }.to_string());
    }
    
    if !rhs.contains('(') && rhs.parse::<f64>().is_ok() && rhs.contains('.') {
        return Some("f64".to_string());
    }
    
    if let Some(paren_pos) = rhs.find('(') {
        let call = rhs[..paren_pos].trim();
        return infer_from_call(call);
    }
    
    if let Some(brace_pos) = rhs.find('{') {
        let type_name = rhs[..brace_pos].trim();
        if type_name.chars().next()?.is_uppercase() {
            return Some(type_name.to_string());
        }
    }
    
    None
}

fn infer_from_call(call: &str) -> Option<String> {
    if let Some(dot_pos) = call.rfind('.') {
        let receiver = call[..dot_pos].trim();
        let method = call[dot_pos + 1..].trim();
        
        if receiver.chars().next()?.is_uppercase() {
            if matches!(method, "new" | "init" | "create" | "default" | "from") {
                return Some(receiver.to_string());
            }
            if let Some(ret) = stdlib_types().get_method_return_type(receiver, method) {
                return Some(format_type(ret));
            }
        }
        
        if let Some(ret) = stdlib_types().get_function_return_type(receiver, method) {
            return Some(format_type(ret));
        }
        
        if let Some(ret) = stdlib_types().get_method_return_type(receiver, method) {
            return Some(format_type(ret));
        }
    } else if call.chars().next()?.is_uppercase() {
        return Some(call.to_string());
    }
    
    None
}

fn collect_hints_from_statements(
    statements: &[Statement],
    content: &str,
    doc: &Document,
    store: &DocumentStore,
    hints: &mut Vec<InlayHint>,
) {
    for stmt in statements {
        match stmt {
            Statement::VariableDeclaration { name, type_, initializer, .. } => {
                if type_.is_none() {
                    if let Some(init) = initializer {
                        if let Some(inferred) = infer_expr_type(init, doc, store) {
                            if let Some(pos) = find_var_pos(content, name) {
                                hints.push(InlayHint {
                                    position: pos,
                                    label: InlayHintLabel::String(format!(": {}", inferred)),
                                    kind: Some(InlayHintKind::TYPE),
                                    text_edits: None,
                                    tooltip: None,
                                    padding_left: None,
                                    padding_right: None,
                                    data: None,
                                });
                            }
                        }
                    }
                }
            }
            Statement::Loop { body, .. } => {
                collect_hints_from_statements(body, content, doc, store, hints);
            }
            _ => {}
        }
    }
}

fn find_var_pos(content: &str, var_name: &str) -> Option<Position> {
    for (line_num, line) in content.lines().enumerate() {
        if let Some(pos) = line.find(var_name) {
            let before = pos == 0 || line.as_bytes().get(pos - 1).map(|&b| b.is_ascii_whitespace()).unwrap_or(true);
            let after = &line[pos + var_name.len()..].trim_start();
            if before && (after.starts_with('=') || after.starts_with(':')) {
                return Some(Position {
                    line: line_num as u32,
                    character: (pos + var_name.len()) as u32,
                });
            }
        }
    }
    None
}

fn infer_expr_type(expr: &Expression, doc: &Document, store: &DocumentStore) -> Option<String> {
    match expr {
        Expression::Integer32(_) => Some("i32".to_string()),
        Expression::Integer64(_) => Some("i64".to_string()),
        Expression::Float32(_) => Some("f32".to_string()),
        Expression::Float64(_) => Some("f64".to_string()),
        Expression::String(_) => Some("StaticString".to_string()),
        Expression::Boolean(_) => Some("bool".to_string()),
        
        Expression::BinaryOp { left, right, .. } => {
            let l = infer_expr_type(left, doc, store)?;
            let r = infer_expr_type(right, doc, store)?;
            Some(if l == "f64" || r == "f64" { "f64" } 
                 else if l == "i64" || r == "i64" { "i64" } 
                 else { "i32" }.to_string())
        }
        
        Expression::FunctionCall { name, .. } => {
            if let Some(dot_pos) = name.rfind('.') {
                let receiver = &name[..dot_pos];
                let method = &name[dot_pos + 1..];
                
                if let Some(ret) = stdlib_types().get_function_return_type(receiver, method) {
                    return Some(format_type(ret));
                }
                if let Some(ret) = stdlib_types().get_method_return_type(receiver, method) {
                    return Some(format_type(ret));
                }
            }
            
            for symbols in [&doc.symbols, &store.stdlib_symbols, &store.workspace_symbols] {
                if let Some(sym) = symbols.get(name) {
                    if let Some(ref ti) = sym.type_info {
                        if let AstType::Function { return_type, .. } = ti {
                            return Some(format_type(return_type));
                        }
                    }
                    if let Some(ref detail) = sym.detail {
                        if let Some(ret) = extract_return_type(detail) {
                            return Some(ret);
                        }
                    }
                }
            }
            None
        }
        
        Expression::StructLiteral { name, .. } => Some(name.clone()),
        
        Expression::MethodCall { object, method, .. } => {
            let recv_type = infer_expr_type(object, doc, store)?;
            if let Some(ret) = stdlib_types().get_method_return_type(&recv_type, method) {
                return Some(format_type(ret));
            }
            None
        }
        
        Expression::Identifier(name) => {
            if let Some(sym) = doc.symbols.get(name) {
                if let Some(ref ti) = sym.type_info {
                    return Some(format_type(ti));
                }
                if let Some(ref detail) = sym.detail {
                    if let Some(colon_pos) = detail.find(':') {
                        let type_part = detail[colon_pos + 1..].trim();
                        if !type_part.is_empty() {
                            return Some(type_part.to_string());
                        }
                    }
                }
            }
            find_variable_type_in_content(&doc.content, name)
        }
        
        _ => None,
    }
}

fn extract_return_type(sig: &str) -> Option<String> {
    let close = sig.rfind(')')?;
    let ret = sig[close + 1..].trim();
    if ret.is_empty() || ret == "{" { None } else { Some(ret.split_whitespace().next()?.to_string()) }
}

fn find_variable_type_in_content(content: &str, var_name: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        
        let pattern = format!("{} = ", var_name);
        let pattern2 = format!("{} ::= ", var_name);
        
        if trimmed.starts_with(&pattern) || trimmed.starts_with(&pattern2) {
            let eq_pos = trimmed.find('=').unwrap();
            let rhs = trimmed[eq_pos + 1..].trim().trim_start_matches('=').trim();
            
            if let Some(dot_pos) = rhs.find('.') {
                let type_name = &rhs[..dot_pos];
                if type_name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    let method = rhs[dot_pos + 1..].split('(').next().unwrap_or("");
                    if matches!(method, "new" | "init" | "create" | "default" | "from" | "with_capacity") {
                        return Some(type_name.to_string());
                    }
                }
            }
        }
    }
    None
}
