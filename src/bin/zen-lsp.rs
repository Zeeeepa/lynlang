// Zen Language Server Protocol Implementation
// Provides IDE support for the Zen programming language

use anyhow::{Context, Result};
use std::io::{self, Write};
use lsp_server::{Connection, Message, Request, RequestId, Response};
use lsp_types::notification::{DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument};
use lsp_types::request::{
    Completion, GotoDefinition, HoverRequest, Initialize, Shutdown,
};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents, HoverParams,
    InitializeParams, InitializeResult, Location, MarkedString, Position, Range,
    ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use zen::lexer::Lexer;
use zen::parser::Parser;

struct LspServer {
    documents: HashMap<String, DocumentState>,
}

struct DocumentState {
    content: String,
    version: i32,
}

impl LspServer {
    fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    fn handle_hover(&self, params: HoverParams) -> Option<Hover> {
        let uri = params.text_document_position_params.text_document.uri.to_string();
        let position = params.text_document_position_params.position;
        
        if let Some(doc) = self.documents.get(&uri) {
            // Parse the document
            let mut lexer = Lexer::new(&doc.content);
            let tokens = lexer.tokenize();
            
            // Find token at position
            for token in &tokens {
                if token.line == position.line && 
                   token.column <= position.character &&
                   token.column + token.lexeme.len() as u32 > position.character {
                    
                    // Generate hover content based on token type
                    let hover_text = format!("**{}**\n\nType: {:?}", token.lexeme, token.kind);
                    
                    return Some(Hover {
                        contents: HoverContents::Scalar(MarkedString::String(hover_text)),
                        range: Some(Range {
                            start: Position {
                                line: token.line,
                                character: token.column,
                            },
                            end: Position {
                                line: token.line,
                                character: token.column + token.lexeme.len() as u32,
                            },
                        }),
                    });
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
        // TODO: Implement go-to-definition
        None
    }

    fn handle_did_open(&mut self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let content = params.text_document.text;
        let version = params.text_document.version;
        
        self.documents.insert(uri.clone(), DocumentState {
            content: content.clone(),
            version,
        });
        
        // Parse and send diagnostics
        self.analyze_and_publish_diagnostics(&uri, &content);
    }

    fn handle_did_change(&mut self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let version = params.text_document.version;
        
        // Assuming full document sync
        if let Some(change) = params.content_changes.first() {
            self.documents.insert(uri.clone(), DocumentState {
                content: change.text.clone(),
                version,
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
        // Parse the document
        let mut lexer = Lexer::new(content);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let _ast = parser.parse();
        
        // TODO: Convert parser errors to LSP diagnostics
        // For now, we'll just parse without sending diagnostics
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
    let mut server = LspServer::new();
    
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