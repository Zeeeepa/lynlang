// Hover response creation utilities

use lsp_server::{RequestId, Response};
use lsp_types::*;
use serde_json::Value;

use super::super::types::SymbolInfo;
use super::super::utils::format_type;

/// Create a hover response from symbol info
pub fn create_hover_response(id: RequestId, symbol_info: &SymbolInfo, range: Option<Range>) -> Response {
    let mut hover_content = Vec::with_capacity(5); // Pre-allocate for common case
    if let Some(detail) = &symbol_info.detail {
        hover_content.push(format!("```zen\n{}\n```", detail));
    }
    if let Some(doc) = &symbol_info.documentation {
        hover_content.push(doc.clone());
    }
    if let Some(type_info) = &symbol_info.type_info {
        hover_content.push(format!("**Type:** `{}`", format_type(type_info)));
    }
    
    // Add location information
    if let Some(def_uri) = &symbol_info.definition_uri {
        if let Ok(path) = def_uri.to_file_path() {
            hover_content.push(format!("**Definition:** `{}:{}`", 
                path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown"),
                symbol_info.range.start.line + 1));
        } else {
            hover_content.push("**Source:** Standard Library".to_string());
        }
    } else {
        hover_content.push("**Source:** Standard Library".to_string());
    }
    
    let contents = HoverContents::Markup(MarkupContent {
        kind: MarkupKind::Markdown,
        value: hover_content.join("\n\n"),
    });
    
    Response {
        id,
        result: Some(serde_json::to_value(Hover {
            contents,
            range,
        }).unwrap_or(Value::Null)),
        error: None,
    }
}

