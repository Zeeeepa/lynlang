// Enhanced LSP Server for Zen Language
// Provides advanced IDE features with compiler integration

use lsp_server::{Connection, Message, Request, Response, ResponseError, ErrorCode, Notification as ServerNotification};
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

#[derive(Debug, Clone)]
struct UfcMethodInfo {
    receiver: String,
    method_name: String,
}

#[derive(Debug)]
enum CompletionContext {
    General,
    UfcMethod { receiver_type: String },
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

        // Parse the document
        let lexer = Lexer::new(content);
        let mut parser = Parser::new(lexer);

        match parser.parse_program() {
            Ok(program) => {
                // Check for allocator issues
                for decl in &program.declarations {
                    if let Declaration::Function(func) = decl {
                        self.check_allocator_usage(&func.body, &mut diagnostics, content);
                    }
                }
            }
            Err(err) => {
                diagnostics.push(self.error_to_diagnostic(err));
            }
        }

        diagnostics
    }

    fn check_allocator_usage(&self, statements: &[Statement], diagnostics: &mut Vec<Diagnostic>, content: &str) {
        for stmt in statements {
            match stmt {
                Statement::Expression(expr) | Statement::Return(expr) => {
                    self.check_allocator_in_expression(expr, diagnostics, content);
                }
                Statement::VariableDeclaration { initializer: Some(expr), .. } |
                Statement::VariableAssignment { value: expr, .. } => {
                    self.check_allocator_in_expression(expr, diagnostics, content);
                }
                _ => {}
            }
        }
    }

    fn check_allocator_in_expression(&self, expr: &Expression, diagnostics: &mut Vec<Diagnostic>, content: &str) {
        match expr {
            Expression::FunctionCall { name, args, .. } => {
                // Enhanced collection constructors checking with generic support
                let collections_requiring_allocator = [
                    "HashMap", "DynVec", "Array", "HashSet", "BTreeMap", "LinkedList"
                ];

                // Check if this is a collection that requires an allocator
                let base_name = if name.contains('<') {
                    // Extract base type from generic like HashMap<String, i32>
                    name.split('<').next().unwrap_or(name)
                } else {
                    name.as_str()
                };

                if collections_requiring_allocator.contains(&base_name) {
                    if args.is_empty() || !self.has_allocator_arg(args) {
                        // Find the position of this function call in the source
                        if let Some(position) = self.find_text_position(name, content) {
                            diagnostics.push(Diagnostic {
                                range: Range {
                                    start: position,
                                    end: Position {
                                        line: position.line,
                                        character: position.character + name.len() as u32,
                                    },
                                },
                                severity: Some(DiagnosticSeverity::ERROR),
                                code: Some(NumberOrString::String("allocator-required".to_string())),
                                code_description: None,
                                source: Some("zen-lsp".to_string()),
                                message: format!(
                                    "{} requires an allocator for memory management. Add get_default_allocator() as the last parameter.",
                                    base_name
                                ),
                                related_information: Some(vec![
                                    DiagnosticRelatedInformation {
                                        location: Location {
                                            uri: Url::parse("file:///").unwrap(), // Placeholder
                                            range: Range {
                                                start: position,
                                                end: position,
                                            },
                                        },
                                        message: format!(
                                            "Quick fix: {}({}, get_default_allocator())",
                                            name, if args.is_empty() { "" } else { "..., " }
                                        ),
                                    }
                                ]),
                                tags: None,
                                data: None,
                            });
                        }
                    }
                }
                // Check method calls that might need allocators
                for arg in args {
                    self.check_allocator_in_expression(arg, diagnostics, content);
                }
            }
            Expression::MethodCall { object, method, args } => {
                // Enhanced checking for methods that require memory allocation
                let collection_allocating_methods = [
                    "push", "insert", "extend", "resize", "reserve",
                    "append", "merge", "clone", "copy", "drain"
                ];

                let string_allocating_methods = [
                    "concat", "repeat", "split", "replace", "join"
                ];

                let is_collection_method = collection_allocating_methods.contains(&method.as_str());
                let is_string_method = string_allocating_methods.contains(&method.as_str());

                if is_collection_method || is_string_method {
                    // Warn about allocating methods
                    let warning_type = if is_string_method { "String operation" } else { "Collection method" };
                    if let Some(position) = self.find_text_position(method, content) {
                        diagnostics.push(Diagnostic {
                            range: Range {
                                start: position,
                                end: Position {
                                    line: position.line,
                                    character: position.character + method.len() as u32,
                                },
                            },
                            severity: Some(DiagnosticSeverity::INFORMATION),
                            code: Some(NumberOrString::String("allocator-method".to_string())),
                            code_description: None,
                            source: Some("zen-lsp".to_string()),
                            message: format!(
                                "{} '{}' requires memory allocation. Ensure the object was created with an allocator.",
                                warning_type, method
                            ),
                            related_information: Some(vec![
                                DiagnosticRelatedInformation {
                                    location: Location {
                                        uri: Url::parse("file:///").unwrap(),
                                        range: Range {
                                            start: position,
                                            end: position,
                                        },
                                    },
                                    message: "Collections and dynamic strings must be initialized with an allocator to support operations that allocate memory.".to_string(),
                                }
                            ]),
                            tags: None,
                            data: None,
                        });
                    }
                }
                self.check_allocator_in_expression(object, diagnostics, content);
                for arg in args {
                    self.check_allocator_in_expression(arg, diagnostics, content);
                }
            }
            Expression::Block(stmts) => {
                self.check_allocator_usage(stmts, diagnostics, content);
            }
            Expression::Conditional { scrutinee, arms } => {
                self.check_allocator_in_expression(scrutinee, diagnostics, content);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        self.check_allocator_in_expression(guard, diagnostics, content);
                    }
                    self.check_allocator_in_expression(&arm.body, diagnostics, content);
                }
            }
            Expression::BinaryOp { left, right, .. } => {
                self.check_allocator_in_expression(left, diagnostics, content);
                self.check_allocator_in_expression(right, diagnostics, content);
            }
            _ => {}
        }
    }

    fn has_allocator_arg(&self, args: &[Expression]) -> bool {
        // Enhanced checking for allocator arguments
        for arg in args {
            match arg {
                Expression::FunctionCall { name, .. } => {
                    // Check for common allocator functions
                    if name.contains("allocator") || name == "get_default_allocator" {
                        return true;
                    }
                }
                Expression::Identifier(name) => {
                    // Check for variables that might be allocators
                    if name.contains("alloc") || name.ends_with("_allocator") || name == "allocator" {
                        return true;
                    }
                }
                Expression::MethodCall { object, method, .. } => {
                    // Check for allocator obtained from method calls
                    if method.contains("allocator") || method == "get_allocator" {
                        return true;
                    }
                    // Recursively check the object
                    if self.has_allocator_arg(&[(**object).clone()]) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn find_text_position(&self, text: &str, content: &str) -> Option<Position> {
        let lines: Vec<&str> = content.lines().collect();
        for (line_num, line) in lines.iter().enumerate() {
            if let Some(col) = line.find(text) {
                return Some(Position {
                    line: line_num as u32,
                    character: col as u32,
                });
            }
        }
        None
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

    fn find_references_in_expression(&self, expr: &Expression, symbols: &mut HashMap<String, SymbolInfo>) {
        match expr {
            Expression::Identifier(name) => {
                // Track reference to this identifier
                if let Some(symbol) = symbols.get_mut(name) {
                    // Add reference location (would need position info)
                }
            }
            Expression::FunctionCall { name, args, .. } => {
                // Track function call reference
                if let Some(symbol) = symbols.get_mut(name) {
                    // Add reference location
                }
                // Recurse into arguments
                for arg in args {
                    self.find_references_in_expression(arg, symbols);
                }
            }
            Expression::MethodCall { object, method: _, args } => {
                // Track UFC method call - recurse into object and args
                self.find_references_in_expression(object, symbols);
                for arg in args {
                    self.find_references_in_expression(arg, symbols);
                }
            }
            Expression::BinaryOp { left, right, .. } => {
                self.find_references_in_expression(left, symbols);
                self.find_references_in_expression(right, symbols);
            }
            Expression::MemberAccess { object, .. } => {
                self.find_references_in_expression(object, symbols);
            }
            Expression::ArrayIndex { array, index } => {
                self.find_references_in_expression(array, symbols);
                self.find_references_in_expression(index, symbols);
            }
            Expression::Conditional { scrutinee, arms } => {
                self.find_references_in_expression(scrutinee, symbols);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        self.find_references_in_expression(guard, symbols);
                    }
                    self.find_references_in_expression(&arm.body, symbols);
                }
            }
            Expression::Closure { body, .. } => {
                // Recurse into closure body expression
                self.find_references_in_expression(body, symbols);
            }
            Expression::Block(stmts) => {
                self.find_references_in_statements(stmts, symbols);
            }
            _ => {}
        }
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
            "textDocument/semanticTokens/full" => self.handle_semantic_tokens(req.clone()),
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
        let mut completions = Vec::new();

        // Check if we're completing after a dot (UFC method call)
        if let Some(doc) = store.documents.get(&params.text_document_position.text_document.uri) {
            let position = params.text_document_position.position;

            if let Some(context) = self.get_completion_context(&doc.content, position) {
                match context {
                    CompletionContext::UfcMethod { receiver_type } => {
                        // Provide UFC method completions
                        completions = self.get_ufc_method_completions(&receiver_type);

                        let response = CompletionResponse::Array(completions);
                        return Response {
                            id: req.id,
                            result: Some(serde_json::to_value(response).unwrap_or(Value::Null)),
                            error: None,
                        };
                    }
                    CompletionContext::General => {
                        // Fall through to provide general completions
                    }
                }
            }
        }

        // Provide general completions
        completions = vec![
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

    fn resolve_ufc_method(&self, method_info: &UfcMethodInfo, store: &DocumentStore) -> Option<Location> {
        // Enhanced UFC method resolution with improved generic type handling
        let receiver_type = self.infer_receiver_type(&method_info.receiver, store);

        // Extract base type and generic parameters for better matching
        let (base_type, _generic_params) = if let Some(ref typ) = receiver_type {
            self.parse_generic_type(typ)
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
        let params: CodeActionParams = match serde_json::from_value(req.params) {
            Ok(p) => p,
            Err(_) => {
                return Response {
                    id: req.id,
                    result: Some(Value::Null),
                    error: None,
                };
            }
        };

        let mut actions = Vec::new();
        let store = self.store.lock().unwrap();

        if let Some(doc) = store.documents.get(&params.text_document.uri) {
            // Check diagnostics in the requested range
            for diagnostic in &params.context.diagnostics {
                if diagnostic.message.contains("requires an allocator") {
                    // Create a code action to add get_default_allocator()
                    actions.push(self.create_allocator_fix_action(diagnostic, &params.text_document.uri, &doc.content));
                }

                // Add code action for string conversions
                if diagnostic.message.contains("type mismatch") &&
                   (diagnostic.message.contains("StaticString") || diagnostic.message.contains("String")) {
                    if let Some(action) = self.create_string_conversion_action(diagnostic, &params.text_document.uri, &doc.content) {
                        actions.push(action);
                    }
                }

                // Add code action for missing error handling
                if diagnostic.message.contains("Result") && diagnostic.message.contains("unwrap") {
                    if let Some(action) = self.create_error_handling_action(diagnostic, &params.text_document.uri) {
                        actions.push(action);
                    }
                }
            }
        }

        Response {
            id: req.id,
            result: Some(serde_json::to_value(actions).unwrap_or(Value::Null)),
            error: None,
        }
    }

    fn create_allocator_fix_action(&self, diagnostic: &Diagnostic, uri: &Url, content: &str) -> CodeAction {
        // Extract the line content
        let lines: Vec<&str> = content.lines().collect();
        let line_content = if (diagnostic.range.start.line as usize) < lines.len() {
            lines[diagnostic.range.start.line as usize]
        } else {
            ""
        };

        // Determine if we need to add allocator parameter or insert new call
        let (new_text, edit_range) = if line_content.contains("()") {
            // Empty parentheses - add allocator as first parameter
            ("get_default_allocator()".to_string(), Range {
                start: Position {
                    line: diagnostic.range.start.line,
                    character: diagnostic.range.end.character - 1,  // Before closing paren
                },
                end: Position {
                    line: diagnostic.range.start.line,
                    character: diagnostic.range.end.character - 1,
                },
            })
        } else if line_content.contains("(") {
            // Has parameters - add allocator as additional parameter
            (", get_default_allocator()".to_string(), Range {
                start: Position {
                    line: diagnostic.range.end.line,
                    character: diagnostic.range.end.character - 1,  // Before closing paren
                },
                end: Position {
                    line: diagnostic.range.end.line,
                    character: diagnostic.range.end.character - 1,
                },
            })
        } else {
            // No parentheses - add full call
            ("(get_default_allocator())".to_string(), diagnostic.range.clone())
        };

        let text_edit = TextEdit {
            range: edit_range,
            new_text,
        };

        let workspace_edit = WorkspaceEdit {
            changes: Some({
                let mut changes = HashMap::new();
                changes.insert(uri.clone(), vec![text_edit]);
                changes
            }),
            document_changes: None,
            change_annotations: None,
        };

        CodeAction {
            title: "Add get_default_allocator()".to_string(),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diagnostic.clone()]),
            edit: Some(workspace_edit),
            command: None,
            is_preferred: Some(true),
            disabled: None,
            data: None,
        }
    }

    fn create_string_conversion_action(&self, diagnostic: &Diagnostic, uri: &Url, content: &str) -> Option<CodeAction> {
        // Determine conversion direction
        let title = if diagnostic.message.contains("expected StaticString") {
            "Convert to StaticString"
        } else if diagnostic.message.contains("expected String") {
            "Convert to String with allocator"
        } else {
            return None;
        };

        let workspace_edit = WorkspaceEdit {
            changes: None,  // Would need more context to implement actual conversion
            document_changes: None,
            change_annotations: None,
        };

        Some(CodeAction {
            title: title.to_string(),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diagnostic.clone()]),
            edit: Some(workspace_edit),
            command: None,
            is_preferred: Some(false),
            disabled: None,
            data: None,
        })
    }

    fn create_error_handling_action(&self, diagnostic: &Diagnostic, uri: &Url) -> Option<CodeAction> {
        let title = "Add proper error handling";

        let workspace_edit = WorkspaceEdit {
            changes: None,  // Would need AST analysis to implement
            document_changes: None,
            change_annotations: None,
        };

        Some(CodeAction {
            title: title.to_string(),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diagnostic.clone()]),
            edit: Some(workspace_edit),
            command: None,
            is_preferred: Some(false),
            disabled: None,
            data: None,
        })
    }

    fn handle_semantic_tokens(&self, req: Request) -> Response {
        let params: SemanticTokensParams = match serde_json::from_value(req.params) {
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
        let doc = match store.documents.get(&params.text_document.uri) {
            Some(doc) => doc,
            None => {
                return Response {
                    id: req.id,
                    result: Some(Value::Null),
                    error: None,
                }
            }
        };

        // Generate semantic tokens for the document
        let tokens = self.generate_semantic_tokens(&doc.content);

        let result = SemanticTokens {
            result_id: None,
            data: tokens,
        };

        Response {
            id: req.id,
            result: Some(serde_json::to_value(result).unwrap_or(Value::Null)),
            error: None,
        }
    }

    fn generate_semantic_tokens(&self, content: &str) -> Vec<SemanticToken> {
        let mut tokens = Vec::new();
        let mut lexer = Lexer::new(content);

        // Token type indices (must match the legend in server capabilities)
        const TYPE_NAMESPACE: u32 = 0;
        const TYPE_TYPE: u32 = 1;
        const TYPE_CLASS: u32 = 2;
        const TYPE_ENUM: u32 = 3;
        const TYPE_INTERFACE: u32 = 4;
        const TYPE_STRUCT: u32 = 5;
        const TYPE_TYPE_PARAM: u32 = 6;
        const TYPE_PARAMETER: u32 = 7;
        const TYPE_VARIABLE: u32 = 8;
        const TYPE_PROPERTY: u32 = 9;
        const TYPE_ENUM_MEMBER: u32 = 10;
        const TYPE_EVENT: u32 = 11;
        const TYPE_FUNCTION: u32 = 12;
        const TYPE_METHOD: u32 = 13;
        const TYPE_MACRO: u32 = 14;
        const TYPE_KEYWORD: u32 = 15;
        const TYPE_MODIFIER: u32 = 16;
        const TYPE_COMMENT: u32 = 17;
        const TYPE_STRING: u32 = 18;
        const TYPE_NUMBER: u32 = 19;
        const TYPE_REGEXP: u32 = 20;
        const TYPE_OPERATOR: u32 = 21;

        // Token modifiers (can be combined)
        const MOD_DECLARATION: u32 = 0b1;
        const MOD_DEFINITION: u32 = 0b10;
        const MOD_READONLY: u32 = 0b100;
        const MOD_STATIC: u32 = 0b1000;
        const MOD_DEPRECATED: u32 = 0b10000;
        const MOD_ABSTRACT: u32 = 0b100000;
        const MOD_ASYNC: u32 = 0b1000000;
        const MOD_MODIFICATION: u32 = 0b10000000;
        const MOD_DOCUMENTATION: u32 = 0b100000000;
        const MOD_DEFAULT_LIBRARY: u32 = 0b1000000000;

        let mut prev_line = 0;
        let mut prev_start = 0;
        let lines: Vec<&str> = content.lines().collect();

        // Track context for better token classification
        let mut in_function = false;
        let mut in_struct = false;
        let mut in_allocator_context = false;

        for (line_idx, line) in lines.iter().enumerate() {
            let mut char_idx = 0;
            let mut chars = line.chars().peekable();

            while let Some(ch) = chars.next() {
                let start = char_idx;
                char_idx += ch.len_utf8();

                // Skip whitespace
                if ch.is_whitespace() {
                    continue;
                }

                // Comments
                if ch == '/' && chars.peek() == Some(&'/') {
                    // Single-line comment
                    let length = line.len() - start;
                    let delta_line = line_idx as u32 - prev_line;
                    let delta_start = if delta_line == 0 {
                        start as u32 - prev_start
                    } else {
                        start as u32
                    };

                    tokens.push(SemanticToken {
                        delta_line,
                        delta_start,
                        length: length as u32,
                        token_type: TYPE_COMMENT,
                        token_modifiers_bitset: 0,
                    });

                    prev_line = line_idx as u32;
                    prev_start = start as u32;
                    break; // Rest of line is comment
                }

                // String literals
                if ch == '"' {
                    let mut string_len = 1;
                    let mut escaped = false;
                    while let Some(&next_ch) = chars.peek() {
                        chars.next();
                        char_idx += next_ch.len_utf8();
                        string_len += next_ch.len_utf8();

                        if escaped {
                            escaped = false;
                        } else if next_ch == '\\' {
                            escaped = true;
                        } else if next_ch == '"' {
                            break;
                        }
                    }

                    let delta_line = line_idx as u32 - prev_line;
                    let delta_start = if delta_line == 0 {
                        start as u32 - prev_start
                    } else {
                        start as u32
                    };

                    tokens.push(SemanticToken {
                        delta_line,
                        delta_start,
                        length: string_len as u32,
                        token_type: TYPE_STRING,
                        token_modifiers_bitset: 0,
                    });

                    prev_line = line_idx as u32;
                    prev_start = start as u32;
                    continue;
                }

                // Numbers
                if ch.is_numeric() {
                    let mut num_len = ch.len_utf8();
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch.is_numeric() || next_ch == '.' || next_ch == '_' {
                            chars.next();
                            char_idx += next_ch.len_utf8();
                            num_len += next_ch.len_utf8();
                        } else {
                            break;
                        }
                    }

                    let delta_line = line_idx as u32 - prev_line;
                    let delta_start = if delta_line == 0 {
                        start as u32 - prev_start
                    } else {
                        start as u32
                    };

                    tokens.push(SemanticToken {
                        delta_line,
                        delta_start,
                        length: num_len as u32,
                        token_type: TYPE_NUMBER,
                        token_modifiers_bitset: 0,
                    });

                    prev_line = line_idx as u32;
                    prev_start = start as u32;
                    continue;
                }

                // Identifiers and keywords
                if ch.is_alphabetic() || ch == '_' {
                    let mut word = String::from(ch);
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch.is_alphanumeric() || next_ch == '_' {
                            chars.next();
                            char_idx += next_ch.len_utf8();
                            word.push(next_ch);
                        } else {
                            break;
                        }
                    }

                    // Check if this is a UFC method call by looking ahead for '.'
                    let is_ufc_call = chars.peek() == Some(&'.');

                    // Check if this is allocator-related
                    let is_allocator_related = word.contains("allocator") || word == "alloc" || word == "dealloc";

                    let (token_type, modifiers) = match word.as_str() {
                        // Keywords
                        "fn" => {
                            in_function = true;
                            (TYPE_KEYWORD, 0)
                        }
                        "struct" => {
                            in_struct = true;
                            (TYPE_KEYWORD, 0)
                        }
                        "enum" => (TYPE_KEYWORD, 0),
                        "let" | "mut" | "const" => (TYPE_KEYWORD, 0),
                        "if" | "else" | "match" | "while" | "for" | "loop" | "break" | "continue" => (TYPE_KEYWORD, 0),
                        "return" => (TYPE_KEYWORD, 0),
                        "raise" => (TYPE_KEYWORD, MOD_ASYNC), // Special highlighting for error propagation
                        "import" | "export" | "pub" => (TYPE_KEYWORD, 0),
                        "true" | "false" | "null" => (TYPE_KEYWORD, 0),

                        // Built-in types
                        "i8" | "i16" | "i32" | "i64" | "i128" |
                        "u8" | "u16" | "u32" | "u64" | "u128" |
                        "f32" | "f64" | "bool" | "void" => (TYPE_TYPE, MOD_DEFAULT_LIBRARY),

                        // Zen-specific types
                        "String" | "StaticString" => (TYPE_TYPE, MOD_DEFAULT_LIBRARY),
                        "Option" | "Result" => (TYPE_ENUM, MOD_DEFAULT_LIBRARY),
                        "HashMap" | "DynVec" | "Vec" | "Array" | "HashSet" => {
                            in_allocator_context = true; // These types need allocators
                            (TYPE_CLASS, MOD_DEFAULT_LIBRARY)
                        }
                        "Allocator" => (TYPE_INTERFACE, MOD_DEFAULT_LIBRARY | MOD_ABSTRACT),

                        // Allocator-related functions (highlight specially)
                        "get_default_allocator" => (TYPE_FUNCTION, MOD_DEFAULT_LIBRARY | MOD_STATIC),

                        // Enum variants
                        "Some" | "None" | "Ok" | "Err" => (TYPE_ENUM_MEMBER, 0),

                        // Function names (when we know we're after 'fn')
                        _ if in_function && prev_line == line_idx as u32 => {
                            in_function = false;
                            (TYPE_FUNCTION, MOD_DECLARATION | MOD_DEFINITION)
                        }

                        // Allocator-related identifiers get special highlighting
                        _ if is_allocator_related => (TYPE_INTERFACE, MOD_ABSTRACT),

                        // UFC calls get method highlighting
                        _ if is_ufc_call => (TYPE_VARIABLE, 0), // Will be followed by method

                        // Default to variable
                        _ => (TYPE_VARIABLE, 0),
                    };

                    let delta_line = line_idx as u32 - prev_line;
                    let delta_start = if delta_line == 0 {
                        start as u32 - prev_start
                    } else {
                        start as u32
                    };

                    tokens.push(SemanticToken {
                        delta_line,
                        delta_start,
                        length: word.len() as u32,
                        token_type,
                        token_modifiers_bitset: modifiers,
                    });

                    prev_line = line_idx as u32;
                    prev_start = start as u32;
                    continue;
                }

                // Handle dot operator specially for UFC method calls
                if ch == '.' {
                    // Mark the dot operator
                    let delta_line = line_idx as u32 - prev_line;
                    let delta_start = if delta_line == 0 {
                        start as u32 - prev_start
                    } else {
                        start as u32
                    };

                    tokens.push(SemanticToken {
                        delta_line,
                        delta_start,
                        length: 1,
                        token_type: TYPE_OPERATOR,
                        token_modifiers_bitset: 0,
                    });

                    prev_line = line_idx as u32;
                    prev_start = start as u32;

                    // Look ahead for method name after dot
                    if let Some(&next_ch) = chars.peek() {
                        if next_ch.is_alphabetic() || next_ch == '_' {
                            // Skip the dot we just processed
                            let method_start = char_idx;
                            let mut method_name = String::new();

                            while let Some(&ch) = chars.peek() {
                                if ch.is_alphanumeric() || ch == '_' {
                                    chars.next();
                                    char_idx += ch.len_utf8();
                                    method_name.push(ch);
                                } else {
                                    break;
                                }
                            }

                            // Add the method name token with special highlighting
                            let delta_line = 0;  // Same line as dot
                            let delta_start = method_start as u32 - prev_start;

                            // Determine if this is a special method
                            let is_allocator_method = method_name.contains("alloc");
                            let is_error_method = method_name == "raise";

                            let (token_type, modifiers) = if is_error_method {
                                (TYPE_METHOD, MOD_ASYNC) // Special for error propagation
                            } else if is_allocator_method {
                                (TYPE_METHOD, MOD_ABSTRACT) // Special for allocator methods
                            } else {
                                (TYPE_METHOD, 0) // Regular UFC method
                            };

                            tokens.push(SemanticToken {
                                delta_line,
                                delta_start,
                                length: method_name.len() as u32,
                                token_type,
                                token_modifiers_bitset: modifiers,
                            });

                            prev_line = line_idx as u32;
                            prev_start = method_start as u32;
                        }
                    }
                    continue;
                }

                // Other operators
                if "+-*/%&|^!<>=".contains(ch) {
                    let delta_line = line_idx as u32 - prev_line;
                    let delta_start = if delta_line == 0 {
                        start as u32 - prev_start
                    } else {
                        start as u32
                    };

                    tokens.push(SemanticToken {
                        delta_line,
                        delta_start,
                        length: 1,
                        token_type: TYPE_OPERATOR,
                        token_modifiers_bitset: 0,
                    });

                    prev_line = line_idx as u32;
                    prev_start = start as u32;
                }
            }
        }

        tokens
    }

    fn parse_generic_type(&self, type_str: &str) -> (String, Vec<String>) {
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

                if !current.trim().is_empty() {
                    params.push(current.trim().to_string());
                }

                params
            };

            (base, params)
        } else {
            (type_str.to_string(), Vec::new())
        }
    }

    fn infer_receiver_type(&self, receiver: &str, store: &DocumentStore) -> Option<String> {
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

        // Check if receiver is a known variable in symbols
        for doc in store.documents.values() {
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
                    // Check for collection types with generics
                    if let Some(cap) = regex::Regex::new(r"(HashMap|DynVec|Vec|Array|Option|Result)<[^>]+>")
                        .ok()?.captures(detail) {
                        return Some(cap[1].to_string());
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

        // Enhanced pattern matching for function calls and constructors
        let patterns = [
            (r"HashMap\s*\(", "HashMap"),
            (r"DynVec\s*\(", "DynVec"),
            (r"Vec\s*[<(]", "Vec"),
            (r"Array\s*\(", "Array"),
            (r"Some\s*\(", "Option"),
            (r"None", "Option"),
            (r"Ok\s*\(", "Result"),
            (r"Err\s*\(", "Result"),
            (r"Result\.", "Result"),
            (r"Option\.", "Option"),
            (r"get_default_allocator\s*\(\)", "Allocator"),
            (r"\[\s*\d+\s*;\s*\d+\s*\]", "Array"), // Fixed array syntax
        ];

        for (pattern, type_name) in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if re.is_match(receiver) {
                    return Some(type_name.to_string());
                }
            }
        }

        // Enhanced support for method call chains (e.g., foo.bar().baz())
        if receiver.contains('.') && receiver.contains('(') {
            return self.infer_chained_method_type(receiver, store);
        }

        None
    }

    fn infer_chained_method_type(&self, receiver: &str, store: &DocumentStore) -> Option<String> {
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
                current_type = self.infer_base_expression_type(part, store);
            } else if let Some(ref curr_type) = current_type {
                // Subsequent parts are method calls on the current type
                if let Some(method_name) = part.split('(').next() {
                    current_type = self.get_method_return_type(curr_type, method_name);
                }
            }
        }

        current_type
    }

    fn infer_base_expression_type(&self, expr: &str, store: &DocumentStore) -> Option<String> {
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
                    // Check if it's a variable
                    for doc in store.documents.values() {
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

    fn get_method_return_type(&self, receiver_type: &str, method_name: &str) -> Option<String> {
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

    fn get_completion_context(&self, content: &str, position: Position) -> Option<CompletionContext> {
        let lines: Vec<&str> = content.lines().collect();
        if position.line as usize >= lines.len() {
            return Some(CompletionContext::General);
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
            let receiver_type = self.infer_receiver_type(receiver, &store)
                .unwrap_or_else(|| "unknown".to_string());

            return Some(CompletionContext::UfcMethod {
                receiver_type,
            });
        }

        Some(CompletionContext::General)
    }

    fn get_ufc_method_completions(&self, receiver_type: &str) -> Vec<CompletionItem> {
        // Provide type-specific completions based on receiver type
        let mut completions = Vec::new();

        // Determine what methods are available based on receiver type
        match receiver_type {
            typ if typ == "Result" || typ.starts_with("Result<") => {
                completions.extend(vec![
                    CompletionItem {
                        label: "raise".to_string(),
                        kind: Some(CompletionItemKind::METHOD),
                        detail: Some("raise() T".to_string()),
                        documentation: Some(Documentation::String(
                            "Extracts the Ok value from Result<T,E> or propagates the error".to_string()
                        )),
                        insert_text: Some("raise()".to_string()),
                        ..Default::default()
                    },
                    CompletionItem {
                        label: "is_ok".to_string(),
                        kind: Some(CompletionItemKind::METHOD),
                        detail: Some("is_ok() bool".to_string()),
                        documentation: Some(Documentation::String("Check if Result is Ok".to_string())),
                        insert_text: Some("is_ok()".to_string()),
                        ..Default::default()
                    },
                    CompletionItem {
                        label: "is_err".to_string(),
                        kind: Some(CompletionItemKind::METHOD),
                        detail: Some("is_err() bool".to_string()),
                        documentation: Some(Documentation::String("Check if Result is Err".to_string())),
                        insert_text: Some("is_err()".to_string()),
                        ..Default::default()
                    },
                ]);
            }
            typ if typ == "Option" || typ.starts_with("Option<") => {
                completions.extend(vec![
                    CompletionItem {
                        label: "is_some".to_string(),
                        kind: Some(CompletionItemKind::METHOD),
                        detail: Some("is_some() bool".to_string()),
                        documentation: Some(Documentation::String("Check if Option is Some".to_string())),
                        insert_text: Some("is_some()".to_string()),
                        ..Default::default()
                    },
                    CompletionItem {
                        label: "is_none".to_string(),
                        kind: Some(CompletionItemKind::METHOD),
                        detail: Some("is_none() bool".to_string()),
                        documentation: Some(Documentation::String("Check if Option is None".to_string())),
                        insert_text: Some("is_none()".to_string()),
                        ..Default::default()
                    },
                    CompletionItem {
                        label: "unwrap".to_string(),
                        kind: Some(CompletionItemKind::METHOD),
                        detail: Some("unwrap() T".to_string()),
                        documentation: Some(Documentation::String("Extract value or panic if None".to_string())),
                        insert_text: Some("unwrap()".to_string()),
                        ..Default::default()
                    },
                    CompletionItem {
                        label: "unwrap_or".to_string(),
                        kind: Some(CompletionItemKind::METHOD),
                        detail: Some("unwrap_or(default: T) T".to_string()),
                        documentation: Some(Documentation::String("Extract value or return default".to_string())),
                        insert_text: Some("unwrap_or(${1:default})".to_string()),
                        ..Default::default()
                    },
                ]);
            }
            "String" | "StaticString" | "str" => {
                completions.extend(self.get_string_method_completions());
            }
            typ if typ == "HashMap" || typ.starts_with("HashMap<") => {
                completions.extend(self.get_hashmap_method_completions());
            }
            typ if typ == "DynVec" || typ.starts_with("DynVec<") => {
                completions.extend(self.get_dynvec_method_completions());
            }
            typ if typ == "Vec" || typ.starts_with("Vec<") => {
                completions.extend(self.get_vec_method_completions());
            }
            typ if typ == "Array" || typ.starts_with("Array<") => {
                completions.extend(self.get_array_method_completions());
            }
            _ => {
                // For unknown types, provide all common methods
                completions.extend(self.get_all_common_methods());
            }
        }

        // Add loop method for all collection types
        if receiver_type.contains("Vec") || receiver_type.contains("Array") ||
           receiver_type.contains("HashMap") || receiver_type.contains("DynVec") {
            completions.push(CompletionItem {
                label: "loop".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("loop((item) { ... })".to_string()),
                documentation: Some(Documentation::String("Iterate over collection".to_string())),
                insert_text: Some("loop((${1:item}) {\n    ${0}\n})".to_string()),
                ..Default::default()
            });
        }

        completions
    }

    fn get_string_method_completions(&self) -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "len".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("len() usize".to_string()),
                documentation: Some(Documentation::String("Returns the length of the string".to_string())),
                insert_text: Some("len()".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "to_i32".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("to_i32() Option<i32>".to_string()),
                documentation: Some(Documentation::String("Parse string to i32".to_string())),
                insert_text: Some("to_i32()".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "to_i64".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("to_i64() Option<i64>".to_string()),
                documentation: Some(Documentation::String("Parse string to i64".to_string())),
                insert_text: Some("to_i64()".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "to_f64".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("to_f64() Option<f64>".to_string()),
                documentation: Some(Documentation::String("Parse string to f64".to_string())),
                insert_text: Some("to_f64()".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "to_upper".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("to_upper() String".to_string()),
                documentation: Some(Documentation::String("Convert to uppercase".to_string())),
                insert_text: Some("to_upper()".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "to_lower".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("to_lower() String".to_string()),
                documentation: Some(Documentation::String("Convert to lowercase".to_string())),
                insert_text: Some("to_lower()".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "trim".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("trim() String".to_string()),
                documentation: Some(Documentation::String("Remove leading/trailing whitespace".to_string())),
                insert_text: Some("trim()".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "split".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("split(delimiter: String) Array<String>".to_string()),
                documentation: Some(Documentation::String("Split string by delimiter (requires allocator)".to_string())),
                insert_text: Some("split(\"${1:,}\")".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "substr".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("substr(start: usize, length: usize) String".to_string()),
                documentation: Some(Documentation::String("Extract substring".to_string())),
                insert_text: Some("substr(${1:0}, ${2:10})".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "char_at".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("char_at(index: usize) Option<char>".to_string()),
                documentation: Some(Documentation::String("Get character at index".to_string())),
                insert_text: Some("char_at(${1:0})".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "contains".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("contains(substr: String) bool".to_string()),
                documentation: Some(Documentation::String("Check if string contains substring".to_string())),
                insert_text: Some("contains(\"${1}\")".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "starts_with".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("starts_with(prefix: String) bool".to_string()),
                documentation: Some(Documentation::String("Check if string starts with prefix".to_string())),
                insert_text: Some("starts_with(\"${1}\")".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "ends_with".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("ends_with(suffix: String) bool".to_string()),
                documentation: Some(Documentation::String("Check if string ends with suffix".to_string())),
                insert_text: Some("ends_with(\"${1}\")".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "index_of".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("index_of(substr: String) Option<usize>".to_string()),
                documentation: Some(Documentation::String("Find index of substring".to_string())),
                insert_text: Some("index_of(\"${1}\")".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "replace".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("replace(old: String, new: String) String".to_string()),
                documentation: Some(Documentation::String("Replace all occurrences".to_string())),
                insert_text: Some("replace(\"${1:old}\", \"${2:new}\")".to_string()),
                ..Default::default()
            },
        ]
    }

    fn get_hashmap_method_completions(&self) -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "insert".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("insert(key: K, value: V) Option<V>".to_string()),
                documentation: Some(Documentation::String("Insert key-value pair, returns previous value if any".to_string())),
                insert_text: Some("insert(${1:key}, ${2:value})".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "get".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("get(key: K) Option<V>".to_string()),
                documentation: Some(Documentation::String("Get value for key".to_string())),
                insert_text: Some("get(${1:key})".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "remove".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("remove(key: K) Option<V>".to_string()),
                documentation: Some(Documentation::String("Remove key-value pair, returns value if existed".to_string())),
                insert_text: Some("remove(${1:key})".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "contains_key".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("contains_key(key: K) bool".to_string()),
                documentation: Some(Documentation::String("Check if key exists".to_string())),
                insert_text: Some("contains_key(${1:key})".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "len".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("len() usize".to_string()),
                documentation: Some(Documentation::String("Get number of key-value pairs".to_string())),
                insert_text: Some("len()".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "clear".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("clear() void".to_string()),
                documentation: Some(Documentation::String("Remove all key-value pairs".to_string())),
                insert_text: Some("clear()".to_string()),
                ..Default::default()
            },
        ]
    }

    fn get_dynvec_method_completions(&self) -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "push".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("push(value: T) void".to_string()),
                documentation: Some(Documentation::String("Add element to end of vector".to_string())),
                insert_text: Some("push(${1:value})".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "pop".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("pop() Option<T>".to_string()),
                documentation: Some(Documentation::String("Remove and return last element".to_string())),
                insert_text: Some("pop()".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "get".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("get(index: usize) Option<T>".to_string()),
                documentation: Some(Documentation::String("Get element at index".to_string())),
                insert_text: Some("get(${1:0})".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "set".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("set(index: usize, value: T) void".to_string()),
                documentation: Some(Documentation::String("Set element at index".to_string())),
                insert_text: Some("set(${1:0}, ${2:value})".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "insert".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("insert(index: usize, value: T) void".to_string()),
                documentation: Some(Documentation::String("Insert element at index".to_string())),
                insert_text: Some("insert(${1:0}, ${2:value})".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "remove".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("remove(index: usize) Option<T>".to_string()),
                documentation: Some(Documentation::String("Remove element at index".to_string())),
                insert_text: Some("remove(${1:0})".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "len".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("len() usize".to_string()),
                documentation: Some(Documentation::String("Get number of elements".to_string())),
                insert_text: Some("len()".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "capacity".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("capacity() usize".to_string()),
                documentation: Some(Documentation::String("Get current capacity".to_string())),
                insert_text: Some("capacity()".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "clear".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("clear() void".to_string()),
                documentation: Some(Documentation::String("Remove all elements".to_string())),
                insert_text: Some("clear()".to_string()),
                ..Default::default()
            },
        ]
    }

    fn get_vec_method_completions(&self) -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "push".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("push(value: T) bool".to_string()),
                documentation: Some(Documentation::String("Add element if space available (fixed-size vector)".to_string())),
                insert_text: Some("push(${1:value})".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "get".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("get(index: usize) Option<T>".to_string()),
                documentation: Some(Documentation::String("Get element at index".to_string())),
                insert_text: Some("get(${1:0})".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "set".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("set(index: usize, value: T) bool".to_string()),
                documentation: Some(Documentation::String("Set element at index".to_string())),
                insert_text: Some("set(${1:0}, ${2:value})".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "len".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("len() usize".to_string()),
                documentation: Some(Documentation::String("Get current number of elements".to_string())),
                insert_text: Some("len()".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "capacity".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("capacity() usize".to_string()),
                documentation: Some(Documentation::String("Get fixed capacity".to_string())),
                insert_text: Some("capacity()".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "clear".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("clear() void".to_string()),
                documentation: Some(Documentation::String("Remove all elements".to_string())),
                insert_text: Some("clear()".to_string()),
                ..Default::default()
            },
        ]
    }

    fn get_array_method_completions(&self) -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "len".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("len() usize".to_string()),
                documentation: Some(Documentation::String("Get array length".to_string())),
                insert_text: Some("len()".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "get".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("get(index: usize) Option<T>".to_string()),
                documentation: Some(Documentation::String("Get element at index".to_string())),
                insert_text: Some("get(${1:0})".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "set".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("set(index: usize, value: T) void".to_string()),
                documentation: Some(Documentation::String("Set element at index".to_string())),
                insert_text: Some("set(${1:0}, ${2:value})".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "push".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("push(value: T) void".to_string()),
                documentation: Some(Documentation::String("Add element to end (may reallocate)".to_string())),
                insert_text: Some("push(${1:value})".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "pop".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("pop() Option<T>".to_string()),
                documentation: Some(Documentation::String("Remove and return last element".to_string())),
                insert_text: Some("pop()".to_string()),
                ..Default::default()
            },
        ]
    }

    fn get_all_common_methods(&self) -> Vec<CompletionItem> {
        // Return a combination of all common methods when type is unknown
        let mut methods = Vec::new();
        methods.extend(self.get_string_method_completions());
        methods.extend(vec![
            CompletionItem {
                label: "raise".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("raise() T".to_string()),
                documentation: Some(Documentation::String(
                    "Extracts the Ok value from Result<T,E> or propagates the error".to_string()
                )),
                insert_text: Some("raise()".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "push".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("push(value: T) void".to_string()),
                documentation: Some(Documentation::String("Add element to collection".to_string())),
                insert_text: Some("push(${1})".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "get".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("get(index) Option<T>".to_string()),
                documentation: Some(Documentation::String("Get element at index".to_string())),
                insert_text: Some("get(${1:0})".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "loop".to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("loop((item) { ... })".to_string()),
                documentation: Some(Documentation::String("Iterate over collection".to_string())),
                insert_text: Some("loop((${1:item}) {\n    ${0}\n})".to_string()),
                ..Default::default()
            },
        ]);
        methods
    }
}