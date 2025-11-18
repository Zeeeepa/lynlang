// Definition handler - go-to-definition

use lsp_server::{Request, Response};
use lsp_types::*;
use serde_json::Value;
use url::Url;
use super::super::document_store::DocumentStore;
use super::utils::{find_symbol_at_position, find_symbol_definition_in_content};
use super::imports::find_import_info;
use super::ufc::{find_ufc_method_at_position, resolve_ufc_method};

/// Handle textDocument/definition requests
pub fn handle_definition(req: Request, store: &std::sync::Arc<std::sync::Mutex<DocumentStore>>) -> Response {
    let params: GotoDefinitionParams = match serde_json::from_value(req.params) {
        Ok(p) => p,
        Err(_) => {
            return Response {
                id: req.id,
                result: Some(Value::Null),
                error: None,
            };
        }
    };

    let store = match store.lock() {
        Ok(s) => s,
        Err(_) => {
            return Response { id: req.id, result: Some(Value::Null), error: None };
        }
    };
    
    if let Some(doc) = store.documents.get(&params.text_document_position_params.text_document.uri) {
        let position = params.text_document_position_params.position;

        // Check if we're on a UFC method call
        if let Some(method_info) = find_ufc_method_at_position(&doc.content, position) {
            if let Some(location) = resolve_ufc_method(&method_info, &store) {
                return Response {
                    id: req.id,
                    result: Some(serde_json::to_value(GotoDefinitionResponse::Scalar(location)).unwrap_or(Value::Null)),
                    error: None,
                };
            }
        }

        // Find the symbol at the cursor position
        if let Some(symbol_name) = find_symbol_at_position(&doc.content, position) {
            eprintln!("[LSP] Go-to-definition for: '{}'", symbol_name);
            
            // Check if clicking on @std.text or similar module path
            if symbol_name.starts_with("@std.") {
                let parts: Vec<&str> = symbol_name.split('.').collect();
                if parts.len() >= 3 {
                    let module_name = parts[1];
                    let symbol_name_in_module = parts[2..].join(".");
                    
                    for (uri, stdlib_doc) in &store.documents {
                        let uri_path = uri.path();
                        if uri_path.contains("stdlib") && (uri_path.ends_with(&format!("{}/{}.zen", module_name, module_name)) || 
                            uri_path.contains(&format!("{}/{}", module_name, module_name))) {
                            
                            if let Some(symbol_info) = stdlib_doc.symbols.get(&symbol_name_in_module) {
                                return Response {
                                    id: req.id,
                                    result: Some(serde_json::to_value(GotoDefinitionResponse::Scalar(Location {
                                        uri: uri.clone(),
                                        range: symbol_info.range.clone(),
                                    })).unwrap_or(Value::Null)),
                                    error: None,
                                };
                            }
                            
                            let location = Location {
                                uri: uri.clone(),
                                range: Range {
                                    start: Position { line: 0, character: 0 },
                                    end: Position { line: 0, character: 0 },
                                },
                            };
                            return Response {
                                id: req.id,
                                result: Some(serde_json::to_value(GotoDefinitionResponse::Scalar(location)).unwrap_or(Value::Null)),
                                error: None,
                            };
                        }
                    }
                }
            }
            
            // Check if clicking on @std or std in the import statement itself
            let line = doc.content.lines().nth(position.line as usize).unwrap_or("");
            if symbol_name == "@std" || symbol_name == "std" || (symbol_name.starts_with("@std") && symbol_name.contains('.')) {
                if line.contains('=') && (line.contains("@std") || line.contains("= @std")) {
                    let module_path = if symbol_name == "@std" || symbol_name == "std" {
                        "@std"
                    } else {
                        &symbol_name
                    };
                    
                    if let Some(file_path) = store.stdlib_resolver.resolve_module_path(module_path) {
                        if let Ok(uri) = Url::from_file_path(&file_path) {
                            if let Some(doc) = store.documents.get(&uri) {
                                let location = Location {
                                    uri: uri.clone(),
                                    range: Range {
                                        start: Position { line: 0, character: 0 },
                                        end: Position { line: 0, character: 0 },
                                    },
                                };
                                return Response {
                                    id: req.id,
                                    result: Some(serde_json::to_value(GotoDefinitionResponse::Scalar(location)).unwrap_or(Value::Null)),
                                    error: None,
                                };
                            } else if let Some(workspace_root) = &store.workspace_root {
                                if let Ok(workspace_path) = workspace_root.to_file_path() {
                                    if file_path.strip_prefix(&workspace_path).is_ok() {
                                        if let Ok(uri) = Url::from_file_path(&file_path) {
                                            let location = Location {
                                                uri,
                                                range: Range {
                                                    start: Position { line: 0, character: 0 },
                                                    end: Position { line: 0, character: 0 },
                                                },
                                            };
                                            return Response {
                                                id: req.id,
                                                result: Some(serde_json::to_value(GotoDefinitionResponse::Scalar(location)).unwrap_or(Value::Null)),
                                                error: None,
                                            };
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // Check if this is an imported symbol (e.g., { io } = @std)
            if let Some(import_info) = find_import_info(&doc.content, &symbol_name, position) {
                eprintln!("[LSP] Found import: {} from {}", symbol_name, import_info.source);
                
                if import_info.source.starts_with("@std") {
                    let module_name = if import_info.source == "@std" {
                        symbol_name.clone()
                    } else if import_info.source == "@std.types" {
                        "types".to_string()
                    } else {
                        import_info.source.strip_prefix("@std.").unwrap_or("io").to_string()
                    };
                    
                    // Module import - resolve to the module file
                    if import_info.source == "@std" && symbol_name == module_name {
                        if let Some(symbol_info) = store.stdlib_symbols.get(&symbol_name) {
                            if let Some(uri) = &symbol_info.definition_uri {
                                let location = Location {
                                    uri: uri.clone(),
                                    range: symbol_info.range.clone(),
                                };
                                return Response {
                                    id: req.id,
                                    result: Some(serde_json::to_value(GotoDefinitionResponse::Scalar(location)).unwrap_or(Value::Null)),
                                    error: None,
                                };
                            }
                        }
                        
                        for (uri, _stdlib_doc) in &store.documents {
                            let uri_path = uri.path();
                            if uri_path.contains("stdlib") && (uri_path.ends_with(&format!("{}/{}.zen", module_name, module_name)) || 
                                uri_path.contains(&format!("{}/{}", module_name, module_name))) {
                                let location = Location {
                                    uri: uri.clone(),
                                    range: Range {
                                        start: Position { line: 0, character: 0 },
                                        end: Position { line: 0, character: 0 },
                                    },
                                };
                                return Response {
                                    id: req.id,
                                    result: Some(serde_json::to_value(GotoDefinitionResponse::Scalar(location)).unwrap_or(Value::Null)),
                                    error: None,
                                };
                            }
                        }
                        
                        if let Some(workspace_root) = &store.workspace_root {
                            if let Some(workspace_path) = workspace_root.to_file_path().ok() {
                                let module_file = workspace_path.join("stdlib").join(&module_name).join(format!("{}.zen", module_name));
                                if module_file.exists() {
                                    if let Ok(uri) = Url::from_file_path(&module_file) {
                                        let location = Location {
                                            uri,
                                            range: Range {
                                                start: Position { line: 0, character: 0 },
                                                end: Position { line: 0, character: 0 },
                                            },
                                        };
                                        return Response {
                                            id: req.id,
                                            result: Some(serde_json::to_value(GotoDefinitionResponse::Scalar(location)).unwrap_or(Value::Null)),
                                            error: None,
                                        };
                                    }
                                }
                            }
                        }
                    } else {
                        // Try to find the symbol in stdlib_symbols first
                        if let Some(symbol_info) = store.stdlib_symbols.get(&symbol_name) {
                            if let Some(uri) = &symbol_info.definition_uri {
                                let location = Location {
                                    uri: uri.clone(),
                                    range: symbol_info.range.clone(),
                                };
                                return Response {
                                    id: req.id,
                                    result: Some(serde_json::to_value(GotoDefinitionResponse::Scalar(location)).unwrap_or(Value::Null)),
                                    error: None,
                                };
                            }
                        }
                        
                        // Try to find the symbol within the module file
                        for (uri, stdlib_doc) in &store.documents {
                            let uri_path = uri.path();
                            if uri_path.contains("stdlib") && (uri_path.ends_with(&format!("{}/{}.zen", module_name, module_name)) || 
                                uri_path.contains(&format!("{}/{}", module_name, module_name))) {
                                
                                if let Some(symbol_info) = stdlib_doc.symbols.get(&symbol_name) {
                                    return Response {
                                        id: req.id,
                                        result: Some(serde_json::to_value(GotoDefinitionResponse::Scalar(Location {
                                            uri: uri.clone(),
                                            range: symbol_info.range.clone(),
                                        })).unwrap_or(Value::Null)),
                                        error: None,
                                    };
                                }
                            }
                        }
                    }
                }
            }
            
            // Check local document symbols (from AST) - prioritize same file
            if let Some(symbol_info) = doc.symbols.get(&symbol_name) {
                eprintln!("[LSP] Found in document symbols at line {}", symbol_info.range.start.line);
                let location = Location {
                    uri: params.text_document_position_params.text_document.uri.clone(),
                    range: symbol_info.range.clone(),
                };
                return Response {
                    id: req.id,
                    result: Some(serde_json::to_value(GotoDefinitionResponse::Scalar(location)).unwrap_or(Value::Null)),
                    error: None,
                };
            }

            // Check workspace symbols (indexed from all files)
            if let Some(symbol_info) = store.workspace_symbols.get(&symbol_name) {
                if let Some(uri) = &symbol_info.definition_uri {
                    let location = Location {
                        uri: uri.clone(),
                        range: symbol_info.range.clone(),
                    };
                    return Response {
                        id: req.id,
                        result: Some(serde_json::to_value(GotoDefinitionResponse::Scalar(location)).unwrap_or(Value::Null)),
                        error: None,
                    };
                }
            }

            // Search for the symbol in other open documents
            let mut test_match: Option<(Url, Range)> = None;
            let current_uri = &params.text_document_position_params.text_document.uri;
            const MAX_DOCS_DEFINITION_SEARCH: usize = 50;
            
            for (uri, other_doc) in store.documents.iter().take(MAX_DOCS_DEFINITION_SEARCH) {
                if uri == current_uri {
                    continue;
                }
                
                if let Some(symbol_info) = other_doc.symbols.get(&symbol_name) {
                    let uri_str = uri.as_str();
                    let is_test = uri_str.contains("/tests/") || uri_str.contains("_test.zen") || uri_str.contains("test_");

                    if is_test {
                        test_match = Some((uri.clone(), symbol_info.range.clone()));
                    } else {
                        let location = Location {
                            uri: uri.clone(),
                            range: symbol_info.range.clone(),
                        };
                        return Response {
                            id: req.id,
                            result: Some(serde_json::to_value(GotoDefinitionResponse::Scalar(location)).unwrap_or(Value::Null)),
                            error: None,
                        };
                    }
                }
            }

            // If we only found test matches, use that
            if let Some((uri, range)) = test_match {
                let location = Location { uri, range };
                return Response {
                    id: req.id,
                    result: Some(serde_json::to_value(GotoDefinitionResponse::Scalar(location)).unwrap_or(Value::Null)),
                    error: None,
                };
            }

            // Search the entire workspace for the symbol
            if let Some((uri, symbol_info)) = store.search_workspace_for_symbol(&symbol_name) {
                let location = Location {
                    uri,
                    range: symbol_info.range,
                };
                return Response {
                    id: req.id,
                    result: Some(serde_json::to_value(GotoDefinitionResponse::Scalar(location)).unwrap_or(Value::Null)),
                    error: None,
                };
            }

            // Fallback: text search within current document
            eprintln!("[LSP] Trying text search fallback for: '{}'", symbol_name);
            if let Some(range) = find_symbol_definition_in_content(&doc.content, &symbol_name) {
                eprintln!("[LSP] Found via text search at line {}", range.start.line);
                let location = Location {
                    uri: params.text_document_position_params.text_document.uri.clone(),
                    range,
                };
                return Response {
                    id: req.id,
                    result: Some(serde_json::to_value(GotoDefinitionResponse::Scalar(location)).unwrap_or(Value::Null)),
                    error: None,
                };
            }
            
            eprintln!("[LSP] No definition found for: '{}'", symbol_name);
        }
    }

    Response {
        id: req.id,
        result: Some(Value::Null),
        error: None,
    }
}

