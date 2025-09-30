// Enhanced LSP Server for Zen Language
// Provides advanced IDE features with compiler integration

use lsp_server::{Connection, Message, Request, Response, Notification as ServerNotification};
use lsp_types::*;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};

use crate::ast::{Declaration, AstType, Expression, Statement};
use crate::lexer::{Lexer, Token};
use crate::parser::Parser;

// ============================================================================
// DOCUMENT STORE
// ============================================================================

#[derive(Debug, Clone)]
struct Document {
    uri: Url,
    version: i32,
    content: String,
    tokens: Vec<Token>,
    ast: Option<Vec<Declaration>>,
    diagnostics: Vec<Diagnostic>,
    symbols: HashMap<String, SymbolInfo>,
}

#[derive(Debug, Clone)]
struct SymbolInfo {
    name: String,
    kind: SymbolKind,
    range: Range,
    selection_range: Range,
    detail: Option<String>,
    documentation: Option<String>,
    type_info: Option<AstType>,
    definition_uri: Option<Url>,
    references: Vec<Range>,
}

struct DocumentStore {
    documents: HashMap<Url, Document>,
}

impl DocumentStore {
    fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    fn open(&mut self, uri: Url, version: i32, content: String) -> Vec<Diagnostic> {
        let diagnostics = self.analyze_document(&content);
        
        let doc = Document {
            uri: uri.clone(),
            version,
            content: content.clone(),
            tokens: self.tokenize(&content),
            ast: self.parse(&content),
            diagnostics: diagnostics.clone(),
            symbols: self.extract_symbols(&content),
        };
        
        self.documents.insert(uri, doc);
        diagnostics
    }

    fn update(&mut self, uri: Url, version: i32, content: String) -> Vec<Diagnostic> {
        let diagnostics = self.analyze_document(&content);
        
        // Calculate values before mutably borrowing
        let tokens = self.tokenize(&content);
        let ast = self.parse(&content);
        let symbols = self.extract_symbols(&content);
        
        if let Some(doc) = self.documents.get_mut(&uri) {
            doc.version = version;
            doc.content = content.clone();
            doc.tokens = tokens;
            doc.ast = ast;
            doc.diagnostics = diagnostics.clone();
            doc.symbols = symbols;
        }
        
        diagnostics
    }

    fn tokenize(&self, content: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(content);
        let mut tokens = Vec::new();
        
        // Collect all tokens
        loop {
            let token = lexer.next_token();
            if matches!(token, Token::Eof) {
                break;
            }
            tokens.push(token);
        }
        
        tokens
    }

    fn parse(&self, content: &str) -> Option<Vec<Declaration>> {
        let lexer = Lexer::new(content);
        let mut parser = Parser::new(lexer);
        
        match parser.parse_program() {
            Ok(program) => Some(program.declarations),
            Err(_) => None,
        }
    }

    fn analyze_document(&self, content: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        
        // Lexical analysis
        let lexer = Lexer::new(content);
        let mut parser = Parser::new(lexer);
        
        if let Err(err) = parser.parse_program() {
            diagnostics.push(self.error_to_diagnostic(err));
        }
        
        diagnostics
    }

    fn error_to_diagnostic(&self, error: crate::error::CompileError) -> Diagnostic {
        let (line, character) = match &error {
            crate::error::CompileError::ParseError(_, Some(span)) |
            crate::error::CompileError::SyntaxError(_, Some(span)) |
            crate::error::CompileError::TypeError(_, Some(span)) |
            crate::error::CompileError::TypeMismatch { span: Some(span), .. } => {
                (span.line as u32, span.column as u32)
            }
            _ => (0, 0),
        };

        Diagnostic {
            range: Range {
                start: Position { line, character },
                end: Position { line, character: character + 1 },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("zen".to_string()),
            message: format!("{}", error),
            related_information: None,
            tags: None,
            data: None,
        }
    }

    fn extract_symbols(&self, content: &str) -> HashMap<String, SymbolInfo> {
        let mut symbols = HashMap::new();

        if let Some(ast) = self.parse(content) {
            // First pass: Extract symbol definitions
            for (decl_index, decl) in ast.iter().enumerate() {
                let (line, _) = self.find_declaration_position(content, &decl, decl_index);
                let range = Range {
                    start: Position { line: line as u32, character: 0 },
                    end: Position { line: line as u32, character: 100 },
                };

                match decl {
                    Declaration::Function(func) => {
                        let detail = format!("{} = ({}) {}",
                            func.name,
                            func.args.iter()
                                .map(|(name, ty)| format!("{}: {}", name, format_type(ty)))
                                .collect::<Vec<_>>()
                                .join(", "),
                            format_type(&func.return_type)
                        );

                        symbols.insert(func.name.clone(), SymbolInfo {
                            name: func.name.clone(),
                            kind: SymbolKind::FUNCTION,
                            range: range.clone(),
                            selection_range: range,
                            detail: Some(detail),
                            documentation: None,
                            type_info: Some(func.return_type.clone()),
                            definition_uri: None,
                            references: Vec::new(),
                        });
                    }
                    Declaration::Struct(struct_def) => {
                        let detail = format!("{} struct with {} fields", struct_def.name, struct_def.fields.len());

                        symbols.insert(struct_def.name.clone(), SymbolInfo {
                            name: struct_def.name.clone(),
                            kind: SymbolKind::STRUCT,
                            range: range.clone(),
                            selection_range: range,
                            detail: Some(detail),
                            documentation: None,
                            type_info: None,
                            definition_uri: None,
                            references: Vec::new(),
                        });
                    }
                    Declaration::Enum(enum_def) => {
                        let detail = format!("{} enum with {} variants", enum_def.name, enum_def.variants.len());

                        symbols.insert(enum_def.name.clone(), SymbolInfo {
                            name: enum_def.name.clone(),
                            kind: SymbolKind::ENUM,
                            range: range.clone(),
                            selection_range: range,
                            detail: Some(detail),
                            documentation: None,
                            type_info: None,
                            definition_uri: None,
                            references: Vec::new(),
                        });

                        // Add enum variants as symbols
                        for variant in &enum_def.variants {
                            let variant_name = format!("{}::{}", enum_def.name, variant.name);
                            symbols.insert(variant_name.clone(), SymbolInfo {
                                name: variant.name.clone(),
                                kind: SymbolKind::ENUM_MEMBER,
                                range: range.clone(),
                                selection_range: range.clone(),
                                detail: Some(format!("{}::{}", enum_def.name, variant.name)),
                                documentation: None,
                                type_info: None,
                                definition_uri: None,
                                references: Vec::new(),
                            });
                        }
                    }
                    Declaration::Constant { name, type_, .. } => {
                        symbols.insert(name.clone(), SymbolInfo {
                            name: name.clone(),
                            kind: SymbolKind::CONSTANT,
                            range: range.clone(),
                            selection_range: range,
                            detail: type_.as_ref().map(|t| format_type(t)),
                            documentation: None,
                            type_info: type_.clone(),
                            definition_uri: None,
                            references: Vec::new(),
                        });
                    }
                    _ => {}
                }
            }

            // Second pass: Find references to symbols
            for decl in ast {
                if let Declaration::Function(func) = decl {
                    self.find_references_in_statements(&func.body, &mut symbols);
                }
            }
        }

        symbols
    }

    fn find_declaration_position(&self, content: &str, decl: &Declaration, _index: usize) -> (usize, usize) {
        // Find the line number where the declaration starts
        let search_str = match decl {
            Declaration::Function(f) => &f.name,
            Declaration::Struct(s) => &s.name,
            Declaration::Enum(e) => &e.name,
            Declaration::Constant { name, .. } => name,
            _ => return (0, 0),
        };

        let lines: Vec<&str> = content.lines().collect();
        for (line_num, line) in lines.iter().enumerate() {
            if line.contains(search_str) && line.contains('=') {
                return (line_num, line.find(search_str).unwrap_or(0));
            }
        }
        (0, 0)
    }

    fn find_references_in_statements(&self, statements: &[Statement], symbols: &mut HashMap<String, SymbolInfo>) {
        for stmt in statements {
            match stmt {
                Statement::Expression(expr) => self.find_references_in_expression(expr, symbols),
                Statement::Return(expr) => self.find_references_in_expression(expr, symbols),
                Statement::VariableDeclaration { initializer: Some(expr), .. } => {
                    self.find_references_in_expression(expr, symbols);
                }
                Statement::VariableAssignment { value, .. } => {
                    self.find_references_in_expression(value, symbols);
                }
                Statement::PointerAssignment { pointer, value } => {
                    self.find_references_in_expression(pointer, symbols);
                    self.find_references_in_expression(value, symbols);
                }
                _ => {}
            }
        }
    }

    fn find_references_in_expression(&self, _expr: &Expression, _symbols: &mut HashMap<String, SymbolInfo>) {
        // TODO: Traverse expression tree to find identifier references
        // This would need to walk through all expressions and track uses of identifiers
    }
}

fn format_symbol_kind(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::FILE => "File",
        SymbolKind::MODULE => "Module",
        SymbolKind::NAMESPACE => "Namespace",
        SymbolKind::PACKAGE => "Package",
        SymbolKind::CLASS => "Class",
        SymbolKind::METHOD => "Method",
        SymbolKind::PROPERTY => "Property",
        SymbolKind::FIELD => "Field",
        SymbolKind::CONSTRUCTOR => "Constructor",
        SymbolKind::ENUM => "Enum",
        SymbolKind::INTERFACE => "Interface",
        SymbolKind::FUNCTION => "Function",
        SymbolKind::VARIABLE => "Variable",
        SymbolKind::CONSTANT => "Constant",
        SymbolKind::STRING => "String",
        SymbolKind::NUMBER => "Number",
        SymbolKind::BOOLEAN => "Boolean",
        SymbolKind::ARRAY => "Array",
        SymbolKind::OBJECT => "Object",
        SymbolKind::KEY => "Key",
        SymbolKind::NULL => "Null",
        SymbolKind::ENUM_MEMBER => "Enum Member",
        SymbolKind::STRUCT => "Struct",
        SymbolKind::EVENT => "Event",
        SymbolKind::OPERATOR => "Operator",
        SymbolKind::TYPE_PARAMETER => "Type Parameter",
        _ => "Unknown",
    }
}

fn format_type(ast_type: &AstType) -> String {
    match ast_type {
        AstType::I8 => "i8".to_string(),
        AstType::I16 => "i16".to_string(),
        AstType::I32 => "i32".to_string(),
        AstType::I64 => "i64".to_string(),
        AstType::U8 => "u8".to_string(),
        AstType::U16 => "u16".to_string(),
        AstType::U32 => "u32".to_string(),
        AstType::U64 => "u64".to_string(),
        AstType::Usize => "usize".to_string(),
        AstType::F32 => "f32".to_string(),
        AstType::F64 => "f64".to_string(),
        AstType::Bool => "bool".to_string(),
        AstType::String => "String".to_string(),
        AstType::Void => "void".to_string(),
        AstType::Ptr(inner) => format!("Ptr<{}>", format_type(inner)),
        AstType::MutPtr(inner) => format!("MutPtr<{}>", format_type(inner)),
        AstType::RawPtr(inner) => format!("RawPtr<{}>", format_type(inner)),
        AstType::Array(elem) => format!("Array<{}>", format_type(elem)),
        AstType::Vec { element_type, size } => format!("Vec<{}, {}>", format_type(element_type), size),
        AstType::DynVec { element_types, .. } => {
            if element_types.len() == 1 {
                format!("DynVec<{}>", format_type(&element_types[0]))
            } else {
                "DynVec<...>".to_string()
            }
        },
        AstType::FixedArray { element_type, size } => format!("[{}; {}]", format_type(element_type), size),
        AstType::Option(inner) => format!("Option<{}>", format_type(inner)),
        AstType::Result { ok_type, err_type } => format!("Result<{}, {}>", format_type(ok_type), format_type(err_type)),
        AstType::Struct { name, .. } => name.clone(),
        AstType::Enum { name, .. } => name.clone(),
        AstType::Generic { name, type_args } => {
            if type_args.is_empty() {
                name.clone()
            } else {
                format!("{}<{}>", name, 
                    type_args.iter().map(|p| format_type(p)).collect::<Vec<_>>().join(", "))
            }
        }
        AstType::Function { args, return_type } => {
            format!("({}) {}", 
                args.iter().map(|p| format_type(p)).collect::<Vec<_>>().join(", "),
                format_type(return_type))
        }
        _ => "unknown".to_string(),  // Fallback for any unhandled types
    }
}

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
        eprintln!("Starting Enhanced Zen Language Server...");
        
        let server_capabilities = serde_json::to_value(&self.capabilities)?;
        let _initialization_params = self.connection.initialize(server_capabilities)?;
        
        eprintln!("Zen LSP initialized with enhanced capabilities");
        self.main_loop()?;
        
        eprintln!("Zen Language Server shutting down");
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
        let response = match req.method.as_str() {
            "textDocument/hover" => self.handle_hover(req.clone()),
            "textDocument/completion" => self.handle_completion(req.clone()),
            "textDocument/definition" => self.handle_definition(req.clone()),
            "textDocument/references" => self.handle_references(req.clone()),
            "textDocument/documentSymbol" => self.handle_document_symbols(req.clone()),
            "textDocument/formatting" => self.handle_formatting(req.clone()),
            "textDocument/rename" => self.handle_rename(req.clone()),
            "textDocument/codeAction" => self.handle_code_action(req.clone()),
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
                eprintln!("Client initialized");
            }
            _ => {}
        }
        Ok(())
    }

    fn publish_diagnostics(&self, uri: Url, diagnostics: Vec<Diagnostic>) -> Result<(), Box<dyn Error>> {
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

        let store = self.store.lock().unwrap();
        if let Some(doc) = store.documents.get(&params.text_document_position_params.text_document.uri) {
            let position = params.text_document_position_params.position;

            // Find the symbol at the cursor position
            if let Some(symbol_name) = self.find_symbol_at_position(&doc.content, position) {
                // Check for symbol info in current document
                if let Some(symbol_info) = doc.symbols.get(&symbol_name) {
                    let mut hover_content = Vec::new();

                    // Add type signature
                    if let Some(detail) = &symbol_info.detail {
                        hover_content.push(format!("```zen\n{}\n```", detail));
                    }

                    // Add documentation
                    if let Some(doc) = &symbol_info.documentation {
                        hover_content.push(doc.clone());
                    }

                    // Add type information
                    if let Some(type_info) = &symbol_info.type_info {
                        hover_content.push(format!("**Type:** `{}`", format_type(type_info)));
                    }

                    // Add symbol kind
                    hover_content.push(format!("**Kind:** {}", format_symbol_kind(symbol_info.kind)));

                    let contents = HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: hover_content.join("\n\n"),
                    });

                    return Response {
                        id: req.id,
                        result: Some(serde_json::to_value(Hover {
                            contents,
                            range: Some(Range {
                                start: Position {
                                    line: position.line,
                                    character: position.character.saturating_sub(symbol_name.len() as u32),
                                },
                                end: Position {
                                    line: position.line,
                                    character: position.character + symbol_name.len() as u32,
                                },
                            }),
                        }).unwrap_or(Value::Null)),
                        error: None,
                    };
                }

                // Check stdlib or other documents
                for (_uri, other_doc) in &store.documents {
                    if let Some(symbol_info) = other_doc.symbols.get(&symbol_name) {
                        let mut hover_content = Vec::new();

                        if let Some(detail) = &symbol_info.detail {
                            hover_content.push(format!("```zen\n{}\n```", detail));
                        }

                        if let Some(type_info) = &symbol_info.type_info {
                            hover_content.push(format!("**Type:** `{}`", format_type(type_info)));
                        }

                        let contents = HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: hover_content.join("\n\n"),
                        });

                        return Response {
                            id: req.id,
                            result: Some(serde_json::to_value(Hover {
                                contents,
                                range: None,
                            }).unwrap_or(Value::Null)),
                            error: None,
                        };
                    }
                }

                // Provide hover for built-in types and keywords
                let hover_text = match symbol_name.as_str() {
                    "Option" => "```zen\nenum Option<T> {\n    Some(T),\n    None,\n}\n```\n\nOptional value type",
                    "Result" => "```zen\nenum Result<T, E> {\n    Ok(T),\n    Err(E),\n}\n```\n\nResult type for error handling",
                    "HashMap" => "```zen\nHashMap<K, V>\n```\n\nHash map collection (requires allocator)",
                    "DynVec" => "```zen\nDynVec<T>\n```\n\nDynamic vector (requires allocator)",
                    "Array" => "```zen\nArray<T>\n```\n\nDynamic array (requires allocator)",
                    "String" => "```zen\nString\n```\n\nDynamic string type (requires allocator)",
                    "StaticString" => "```zen\nStaticString\n```\n\nStatic string type (compile-time, no allocator)",
                    "loop" => "```zen\nloop() { ... }\nloop((handle) { ... })\n(range).loop((i) { ... })\n```\n\nLoop construct with internal state management",
                    "raise" => "```zen\nexpr.raise()\n```\n\nPropagate errors from Result types",
                    _ => "Zen language element",
                };

                let contents = HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: hover_text.to_string(),
                });

                return Response {
                    id: req.id,
                    result: Some(serde_json::to_value(Hover {
                        contents,
                        range: None,
                    }).unwrap_or(Value::Null)),
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

    fn handle_completion(&self, req: Request) -> Response {
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
        
        // Provide enhanced completions based on context
        let mut completions = vec![
            // Keywords
            CompletionItem {
                label: "main".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("main = () i32 { ... }".to_string()),
                documentation: Some(Documentation::String("Entry point function".to_string())),
                ..Default::default()
            },
            CompletionItem {
                label: "loop".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("loop() { ... }".to_string()),
                documentation: Some(Documentation::String("Infinite loop with break statement".to_string())),
                ..Default::default()
            },
            CompletionItem {
                label: "return".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("return value".to_string()),
                documentation: Some(Documentation::String("Return from function".to_string())),
                ..Default::default()
            },
            CompletionItem {
                label: "break".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("break".to_string()),
                documentation: Some(Documentation::String("Break from loop".to_string())),
                ..Default::default()
            },
            CompletionItem {
                label: "continue".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("continue".to_string()),
                documentation: Some(Documentation::String("Continue to next iteration".to_string())),
                ..Default::default()
            },
            // Common types
            CompletionItem {
                label: "Option".to_string(),
                kind: Some(CompletionItemKind::ENUM),
                detail: Some("Option<T>".to_string()),
                documentation: Some(Documentation::String("Optional value type".to_string())),
                ..Default::default()
            },
            CompletionItem {
                label: "Result".to_string(),
                kind: Some(CompletionItemKind::ENUM),
                detail: Some("Result<T, E>".to_string()),
                documentation: Some(Documentation::String("Result type for error handling".to_string())),
                ..Default::default()
            },
            CompletionItem {
                label: "Some".to_string(),
                kind: Some(CompletionItemKind::ENUM_MEMBER),
                detail: Some("Some(value)".to_string()),
                documentation: Some(Documentation::String("Option variant with value".to_string())),
                ..Default::default()
            },
            CompletionItem {
                label: "None".to_string(),
                kind: Some(CompletionItemKind::ENUM_MEMBER),
                detail: Some("None".to_string()),
                documentation: Some(Documentation::String("Option variant without value".to_string())),
                ..Default::default()
            },
            CompletionItem {
                label: "Ok".to_string(),
                kind: Some(CompletionItemKind::ENUM_MEMBER),
                detail: Some("Ok(value)".to_string()),
                documentation: Some(Documentation::String("Success variant of Result".to_string())),
                ..Default::default()
            },
            CompletionItem {
                label: "Err".to_string(),
                kind: Some(CompletionItemKind::ENUM_MEMBER),
                detail: Some("Err(error)".to_string()),
                documentation: Some(Documentation::String("Error variant of Result".to_string())),
                ..Default::default()
            },
            // Collections
            CompletionItem {
                label: "Vec".to_string(),
                kind: Some(CompletionItemKind::STRUCT),
                detail: Some("Vec<T, size>".to_string()),
                documentation: Some(Documentation::String("Fixed-size vector (stack allocated)".to_string())),
                ..Default::default()
            },
            CompletionItem {
                label: "DynVec".to_string(),
                kind: Some(CompletionItemKind::STRUCT),
                detail: Some("DynVec<T>".to_string()),
                documentation: Some(Documentation::String("Dynamic vector (requires allocator)".to_string())),
                ..Default::default()
            },
            CompletionItem {
                label: "HashMap".to_string(),
                kind: Some(CompletionItemKind::STRUCT),
                detail: Some("HashMap<K, V>".to_string()),
                documentation: Some(Documentation::String("Hash map (requires allocator)".to_string())),
                ..Default::default()
            },
            // Module imports
            CompletionItem {
                label: "@std".to_string(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("Standard library".to_string()),
                documentation: Some(Documentation::String("Import standard library modules".to_string())),
                ..Default::default()
            },
            CompletionItem {
                label: "@this".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Current module reference".to_string()),
                documentation: Some(Documentation::String("Reference to current module".to_string())),
                ..Default::default()
            },
        ];
        
        // Add primitive types
        for ty in ["i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64", "bool"] {
            completions.push(CompletionItem {
                label: ty.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some(format!("{} type", ty)),
                documentation: None,
                ..Default::default()
            });
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

                // TODO: Check stdlib and imported symbols
                // For now, we'll search for the symbol in all open documents
                for (uri, other_doc) in &store.documents {
                    if let Some(symbol_info) = other_doc.symbols.get(&symbol_name) {
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
        }

        Response {
            id: req.id,
            result: Some(Value::Null),
            error: None,
        }
    }

    fn find_symbol_at_position(&self, content: &str, position: Position) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();
        if position.line as usize >= lines.len() {
            return None;
        }

        let line = lines[position.line as usize];
        let char_pos = position.character as usize;

        // Find word boundaries around the cursor position
        let mut start = char_pos;
        let mut end = char_pos;

        let chars: Vec<char> = line.chars().collect();

        // Move start backwards to find word beginning
        while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
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
                // Search for references in all open documents
                for (uri, doc) in &store.documents {
                    // Find all occurrences of the symbol in the document
                    let lines: Vec<&str> = doc.content.lines().collect();
                    for (line_num, line) in lines.iter().enumerate() {
                        // Simple text search - could be improved with proper AST traversal
                        if let Some(col) = line.find(&symbol_name) {
                            // Verify it's a whole word match
                            let before_ok = col == 0 || !line.chars().nth(col - 1).unwrap_or(' ').is_alphanumeric();
                            let after_ok = col + symbol_name.len() >= line.len() ||
                                !line.chars().nth(col + symbol_name.len()).unwrap_or(' ').is_alphanumeric();

                            if before_ok && after_ok {
                                locations.push(Location {
                                    uri: uri.clone(),
                                    range: Range {
                                        start: Position {
                                            line: line_num as u32,
                                            character: col as u32,
                                        },
                                        end: Position {
                                            line: line_num as u32,
                                            character: (col + symbol_name.len()) as u32,
                                        },
                                    },
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
        // TODO: Implement formatting
        Response {
            id: req.id,
            result: Some(Value::Null),
            error: None,
        }
    }

    fn handle_rename(&self, req: Request) -> Response {
        // TODO: Implement rename
        Response {
            id: req.id,
            result: Some(Value::Null),
            error: None,
        }
    }

    fn handle_code_action(&self, req: Request) -> Response {
        // TODO: Implement code actions
        Response {
            id: req.id,
            result: Some(Value::Null),
            error: None,
        }
    }
}