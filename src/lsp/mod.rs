use std::collections::HashMap;
use std::sync::Arc;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tokio::sync::RwLock;

use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::ast::{Program, Declaration};

#[derive(Debug)]
pub struct ZenServer {
    client: Client,
    documents: Arc<RwLock<HashMap<String, String>>>,
}

impl ZenServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn parse_document(&self, uri: &str) -> Result<Program> {
        let documents = self.documents.read().await;
        let content = documents.get(uri)
            .ok_or_else(|| tower_lsp::jsonrpc::Error::new(tower_lsp::jsonrpc::ErrorCode::InvalidParams))?;
        
        let lexer = Lexer::new(content);
        let mut parser = Parser::new(lexer);
        
        parser.parse_program()
            .map_err(|_e| tower_lsp::jsonrpc::Error::new(tower_lsp::jsonrpc::ErrorCode::ParseError))
    }

    async fn get_diagnostics(&self, uri: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        
        match self.parse_document(uri).await {
            Ok(program) => {
                // Check for import validation
                diagnostics.extend(self.check_import_placement(&program));
            }
            Err(_) => {
                // Add a generic error diagnostic
                diagnostics.push(Diagnostic {
                    range: Range::new(Position::new(0, 0), Position::new(0, 0)),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some("zen".to_string()),
                    message: "Failed to parse document".to_string(),
                    related_information: None,
                    tags: None,
                    data: None,
                });
            }
        }
        
        diagnostics
    }
    
    fn check_import_placement(&self, program: &Program) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        
        // Check declarations for improper import placement
        for declaration in &program.declarations {
            use crate::ast::Declaration;
            
            match declaration {
                Declaration::Function(func) => {
                    // Check function body for imports
                    for stmt in &func.body {
                        if let Some(diag) = self.check_statement_for_imports(stmt, true) {
                            diagnostics.push(diag);
                        }
                    }
                }
                Declaration::ComptimeBlock(stmts) => {
                    // Check comptime block for imports - these are NOT allowed
                    for stmt in stmts {
                        if let Some(diag) = self.check_statement_for_imports(stmt, true) {
                            diagnostics.push(diag);
                        }
                    }
                }
                _ => {}
            }
        }
        
        diagnostics
    }
    
    fn check_statement_for_imports(&self, statement: &crate::ast::Statement, in_nested_context: bool) -> Option<Diagnostic> {
        use crate::ast::Statement;
        
        match statement {
            Statement::ModuleImport { .. } => {
                if in_nested_context {
                    return Some(Diagnostic {
                        range: Range::new(
                            Position::new(0, 0),
                            Position::new(0, 10),
                        ),
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: None,
                        code_description: None,
                        source: Some("zen".to_string()),
                        message: "Import statements must be at module level, not inside functions or blocks".to_string(),
                        related_information: None,
                        tags: None,
                        data: None,
                    });
                }
            }
            Statement::VariableDeclaration { initializer, .. } => {
                if let Some(init) = initializer {
                    if self.is_import_expression(init) && in_nested_context {
                        return Some(Diagnostic {
                            range: Range::new(
                                Position::new(0, 0),
                                Position::new(0, 10),
                            ),
                            severity: Some(DiagnosticSeverity::ERROR),
                            code: None,
                            code_description: None,
                            source: Some("zen".to_string()),
                            message: "Import statements must be at module level, not inside functions or blocks".to_string(),
                            related_information: None,
                            tags: None,
                            data: None,
                        });
                    }
                }
            }
            Statement::VariableAssignment { value, .. } => {
                if self.is_import_expression(value) && in_nested_context {
                    return Some(Diagnostic {
                        range: Range::new(
                            Position::new(0, 0),
                            Position::new(0, 10),
                        ),
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: None,
                        code_description: None,
                        source: Some("zen".to_string()),
                        message: "Import statements must be at module level, not inside functions or blocks".to_string(),
                        related_information: None,
                        tags: None,
                        data: None,
                    });
                }
            }
            Statement::Loop { body, .. } => {
                // Check loop body for imports
                for stmt in body {
                    if let Some(diag) = self.check_statement_for_imports(stmt, true) {
                        return Some(diag);
                    }
                }
            }
            Statement::ComptimeBlock(block) => {
                // Check comptime block for imports - these are NOT allowed
                for stmt in block {
                    match stmt {
                        Statement::ModuleImport { .. } => {
                            return Some(Diagnostic {
                                range: Range::new(
                                    Position::new(0, 0),
                                    Position::new(0, 10),
                                ),
                                severity: Some(DiagnosticSeverity::ERROR),
                                code: None,
                                code_description: None,
                                source: Some("zen".to_string()),
                                message: "Imports are not allowed inside comptime blocks. Use comptime for meta-programming only.".to_string(),
                                related_information: None,
                                tags: None,
                                data: None,
                            });
                        }
                        Statement::VariableDeclaration { initializer, .. } => {
                            if let Some(init) = initializer {
                                if self.is_import_expression(init) {
                                    return Some(Diagnostic {
                                        range: Range::new(
                                            Position::new(0, 0),
                                            Position::new(0, 10),
                                        ),
                                        severity: Some(DiagnosticSeverity::ERROR),
                                        code: None,
                                        code_description: None,
                                        source: Some("zen".to_string()),
                                        message: "Imports are not allowed inside comptime blocks. Use comptime for meta-programming only.".to_string(),
                                        related_information: None,
                                        tags: None,
                                        data: None,
                                    });
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        
        None
    }
    
    fn is_import_expression(&self, expr: &crate::ast::Expression) -> bool {
        match expr {
            crate::ast::Expression::Identifier(id) if id.starts_with("@std") => true,
            crate::ast::Expression::MemberAccess { object, .. } => {
                if let crate::ast::Expression::Identifier(id) = &**object {
                    id.starts_with("@std") || id == "build"
                } else {
                    self.is_import_expression(object)
                }
            }
            crate::ast::Expression::FunctionCall { name, .. } if name.contains("import") => true,
            _ => false
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for ZenServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "zen-lsp".to_string(),
                version: Some("0.1.0".to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![
                        ".".to_string(),
                        ":".to_string(),
                        "=".to_string(),
                        "(".to_string(),
                    ]),
                    all_commit_characters: None,
                    completion_item: None,
                    work_done_progress_options: Default::default(),
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
                    identifier: Some("zen".to_string()),
                    inter_file_dependencies: true,
                    workspace_diagnostics: true,
                    work_done_progress_options: Default::default(),
                })),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Zen Language Server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let text = params.text_document.text;
        
        {
            let mut documents = self.documents.write().await;
            documents.insert(uri.clone(), text);
        }
        
        let diagnostics = self.get_diagnostics(&uri).await;
        self.client
            .publish_diagnostics(uri.parse().unwrap(), diagnostics, None)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        
        // Update document content
        for change in params.content_changes {
            let text = change.text;
            let mut documents = self.documents.write().await;
            documents.insert(uri.clone(), text);
        }
        
        // Publish diagnostics
        let diagnostics = self.get_diagnostics(&uri).await;
        self.client
            .publish_diagnostics(uri.parse().unwrap(), diagnostics, None)
            .await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let mut documents = self.documents.write().await;
        documents.remove(&uri);
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri.to_string();
        let _position = params.text_document_position.position;
        
        // Get document content
        let documents = self.documents.read().await;
        let _content = documents.get(&uri)
            .ok_or_else(|| tower_lsp::jsonrpc::Error::new(tower_lsp::jsonrpc::ErrorCode::InvalidParams))?;
        
        // Simple completion based on position
        let mut completions = Vec::new();
        
        // Add basic keywords
        completions.push(CompletionItem {
            label: "loop".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Loop construct".to_string()),
            ..Default::default()
        });
        
        completions.push(CompletionItem {
            label: "break".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Break from loop".to_string()),
            ..Default::default()
        });
        
        completions.push(CompletionItem {
            label: "continue".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Continue loop iteration".to_string()),
            ..Default::default()
        });
        
        completions.push(CompletionItem {
            label: "struct".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Define a struct".to_string()),
            ..Default::default()
        });
        
        completions.push(CompletionItem {
            label: "enum".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Define an enum".to_string()),
            ..Default::default()
        });
        
        completions.push(CompletionItem {
            label: "comptime".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Compile-time evaluation".to_string()),
            ..Default::default()
        });
        
        Ok(Some(CompletionResponse::Array(completions)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri.to_string();
        let position = params.text_document_position_params.position;
        
        // Get document content
        let documents = self.documents.read().await;
        let content = documents.get(&uri)
            .ok_or_else(|| tower_lsp::jsonrpc::Error::new(tower_lsp::jsonrpc::ErrorCode::InvalidParams))?;
        
        // Parse to find what's at the position
        let lines: Vec<&str> = content.lines().collect();
        if position.line as usize >= lines.len() {
            return Ok(None);
        }
        
        let line = lines[position.line as usize];
        let char_pos = position.character as usize;
        
        // Find the identifier at the position
        let mut start = char_pos;
        let mut end = char_pos;
        let chars: Vec<char> = line.chars().collect();
        
        // Find start of identifier
        while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_' || chars[start - 1] == '@') {
            start -= 1;
        }
        
        // Find end of identifier
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_' || chars[end] == '.') {
            end += 1;
        }
        
        if start >= end {
            return Ok(None);
        }
        
        let identifier: String = chars[start..end].iter().collect();
        
        // Provide hover information based on identifier
        let hover_content = match identifier.as_str() {
            s if s.starts_with("@std") => {
                format!("**Standard Library Module**\n\n`{}`\n\nBuilt-in Zen standard library module", identifier)
            }
            "comptime" => {
                "**comptime**\n\nCompile-time evaluation block.\n\n⚠️ **Note**: Imports are NOT allowed inside comptime blocks".to_string()
            }
            "loop" => {
                "**loop**\n\nInfinite loop construct. Use `break` to exit.".to_string()
            }
            "struct" => {
                "**struct**\n\nDefines a structured data type with fields.".to_string()
            }
            "enum" => {
                "**enum**\n\nDefines an enumeration with variants.".to_string()
            }
            "behavior" => {
                "**behavior**\n\nDefines a trait or interface for types.".to_string()
            }
            _ => {
                format!("**{}**\n\nIdentifier at position {}:{}", identifier, position.line, position.character)
            }
        };
        
        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: hover_content,
            }),
            range: Some(Range::new(
                Position::new(position.line, start as u32),
                Position::new(position.line, end as u32),
            )),
        }))
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri.to_string();
        let position = params.text_document_position_params.position;
        
        // Parse the document to find definitions
        let program = match self.parse_document(&uri).await {
            Ok(prog) => prog,
            Err(_) => return Ok(None),
        };
        
        // Get document content to find identifier at position
        let documents = self.documents.read().await;
        let content = documents.get(&uri)
            .ok_or_else(|| tower_lsp::jsonrpc::Error::new(tower_lsp::jsonrpc::ErrorCode::InvalidParams))?;
        
        let lines: Vec<&str> = content.lines().collect();
        if position.line as usize >= lines.len() {
            return Ok(None);
        }
        
        let line = lines[position.line as usize];
        let char_pos = position.character as usize;
        
        // Find the identifier at the position
        let mut start = char_pos;
        let mut end = char_pos;
        let chars: Vec<char> = line.chars().collect();
        
        while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
            start -= 1;
        }
        
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }
        
        if start >= end {
            return Ok(None);
        }
        
        let identifier: String = chars[start..end].iter().collect();
        
        // Search for the definition in the AST
        for (idx, decl) in program.declarations.iter().enumerate() {
            match decl {
                Declaration::Function(func) if func.name == identifier => {
                    // Found function definition
                    return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                        uri: params.text_document_position_params.text_document.uri.clone(),
                        range: Range::new(
                            Position::new(idx as u32, 0),
                            Position::new(idx as u32, 10),
                        ),
                    })));
                }
                Declaration::Struct(s) if s.name == identifier => {
                    // Found struct definition
                    return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                        uri: params.text_document_position_params.text_document.uri.clone(),
                        range: Range::new(
                            Position::new(idx as u32, 0),
                            Position::new(idx as u32, 10),
                        ),
                    })));
                }
                _ => {}
            }
        }
        
        Ok(None)
    }
}

pub async fn run_lsp_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| ZenServer::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
} 