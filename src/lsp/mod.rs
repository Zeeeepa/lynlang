use std::collections::HashMap;
use std::sync::Arc;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tokio::sync::RwLock;

use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::ast::{Program, Declaration};

mod enhanced;

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
            .ok_or_else(|| {
                let mut err = tower_lsp::jsonrpc::Error::new(tower_lsp::jsonrpc::ErrorCode::InvalidParams);
                err.message = format!("Document not found: {}", uri).into();
                err
            })?;
        
        let lexer = Lexer::new(content);
        let mut parser = Parser::new(lexer);
        
        parser.parse_program()
            .map_err(|e| {
                // Create a JSON-RPC error with detailed context
                let lines: Vec<&str> = content.lines().collect();
                let mut err = tower_lsp::jsonrpc::Error::new(tower_lsp::jsonrpc::ErrorCode::ParseError);
                
                // Use the enhanced detailed_message method for better error reporting
                let error_msg = e.detailed_message(&lines);
                
                // Analyze the error context for better suggestions
                let context_msg = if let Some(span) = e.position() {
                    if span.line > 0 && span.line <= lines.len() {
                        let line = lines[span.line - 1];
                        let trimmed = line.trim();
                        
                        // Provide context-specific help based on what's in the line
                        let mut help = String::new();
                        
                        // Check for forbidden keywords with more detailed help
                        if trimmed.starts_with("if ") || trimmed.contains(" if ") {
                            help.push_str("\n\n🚫 Language Spec Violation: 'if' keyword\n");
                            help.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
                            help.push_str("Zen uses pattern matching with '?' operator:\n\n");
                            help.push_str("  • Simple bool: condition ? { action }\n");
                            help.push_str("  • With else: condition ? | true => action1 | false => action2\n");
                            help.push_str("  • Multiple: value ? | 0 => \"zero\" | 1 => \"one\" | _ => \"other\"\n");
                        } else if trimmed.starts_with("else") {
                            help.push_str("\n\n🚫 Language Spec Violation: 'else' keyword\n");
                            help.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
                            help.push_str("All branches handled in pattern matching:\n\n");
                            help.push_str("  value ? | pattern1 => result1\n");
                            help.push_str("          | pattern2 => result2\n");
                            help.push_str("          | _ => default_result\n");
                        } else if trimmed.starts_with("match ") {
                            help.push_str("\n\n🚫 Language Spec Violation: 'match' keyword\n");
                            help.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
                            help.push_str("Use '?' operator for pattern matching:\n\n");
                            help.push_str("  value ? | .Ok -> x => process(x)\n");
                            help.push_str("          | .Err -> e => handle_error(e)\n");
                        } else if trimmed.starts_with("fn ") || trimmed.starts_with("func ") || trimmed.starts_with("function ") {
                            help.push_str("\n\n🚫 Language Spec Violation: function keywords\n");
                            help.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
                            help.push_str("Zen function syntax:\n\n");
                            help.push_str("  name = (params) ReturnType { body }\n\n");
                            help.push_str("Examples:\n");
                            help.push_str("  add = (a: i32, b: i32) i32 { a + b }\n");
                            help.push_str("  greet = () void { print(\"Hello\") }\n");
                            help.push_str("  identity<T> = (val: T) T { val }\n");
                        } else if trimmed.starts_with("let ") || trimmed.starts_with("var ") || trimmed.starts_with("const ") {
                            help.push_str("\n\n🚫 Language Spec Violation: variable keywords\n");
                            help.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
                            help.push_str("Zen variable declarations:\n\n");
                            help.push_str("  • Immutable: name := value\n");
                            help.push_str("  • Mutable:   name ::= value\n");
                            help.push_str("  • Typed imm: name: Type = value\n");
                            help.push_str("  • Typed mut: name:: Type = value\n");
                        } else if trimmed.contains("while ") {
                            help.push_str("\n\n🚫 Language Spec Violation: 'while' keyword\n");
                            help.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
                            help.push_str("Zen loop syntax:\n\n");
                            help.push_str("  • Conditional: loop (condition) { body }\n");
                            help.push_str("  • Infinite:    loop { body }\n");
                            help.push_str("  • With break:  loop { condition ? | true => break }\n");
                        } else if trimmed.contains("for ") {
                            help.push_str("\n\n🚫 Language Spec Violation: 'for' keyword\n");
                            help.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
                            help.push_str("Zen iteration:\n\n");
                            help.push_str("  • Range:   (0..10).loop((i) => { body })\n");
                            help.push_str("  • Array:   items.loop((item) => { body })\n");
                            help.push_str("  • Indexed: items.enumerate().loop((i, val) => { body })\n");
                        }
                        help
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
                
                // Also add specific handling for common error patterns
                let enhanced_msg = match &e {
                    crate::error::CompileError::InvalidSyntax { message, suggestion, .. } => {
                        format!("❌ Parse Error: {}\n{}\n\n✅ Fix: {}\n\n📚 See LANGUAGE_SPEC.md for complete syntax rules", 
                                message, error_msg, suggestion)
                    },
                    crate::error::CompileError::UnexpectedToken { expected, found, .. } => {
                        let expected_list = if expected.len() > 3 {
                            format!("{}...", expected[..3].join(", "))
                        } else {
                            expected.join(", ")
                        };
                        format!("❌ Unexpected Token\n{}{}\n\n📝 Expected: {}\n❓ Found: '{}'", 
                                error_msg, context_msg, expected_list, found)
                    },
                    crate::error::CompileError::ParseError(msg, _) => {
                        format!("❌ Parse Error: {}\n{}{}\n\n💡 Debug Tips:\n  1. Check for missing semicolons or braces\n  2. Verify syntax matches LANGUAGE_SPEC.md\n  3. Look for unmatched parentheses\n  4. Ensure proper use of '?' operator", 
                                msg, error_msg, context_msg)
                    },
                    crate::error::CompileError::SyntaxError(msg, _) => {
                        format!("❌ Syntax Error: {}\n{}{}", msg, error_msg, context_msg)
                    },
                    _ => format!("{}{}", error_msg, context_msg),
                };
                
                err.message = enhanced_msg.into();
                err
            })
    }

    async fn get_diagnostics(&self, uri: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        
        // Try to parse the document for more detailed error messages
        let documents = self.documents.read().await;
        if let Some(content) = documents.get(uri) {
            let lexer = Lexer::new(content);
            let mut parser = Parser::new(lexer);
            
            match parser.parse_program() {
                Ok(program) => {
                    // Check for import validation
                    diagnostics.extend(self.check_import_placement(&program));
                }
                Err(parse_error) => {
                    // Extract position information from the parse error if available
                    let (line, column, end_column) = if let Some(span) = parse_error.position() {
                        // Use the full span information for better error highlighting
                        let end_col = if span.end > span.start {
                            span.column + (span.end - span.start)
                        } else {
                            span.column + 1
                        };
                        (span.line as u32, span.column as u32, end_col as u32)
                    } else {
                        (0, 0, 1)
                    };
                    
                    // Get the error line for context
                    let lines: Vec<&str> = content.lines().collect();
                    let error_line = if line > 0 && (line as usize) <= lines.len() {
                        Some(lines[(line - 1) as usize])
                    } else {
                        None
                    };
                    
                    // Use the enhanced detailed_message for comprehensive error reporting
                    let lines_for_error: Vec<&str> = content.lines().collect();
                    let _detailed_error = parse_error.detailed_message(&lines_for_error);
                    
                    // Add additional LSP-specific formatting
                    let error_message = match &parse_error {
                        crate::error::CompileError::SyntaxError(msg, _) => {
                            let hint = if msg.contains("unexpected") || msg.contains("expected") {
                                "\n💡 Hint: Check for missing semicolons, parentheses, or braces."
                            } else if msg.contains("invalid") {
                                "\n💡 Hint: Review the Zen syntax rules - no 'if/else/match', use '?' operator."
                            } else {
                                "\n💡 Hint: Ensure your syntax follows Zen language specification."
                            };
                            format!("Syntax error: {}{}", msg, hint)
                        }
                        crate::error::CompileError::ParseError(msg, _) => {
                            let hint = if msg.contains("function") {
                                "\n💡 Hint: Functions are defined as 'name = (params) ReturnType { body }'."
                            } else if msg.contains("type") {
                                "\n💡 Hint: Use explicit types or type inference with ':=' or '::='."
                            } else if msg.contains("pattern") {
                                "\n💡 Hint: Pattern matching uses '?' operator: value ? | pattern => result."
                            } else {
                                "\n💡 Hint: Check the surrounding code for syntax issues."
                            };
                            format!("Parse error: {}{}", msg, hint)
                        }
                        crate::error::CompileError::TypeMismatch { expected, found, .. } => {
                            format!("Type mismatch: expected '{}', found '{}'\n💡 Hint: Check type annotations and ensure consistent types.", expected, found)
                        }
                        crate::error::CompileError::UndeclaredVariable(name, _) => {
                            format!("Undeclared variable: '{}'\n💡 Hint: Declare with 'name := value' (immutable) or 'name ::= value' (mutable).", name)
                        }
                        crate::error::CompileError::UndeclaredFunction(name, _) => {
                            format!("Undeclared function: '{}'\n💡 Hint: Define as 'name = (params) ReturnType {{ body }}'.", name)
                        }
                        crate::error::CompileError::InvalidLoopCondition(msg, _) => {
                            format!("Invalid loop condition: {}\n💡 Hint: Use 'loop (condition) {{ body }}' or infinite 'loop {{ body }}'.", msg)
                        }
                        crate::error::CompileError::MissingReturnStatement(func, _) => {
                            format!("Missing return in function '{}'\n💡 Hint: Either use explicit 'return value' or ensure last expression has no semicolon.", func)
                        }
                        crate::error::CompileError::ComptimeError(msg) => {
                            format!("Compile-time error: {}\n💡 Hint: Comptime blocks must be deterministic and cannot have side effects.", msg)
                        }
                        _ => format!("{}\n💡 Hint: Review the Zen language specification for correct syntax.", parse_error),
                    };
                    
                    // Add context line if available
                    let final_message = if let Some(line_content) = error_line {
                        format!("{}\n\n📍 Context:\n  {}\n  {}^", 
                               error_message, 
                               line_content,
                               " ".repeat(column as usize))
                    } else {
                        error_message
                    };
                    
                    // Add a detailed error diagnostic with actual parse error message
                    diagnostics.push(Diagnostic {
                        range: Range::new(
                            Position::new(line, column),
                            Position::new(line, end_column)
                        ),
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: None,
                        code_description: None,
                        source: Some("zen".to_string()),
                        message: final_message,
                        related_information: None,
                        tags: None,
                        data: None,
                    });
                }
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
                document_symbol_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default(),
                })),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                // Disable pull-based diagnostics for now - we use push-based via publish_diagnostics
                diagnostic_provider: None,
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

    async fn document_symbol(&self, params: DocumentSymbolParams) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri.to_string();
        let documents = self.documents.read().await;
        
        if let Some(content) = documents.get(&uri) {
            let symbols = enhanced::get_document_symbols(content);
            if !symbols.is_empty() {
                return Ok(Some(DocumentSymbolResponse::Nested(symbols)));
            }
        }
        
        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri.to_string();
        let position = params.text_document_position.position;
        let documents = self.documents.read().await;
        
        if let Some(content) = documents.get(&uri) {
            let references = enhanced::find_references(content, position);
            if !references.is_empty() {
                return Ok(Some(references.into_iter().map(|range| Location {
                    uri: params.text_document_position.text_document.uri.clone(),
                    range,
                }).collect()));
            }
        }
        
        Ok(None)
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri.to_string();
        let position = params.text_document_position.position;
        let new_name = params.new_name;
        let documents = self.documents.read().await;
        
        if let Some(content) = documents.get(&uri) {
            if let Some(edits) = enhanced::rename_symbol(content, position, &new_name) {
                let mut changes = HashMap::new();
                changes.insert(params.text_document_position.text_document.uri, edits);
                return Ok(Some(WorkspaceEdit {
                    changes: Some(changes),
                    document_changes: None,
                    change_annotations: None,
                }));
            }
        }
        
        Ok(None)
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri.to_string();
        let range = params.range;
        let documents = self.documents.read().await;
        
        if let Some(content) = documents.get(&uri) {
            let actions = enhanced::get_code_actions(content, range, &params.context);
            if !actions.is_empty() {
                return Ok(Some(actions));
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