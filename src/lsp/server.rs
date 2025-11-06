// Enhanced LSP Server for Zen Language
// Provides advanced IDE features with compiler integration

use lsp_server::{Connection, Message, Request, Response, ResponseError, ErrorCode, Notification as ServerNotification};
use lsp_types::*;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Sender, Receiver, TryRecvError};
use std::thread;

use inkwell::context::Context;

// AST types used in type_inference.rs
use crate::compiler::Compiler;
use crate::error::CompileError;

use super::types::{Document, SymbolInfo, AnalysisJob, AnalysisResult, UfcMethodInfo, ZenCompletionContext, SymbolScope};
use super::document_store::DocumentStore;
use super::utils::{format_type, symbol_kind_to_completion_kind};
use super::type_inference::{
    parse_generic_type, infer_receiver_type_from_store, infer_function_return_types
};
use super::hover::handle_hover;
use super::formatting::handle_formatting;
use super::semantic_tokens::handle_semantic_tokens;
use super::call_hierarchy::{handle_prepare_call_hierarchy, handle_incoming_calls, handle_outgoing_calls};
use super::rename::{handle_rename, determine_symbol_scope, find_function_range};
use super::signature_help::handle_signature_help;
use super::inlay_hints::handle_inlay_hints;
use super::code_lens::handle_code_lens;
use super::code_action::handle_code_action;

// ============================================================================
// TYPES (moved to types.rs and document_store.rs)
// ============================================================================

// Convert CompileError to LSP Diagnostic (standalone function for reuse)
fn compile_error_to_diagnostic(error: CompileError) -> Diagnostic {

    // Extract span and determine severity
    let (span, severity, code) = match &error {
        CompileError::ParseError(_, span) => (span.clone(), DiagnosticSeverity::ERROR, Some("parse-error")),
        CompileError::SyntaxError(_, span) => (span.clone(), DiagnosticSeverity::ERROR, Some("syntax-error")),
        CompileError::TypeError(_, span) => (span.clone(), DiagnosticSeverity::ERROR, Some("type-error")),
        CompileError::TypeMismatch { span, .. } => (span.clone(), DiagnosticSeverity::ERROR, Some("type-mismatch")),
        CompileError::UndeclaredVariable(_, span) => (span.clone(), DiagnosticSeverity::ERROR, Some("undeclared-variable")),
        CompileError::UndeclaredFunction(_, span) => (span.clone(), DiagnosticSeverity::ERROR, Some("undeclared-function")),
        CompileError::UnexpectedToken { span, .. } => (span.clone(), DiagnosticSeverity::ERROR, Some("unexpected-token")),
        CompileError::InvalidPattern(_, span) => (span.clone(), DiagnosticSeverity::ERROR, Some("invalid-pattern")),
        CompileError::InvalidSyntax { span, .. } => (span.clone(), DiagnosticSeverity::ERROR, Some("invalid-syntax")),
        CompileError::MissingTypeAnnotation(_, span) => (span.clone(), DiagnosticSeverity::WARNING, Some("missing-type")),
        CompileError::DuplicateDeclaration { duplicate_location, .. } => (duplicate_location.clone(), DiagnosticSeverity::ERROR, Some("duplicate-declaration")),
        CompileError::ImportError(_, span) => (span.clone(), DiagnosticSeverity::ERROR, Some("import-error")),
        CompileError::FFIError(_, span) => (span.clone(), DiagnosticSeverity::ERROR, Some("ffi-error")),
        CompileError::InvalidLoopCondition(_, span) => (span.clone(), DiagnosticSeverity::ERROR, Some("invalid-loop")),
        CompileError::MissingReturnStatement(_, span) => (span.clone(), DiagnosticSeverity::WARNING, Some("missing-return")),
        CompileError::InternalError(_, span) => (span.clone(), DiagnosticSeverity::ERROR, Some("internal-error")),
        CompileError::UnsupportedFeature(_, span) => (span.clone(), DiagnosticSeverity::WARNING, Some("unsupported-feature")),
        CompileError::FileNotFound(_, _) => (None, DiagnosticSeverity::ERROR, Some("file-not-found")),
        CompileError::ComptimeError(_) => (None, DiagnosticSeverity::ERROR, Some("comptime-error")),
        CompileError::BuildError(_) => (None, DiagnosticSeverity::ERROR, Some("build-error")),
        CompileError::FileError(_) => (None, DiagnosticSeverity::ERROR, Some("file-error")),
        CompileError::CyclicDependency(_) => (None, DiagnosticSeverity::ERROR, Some("cyclic-dependency")),
    };

    // Convert span to LSP range
    let (start_pos, end_pos) = if let Some(span) = span {
        let start = Position {
            line: if span.line > 0 { span.line as u32 - 1 } else { 0 },
            character: span.column as u32,
        };
        let end = Position {
            line: if span.line > 0 { span.line as u32 - 1 } else { 0 },
            character: (span.column + (span.end - span.start).max(1)) as u32,
        };
        (start, end)
    } else {
        (Position { line: 0, character: 0 }, Position { line: 0, character: 1 })
    };

    Diagnostic {
        range: Range {
            start: start_pos,
            end: end_pos,
        },
        severity: Some(severity),
        code: code.map(|c| lsp_types::NumberOrString::String(c.to_string())),
        code_description: None,
        source: Some("zen-compiler".to_string()),
        message: format!("{}", error),
        related_information: None,
        tags: None,
        data: None,
    }
}

// DocumentStore implementation moved to document_store.rs
// Using the one from document_store module

// ============================================================================
// ENHANCED LSP SERVER
// ============================================================================

pub struct ZenLanguageServer {
    connection: Connection,
    store: Arc<Mutex<DocumentStore>>,
    capabilities: ServerCapabilities,
}

impl ZenLanguageServer {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let (connection, _io_threads) = Connection::stdio();
        
        let capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::INCREMENTAL,
            )),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            completion_provider: Some(CompletionOptions {
                resolve_provider: Some(true),
                trigger_characters: Some(vec![
                    ".".to_string(), 
                    ":".to_string(),
                    "@".to_string(),
                    "?".to_string(),
                ]),
                work_done_progress_options: WorkDoneProgressOptions::default(),
                all_commit_characters: None,
                completion_item: None,
            }),
            signature_help_provider: Some(SignatureHelpOptions {
                trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                retrigger_characters: None,
                work_done_progress_options: WorkDoneProgressOptions::default(),
            }),
            definition_provider: Some(OneOf::Left(true)),
            type_definition_provider: Some(TypeDefinitionProviderCapability::Simple(true)),
            references_provider: Some(OneOf::Left(true)),
            document_highlight_provider: Some(OneOf::Left(true)),
            document_symbol_provider: Some(OneOf::Left(true)),
            workspace_symbol_provider: Some(OneOf::Left(true)),
            code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
            code_lens_provider: Some(CodeLensOptions {
                resolve_provider: Some(false),
            }),
            document_formatting_provider: Some(OneOf::Left(true)),
            document_range_formatting_provider: Some(OneOf::Left(true)),
            rename_provider: Some(OneOf::Right(RenameOptions {
                prepare_provider: Some(true),
                work_done_progress_options: WorkDoneProgressOptions::default(),
            })),
            folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
            inlay_hint_provider: Some(OneOf::Left(true)),
            call_hierarchy_provider: Some(CallHierarchyServerCapability::Simple(true)),
            semantic_tokens_provider: Some(
                SemanticTokensServerCapabilities::SemanticTokensRegistrationOptions(
                    SemanticTokensRegistrationOptions {
                        text_document_registration_options: TextDocumentRegistrationOptions {
                            document_selector: Some(vec![
                                DocumentFilter {
                                    language: Some("zen".to_string()),
                                    scheme: None,
                                    pattern: None,
                                }
                            ]),
                        },
                        semantic_tokens_options: SemanticTokensOptions {
                            work_done_progress_options: WorkDoneProgressOptions { work_done_progress: None },
                            legend: SemanticTokensLegend {
                                token_types: vec![
                                    SemanticTokenType::NAMESPACE,
                                    SemanticTokenType::TYPE,
                                    SemanticTokenType::CLASS,
                                    SemanticTokenType::ENUM,
                                    SemanticTokenType::INTERFACE,
                                    SemanticTokenType::STRUCT,
                                    SemanticTokenType::TYPE_PARAMETER,
                                    SemanticTokenType::PARAMETER,
                                    SemanticTokenType::VARIABLE,
                                    SemanticTokenType::PROPERTY,
                                    SemanticTokenType::ENUM_MEMBER,
                                    SemanticTokenType::EVENT,
                                    SemanticTokenType::FUNCTION,
                                    SemanticTokenType::METHOD,
                                    SemanticTokenType::MACRO,
                                    SemanticTokenType::KEYWORD,
                                    SemanticTokenType::MODIFIER,
                                    SemanticTokenType::COMMENT,
                                    SemanticTokenType::STRING,
                                    SemanticTokenType::NUMBER,
                                    SemanticTokenType::REGEXP,
                                    SemanticTokenType::OPERATOR,
                                ],
                                token_modifiers: vec![
                                    SemanticTokenModifier::DECLARATION,
                                    SemanticTokenModifier::DEFINITION,
                                    SemanticTokenModifier::READONLY,
                                    SemanticTokenModifier::STATIC,
                                    SemanticTokenModifier::DEPRECATED,
                                    SemanticTokenModifier::ABSTRACT,
                                    SemanticTokenModifier::ASYNC,
                                    SemanticTokenModifier::MODIFICATION,
                                    SemanticTokenModifier::DOCUMENTATION,
                                    SemanticTokenModifier::DEFAULT_LIBRARY,
                                ],
                            },
                            full: Some(SemanticTokensFullOptions::Delta { delta: Some(true) }),
                            range: Some(false),
                        },
                        static_registration_options: StaticRegistrationOptions { id: None },
                    }
                )
            ),
            ..Default::default()
        };

        Ok(Self {
            connection,
            store: Arc::new(Mutex::new(DocumentStore::new())),
            capabilities,
        })
    }

    pub fn run(mut self) -> Result<(), Box<dyn Error>> {
        // Starting Enhanced Zen Language Server

        let server_capabilities = serde_json::to_value(&self.capabilities)?;
        let initialization_params = self.connection.initialize(server_capabilities)?;

        if let Ok(params) = serde_json::from_value::<InitializeParams>(initialization_params) {
            // Try workspace_folders first (preferred), fall back to root_uri for compatibility
            #[allow(deprecated)]
            if let Some(folders) = params.workspace_folders {
                if let Some(first_folder) = folders.first() {
                    self.store.lock().unwrap().set_workspace_root(first_folder.uri.clone());
                }
            } else if let Some(root_uri) = params.root_uri {
                self.store.lock().unwrap().set_workspace_root(root_uri);
            }
        }

        // Start background analysis thread
        let (analysis_tx, analysis_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();

        // Give the analysis sender to the document store
        self.store.lock().unwrap().set_analysis_sender(analysis_tx);

        // Spawn background analysis worker
        let _analysis_thread = thread::spawn(move || {
            Self::background_analysis_worker(analysis_rx, result_tx);
        });

        // Zen LSP initialized with enhanced capabilities

        // Start main loop with result receiver for async diagnostics
        self.main_loop_with_background(result_rx)?;

        // Zen Language Server shutting down
        Ok(())
    }

    fn background_analysis_worker(job_rx: Receiver<AnalysisJob>, result_tx: Sender<AnalysisResult>) {
        // Background analysis worker started

        // Create LLVM context and compiler (reused for all analyses)
        let context = Context::create();
        let compiler = Compiler::new(&context);

        while let Ok(job) = job_rx.recv() {
            // Analyzing document

            let errors = compiler.analyze_for_diagnostics(&job.program);

            // Analysis complete

            // Convert compiler errors to LSP diagnostics using the shared function
            let diagnostics: Vec<Diagnostic> = errors
                .into_iter()
                .map(compile_error_to_diagnostic)
                .collect();

            let result = AnalysisResult {
                uri: job.uri,
                version: job.version,
                diagnostics,
            };

            // Send result back (ignore if receiver disconnected)
            let _ = result_tx.send(result);
        }

        // Background analysis worker stopped
    }

    fn main_loop_with_background(&mut self, result_rx: Receiver<AnalysisResult>) -> Result<(), Box<dyn Error>> {
        loop {
            // Check for background analysis results (non-blocking)
            match result_rx.try_recv() {
                Ok(result) => {
                    // Publishing background diagnostics
                    self.publish_diagnostics(result.uri, result.diagnostics)?;
                }
                Err(TryRecvError::Empty) => {
                    // No results ready, continue
                }
                Err(TryRecvError::Disconnected) => {
                    // Background analysis thread disconnected
                    break;
                }
            }

            // Handle LSP messages (with timeout to check background results)
            use std::time::Duration;
            let timeout = Duration::from_millis(100);

            match self.connection.receiver.recv_timeout(timeout) {
                Ok(msg) => {
                    match msg {
                        Message::Request(req) => {
                            if self.connection.handle_shutdown(&req)? {
                                return Ok(());
                            }
                            self.handle_request(req)?;
                        }
                        Message::Notification(notif) => {
                            self.handle_notification(notif)?;
                        }
                        Message::Response(_) => {}
                    }
                }
                Err(err) => {
                    // Handle connection errors
                    // If it's a timeout, continue; if disconnected, exit
                    if err.is_timeout() {
                        // Timeout is expected, continue loop
                    } else {
                        // Connection closed or other error, exit gracefully
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    fn main_loop(&mut self) -> Result<(), Box<dyn Error>> {
        for msg in &self.connection.receiver {
            match msg {
                Message::Request(req) => {
                    if self.connection.handle_shutdown(&req)? {
                        return Ok(());
                    }
                    self.handle_request(req)?;
                }
                Message::Notification(notif) => {
                    self.handle_notification(notif)?;
                }
                Message::Response(_) => {}
            }
        }
        Ok(())
    }

    fn handle_request(&self, req: Request) -> Result<(), Box<dyn Error>> {
        // Handling request
        let response = match req.method.as_str() {
            "textDocument/hover" => self.handle_hover(req.clone()),
            "textDocument/completion" => self.handle_completion(req.clone()),
            "textDocument/definition" => self.handle_definition(req.clone()),
            "textDocument/typeDefinition" => self.handle_type_definition(req.clone()),
            "textDocument/references" => self.handle_references(req.clone()),
            "textDocument/documentHighlight" => self.handle_document_highlight(req.clone()),
            "textDocument/documentSymbol" => self.handle_document_symbols(req.clone()),
            "textDocument/formatting" => self.handle_formatting(req.clone()),
            "textDocument/rename" => self.handle_rename(req.clone()),
            "textDocument/codeAction" => self.handle_code_action(req.clone()),
            "textDocument/semanticTokens/full" => self.handle_semantic_tokens(req.clone()),
            "textDocument/signatureHelp" => self.handle_signature_help(req.clone()),
            "textDocument/inlayHint" => self.handle_inlay_hints(req.clone()),
            "textDocument/codeLens" => self.handle_code_lens(req.clone()),
            "workspace/symbol" => self.handle_workspace_symbol(req.clone()),
            "textDocument/prepareCallHierarchy" => self.handle_prepare_call_hierarchy(req.clone()),
            "callHierarchy/incomingCalls" => self.handle_incoming_calls(req.clone()),
            "callHierarchy/outgoingCalls" => self.handle_outgoing_calls(req.clone()),
            _ => Response {
                id: req.id,
                result: Some(Value::Null),
                error: None,
            },
        };

        self.connection.sender.send(Message::Response(response))?;
        Ok(())
    }

    fn handle_notification(&self, notif: ServerNotification) -> Result<(), Box<dyn Error>> {
        match notif.method.as_str() {
            "textDocument/didOpen" => {
                let params: DidOpenTextDocumentParams = serde_json::from_value(notif.params)?;
                let diagnostics = self.store.lock().unwrap().open(
                    params.text_document.uri.clone(),
                    params.text_document.version,
                    params.text_document.text,
                );
                self.publish_diagnostics(params.text_document.uri, diagnostics)?;
            }
            "textDocument/didChange" => {
                let params: DidChangeTextDocumentParams = serde_json::from_value(notif.params)?;
                if let Some(change) = params.content_changes.first() {
                    let diagnostics = self.store.lock().unwrap().update(
                        params.text_document.uri.clone(),
                        params.text_document.version,
                        change.text.clone(),
                    );
                    self.publish_diagnostics(params.text_document.uri, diagnostics)?;
                }
            }
            "initialized" => {
                // Client initialized
            }
            _ => {}
        }
        Ok(())
    }

    fn publish_diagnostics(&self, uri: Url, diagnostics: Vec<Diagnostic>) -> Result<(), Box<dyn Error>> {
        // Publishing diagnostics
        for (_i, _diag) in diagnostics.iter().enumerate() {
            // Diagnostic logged
        }

        let params = PublishDiagnosticsParams {
            uri,
            diagnostics,
            version: None,
        };

        let notification = ServerNotification {
            method: "textDocument/publishDiagnostics".to_string(),
            params: serde_json::to_value(params)?,
        };

        self.connection.sender.send(Message::Notification(notification))?;
        Ok(())
    }

    fn handle_hover(&self, req: Request) -> Response {
        handle_hover(req, &self.store)
    }

    fn handle_completion(&self, req: Request) -> Response {
        let params: CompletionParams = match serde_json::from_value(req.params) {
            Ok(p) => p,
            Err(_) => {
                return Response {
                    id: req.id,
                    result: Some(Value::Null),
                    error: None,
                };
            }
        };

        let store = self.store.lock().unwrap();

        // Check if we're completing after a dot (UFC method call)
        if let Some(doc) = store.documents.get(&params.text_document_position.text_document.uri) {
            let position = params.text_document_position.position;

            if let Some(context) = self.get_completion_context(&doc.content, position) {
                match context {
                    ZenCompletionContext::UfcMethod { receiver_type } => {
                        // Provide UFC method completions
                        let completions = self.get_ufc_method_completions(&receiver_type);

                        let response = CompletionResponse::Array(completions);
                        return Response {
                            id: req.id,
                            result: Some(serde_json::to_value(response).unwrap_or(Value::Null)),
                            error: None,
                        };
                    }
                    ZenCompletionContext::General => {
                        // Fall through to provide general completions
                    }
                }
            }
        }

        // Provide general completions
        // TODO: Get keywords and types from compiler instead of hardcoded data
        let mut completions = Vec::new();

        // Add document symbols (functions, structs, enums defined in current file)
        if let Some(doc) = store.documents.get(&params.text_document_position.text_document.uri) {
            for (name, symbol) in &doc.symbols {
                completions.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(symbol_kind_to_completion_kind(symbol.kind)),
                    detail: symbol.detail.clone(),
                    documentation: None,
                    ..Default::default()
                });
            }
        }

        // Add stdlib symbols (functions and types from standard library)
        for (name, symbol) in &store.stdlib_symbols {
            completions.push(CompletionItem {
                label: name.clone(),
                kind: Some(symbol_kind_to_completion_kind(symbol.kind)),
                detail: symbol.detail.clone(),
                documentation: None,
                ..Default::default()
            });
        }

        // Add workspace symbols (from other files in the project)
        // Limit to prevent overwhelming the user
        let mut workspace_count = 0;
        const MAX_WORKSPACE_COMPLETIONS: usize = 50;

        for (name, symbol) in &store.workspace_symbols {
            if workspace_count >= MAX_WORKSPACE_COMPLETIONS {
                break;
            }

            // Only add if not already in completions (avoid duplicates)
            if !completions.iter().any(|c| c.label == *name) {
                completions.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(symbol_kind_to_completion_kind(symbol.kind)),
                    detail: symbol.detail.clone(),
                    documentation: None,
                    ..Default::default()
                });
                workspace_count += 1;
            }
        }

        let response = CompletionResponse::Array(completions);
        
        Response {
            id: req.id,
            result: Some(serde_json::to_value(response).unwrap_or(Value::Null)),
            error: None,
        }
    }

    fn handle_definition(&self, req: Request) -> Response {
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

        let store = self.store.lock().unwrap();
        if let Some(doc) = store.documents.get(&params.text_document_position_params.text_document.uri) {
            let position = params.text_document_position_params.position;

            // Check if we're on a UFC method call
            if let Some(method_info) = self.find_ufc_method_at_position(&doc.content, position) {
                // Try to resolve the UFC method
                if let Some(location) = self.resolve_ufc_method(&method_info, &store) {
                    return Response {
                        id: req.id,
                        result: Some(serde_json::to_value(GotoDefinitionResponse::Scalar(location)).unwrap_or(Value::Null)),
                        error: None,
                    };
                }
            }

            // Find the symbol at the cursor position
            if let Some(symbol_name) = self.find_symbol_at_position(&doc.content, position) {
                // Check local document symbols first
                if let Some(symbol_info) = doc.symbols.get(&symbol_name) {
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

                // Check stdlib symbols
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

                // Search for the symbol in all open documents
                // Prioritize non-test files over test files
                let mut test_match: Option<(Url, Range)> = None;

                for (uri, other_doc) in &store.documents {
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
            }
        }

        Response {
            id: req.id,
            result: Some(Value::Null),
            error: None,
        }
    }

    fn handle_type_definition(&self, req: Request) -> Response {
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

        let store = self.store.lock().unwrap();
        if let Some(doc) = store.documents.get(&params.text_document_position_params.text_document.uri) {
            let position = params.text_document_position_params.position;

            if let Some(symbol_name) = self.find_symbol_at_position(&doc.content, position) {
                // For type definition, we want to find the definition of the type, not the variable
                // First, check if it's a variable and get its type
                if let Some(symbol_info) = doc.symbols.get(&symbol_name) {
                    // If it's a variable/parameter, extract the type name from its detail
                    if let Some(detail) = &symbol_info.detail {
                        // Extract type name from patterns like "name: Type" or "val: Result<f64, E>"
                        if let Some(type_name) = self.extract_type_name(detail) {
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

    fn extract_type_name(&self, detail: &str) -> Option<String> {
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

    fn handle_document_highlight(&self, req: Request) -> Response {
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

        let store = self.store.lock().unwrap();
        if let Some(doc) = store.documents.get(&params.text_document_position_params.text_document.uri) {
            let position = params.text_document_position_params.position;

            if let Some(symbol_name) = self.find_symbol_at_position(&doc.content, position) {
                // Find all occurrences of this symbol in the current document
                let highlights = self.find_symbol_occurrences(&doc.content, &symbol_name);

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

    fn find_symbol_occurrences(&self, content: &str, symbol_name: &str) -> Vec<DocumentHighlight> {
        let mut highlights = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        for (line_idx, line) in lines.iter().enumerate() {
            let mut start_pos = 0;
            while let Some(pos) = line[start_pos..].find(symbol_name) {
                let actual_pos = start_pos + pos;

                // Check if this is a whole word match (not part of another identifier)
                let is_word_start = actual_pos == 0 || !line.chars().nth(actual_pos - 1).unwrap_or(' ').is_alphanumeric();
                let end_pos = actual_pos + symbol_name.len();
                let is_word_end = end_pos >= line.len() || !line.chars().nth(end_pos).unwrap_or(' ').is_alphanumeric();

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

    fn find_symbol_at_position(&self, content: &str, position: Position) -> Option<String> {
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

        // Move start backwards to find word beginning
        while start > 0 && start <= chars.len() && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
            start -= 1;
        }

        // Move end forward to find word end
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }

        if start < end {
            Some(chars[start..end].iter().collect())
        } else {
            None
        }
    }

    fn find_ufc_method_at_position(&self, content: &str, position: Position) -> Option<UfcMethodInfo> {
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

    // Code lens helper functions moved to code_lens.rs

    fn resolve_ufc_method(&self, method_info: &UfcMethodInfo, store: &DocumentStore) -> Option<Location> {
        // Enhanced UFC method resolution with improved generic type handling
        let receiver_type = infer_receiver_type_from_store(&method_info.receiver, store);

        // Extract base type and generic parameters for better matching
        let (base_type, _generic_params) = if let Some(ref typ) = receiver_type {
            parse_generic_type(typ)
        } else {
            (String::new(), Vec::new())
        };

        // Handle built-in methods based on base type
        match base_type.as_str() {
            "Result" => {
                // Result methods with error propagation
                let result_methods = [
                    "raise", "is_ok", "is_err", "map", "map_err", "unwrap",
                    "unwrap_or", "expect", "unwrap_err", "and_then", "or_else"
                ];
                if result_methods.contains(&method_info.method_name.as_str()) {
                    return self.find_stdlib_location("core/result.zen", &method_info.method_name, store);
                }
            }
            "Option" => {
                // Option methods with monadic operations
                let option_methods = [
                    "is_some", "is_none", "unwrap", "unwrap_or", "map",
                    "or", "and", "expect", "and_then", "or_else", "filter"
                ];
                if option_methods.contains(&method_info.method_name.as_str()) {
                    return self.find_stdlib_location("core/option.zen", &method_info.method_name, store);
                }
            }
            "String" | "StaticString" | "str" => {
                // String methods - comprehensive list
                let string_methods = [
                    "len", "to_i32", "to_i64", "to_f64", "to_upper", "to_lower",
                    "trim", "split", "substr", "char_at", "contains", "starts_with",
                    "ends_with", "index_of", "replace", "concat", "repeat", "reverse",
                    "strip_prefix", "strip_suffix", "to_bytes", "from_bytes"
                ];
                if string_methods.contains(&method_info.method_name.as_str()) {
                    return self.find_stdlib_location("string.zen", &method_info.method_name, store);
                }
            }
            "HashMap" => {
                // HashMap methods - comprehensive list
                let hashmap_methods = [
                    "insert", "get", "remove", "contains_key", "keys", "values",
                    "len", "clear", "is_empty", "iter", "drain", "extend", "merge"
                ];
                if hashmap_methods.contains(&method_info.method_name.as_str()) {
                    return self.find_stdlib_location("collections/hashmap.zen", &method_info.method_name, store);
                }
            }
            "DynVec" => {
                // DynVec methods - comprehensive list
                let dynvec_methods = [
                    "push", "pop", "get", "set", "len", "clear", "capacity",
                    "insert", "remove", "is_empty", "resize", "extend", "drain",
                    "first", "last", "sort", "reverse", "contains"
                ];
                if dynvec_methods.contains(&method_info.method_name.as_str()) {
                    return self.find_stdlib_location("vec.zen", &method_info.method_name, store);
                }
            }
            "Vec" => {
                // Vec (fixed-size) methods
                let vec_methods = [
                    "push", "get", "set", "len", "clear", "capacity", "is_full", "is_empty"
                ];
                if vec_methods.contains(&method_info.method_name.as_str()) {
                    return self.find_stdlib_location("vec.zen", &method_info.method_name, store);
                }
            }
            "Array" => {
                // Array methods
                let array_methods = [
                    "len", "get", "set", "push", "pop", "first", "last",
                    "slice", "contains", "find", "sort", "reverse"
                ];
                if array_methods.contains(&method_info.method_name.as_str()) {
                    return self.find_stdlib_location("collections/array.zen", &method_info.method_name, store);
                }
            }
            "Allocator" => {
                // Allocator methods
                let allocator_methods = ["alloc", "dealloc", "realloc", "clone"];
                if allocator_methods.contains(&method_info.method_name.as_str()) {
                    return self.find_stdlib_location("memory_unified.zen", &method_info.method_name, store);
                }
            }
            _ => {}
        }

        // Check for generic iterator/collection methods
        if method_info.method_name == "loop" || method_info.method_name == "iter" {
            // loop/iter is available on all iterable types
            return self.find_stdlib_location("iterator.zen", &method_info.method_name, store);
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

    fn handle_references(&self, req: Request) -> Response {
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

        let store = self.store.lock().unwrap();
        let mut locations = Vec::new();

        if let Some(doc) = store.documents.get(&params.text_document_position.text_document.uri) {
            let position = params.text_document_position.position;

            // Find the symbol at the cursor position
            if let Some(symbol_name) = self.find_symbol_at_position(&doc.content, position) {
                eprintln!("[LSP] Find references for symbol: '{}'", symbol_name);

                // Determine the scope of the symbol
                let symbol_scope = determine_symbol_scope(&doc, &symbol_name, position);
                eprintln!("[LSP] Symbol scope: {:?}", symbol_scope);

                match symbol_scope {
                    SymbolScope::Local { ref function_name } => {
                        // Local variable - only find references within the function
                        let current_uri = params.text_document_position.text_document.uri.clone();
                        if let Some(refs) = self.find_local_references(
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
                        // Module-level symbol - search across all documents
                        for (uri, search_doc) in &store.documents {
                            let refs = self.find_references_in_document(
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

    fn handle_document_symbols(&self, req: Request) -> Response {
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

        if let Some(doc) = self.store.lock().unwrap().documents.get(&params.text_document.uri) {
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

    fn handle_formatting(&self, req: Request) -> Response {
        handle_formatting(req, Arc::clone(&self.store))
    }

    fn handle_rename(&self, req: Request) -> Response {
        handle_rename(req, &self.store)
    }

    fn handle_signature_help(&self, req: Request) -> Response {
        handle_signature_help(req, &self.store)
    }

    fn handle_inlay_hints(&self, req: Request) -> Response {
        handle_inlay_hints(req, &self.store)
    }

    fn handle_code_lens(&self, req: Request) -> Response {
        handle_code_lens(req, &self.store)
    }

    fn handle_code_action(&self, req: Request) -> Response {
        handle_code_action(req, &self.store)
    }

    // Code action helper functions moved to code_action.rs

    fn handle_semantic_tokens(&self, req: Request) -> Response {
        handle_semantic_tokens(req, &self.store)
    }

    // generate_semantic_tokens moved to semantic_tokens.rs

    // Type inference functions moved to type_inference.rs

    fn infer_variable_type(
        &self,
        content: &str,
        var_name: &str,
        local_symbols: &HashMap<String, SymbolInfo>,
        stdlib_symbols: &HashMap<String, SymbolInfo>,
        workspace_symbols: &HashMap<String, SymbolInfo>
    ) -> Option<String> {
        // Look for variable assignment: var_name = function_call() or var_name: Type = ...
        let lines: Vec<&str> = content.lines().collect();

        for line in lines {
            // Pattern: var_name = function_call()
            if line.contains(&format!("{} =", var_name)) || line.contains(&format!("{}=", var_name)) {
                // Check for type annotation first: var_name: Type = ...
                if let Some(colon_pos) = line.find(':') {
                    if let Some(eq_pos) = line.find('=') {
                        if colon_pos < eq_pos {
                            // Extract type between : and =
                            let type_str = line[colon_pos + 1..eq_pos].trim();
                            if !type_str.is_empty() {
                                return Some(format!("```zen\n{}: {}\n```\n\n**Type:** `{}`", var_name, type_str, type_str));
                            }
                        }
                    }
                }

                // Try to find function call and infer return type
                if let Some(eq_pos) = line.find('=') {
                    let rhs = &line[eq_pos + 1..].trim();

                    // Check if it's a function call
                    if let Some(paren_pos) = rhs.find('(') {
                        let func_name = rhs[..paren_pos].trim();

                        // Look up function in symbols
                        if let Some(func_info) = local_symbols.get(func_name)
                            .or_else(|| stdlib_symbols.get(func_name))
                            .or_else(|| workspace_symbols.get(func_name)) {

                            if let Some(type_info) = &func_info.type_info {
                                let type_str = format_type(type_info);
                                return Some(format!(
                                    "```zen\n{} = {}()\n```\n\n**Type:** `{}`\n\n**Inferred from:** `{}` function return type",
                                    var_name, func_name, type_str, func_name
                                ));
                            }

                            // Try to parse from detail string
                            if let Some(detail) = &func_info.detail {
                                // Parse "func_name = (args) return_type"
                                if let Some(arrow_or_paren_close) = detail.rfind(')') {
                                    let return_part = detail[arrow_or_paren_close + 1..].trim();
                                    if !return_part.is_empty() && return_part != "void" {
                                        return Some(format!(
                                            "```zen\n{} = {}()\n```\n\n**Type:** `{}`\n\n**Inferred from:** `{}` function return type",
                                            var_name, func_name, return_part, func_name
                                        ));
                                    }
                                }
                            }
                        }
                    }

                    // Check for constructor calls (Type { ... } or Type(...))
                    if let Some(brace_pos) = rhs.find('{') {
                        let type_name = rhs[..brace_pos].trim();
                        if !type_name.is_empty() && type_name.chars().next().unwrap().is_uppercase() {
                            return Some(format!(
                                "```zen\n{} = {} {{ ... }}\n```\n\n**Type:** `{}`\n\n**Inferred from:** constructor",
                                var_name, type_name, type_name
                            ));
                        }
                    }

                    // Check for literals
                    let trimmed = rhs.trim();
                    if trimmed.starts_with('"') || trimmed.starts_with('\'') {
                        return Some(format!(
                            "```zen\n{} = {}\n```\n\n**Type:** `StaticString`",
                            var_name, trimmed
                        ));
                    }
                    if trimmed.parse::<i32>().is_ok() {
                        return Some(format!(
                            "```zen\n{} = {}\n```\n\n**Type:** `i32`",
                            var_name, trimmed
                        ));
                    }
                    if trimmed.parse::<f64>().is_ok() && trimmed.contains('.') {
                        return Some(format!(
                            "```zen\n{} = {}\n```\n\n**Type:** `f64`",
                            var_name, trimmed
                        ));
                    }
                    if trimmed == "true" || trimmed == "false" {
                        return Some(format!(
                            "```zen\n{} = {}\n```\n\n**Type:** `bool`",
                            var_name, trimmed
                        ));
                    }
                }
            }
        }

        None
    }

    // Type inference functions moved to type_inference.rs

    fn find_stdlib_location(&self, stdlib_path: &str, method_name: &str, store: &DocumentStore) -> Option<Location> {
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

    fn get_completion_context(&self, content: &str, position: Position) -> Option<ZenCompletionContext> {
        let lines: Vec<&str> = content.lines().collect();
        if position.line as usize >= lines.len() {
            return Some(ZenCompletionContext::General);
        }

        let line = lines[position.line as usize];
        let char_pos = position.character as usize;

        // Check if we're after a dot
        if char_pos > 0 && line.chars().nth(char_pos - 1) == Some('.') {
            // Extract the receiver expression before the dot
            let chars: Vec<char> = line.chars().collect();
            let mut start = if char_pos > 1 { char_pos - 2 } else { 0 };
            let mut paren_depth = 0;

            // Find the start of the receiver expression
            while start > 0 {
                match chars[start] {
                    ')' => paren_depth += 1,
                    '(' => {
                        if paren_depth > 0 {
                            paren_depth -= 1;
                        } else {
                            break;
                        }
                    }
                    ' ' | '\t' | '=' | '{' | '[' | ',' | ';' if paren_depth == 0 => {
                        start += 1;
                        break;
                    }
                    _ => {}
                }
                start -= 1;
            }

            let receiver: String = chars[start..(char_pos - 1)].iter().collect();
            let receiver = receiver.trim();

            // Try to infer the receiver type
            let store = self.store.lock().unwrap();
            let receiver_type = infer_receiver_type_from_store(receiver, &store)
                .unwrap_or_else(|| "unknown".to_string());

            return Some(ZenCompletionContext::UfcMethod {
                receiver_type,
            });
        }

        Some(ZenCompletionContext::General)
    }

    fn get_ufc_method_completions(&self, _receiver_type: &str) -> Vec<CompletionItem> {
        // TODO: Query real method definitions from compiler/AST instead of hardcoded data
        Vec::new()
    }

    // Signature help helper functions moved to signature_help.rs

    // Inlay hints helper functions moved to inlay_hints.rs

    fn handle_workspace_symbol(&self, req: Request) -> Response {
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

        let store = self.store.lock().unwrap();
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

    fn handle_prepare_call_hierarchy(&self, req: Request) -> Response {
        handle_prepare_call_hierarchy(req, &self.store)
    }

    fn handle_incoming_calls(&self, req: Request) -> Response {
        handle_incoming_calls(req, &self.store)
    }

    fn handle_outgoing_calls(&self, req: Request) -> Response {
        handle_outgoing_calls(req, &self.store)
    }

    // Call hierarchy helper functions moved to call_hierarchy.rs
    // Code action helper functions moved to code_action.rs

    // Type inference parsing functions moved to type_inference.rs

    fn get_pattern_match_hover(
        &self,
        content: &str,
        position: Position,
        symbol_name: &str,
        _local_symbols: &HashMap<String, SymbolInfo>,
        _stdlib_symbols: &HashMap<String, SymbolInfo>,
        all_docs: &HashMap<Url, Document>
    ) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();
        if position.line as usize >= lines.len() {
            return None;
        }

        let current_line = lines[position.line as usize];

        // Check if we're in a pattern match arm (contains '|' and '{')
        if !current_line.contains('|') {
            return None;
        }

        // Find the scrutinee by looking backwards for 'variable ?'
        let mut scrutinee_name = None;
        let mut scrutinee_line = None;
        for i in (0..=position.line).rev() {
            let line = lines[i as usize].trim();
            if line.contains('?') && !line.starts_with("//") {
                // Found the pattern match - extract variable name
                if let Some(q_pos) = line.find('?') {
                    let before_q = line[..q_pos].trim();
                    // Get the last word before '?'
                    if let Some(var) = before_q.split_whitespace().last() {
                        scrutinee_name = Some(var.to_string());
                        scrutinee_line = Some(i);
                        break;
                    }
                }
            }
            // Don't search too far back
            if position.line - i > 10 {
                break;
            }
        }

        if let Some(scrutinee) = scrutinee_name {
            // Try to infer the type of the scrutinee by looking at its definition
            if let Some(scrutinee_line_num) = scrutinee_line {
                // Look backwards from scrutinee for its definition
                for i in (0..scrutinee_line_num).rev() {
                    let line = lines[i as usize];
                    // Check if this line defines the scrutinee
                    if line.contains(&format!("{} =", scrutinee)) {
                        // Try to infer type from the assignment
                        // Example: result = divide(10.0, 2.0)
                        if let Some(eq_pos) = line.find('=') {
                            let rhs = line[eq_pos+1..].trim();

                            // Check if it's a function call
                            if let Some(paren_pos) = rhs.find('(') {
                                let func_name = rhs[..paren_pos].trim();

                                // Try to find the function definition and extract its return type
                                let (concrete_ok_type, concrete_err_type) = infer_function_return_types(func_name, all_docs);

                                // Now determine what pattern variable we're hovering over
                                // Example: | Ok(val) or | Err(msg)
                                let pattern_arm = current_line.trim();

                                if pattern_arm.contains(&format!("Ok({}", symbol_name)) || pattern_arm.contains(&format!("Ok({})", symbol_name)) {
                                    // This is the Ok variant - extract the success type
                                    let type_display = concrete_ok_type.clone().unwrap_or_else(|| "T".to_string());
                                    let full_result_type = if let (Some(ok), Some(err)) = (&concrete_ok_type, &concrete_err_type) {
                                        format!("Result<{}, {}>", ok, err)
                                    } else {
                                        "Result<T, E>".to_string()
                                    };

                                    return Some(format!(
                                        "```zen\n{}: {}\n```\n\n**Pattern match variable**\n\nExtracted from `{}` (assigned from `{}()`)\n\nThis is the success value from the `Ok` variant.",
                                        symbol_name,
                                        type_display,
                                        full_result_type,
                                        func_name
                                    ));
                                } else if pattern_arm.contains(&format!("Err({}", symbol_name)) || pattern_arm.contains(&format!("Err({})", symbol_name)) {
                                    // This is the Err variant - extract the error type
                                    let type_display = concrete_err_type.clone().unwrap_or_else(|| "E".to_string());
                                    let full_result_type = if let (Some(ok), Some(err)) = (&concrete_ok_type, &concrete_err_type) {
                                        format!("Result<{}, {}>", ok, err)
                                    } else {
                                        "Result<T, E>".to_string()
                                    };

                                    return Some(format!(
                                        "```zen\n{}: {}\n```\n\n**Pattern match variable**\n\nExtracted from `{}` (assigned from `{}()`)\n\nThis is the error value from the `Err` variant.",
                                        symbol_name,
                                        type_display,
                                        full_result_type,
                                        func_name
                                    ));
                                } else if pattern_arm.contains(&format!("Some({}", symbol_name)) || pattern_arm.contains(&format!("Some({})", symbol_name)) {
                                    // This is Option.Some
                                    let inner_type = concrete_ok_type.clone().unwrap_or_else(|| "T".to_string());
                                    return Some(format!(
                                        "```zen\n{}: {}\n```\n\n**Pattern match variable**\n\nExtracted from `Option<{}>` (assigned from `{}()`)\n\nThis is the value from the `Some` variant.",
                                        symbol_name,
                                        inner_type,
                                        inner_type,
                                        func_name
                                    ));
                                }
                            }
                        }
                    }
                    // Don't search too far back
                    if scrutinee_line_num - i > 20 {
                        break;
                    }
                }
            }
        }

        None
    }

    fn get_enum_variant_hover(&self, content: &str, position: Position, symbol_name: &str) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();
        if position.line as usize >= lines.len() {
            return None;
        }

        let current_line = lines[position.line as usize];

        // Check if this line is an enum variant (has ':' after the symbol)
        if !current_line.contains(&format!("{}:", symbol_name)) && !current_line.contains(&format!("{},", symbol_name)) {
            return None;
        }

        // Find the enum name by looking backwards
        let mut enum_name = None;
        for i in (0..position.line).rev() {
            let line = lines[i as usize].trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            // Check if this line is an enum definition (identifier followed by ':' at end)
            if line.ends_with(':') && !line.contains("::") {
                let name = line.trim_end_matches(':').trim();
                if name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '<' || c == '>' || c == ',') {
                    enum_name = Some(name.to_string());
                    break;
                }
            }
            // If we hit something that's not a variant, stop
            if !line.contains(':') && !line.contains(',') {
                break;
            }
        }

        if let Some(enum_name) = enum_name {
            // Extract variant payload info from current line
            let payload_info = if current_line.contains('{') {
                // Struct-like payload
                let start = current_line.find('{')?;
                let end = current_line.rfind('}')?;
                let fields = &current_line[start+1..end];
                format!(" with fields: `{}`", fields.trim())
            } else if current_line.contains(": ") {
                // Type payload
                let parts: Vec<&str> = current_line.split(':').collect();
                if parts.len() >= 2 {
                    let type_part = parts[1].trim().trim_end_matches(',');
                    format!(" of type `{}`", type_part)
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            Some(format!(
                "```zen\n{}\n    {}...\n```\n\n**Enum variant** `{}`{}\n\nPart of enum `{}`",
                enum_name,
                symbol_name,
                symbol_name,
                payload_info,
                enum_name.split('<').next().unwrap_or(&enum_name)
            ))
        } else {
            None
        }
    }

    // Rename helper functions moved to rename.rs

    fn find_local_references(&self, content: &str, symbol_name: &str, function_name: &str) -> Option<Vec<Range>> {
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

                let before_ok = actual_col == 0 ||
                    !line.chars().nth(actual_col - 1).unwrap_or(' ').is_alphanumeric();
                let after_ok = actual_col + symbol_name.len() >= line.len() ||
                    !line.chars().nth(actual_col + symbol_name.len()).unwrap_or(' ').is_alphanumeric();

                if before_ok && after_ok && !self.is_in_string_or_comment(line, actual_col) {
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

    fn is_in_string_or_comment(&self, line: &str, col: usize) -> bool {
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

    fn find_references_in_document(&self, content: &str, symbol_name: &str) -> Vec<Range> {
        let mut references = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            let mut start_col = 0;

            while let Some(col) = line[start_col..].find(symbol_name) {
                let actual_col = start_col + col;

                let before_ok = actual_col == 0 ||
                    !line.chars().nth(actual_col - 1).unwrap_or(' ').is_alphanumeric();
                let after_ok = actual_col + symbol_name.len() >= line.len() ||
                    !line.chars().nth(actual_col + symbol_name.len()).unwrap_or(' ').is_alphanumeric();

                if before_ok && after_ok && !self.is_in_string_or_comment(line, actual_col) {
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

    fn find_zen_files_in_workspace(&self, root_path: &std::path::Path) -> Result<Vec<std::path::PathBuf>, std::io::Error> {
        let mut zen_files = Vec::new();
        self.collect_zen_files_recursive(root_path, &mut zen_files)?;
        Ok(zen_files)
    }

    fn collect_zen_files_recursive(&self, path: &std::path::Path, zen_files: &mut Vec<std::path::PathBuf>) -> Result<(), std::io::Error> {
        if !path.is_dir() {
            return Ok(());
        }

        // Skip common directories we don't want to search
        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
            if dir_name == "target" || dir_name == "node_modules" || dir_name == ".git"
                || dir_name.starts_with('.') {
                return Ok(());
            }
        }

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();

            if entry_path.is_dir() {
                self.collect_zen_files_recursive(&entry_path, zen_files)?;
            } else if let Some(ext) = entry_path.extension() {
                if ext == "zen" {
                    zen_files.push(entry_path);
                }
            }
        }

        Ok(())
    }
}