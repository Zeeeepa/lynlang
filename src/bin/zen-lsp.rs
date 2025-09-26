use lsp_server::{Connection, Message, Request, Response};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionOptions, CompletionParams, CompletionResponse,
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
    HoverContents, HoverParams, HoverProviderCapability, InitializeParams, InitializeResult,
    InitializedParams, MarkedString, Position, PublishDiagnosticsParams, Range, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, Url, WorkDoneProgressOptions,
};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use zen::compiler::Compiler;
use zen::lexer::Lexer;
use zen::parser::Parser;

fn main() -> Result<(), Box<dyn Error>> {
    eprintln!("Starting Zen Language Server...");

    // Create connection
    let (connection, io_threads) = Connection::stdio();

    // Wait for client to initialize
    let server_capabilities = serde_json::to_value(&ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::FULL,
        )),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        completion_provider: Some(CompletionOptions {
            resolve_provider: Some(false),
            trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
            work_done_progress_options: WorkDoneProgressOptions::default(),
            all_commit_characters: None,
            completion_item: None,
        }),
        ..Default::default()
    })?;

    let initialization_params = connection.initialize(server_capabilities)?;
    main_loop(connection, initialization_params)?;
    io_threads.join()?;

    eprintln!("Zen Language Server shutting down");
    Ok(())
}

fn main_loop(
    connection: Connection,
    params: serde_json::Value,
) -> Result<(), Box<dyn Error>> {
    let _params: InitializeParams = serde_json::from_value(params)?;
    eprintln!("Zen LSP initialized");

    // Track open documents
    let mut documents: HashMap<Url, String> = HashMap::new();

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                
                // Handle different request types
                let response = if req.method == "textDocument/hover" {
                    handle_hover(req.clone())
                } else if req.method == "textDocument/completion" {
                    handle_completion(req.clone())
                } else {
                    // Return empty response for unsupported requests
                    Response {
                        id: req.id.clone(),
                        result: Some(Value::Null),
                        error: None,
                    }
                };
                
                connection.sender.send(Message::Response(response))?;
            }
            Message::Notification(notif) => {
                if notif.method == "textDocument/didOpen" {
                    let params: DidOpenTextDocumentParams = serde_json::from_value(notif.params)?;
                    let uri = params.text_document.uri;
                    let text = params.text_document.text;
                    
                    // Store document
                    documents.insert(uri.clone(), text.clone());
                    
                    // Validate and publish diagnostics
                    let diagnostics = validate_document(&text);
                    let params = PublishDiagnosticsParams {
                        uri,
                        diagnostics,
                        version: None,
                    };
                    
                    let notification = lsp_server::Notification {
                        method: "textDocument/publishDiagnostics".to_string(),
                        params: serde_json::to_value(params)?,
                    };
                    connection.sender.send(Message::Notification(notification))?;
                } else if notif.method == "textDocument/didChange" {
                    let params: DidChangeTextDocumentParams = serde_json::from_value(notif.params)?;
                    let uri = params.text_document.uri;
                    
                    // Update document
                    if let Some(changes) = params.content_changes.first() {
                        documents.insert(uri.clone(), changes.text.clone());
                        
                        // Re-validate and publish diagnostics
                        let diagnostics = validate_document(&changes.text);
                        let params = PublishDiagnosticsParams {
                            uri,
                            diagnostics,
                            version: None,
                        };
                        
                        let notification = lsp_server::Notification {
                            method: "textDocument/publishDiagnostics".to_string(),
                            params: serde_json::to_value(params)?,
                        };
                        connection.sender.send(Message::Notification(notification))?;
                    }
                } else if notif.method == "initialized" {
                    let _params: InitializedParams = serde_json::from_value(notif.params)?;
                    eprintln!("Client initialized");
                }
            }
            Message::Response(_) => {}
        }
    }
    
    Ok(())
}

fn validate_document(text: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    
    // Try to parse the document
    let lexer = Lexer::new(text);
    let mut parser = Parser::new(lexer);
    
    match parser.parse_program() {
        Ok(_) => {
            // Document is valid
        }
        Err(err) => {
            // Create diagnostic from parse error
            let diagnostic = Diagnostic {
                range: Range {
                    start: Position { line: 0, character: 0 },
                    end: Position { line: 0, character: 1 },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("zen".to_string()),
                message: format!("{:?}", err),
                related_information: None,
                tags: None,
                data: None,
            };
            diagnostics.push(diagnostic);
        }
    }
    
    diagnostics
}

fn handle_hover(req: Request) -> Response {
    let params: HoverParams = match serde_json::from_value(req.params) {
        Ok(p) => p,
        Err(_) => {
            return Response {
                id: req.id,
                result: Some(Value::Null),
                error: None,
            };
        }
    };
    
    // For now, return basic hover info
    let hover_text = "Zen Language Element";
    let contents = HoverContents::Scalar(MarkedString::String(hover_text.to_string()));
    
    Response {
        id: req.id,
        result: Some(serde_json::to_value(lsp_types::Hover {
            contents,
            range: None,
        }).unwrap_or(Value::Null)),
        error: None,
    }
}

fn handle_completion(req: Request) -> Response {
    let _params: CompletionParams = match serde_json::from_value(req.params) {
        Ok(p) => p,
        Err(_) => {
            return Response {
                id: req.id,
                result: Some(Value::Null),
                error: None,
            };
        }
    };
    
    // Provide basic completions
    let completions = vec![
        CompletionItem {
            label: "main".to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("main = () i32 { ... }".to_string()),
            documentation: None,
            ..Default::default()
        },
        CompletionItem {
            label: "Option".to_string(),
            kind: Some(CompletionItemKind::ENUM),
            detail: Some("Option<T> enum type".to_string()),
            documentation: None,
            ..Default::default()
        },
        CompletionItem {
            label: "Result".to_string(),
            kind: Some(CompletionItemKind::ENUM),
            detail: Some("Result<T, E> enum type".to_string()),
            documentation: None,
            ..Default::default()
        },
        CompletionItem {
            label: "Some".to_string(),
            kind: Some(CompletionItemKind::ENUM_MEMBER),
            detail: Some("Some(value) - Option variant".to_string()),
            documentation: None,
            ..Default::default()
        },
        CompletionItem {
            label: "None".to_string(),
            kind: Some(CompletionItemKind::ENUM_MEMBER),
            detail: Some("None - Option variant".to_string()),
            documentation: None,
            ..Default::default()
        },
        CompletionItem {
            label: "@std".to_string(),
            kind: Some(CompletionItemKind::MODULE),
            detail: Some("Standard library module".to_string()),
            documentation: None,
            ..Default::default()
        },
        CompletionItem {
            label: "loop".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("loop() { ... } - Infinite loop with break".to_string()),
            documentation: None,
            ..Default::default()
        },
        CompletionItem {
            label: "return".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("return value - Return from function".to_string()),
            documentation: None,
            ..Default::default()
        },
    ];
    
    let response = CompletionResponse::Array(completions);
    
    Response {
        id: req.id,
        result: Some(serde_json::to_value(response).unwrap_or(Value::Null)),
        error: None,
    }
}