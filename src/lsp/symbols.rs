// Symbols Module for Zen LSP
// Handles document symbols and workspace symbol search

use lsp_server::{Request, Response, ResponseError, ErrorCode};
use lsp_types::*;
use serde_json::Value;

use super::types::SymbolInfo;
use super::document_store::DocumentStore;

// ============================================================================
// PUBLIC HANDLER FUNCTIONS
// ============================================================================

/// Handle textDocument/documentSymbol requests
pub fn handle_document_symbols(req: Request, store: &std::sync::Arc<std::sync::Mutex<DocumentStore>>) -> Response {
    let params: DocumentSymbolParams = match serde_json::from_value(req.params) {
        Ok(p) => p,
        Err(_) => {
            return Response {
                id: req.id,
                result: Some(Value::Null),
                error: None,
            };
        }
    };

    let store = store.lock().unwrap();
    if let Some(doc) = store.documents.get(&params.text_document.uri) {
        let symbols: Vec<DocumentSymbol> = doc.symbols.values().map(|sym| {
            DocumentSymbol {
                name: sym.name.clone(),
                detail: sym.detail.clone(),
                kind: sym.kind,
                tags: None,
                #[allow(deprecated)]
                deprecated: None,
                range: sym.range,
                selection_range: sym.range,
                children: None,
            }
        }).collect();
        
        return Response {
            id: req.id,
            result: Some(serde_json::to_value(symbols).unwrap_or(Value::Null)),
            error: None,
        };
    }
    
    Response {
        id: req.id,
        result: Some(Value::Null),
        error: None,
    }
}

/// Handle workspace/symbol requests
pub fn handle_workspace_symbol(req: Request, store: &std::sync::Arc<std::sync::Mutex<DocumentStore>>) -> Response {
    let params: WorkspaceSymbolParams = match serde_json::from_value(req.params) {
        Ok(p) => p,
        Err(_) => {
            return Response {
                id: req.id,
                result: Some(Value::Null),
                error: Some(ResponseError {
                    code: ErrorCode::InvalidParams as i32,
                    message: "Invalid parameters".to_string(),
                    data: None,
                }),
            }
        }
    };

    let store = store.lock().unwrap();
    let query = params.query.to_lowercase();
    let mut symbols = Vec::new();

    // Search in all open documents
    for (uri, doc) in &store.documents {
        for (name, symbol_info) in &doc.symbols {
            // Fuzzy match: check if query is a substring of the symbol name
            if name.to_lowercase().contains(&query) {
                symbols.push(SymbolInformation {
                    name: symbol_info.name.clone(),
                    kind: symbol_info.kind,
                    tags: None,
                    #[allow(deprecated)]
                    deprecated: None,
                    location: Location {
                        uri: uri.clone(),
                        range: symbol_info.range,
                    },
                    container_name: None,
                });
            }
        }
    }

    // Search in stdlib symbols
    for (name, symbol_info) in &store.stdlib_symbols {
        if name.to_lowercase().contains(&query) {
            if let Some(def_uri) = &symbol_info.definition_uri {
                symbols.push(SymbolInformation {
                    name: symbol_info.name.clone(),
                    kind: symbol_info.kind,
                    tags: None,
                    #[allow(deprecated)]
                    deprecated: None,
                    location: Location {
                        uri: def_uri.clone(),
                        range: symbol_info.range,
                    },
                    container_name: Some("stdlib".to_string()),
                });
            }
        }
    }

    // Search in workspace symbols (indexed from all files)
    for (name, symbol_info) in &store.workspace_symbols {
        if name.to_lowercase().contains(&query) {
            if let Some(def_uri) = &symbol_info.definition_uri {
                symbols.push(SymbolInformation {
                    name: symbol_info.name.clone(),
                    kind: symbol_info.kind,
                    tags: None,
                    #[allow(deprecated)]
                    deprecated: None,
                    location: Location {
                        uri: def_uri.clone(),
                        range: symbol_info.range,
                    },
                    container_name: Some("workspace".to_string()),
                });
            }
        }
    }

    // Limit results to avoid overwhelming the client
    symbols.truncate(100);

    Response {
        id: req.id,
        result: Some(serde_json::to_value(symbols).unwrap_or(Value::Null)),
        error: None,
    }
}
