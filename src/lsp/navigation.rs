// Navigation Module for Zen LSP
// Handles go-to-definition, references, type definitions, and document highlights

use lsp_server::{Request, Response};
use lsp_types::*;
use serde_json::Value;

use crate::ast::{Declaration, Statement};
use super::types::*;
use super::utils::format_type;
use super::document_store::DocumentStore;

// Helper to find import information for a symbol
struct ImportInfo {
    import_line: String,
    source: String,
}

fn find_import_info(content: &str, symbol_name: &str, position: Position) -> Option<ImportInfo> {
    // Optimized: iterate lines directly without collecting
    let start_line = position.line as usize;
    
    // Collect lines up to start_line, then search backwards
    let lines: Vec<&str> = content.lines().take(start_line + 1).collect();
    
    // Search backwards from current position for import statements
    for (_line_num, line) in lines.iter().enumerate().rev() {
        let trimmed = line.trim();
        
        // Look for pattern: { symbol_name } = @std or { symbol_name, ... } = @std
        if trimmed.starts_with('{') && trimmed.contains('}') && trimmed.contains('=') {
            // Extract the import pattern
            if let Some(brace_end) = trimmed.find('}') {
                let import_part = &trimmed[1..brace_end];
                let imports: Vec<&str> = import_part.split(',').map(|s| s.trim()).collect();
                
                if imports.contains(&symbol_name) {
                    // Extract the source (after =)
                    if let Some(eq_pos) = trimmed.find('=') {
                        let source = trimmed[eq_pos + 1..].trim();
                        return Some(ImportInfo {
                            import_line: trimmed.to_string(),
                            source: source.to_string(),
                        });
                    }
                }
            }
        }
    }
    
    None
}

pub(crate) fn find_symbol_definition_in_content(content: &str, symbol_name: &str) -> Option<Range> {
    let lines: Vec<&str> = content.lines().collect();

    // First pass: look for actual definitions (function, variable, etc.)
    for (line_idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        
        // Skip comments
        if trimmed.starts_with("//") {
            continue;
        }
        
        // Look for symbol at word boundaries
        if let Some(pos) = find_word_in_line(line, symbol_name) {
            // Optimized: use helper function for word boundary checking
            let is_word_boundary_start = pos == 0 || is_word_boundary_char(line, pos.saturating_sub(1));
            let end_pos = pos + symbol_name.len();
            let is_word_boundary_end = end_pos >= line.len() || is_word_boundary_char(line, end_pos);

            if is_word_boundary_start && is_word_boundary_end {
                // Check if this looks like a definition
                // Pattern: symbol_name = ... or symbol_name: Type or symbol_name: Type =
                let after_symbol = &line[pos + symbol_name.len()..].trim();
                let is_definition = after_symbol.starts_with('=')
                    || after_symbol.starts_with(':')
                    || after_symbol.starts_with('(')
                    || (line[..pos].trim().is_empty() && (after_symbol.starts_with('=') || after_symbol.starts_with(':')));
                
                if is_definition {
                    return Some(Range {
                        start: Position { line: line_idx as u32, character: pos as u32 },
                        end: Position { line: line_idx as u32, character: end_pos as u32 },
                    });
                }
            }
        }
    }

    None
}

// Helper to check if a byte position is a word boundary character
// Optimized for ASCII (most common case in code)
#[inline]
fn is_word_boundary_char(line: &str, byte_pos: usize) -> bool {
    if byte_pos >= line.len() {
        return true;
    }
    // For ASCII characters (most common case), use direct byte access
    if let Some(&b) = line.as_bytes().get(byte_pos) {
        // Only valid for ASCII (0-127)
        if b < 128 {
            let ch = b as char;
            return !ch.is_alphanumeric() && ch != '_';
        }
    }
    // For non-ASCII, need to find the char at this byte position
    // This is slower but correct for UTF-8
    let mut char_count = 0;
    for (idx, _) in line.char_indices() {
        if idx >= byte_pos {
            break;
        }
        char_count += 1;
    }
    line.chars().nth(char_count)
        .map(|c| !c.is_alphanumeric() && c != '_')
        .unwrap_or(true)
}

// Helper to find a word in a line, respecting word boundaries better
fn find_word_in_line(line: &str, word: &str) -> Option<usize> {
    let mut search_pos = 0;
    loop {
        if let Some(pos) = line[search_pos..].find(word) {
            let actual_pos = search_pos + pos;
            
            // Check word boundaries - optimized: use helper function
            let before_ok = actual_pos == 0 || is_word_boundary_char(line, actual_pos.saturating_sub(1));
            let after_pos = actual_pos + word.len();
            let after_ok = after_pos >= line.len() || is_word_boundary_char(line, after_pos);
            
            if before_ok && after_ok {
                return Some(actual_pos);
            }
            
            search_pos = actual_pos + 1;
        } else {
            break;
        }
    }
    None
}

// ============================================================================
// PUBLIC HANDLER FUNCTIONS
// ============================================================================

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
            // Try to resolve the UFC method
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
            
            // Check if clicking on @std.text or similar module path (e.g., @std.types.StaticString)
            if symbol_name.starts_with("@std.") {
                let parts: Vec<&str> = symbol_name.split('.').collect();
                if parts.len() >= 3 {
                    // @std.module.symbol format
                    let module_name = parts[1];
                    let symbol_name_in_module = parts[2..].join(".");
                    
                    eprintln!("[LSP] Module path: @std.{} -> symbol: {}", module_name, symbol_name_in_module);
                    
                    // Search for the module file
                    for (uri, stdlib_doc) in &store.documents {
                        let uri_path = uri.path();
                        if uri_path.contains("stdlib") && (uri_path.ends_with(&format!("{}/{}.zen", module_name, module_name)) || 
                            uri_path.contains(&format!("{}/{}", module_name, module_name))) {
                            eprintln!("[LSP] Found stdlib module at {}", uri_path);
                            
                            // Try to find the symbol in the module
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
                            
                            // If symbol not found, return module file location
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
                // Check if we're in an import line
                if line.contains('=') && (line.contains("@std") || line.contains("= @std")) {
                    // Find the module path from the symbol
                    let module_path = if symbol_name == "@std" || symbol_name == "std" {
                        "@std"
                    } else {
                        &symbol_name
                    };
                    
                    // Use stdlib resolver to find the actual file
                    // Clone the resolver to avoid borrow checker issues
                    let resolver = store.stdlib_resolver.clone();
                    if let Some(file_path) = resolver.resolve_module_path(module_path) {
                        // Convert file path to URI
                        if let Ok(uri) = Url::from_file_path(&file_path) {
                            // Check if this file is already open in documents
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
                            } else {
                                // File not open, but we found it - construct URI from workspace root
                                if let Some(workspace_root) = &store.workspace_root {
                                    if let Ok(workspace_path) = workspace_root.to_file_path() {
                                        if let Ok(relative) = file_path.strip_prefix(&workspace_path) {
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
            }
            
            // FIRST: Check if this is an imported symbol (e.g., { io } = @std)
            // This should take priority to resolve to stdlib
            if let Some(import_info) = find_import_info(&doc.content, &symbol_name, position) {
                eprintln!("[LSP] Found import: {} from {}", symbol_name, import_info.source);
                
                // If it's a module import like @std, try to find the module file
                if import_info.source.starts_with("@std") {
                    // Map @std imports to actual stdlib files
                    let module_name = if import_info.source == "@std" {
                        // For @std, the symbol name IS the module name (e.g., { io } = @std means io module)
                        symbol_name.clone()
                    } else if import_info.source == "@std.types" {
                        "types".to_string()
                    } else {
                        import_info.source.strip_prefix("@std.").unwrap_or("io").to_string()
                    };
                    
                    eprintln!("[LSP] Resolving module: {} for symbol: {}", module_name, symbol_name);
                    
                    // For @std imports, the symbol name IS the module name (e.g., { io } = @std)
                    // So we should resolve to the module file, not look for a symbol named "io"
                    if import_info.source == "@std" && symbol_name == module_name {
                        // This is a module import - resolve to the module file
                        let module_file_path = format!("stdlib/{}/{}.zen", module_name, module_name);
                        eprintln!("[LSP] Module import detected: {} -> {}", symbol_name, module_file_path);
                        
                        // First try stdlib_symbols - might have module-level symbols
                        if let Some(symbol_info) = store.stdlib_symbols.get(&symbol_name) {
                            if let Some(uri) = &symbol_info.definition_uri {
                                eprintln!("[LSP] Resolved module import to stdlib at {}", uri);
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
                        
                        // Search for the module file in stdlib documents
                        for (uri, _stdlib_doc) in &store.documents {
                            let uri_path = uri.path();
                            if uri_path.contains("stdlib") && (uri_path.ends_with(&format!("{}/{}.zen", module_name, module_name)) || 
                                uri_path.contains(&format!("{}/{}", module_name, module_name))) {
                                eprintln!("[LSP] Found stdlib module file at {}", uri_path);
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
                        
                        // If not found in open documents, try to construct URI from workspace root
                        if let Some(workspace_root) = &store.workspace_root {
                            if let Some(workspace_path) = workspace_root.to_file_path().ok() {
                                let module_file = workspace_path.join("stdlib").join(&module_name).join(format!("{}.zen", module_name));
                                if module_file.exists() {
                                    if let Ok(uri) = Url::from_file_path(&module_file) {
                                        eprintln!("[LSP] Constructed stdlib module URI: {}", uri);
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
                                eprintln!("[LSP] Resolved import to stdlib symbol at {}", uri);
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
                        let module_file_path = format!("stdlib/{}/{}.zen", module_name, module_name);
                        eprintln!("[LSP] Looking for symbol {} in module: {}", symbol_name, module_file_path);
                        
                        for (uri, stdlib_doc) in &store.documents {
                            let uri_path = uri.path();
                            if uri_path.contains("stdlib") && (uri_path.ends_with(&format!("{}/{}.zen", module_name, module_name)) || 
                                uri_path.contains(&format!("{}/{}", module_name, module_name))) {
                                eprintln!("[LSP] Found stdlib module at {}", uri_path);
                                
                                // Try to find the specific symbol in this module
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
            
            // SECOND: Check local document symbols (from AST) - prioritize same file
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

            // FIFTH: Search for the symbol in other open documents (but NOT current file - already checked)
            // Prioritize non-test files over test files
            let mut test_match: Option<(Url, Range)> = None;
            let current_uri = &params.text_document_position_params.text_document.uri;

            // Limit search for performance (prioritize open files)
            const MAX_DOCS_DEFINITION_SEARCH: usize = 50;
            for (uri, other_doc) in store.documents.iter().take(MAX_DOCS_DEFINITION_SEARCH) {
                // Skip current document - we already checked it above
                if uri == current_uri {
                    continue;
                }
                
                if let Some(symbol_info) = other_doc.symbols.get(&symbol_name) {
                    let uri_str = uri.as_str();
                    let is_test = uri_str.contains("/tests/") || uri_str.contains("_test.zen") || uri_str.contains("test_");

                    if is_test {
                        // Save test match but keep looking for non-test
                        test_match = Some((uri.clone(), symbol_info.range.clone()));
                    } else {
                        // Found in non-test file - return immediately
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

            // Fallback: text search within current document (when AST parsing fails)
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

/// Handle textDocument/typeDefinition requests
pub fn handle_type_definition(req: Request, store: &std::sync::Arc<std::sync::Mutex<DocumentStore>>) -> Response {
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

        if let Some(symbol_name) = find_symbol_at_position(&doc.content, position) {
            // For type definition, we want to find the definition of the type, not the variable
            // First, check if it's a variable and get its type
            if let Some(symbol_info) = doc.symbols.get(&symbol_name) {
                // If it's a variable/parameter, extract the type name from its detail
                if let Some(detail) = &symbol_info.detail {
                    // Extract type name from patterns like "name: Type" or "val: Result<f64, E>"
                    if let Some(type_name) = extract_type_name(detail) {
                        // Now find the definition of this type
                        if let Some(type_symbol) = doc.symbols.get(&type_name)
                            .or_else(|| store.stdlib_symbols.get(&type_name))
                            .or_else(|| store.workspace_symbols.get(&type_name)) {

                            let uri = type_symbol.definition_uri.as_ref()
                                .unwrap_or(&params.text_document_position_params.text_document.uri);

                            let location = Location {
                                uri: uri.clone(),
                                range: type_symbol.range.clone(),
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

            // If the symbol itself is a type, just use handle_definition logic
            if let Some(symbol_info) = doc.symbols.get(&symbol_name)
                .or_else(|| store.stdlib_symbols.get(&symbol_name))
                .or_else(|| store.workspace_symbols.get(&symbol_name)) {

                let uri = symbol_info.definition_uri.as_ref()
                    .unwrap_or(&params.text_document_position_params.text_document.uri);

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

    Response {
        id: req.id,
        result: Some(Value::Null),
        error: None,
    }
}

/// Handle textDocument/references requests
pub fn handle_references(
    req: Request,
    store: &std::sync::Arc<std::sync::Mutex<DocumentStore>>,
) -> Response {
    let params: ReferenceParams = match serde_json::from_value(req.params) {
        Ok(p) => p,
        Err(_) => {
            return Response {
                id: req.id,
                result: Some(Value::Null),
                error: None,
            };
        }
    };

    let store_lock = match store.lock() {
        Ok(s) => s,
        Err(_) => {
            return Response { id: req.id, result: Some(serde_json::to_value(Vec::<Location>::new()).unwrap_or(Value::Null)), error: None };
        }
    };
    let mut locations = Vec::new();

    if let Some(doc) = store_lock.documents.get(&params.text_document_position.text_document.uri) {
        let position = params.text_document_position.position;

        // Find the symbol at the cursor position
        if let Some(symbol_name) = find_symbol_at_position(&doc.content, position) {
            eprintln!("[LSP] Find references for symbol: '{}'", symbol_name);

            // Determine the scope of the symbol
            let symbol_scope = determine_symbol_scope(&doc, &symbol_name, position);
            eprintln!("[LSP] Symbol scope: {:?}", symbol_scope);

            match symbol_scope {
                SymbolScope::Local { ref function_name } => {
                    // Local variable - only find references within the function
                    let current_uri = params.text_document_position.text_document.uri.clone();
                    if let Some(refs) = find_local_references(
                        &doc.content,
                        &symbol_name,
                        function_name,
                    ) {
                        for range in refs {
                            locations.push(Location {
                                uri: current_uri.clone(),
                                range,
                            });
                        }
                    }
                }
                SymbolScope::ModuleLevel | SymbolScope::Unknown => {
                    // Module-level symbol - search across documents (limited for performance)
                    const MAX_DOCS_REFERENCES_SEARCH: usize = 50;
                    for (uri, search_doc) in store_lock.documents.iter().take(MAX_DOCS_REFERENCES_SEARCH) {
                        let refs = find_references_in_document(
                            &search_doc.content,
                            &symbol_name,
                        );
                        for range in refs {
                            locations.push(Location {
                                uri: uri.clone(),
                                range,
                            });
                        }
                    }
                }
            }

            // Include the definition if requested
            if params.context.include_declaration {
                if let Some(symbol_info) = doc.symbols.get(&symbol_name) {
                    locations.push(Location {
                        uri: params.text_document_position.text_document.uri.clone(),
                        range: symbol_info.range.clone(),
                    });
                }
            }

            eprintln!("[LSP] Found {} reference(s)", locations.len());
        }
    }

    Response {
        id: req.id,
        result: Some(serde_json::to_value(locations).unwrap_or(Value::Null)),
        error: None,
    }
}

/// Handle textDocument/documentHighlight requests
pub fn handle_document_highlight(req: Request, store: &std::sync::Arc<std::sync::Mutex<DocumentStore>>) -> Response {
    let params: DocumentHighlightParams = match serde_json::from_value(req.params) {
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

        if let Some(symbol_name) = find_symbol_at_position(&doc.content, position) {
            // Find all occurrences of this symbol in the current document
            let highlights = find_symbol_occurrences(&doc.content, &symbol_name);

            return Response {
                id: req.id,
                result: Some(serde_json::to_value(highlights).unwrap_or(Value::Null)),
                error: None,
            };
        }
    }

    Response {
        id: req.id,
        result: Some(Value::Null),
        error: None,
    }
}

// ============================================================================
// PRIVATE HELPER FUNCTIONS - Symbol Finding
// ============================================================================

/// Find the symbol (identifier) at the given position in the document
pub fn find_symbol_at_position(content: &str, position: Position) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    if position.line as usize >= lines.len() {
        return None;
    }

    let line = lines[position.line as usize];
    let char_pos = position.character as usize;

    // Find word boundaries around the cursor position
    let chars: Vec<char> = line.chars().collect();

    // Check if position is valid
    if chars.is_empty() || char_pos > chars.len() {
        return None;
    }

    let mut start = char_pos.min(chars.len());
    let mut end = char_pos.min(chars.len());

    // Move start backwards to find word beginning (allow @ for @std and . for module paths)
    while start > 0 && start <= chars.len() {
        let ch = chars[start - 1];
        if ch.is_alphanumeric() || ch == '_' || ch == '.' || (start == 1 && ch == '@') {
            start -= 1;
            // If we hit @, stop (don't go before it)
            if start == 0 && chars.get(0) == Some(&'@') {
                break;
            }
        } else {
            break;
        }
    }

    // Move end forward to find word end (allow . for module paths like @std.text)
    while end < chars.len() {
        let ch = chars[end];
        if ch.is_alphanumeric() || ch == '_' || ch == '.' {
            end += 1;
        } else {
            break;
        }
    }

    if start < end {
        let symbol: String = chars[start..end].iter().collect();
        // Don't return empty or just punctuation
        if !symbol.is_empty() && symbol.chars().any(|c| c.is_alphanumeric()) {
            Some(symbol)
        } else {
            None
        }
    } else {
        None
    }
}

/// Find all occurrences of a symbol in a document for highlighting
fn find_symbol_occurrences(content: &str, symbol_name: &str) -> Vec<DocumentHighlight> {
    let mut highlights = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (line_idx, line) in lines.iter().enumerate() {
        let mut start_pos = 0;
        while let Some(pos) = line[start_pos..].find(symbol_name) {
            let actual_pos = start_pos + pos;

            // Check if this is a whole word match (not part of another identifier) - optimized
            let is_word_start = actual_pos == 0 || is_word_boundary_char(line, actual_pos.saturating_sub(1));
            let end_pos = actual_pos + symbol_name.len();
            let is_word_end = end_pos >= line.len() || is_word_boundary_char(line, end_pos);

            if is_word_start && is_word_end {
                highlights.push(DocumentHighlight {
                    range: Range {
                        start: Position {
                            line: line_idx as u32,
                            character: actual_pos as u32,
                        },
                        end: Position {
                            line: line_idx as u32,
                            character: end_pos as u32,
                        },
                    },
                    kind: Some(DocumentHighlightKind::TEXT),
                });
            }

            start_pos = actual_pos + 1;
        }
    }

    highlights
}

// ============================================================================
// PRIVATE HELPER FUNCTIONS - UFC (Uniform Function Call) Support
// ============================================================================

/// Find UFC method call at the given position (e.g., receiver.method())
fn find_ufc_method_at_position(content: &str, position: Position) -> Option<UfcMethodInfo> {
    let lines: Vec<&str> = content.lines().collect();
    if position.line as usize >= lines.len() {
        return None;
    }

    let line = lines[position.line as usize];
    let char_pos = position.character as usize;
    let chars: Vec<char> = line.chars().collect();

    // Check if we're after a dot (UFC method call)
    if char_pos > 0 {
        // Find the dot before the cursor
        let mut dot_pos = None;
        for i in (0..char_pos).rev() {
            if chars[i] == '.' {
                dot_pos = Some(i);
                break;
            } else if chars[i] == ' ' || chars[i] == '(' || chars[i] == ')' {
                // Stop if we hit whitespace or parens before finding a dot
                break;
            }
        }

        if let Some(dot) = dot_pos {
            // Extract the method name after the dot
            let mut method_end = dot + 1;
            while method_end < chars.len() && (chars[method_end].is_alphanumeric() || chars[method_end] == '_') {
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

/// Resolve a UFC method call to its definition location
fn resolve_ufc_method(method_info: &UfcMethodInfo, store: &DocumentStore) -> Option<Location> {
    // Enhanced UFC method resolution with improved generic type handling
    let receiver_type = infer_receiver_type(&method_info.receiver, store);

    // First, try to find the method in stdlib_symbols (indexed from actual stdlib files)
    // This queries the actual stdlib AST instead of hardcoded lists
    if let Some(symbol) = store.stdlib_symbols.get(&method_info.method_name) {
        if let Some(def_uri) = &symbol.definition_uri {
            // Check if this method is compatible with the receiver type
            // For now, if it's in stdlib, assume it's valid (we could add type checking later)
            return Some(Location {
                uri: def_uri.clone(),
                range: symbol.range.clone(),
            });
        }
    }

    // Extract base type and generic parameters for better matching
    let (base_type, _generic_params) = if let Some(ref typ) = receiver_type {
        parse_generic_type(typ)
    } else {
        (String::new(), Vec::new())
    };

    // Fallback: Try to find method by searching stdlib symbols with receiver type prefix
    // This is a heuristic - methods might be named like "string_len" or "result_unwrap"
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

    // Legacy fallback: Hardcoded method lists (will be removed once stdlib is fully indexed)
    // This is kept as a temporary fallback for methods not yet indexed
    match base_type.as_str() {
        "Result" => {
            let result_methods = [
                "raise", "is_ok", "is_err", "map", "map_err", "unwrap",
                "unwrap_or", "expect", "unwrap_err", "and_then", "or_else"
            ];
            if result_methods.contains(&method_info.method_name.as_str()) {
                return find_stdlib_location("core/result.zen", &method_info.method_name, store);
            }
        }
        "Option" => {
            let option_methods = [
                "is_some", "is_none", "unwrap", "unwrap_or", "map",
                "or", "and", "expect", "and_then", "or_else", "filter"
            ];
            if option_methods.contains(&method_info.method_name.as_str()) {
                return find_stdlib_location("core/option.zen", &method_info.method_name, store);
            }
        }
        "String" | "StaticString" | "str" => {
            let string_methods = [
                "len", "to_i32", "to_i64", "to_f64", "to_upper", "to_lower",
                "trim", "split", "substr", "char_at", "contains", "starts_with",
                "ends_with", "index_of", "replace", "concat", "repeat", "reverse",
                "strip_prefix", "strip_suffix", "to_bytes", "from_bytes"
            ];
            if string_methods.contains(&method_info.method_name.as_str()) {
                return find_stdlib_location("string.zen", &method_info.method_name, store);
            }
        }
        "HashMap" => {
            let hashmap_methods = [
                "insert", "get", "remove", "contains_key", "keys", "values",
                "len", "clear", "is_empty", "iter", "drain", "extend", "merge"
            ];
            if hashmap_methods.contains(&method_info.method_name.as_str()) {
                return find_stdlib_location("collections/hashmap.zen", &method_info.method_name, store);
            }
        }
        "DynVec" => {
            let dynvec_methods = [
                "push", "pop", "get", "set", "len", "clear", "capacity",
                "insert", "remove", "is_empty", "resize", "extend", "drain",
                "first", "last", "sort", "reverse", "contains"
            ];
            if dynvec_methods.contains(&method_info.method_name.as_str()) {
                return find_stdlib_location("vec.zen", &method_info.method_name, store);
            }
        }
        "Vec" => {
            let vec_methods = [
                "push", "get", "set", "len", "clear", "capacity", "is_full", "is_empty"
            ];
            if vec_methods.contains(&method_info.method_name.as_str()) {
                return find_stdlib_location("vec.zen", &method_info.method_name, store);
            }
        }
        "Array" => {
            let array_methods = [
                "len", "get", "set", "push", "pop", "first", "last",
                "slice", "contains", "find", "sort", "reverse"
            ];
            if array_methods.contains(&method_info.method_name.as_str()) {
                return find_stdlib_location("collections/array.zen", &method_info.method_name, store);
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
    // if the first parameter type matches the receiver type
    for (uri, doc) in &store.documents {
        // Direct method name match
        if let Some(symbol) = doc.symbols.get(&method_info.method_name) {
            if matches!(symbol.kind, SymbolKind::FUNCTION | SymbolKind::METHOD) {
                // Check if this function can be called with UFC on the receiver type
                if let Some(detail) = &symbol.detail {
                    // Parse the function signature to check first parameter
                    if let Some(params_start) = detail.find('(') {
                        if let Some(params_end) = detail.find(')') {
                            let params = &detail[params_start + 1..params_end];
                            if !params.is_empty() {
                                // Check if first parameter type matches receiver type
                                if let Some(first_param) = params.split(',').next() {
                                    if let Some(receiver_type) = &receiver_type {
                                        // Simple heuristic: check if parameter type contains receiver type
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

                // Fallback: if we can't parse the signature, still return it as a possibility
                return Some(Location {
                    uri: uri.clone(),
                    range: symbol.range.clone(),
                });
            }
        }

        // Search for functions that might be UFC-callable
        for (name, symbol) in &doc.symbols {
            // Check if the function name matches and is a function
            if name == &method_info.method_name && matches!(symbol.kind, SymbolKind::FUNCTION) {
                return Some(Location {
                    uri: uri.clone(),
                    range: symbol.range.clone(),
                });
            }

            // Also check for pattern: type_method (e.g., string_len for String.len)
            if let Some(receiver_type) = &receiver_type {
                let prefixed_name = format!("{}_{}", receiver_type.to_lowercase(), method_info.method_name);
                if name == &prefixed_name && matches!(symbol.kind, SymbolKind::FUNCTION) {
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

// ============================================================================
// PRIVATE HELPER FUNCTIONS - Type Inference
// ============================================================================

/// Extract type name from a detail string like "name: Type" or "val: Result<f64, E>"
fn extract_type_name(detail: &str) -> Option<String> {
    // Extract type from patterns like "name: Type" or "val: Result<f64, E>"
    if let Some(colon_pos) = detail.find(':') {
        let type_part = detail[colon_pos + 1..].trim();
        // Extract the base type name (before any generic parameters)
        let type_name = if let Some(generic_start) = type_part.find('<') {
            &type_part[..generic_start]
        } else if let Some(space_pos) = type_part.find(' ') {
            &type_part[..space_pos]
        } else {
            type_part
        };
        return Some(type_name.trim().to_string());
    }
    None
}

/// Infer the type of a receiver expression for UFC method resolution
fn infer_receiver_type(receiver: &str, store: &DocumentStore) -> Option<String> {
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
    for doc in store.documents.values().take(MAX_DOCS_FOR_TYPE_INFERENCE) {
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
                // Check for collection types with generics - use simple string matching instead of regex
                for type_name in ["HashMap<", "DynVec<", "Vec<", "Array<", "Option<", "Result<"] {
                    if detail.contains(type_name) {
                        return Some(type_name.trim_end_matches('<').to_string());
                    }
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

    // Enhanced pattern matching for function calls and constructors - optimized: use string matching instead of regex
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
        return Some("Option".to_string());
    }
    if receiver_trim == "None" {
        return Some("Option".to_string());
    }
    if receiver_trim.starts_with("Ok(") || receiver_trim.starts_with("Err(") {
        return Some("Result".to_string());
    }
    if receiver_trim.starts_with("Result.") {
        return Some("Result".to_string());
    }
    if receiver_trim.starts_with("Option.") {
        return Some("Option".to_string());
    }
    if receiver_trim.starts_with("get_default_allocator()") {
        return Some("Allocator".to_string());
    }
    if receiver_trim.starts_with('[') && receiver_trim.contains(';') && receiver_trim.ends_with(']') {
        return Some("Array".to_string());
    }

    // Enhanced support for method call chains (e.g., foo.bar().baz())
    if receiver.contains('.') && receiver.contains('(') {
        return infer_chained_method_type(receiver, store);
    }

    None
}

/// Parse a generic type like HashMap<K, V> into ("HashMap", ["K", "V"])
fn parse_generic_type(type_str: &str) -> (String, Vec<String>) {
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

/// Infer the type of a chained method call expression
fn infer_chained_method_type(receiver: &str, store: &DocumentStore) -> Option<String> {
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
            current_type = infer_base_expression_type(part, store);
        } else if let Some(ref curr_type) = current_type {
            // Subsequent parts are method calls on the current type
            if let Some(method_name) = part.split('(').next() {
                current_type = get_method_return_type(curr_type, method_name);
            }
        }
    }

    current_type
}

/// Infer the type of a base expression (before any method calls)
fn infer_base_expression_type(expr: &str, store: &DocumentStore) -> Option<String> {
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
                for doc in store.documents.values().take(MAX_DOCS_SEARCH) {
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

/// Get the return type of a method call on a given receiver type
fn get_method_return_type(receiver_type: &str, method_name: &str) -> Option<String> {
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

/// Find a method in a stdlib file
fn find_stdlib_location(stdlib_path: &str, method_name: &str, store: &DocumentStore) -> Option<Location> {
    // Try to find the method in the stdlib file
    // First check if we have the stdlib file open
    for (uri, doc) in &store.documents {
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

// ============================================================================
// PRIVATE HELPER FUNCTIONS - References and Scope
// ============================================================================

/// Determine the scope of a symbol (local, module-level, or unknown)
fn determine_symbol_scope(doc: &Document, symbol_name: &str, position: Position) -> SymbolScope {
    if let Some(ast) = &doc.ast {
        for decl in ast {
            if let Declaration::Function(func) = decl {
                if let Some(func_range) = find_function_range(&doc.content, &func.name) {
                    if position.line >= func_range.start.line && position.line <= func_range.end.line {
                        if is_local_symbol_in_function(func, symbol_name) {
                            return SymbolScope::Local {
                                function_name: func.name.clone()
                            };
                        }
                    }
                }
            }
        }
    }

    if doc.symbols.contains_key(symbol_name) {
        return SymbolScope::ModuleLevel;
    }

    SymbolScope::Unknown
}

/// Check if a symbol is local to a function (parameter or local variable)
fn is_local_symbol_in_function(func: &crate::ast::Function, symbol_name: &str) -> bool {
    for (param_name, _param_type) in &func.args {
        if param_name == symbol_name {
            return true;
        }
    }

    is_symbol_in_statements(&func.body, symbol_name)
}

/// Check if a symbol is declared in the given statements
fn is_symbol_in_statements(statements: &[Statement], symbol_name: &str) -> bool {
    for stmt in statements {
        match stmt {
            Statement::VariableDeclaration { name, .. } if name == symbol_name => {
                return true;
            }
            Statement::Loop { body, .. } => {
                if is_symbol_in_statements(body, symbol_name) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// Find the range of a function in the document content
fn find_function_range(content: &str, func_name: &str) -> Option<Range> {
    let lines: Vec<&str> = content.lines().collect();
    let mut start_line = None;

    for (line_num, line) in lines.iter().enumerate() {
        if line.contains(&format!("{} =", func_name)) {
            start_line = Some(line_num);
            break;
        }
    }

    if let Some(start) = start_line {
        let mut brace_depth = 0;
        let mut found_opening = false;

        for (line_num, line) in lines.iter().enumerate().skip(start) {
            for ch in line.chars() {
                if ch == '{' {
                    brace_depth += 1;
                    found_opening = true;
                } else if ch == '}' {
                    brace_depth -= 1;
                    if found_opening && brace_depth == 0 {
                        return Some(Range {
                            start: Position { line: start as u32, character: 0 },
                            end: Position { line: line_num as u32, character: line.len() as u32 }
                        });
                    }
                }
            }
        }
    }

    None
}

/// Find all references to a symbol within a function
fn find_local_references(content: &str, symbol_name: &str, function_name: &str) -> Option<Vec<Range>> {
    let func_range = find_function_range(content, function_name)?;
    let mut references = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for line_num in func_range.start.line..=func_range.end.line {
        if line_num as usize >= lines.len() {
            break;
        }

        let line = lines[line_num as usize];
        let mut start_col = 0;

        while let Some(col) = line[start_col..].find(symbol_name) {
            let actual_col = start_col + col;

            let before_ok = actual_col == 0 || is_word_boundary_char(line, actual_col.saturating_sub(1));
            let after_ok = actual_col + symbol_name.len() >= line.len() || 
                is_word_boundary_char(line, actual_col + symbol_name.len());

            if before_ok && after_ok && !is_in_string_or_comment(line, actual_col) {
                references.push(Range {
                    start: Position {
                        line: line_num,
                        character: actual_col as u32,
                    },
                    end: Position {
                        line: line_num,
                        character: (actual_col + symbol_name.len()) as u32,
                    },
                });
            }

            start_col = actual_col + 1;
        }
    }

    Some(references)
}

/// Find all references to a symbol in a document
fn find_references_in_document(content: &str, symbol_name: &str) -> Vec<Range> {
    let mut references = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (line_num, line) in lines.iter().enumerate() {
        let mut start_col = 0;

        while let Some(col) = line[start_col..].find(symbol_name) {
            let actual_col = start_col + col;

            let before_ok = actual_col == 0 || is_word_boundary_char(line, actual_col.saturating_sub(1));
            let after_ok = actual_col + symbol_name.len() >= line.len() || 
                is_word_boundary_char(line, actual_col + symbol_name.len());

            if before_ok && after_ok && !is_in_string_or_comment(line, actual_col) {
                references.push(Range {
                    start: Position {
                        line: line_num as u32,
                        character: actual_col as u32,
                    },
                    end: Position {
                        line: line_num as u32,
                        character: (actual_col + symbol_name.len()) as u32,
                    },
                });
            }

            start_col = actual_col + 1;
        }
    }

    references
}

/// Check if a position in a line is inside a string or comment
fn is_in_string_or_comment(line: &str, col: usize) -> bool {
    let mut in_string = false;
    let mut in_comment = false;
    let mut prev_char = ' ';

    for (i, ch) in line.chars().enumerate() {
        if i >= col {
            break;
        }

        if in_comment {
            continue;
        }

        if ch == '"' && prev_char != '\\' {
            in_string = !in_string;
        } else if !in_string && ch == '/' && prev_char == '/' {
            in_comment = true;
        }

        prev_char = ch;
    }

    in_string || in_comment
}
