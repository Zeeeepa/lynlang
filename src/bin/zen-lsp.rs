// Zen Language Server Protocol Implementation
// Provides IDE support for the Zen programming language

use anyhow::{Context, Result};
use std::io::{self, Write};
use lsp_server::{Connection, Message, Request, RequestId, Response};
use lsp_types::notification::{DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, PublishDiagnostics};
use lsp_types::request::{
    Completion, GotoDefinition, HoverRequest, Initialize, Shutdown,
};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents, HoverParams,
    InitializeParams, InitializeResult, Location, MarkedString, Position, Range,
    ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
    PublishDiagnosticsParams, Diagnostic, DiagnosticSeverity, Url,
};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use zen::lexer::Lexer;
use zen::parser::Parser;

struct LspServer {
    documents: HashMap<String, DocumentState>,
    connection: Connection,
}

struct DocumentState {
    content: String,
    version: i32,
    symbols: HashMap<String, SymbolInfo>,
}

#[derive(Clone, Debug)]
struct SymbolInfo {
    kind: SymbolKind,
    location: Location,
    type_info: Option<String>,
}

#[derive(Clone, Debug)]
enum SymbolKind {
    Function,
    Struct,
    Enum,
    Variable,
    Constant,
    Type,
}

impl LspServer {
    fn new(connection: Connection) -> Self {
        Self {
            documents: HashMap::new(),
            connection,
        }
    }

    fn handle_hover(&self, params: HoverParams) -> Option<Hover> {
        let uri = params.text_document_position_params.text_document.uri.to_string();
        let position = params.text_document_position_params.position;
        
        if let Some(doc) = self.documents.get(&uri) {
            // Find token at position
            let mut lexer = Lexer::new(&doc.content);
            let line_col = (position.line as usize, position.character as usize);
            
            // Simple line/column based token finding
            let lines: Vec<&str> = doc.content.lines().collect();
            if position.line < lines.len() as u32 {
                let line = lines[position.line as usize];
                
                // Find word at position
                let mut start = position.character as usize;
                let mut end = position.character as usize;
                
                // Find start of word
                while start > 0 && line.chars().nth(start - 1).map_or(false, |c| c.is_alphanumeric() || c == '_') {
                    start -= 1;
                }
                
                // Find end of word
                while end < line.len() && line.chars().nth(end).map_or(false, |c| c.is_alphanumeric() || c == '_') {
                    end += 1;
                }
                
                if start < end {
                    let identifier = &line[start..end];
                    
                    // Check symbol table first
                    if let Some(symbol) = doc.symbols.get(identifier) {
                        let hover_text = match symbol.kind {
                            SymbolKind::Function => {
                                format!("```zen\n{}: {}\n```\n\nFunction definition",
                                    identifier,
                                    symbol.type_info.as_ref().unwrap_or(&"unknown".to_string())
                                )
                            }
                            SymbolKind::Struct => {
                                format!("```zen\n{}: struct\n```\n\n{}",
                                    identifier,
                                    symbol.type_info.as_ref().unwrap_or(&"Struct definition".to_string())
                                )
                            }
                            SymbolKind::Enum => {
                                format!("```zen\n{}: enum\n```\n\n{}",
                                    identifier,
                                    symbol.type_info.as_ref().unwrap_or(&"Enum definition".to_string())
                                )
                            }
                            SymbolKind::Constant => {
                                format!("```zen\n{} := value\n```\n\nConstant",
                                    identifier
                                )
                            }
                            _ => format!("**{}**\n\nSymbol", identifier),
                        };
                        
                        return Some(Hover {
                            contents: HoverContents::Scalar(MarkedString::String(hover_text)),
                            range: Some(Range {
                                start: Position {
                                    line: position.line,
                                    character: start as u32,
                                },
                                end: Position {
                                    line: position.line,
                                    character: end as u32,
                                },
                            }),
                        });
                    }
                    
                    // Check for built-in types
                    let builtin_types = vec![
                        "bool", "void", "i8", "i16", "i32", "i64",
                        "u8", "u16", "u32", "u64", "usize",
                        "f32", "f64", "string",
                    ];
                    
                    if builtin_types.contains(&identifier) {
                        let hover_text = format!("```zen\n{}\n```\n\nBuilt-in type", identifier);
                        return Some(Hover {
                            contents: HoverContents::Scalar(MarkedString::String(hover_text)),
                            range: Some(Range {
                                start: Position {
                                    line: position.line,
                                    character: start as u32,
                                },
                                end: Position {
                                    line: position.line,
                                    character: end as u32,
                                },
                            }),
                        });
                    }
                }
            }
        }
        
        None
    }

    fn handle_completion(&self, params: CompletionParams) -> CompletionResponse {
        let mut items = Vec::new();
        
        // Add Zen keywords (note: Zen doesn't use if/else/match)
        let keywords = vec![
            "loop", "return", "break", "continue", "defer", "comptime",
            "type", "struct", "enum", "import", "extern", "test",
        ];
        
        for keyword in keywords {
            items.push(CompletionItem {
                label: keyword.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Zen keyword".to_string()),
                ..Default::default()
            });
        }
        
        // Add Zen primitive types
        let types = vec![
            "bool", "void",
            "i8", "i16", "i32", "i64",
            "u8", "u16", "u32", "u64", "usize",
            "f32", "f64",
            "string",
        ];
        
        for type_name in types {
            items.push(CompletionItem {
                label: type_name.to_string(),
                kind: Some(CompletionItemKind::CLASS),
                detail: Some("Built-in type".to_string()),
                ..Default::default()
            });
        }
        
        // Add pattern matching operators
        items.push(CompletionItem {
            label: "?".to_string(),
            kind: Some(CompletionItemKind::OPERATOR),
            detail: Some("Pattern matching operator".to_string()),
            ..Default::default()
        });
        
        CompletionResponse::Array(items)
    }

    fn handle_goto_definition(&self, params: GotoDefinitionParams) -> Option<GotoDefinitionResponse> {
        let uri = params.text_document_position_params.text_document.uri.to_string();
        let position = params.text_document_position_params.position;
        
        if let Some(doc) = self.documents.get(&uri) {
            // Find word at position
            let lines: Vec<&str> = doc.content.lines().collect();
            if position.line < lines.len() as u32 {
                let line = lines[position.line as usize];
                
                // Find start and end of word at position
                let mut start = position.character as usize;
                let mut end = position.character as usize;
                
                // Find start of word
                while start > 0 && line.chars().nth(start - 1).map_or(false, |c| c.is_alphanumeric() || c == '_') {
                    start -= 1;
                }
                
                // Find end of word
                while end < line.len() && line.chars().nth(end).map_or(false, |c| c.is_alphanumeric() || c == '_') {
                    end += 1;
                }
                
                if start < end {
                    let identifier = &line[start..end];
                    
                    // First check local symbol table
                    if let Some(symbol) = doc.symbols.get(identifier) {
                        return Some(GotoDefinitionResponse::Scalar(symbol.location.clone()));
                    }
                    
                    // Parse and search for definition in the AST
                    let lexer = Lexer::new(&doc.content);
                    let mut parser = Parser::new(lexer);
                    
                    if let Ok(program) = parser.parse_program() {
                        // Search through declarations to find definition
                        for (index, line_content) in doc.content.lines().enumerate() {
                            // Simple heuristic: look for pattern "identifier:" at start of line
                            let trimmed = line_content.trim_start();
                            if trimmed.starts_with(identifier) {
                                let after_ident = &trimmed[identifier.len()..];
                                if after_ident.starts_with(':') || after_ident.starts_with("<") {
                                    // Found a likely definition
                                    let column = line_content.len() - trimmed.len();
                                    return Some(GotoDefinitionResponse::Scalar(Location {
                                        uri: params.text_document_position_params.text_document.uri.clone(),
                                        range: Range {
                                            start: Position { 
                                                line: index as u32, 
                                                character: column as u32 
                                            },
                                            end: Position { 
                                                line: index as u32, 
                                                character: (column + identifier.len()) as u32 
                                            },
                                        },
                                    }));
                                }
                            }
                        }
                    }
                }
            }
        }
        
        None
    }

    fn handle_did_open(&mut self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let content = params.text_document.text;
        let version = params.text_document.version;
        
        let mut symbols = HashMap::new();
        self.extract_symbols(&content, &uri, &mut symbols);
        
        self.documents.insert(uri.clone(), DocumentState {
            content: content.clone(),
            version,
            symbols,
        });
        
        // Parse and send diagnostics
        self.analyze_and_publish_diagnostics(&uri, &content);
    }

    fn handle_did_change(&mut self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let version = params.text_document.version;
        
        // Assuming full document sync
        if let Some(change) = params.content_changes.first() {
            let mut symbols = HashMap::new();
            self.extract_symbols(&change.text, &uri, &mut symbols);
            
            self.documents.insert(uri.clone(), DocumentState {
                content: change.text.clone(),
                version,
                symbols,
            });
            
            // Re-analyze and send diagnostics
            self.analyze_and_publish_diagnostics(&uri, &change.text);
        }
    }

    fn handle_did_close(&mut self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        self.documents.remove(&uri);
    }

    fn analyze_and_publish_diagnostics(&self, uri: &str, content: &str) {
        let mut diagnostics = Vec::new();
        
        // Parse the document
        let lexer = Lexer::new(content);
        let mut parser = Parser::new(lexer);
        
        // Try to parse and collect errors
        if let Err(e) = parser.parse_program() {
            // Extract error position from the error if available
            let (line, character) = if let zen::error::CompileError::SyntaxError(msg, Some(span)) = &e {
                // Use actual span from error, converting from 1-based to 0-based
                (span.line.saturating_sub(1) as u32, span.column.saturating_sub(1) as u32)
            } else if let zen::error::CompileError::UnexpectedToken(token, Some(span)) = &e {
                (span.line.saturating_sub(1) as u32, span.column.saturating_sub(1) as u32)
            } else {
                // Default to first line if no span available
                (0, 0)
            };
            
            // Create a more accurate diagnostic
            let diagnostic = Diagnostic {
                range: Range {
                    start: Position { line, character },
                    end: Position { line, character: character + 10 }, // Approximate end
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                source: Some("zen-lsp".to_string()),
                message: match &e {
                    zen::error::CompileError::SyntaxError(msg, _) => msg.clone(),
                    zen::error::CompileError::UnexpectedToken(token, _) => 
                        format!("Unexpected token: {:?}", token),
                    _ => format!("{:?}", e),
                },
                related_information: None,
                tags: None,
                code_description: None,
                data: None,
            };
            diagnostics.push(diagnostic);
        }
        
        // Publish diagnostics
        if let Ok(uri) = Url::parse(uri) {
            let params = PublishDiagnosticsParams {
                uri,
                diagnostics,
                version: None,
            };
            
            // Send notification through connection
            let notification = lsp_server::Notification {
                method: PublishDiagnostics::METHOD.to_string(),
                params: serde_json::to_value(params).unwrap(),
            };
            
            let _ = self.connection.sender.send(Message::Notification(notification));
        }
    }
    
    fn extract_symbols(&self, content: &str, uri: &str, symbols: &mut HashMap<String, SymbolInfo>) {
        let lexer = Lexer::new(content);
        let mut parser = Parser::new(lexer);
        
        if let Ok(program) = parser.parse_program() {
            for decl in &program.declarations {
                use zen::ast::Declaration;
                match decl {
                    Declaration::Function(func) => {
                        let type_info = format!("({}) -> {:?}", 
                            func.args.iter()
                                .map(|(name, ty)| format!("{}: {:?}", name, ty))
                                .collect::<Vec<_>>()
                                .join(", "),
                            func.return_type
                        );
                        
                        symbols.insert(func.name.clone(), SymbolInfo {
                            kind: SymbolKind::Function,
                            location: Location {
                                uri: Url::parse(uri).unwrap_or_else(|_| Url::parse("file:///unknown").unwrap()),
                                range: Range {
                                    start: Position { line: 0, character: 0 },
                                    end: Position { line: 0, character: 0 },
                                },
                            },
                            type_info: Some(type_info),
                        });
                    }
                    Declaration::Struct(s) => {
                        symbols.insert(s.name.clone(), SymbolInfo {
                            kind: SymbolKind::Struct,
                            location: Location {
                                uri: Url::parse(uri).unwrap_or_else(|_| Url::parse("file:///unknown").unwrap()),
                                range: Range {
                                    start: Position { line: 0, character: 0 },
                                    end: Position { line: 0, character: 0 },
                                },
                            },
                            type_info: Some(format!("struct with {} fields", s.fields.len())),
                        });
                    }
                    Declaration::Enum(e) => {
                        symbols.insert(e.name.clone(), SymbolInfo {
                            kind: SymbolKind::Enum,
                            location: Location {
                                uri: Url::parse(uri).unwrap_or_else(|_| Url::parse("file:///unknown").unwrap()),
                                range: Range {
                                    start: Position { line: 0, character: 0 },
                                    end: Position { line: 0, character: 0 },
                                },
                            },
                            type_info: Some(format!("enum with {} variants", e.variants.len())),
                        });
                    }
                    Declaration::Constant { name, .. } => {
                        symbols.insert(name.clone(), SymbolInfo {
                            kind: SymbolKind::Constant,
                            location: Location {
                                uri: Url::parse(uri).unwrap_or_else(|_| Url::parse("file:///unknown").unwrap()),
                                range: Range {
                                    start: Position { line: 0, character: 0 },
                                    end: Position { line: 0, character: 0 },
                                },
                            },
                            type_info: None,
                        });
                    }
                    _ => {}
                }
            }
        }
    }
}

fn main() {
    // Handle command line arguments BEFORE creating connection
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        if args[1] == "--version" || args[1] == "-v" {
            println!("zen-lsp 0.1.0");
            std::process::exit(0);
        }
        if args[1] == "--help" || args[1] == "-h" {
            println!("Zen Language Server");
            println!("Usage: zen-lsp [options]");
            println!("Options:");
            println!("  --version, -v  Show version");
            println!("  --help, -h     Show this help");
            std::process::exit(0);
        }
    }
    
    // Run the LSP server
    if let Err(e) = run_server() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run_server() -> Result<()> {
    eprintln!("Starting Zen Language Server...");
    
    // Create the transport
    let (connection, io_threads) = Connection::stdio();
    
    // Run the server
    let server_capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::FULL,
        )),
        hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
        completion_provider: Some(lsp_types::CompletionOptions {
            resolve_provider: Some(false),
            trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
            ..Default::default()
        }),
        definition_provider: Some(lsp_types::OneOf::Left(true)),
        ..Default::default()
    };
    
    let initialize_params = connection.initialize(serde_json::to_value(server_capabilities)?)?;
    main_loop(connection, initialize_params)?;
    io_threads.join()?;
    
    eprintln!("Zen Language Server shutting down");
    Ok(())
}

fn main_loop(connection: Connection, params: Value) -> Result<()> {
    let _params: InitializeParams = serde_json::from_value(params)?;
    let mut server = LspServer::new(connection.clone());
    
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                
                let req_id = req.id.clone();
                
                if req.method == Initialize::METHOD {
                    // Already handled by connection.initialize()
                    continue;
                }
                
                let response = match req.method.as_str() {
                    HoverRequest::METHOD => {
                        let params: HoverParams = serde_json::from_value(req.params)?;
                        let hover = server.handle_hover(params);
                        Response {
                            id: req_id,
                            result: Some(serde_json::to_value(hover)?),
                            error: None,
                        }
                    }
                    Completion::METHOD => {
                        let params: CompletionParams = serde_json::from_value(req.params)?;
                        let completions = server.handle_completion(params);
                        Response {
                            id: req_id,
                            result: Some(serde_json::to_value(completions)?),
                            error: None,
                        }
                    }
                    GotoDefinition::METHOD => {
                        let params: GotoDefinitionParams = serde_json::from_value(req.params)?;
                        let definition = server.handle_goto_definition(params);
                        Response {
                            id: req_id,
                            result: Some(serde_json::to_value(definition)?),
                            error: None,
                        }
                    }
                    _ => {
                        eprintln!("Unhandled request: {}", req.method);
                        continue;
                    }
                };
                
                connection.sender.send(Message::Response(response))?;
            }
            Message::Notification(notif) => {
                match notif.method.as_str() {
                    DidOpenTextDocument::METHOD => {
                        let params: DidOpenTextDocumentParams = serde_json::from_value(notif.params)?;
                        server.handle_did_open(params);
                    }
                    DidChangeTextDocument::METHOD => {
                        let params: DidChangeTextDocumentParams = serde_json::from_value(notif.params)?;
                        server.handle_did_change(params);
                    }
                    DidCloseTextDocument::METHOD => {
                        let params: DidCloseTextDocumentParams = serde_json::from_value(notif.params)?;
                        server.handle_did_close(params);
                    }
                    _ => {
                        eprintln!("Unhandled notification: {}", notif.method);
                    }
                }
            }
            Message::Response(_) => {
                // We don't send requests, so we shouldn't receive responses
            }
        }
    }
    
    Ok(())
}