// Enhanced LSP Server for Zen Language
// Provides advanced IDE features with compiler integration

use lsp_server::{Connection, Message, Request, Response, ResponseError, ErrorCode, Notification as ServerNotification};
use lsp_types::*;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::sync::mpsc::{self, Sender, Receiver};
use std::thread;

use crate::ast::{Declaration, AstType, Expression, Statement, Program};
use crate::lexer::{Lexer, Token};
use crate::parser::Parser;
use crate::typechecker::TypeChecker;
use crate::compiler::Compiler;

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
    last_analysis: Option<Instant>,
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

#[derive(Debug)]
enum SymbolScope {
    Local { function_name: String },
    ModuleLevel,
    Unknown,
}

// Background analysis job
#[derive(Debug, Clone)]
struct AnalysisJob {
    uri: Url,
    version: i32,
    content: String,
    program: Program,
}

// Background analysis result
#[derive(Debug, Clone)]
struct AnalysisResult {
    uri: Url,
    version: i32,
    diagnostics: Vec<Diagnostic>,
}

// Convert CompileError to LSP Diagnostic (standalone function for reuse)
fn compile_error_to_diagnostic(error: crate::error::CompileError) -> Diagnostic {
    use crate::error::CompileError;

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

struct DocumentStore {
    documents: HashMap<Url, Document>,
    stdlib_symbols: HashMap<String, SymbolInfo>,
    workspace_symbols: HashMap<String, SymbolInfo>,  // Indexed workspace symbols
    workspace_root: Option<Url>,
    analysis_sender: Option<Sender<AnalysisJob>>,
}

impl DocumentStore {
    fn new() -> Self {
        let mut store = Self {
            documents: HashMap::new(),
            stdlib_symbols: HashMap::new(),
            workspace_symbols: HashMap::new(),
            workspace_root: None,
            analysis_sender: None,
        };

        // Index stdlib on initialization
        store.index_stdlib();
        store
    }

    fn set_analysis_sender(&mut self, sender: Sender<AnalysisJob>) {
        self.analysis_sender = Some(sender);
    }

    fn set_workspace_root(&mut self, root_uri: Url) {
        self.workspace_root = Some(root_uri.clone());
        // Index workspace symbols after setting root
        self.index_workspace(&root_uri);
    }

    fn index_workspace(&mut self, root_uri: &Url) {
        if let Ok(root_path) = root_uri.to_file_path() {
            eprintln!("[LSP] Indexing workspace: {}", root_path.display());
            let start = Instant::now();

            let count = self.index_workspace_directory(&root_path);

            let duration = start.elapsed();
            eprintln!("[LSP] Indexed {} symbols from workspace in {:?}", count, duration);
        }
    }

    fn index_workspace_directory(&mut self, path: &std::path::Path) -> usize {
        use std::fs;

        let mut symbol_count = 0;

        // Skip common directories we don't want to index
        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
            if dir_name == "target" || dir_name == "node_modules" || dir_name == ".git"
                || dir_name == "tests" || dir_name.starts_with('.') {
                return 0;
            }
        }

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();

                if entry_path.is_file() && entry_path.extension().map_or(false, |e| e == "zen") {
                    // Skip test files
                    if let Some(file_name) = entry_path.file_name().and_then(|n| n.to_str()) {
                        if file_name.starts_with("test_") || file_name.contains("_test.zen") {
                            continue;
                        }
                    }

                    if let Ok(content) = fs::read_to_string(&entry_path) {
                        let symbols = self.extract_symbols(&content);

                        // Convert path to URI for workspace symbols
                        if let Ok(uri) = Url::from_file_path(&entry_path) {
                            for (name, mut symbol) in symbols {
                                symbol.definition_uri = Some(uri.clone());
                                // Only add if not already in stdlib (stdlib takes priority)
                                if !self.stdlib_symbols.contains_key(&name) {
                                    self.workspace_symbols.insert(name, symbol);
                                    symbol_count += 1;
                                }
                            }
                        }
                    }
                } else if entry_path.is_dir() {
                    // Recursively index subdirectories
                    symbol_count += self.index_workspace_directory(&entry_path);
                }
            }
        }

        symbol_count
    }

    fn index_stdlib(&mut self) {
        // Find stdlib directory relative to the workspace
        let stdlib_paths = vec![
            std::path::PathBuf::from("./stdlib"),
            std::path::PathBuf::from("../stdlib"),
            std::path::PathBuf::from("../../stdlib"),
            std::path::PathBuf::from("/home/ubuntu/zenlang/stdlib"),
        ];

        for stdlib_path in stdlib_paths {
            if stdlib_path.exists() {
                self.index_stdlib_directory(&stdlib_path);
                eprintln!("[LSP] Indexed stdlib from: {}", stdlib_path.display());
                break;
            }
        }
    }

    fn index_stdlib_directory(&mut self, path: &std::path::Path) {
        use std::fs;

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();

                if entry_path.is_file() && entry_path.extension().map_or(false, |e| e == "zen") {
                    if let Ok(content) = fs::read_to_string(&entry_path) {
                        let symbols = self.extract_symbols(&content);

                        // Convert path to URI for stdlib symbols
                        if let Ok(uri) = Url::from_file_path(&entry_path) {
                            for (name, mut symbol) in symbols {
                                symbol.definition_uri = Some(uri.clone());
                                self.stdlib_symbols.insert(name, symbol);
                            }
                        }
                    }
                } else if entry_path.is_dir() {
                    // Recursively index subdirectories
                    self.index_stdlib_directory(&entry_path);
                }
            }
        }
    }

    fn open(&mut self, uri: Url, version: i32, content: String) -> Vec<Diagnostic> {
        let diagnostics = self.analyze_document(&content, false);

        let doc = Document {
            uri: uri.clone(),
            version,
            content: content.clone(),
            tokens: self.tokenize(&content),
            ast: self.parse(&content),
            diagnostics: diagnostics.clone(),
            symbols: self.extract_symbols(&content),
            last_analysis: Some(Instant::now()),
        };

        self.documents.insert(uri, doc);
        diagnostics
    }

    fn update(&mut self, uri: Url, version: i32, content: String) -> Vec<Diagnostic> {
        const DEBOUNCE_MS: u128 = 300;

        let should_run_analysis = self.documents
            .get(&uri)
            .and_then(|doc| doc.last_analysis)
            .map(|last| last.elapsed().as_millis() >= DEBOUNCE_MS)
            .unwrap_or(true);

        // Quick diagnostics from TypeChecker (always run for immediate feedback)
        let diagnostics = self.analyze_document(&content, !should_run_analysis);

        let tokens = self.tokenize(&content);
        let ast = self.parse(&content);
        let symbols = self.extract_symbols(&content);

        // Send to background thread for full analysis if enabled and debounced
        if should_run_analysis {
            if let Some(ast_decls) = &ast {
                if let Some(sender) = &self.analysis_sender {
                    let job = AnalysisJob {
                        uri: uri.clone(),
                        version,
                        content: content.clone(),
                        program: Program {
                            declarations: ast_decls.clone(),
                            statements: vec![],
                        },
                    };
                    // Send job to background thread (ignore if receiver dropped)
                    let _ = sender.send(job);
                }
            }
        }

        if let Some(doc) = self.documents.get_mut(&uri) {
            doc.version = version;
            doc.content = content.clone();
            doc.tokens = tokens;
            doc.ast = ast;
            doc.diagnostics = diagnostics.clone();
            doc.symbols = symbols;
            if should_run_analysis {
                doc.last_analysis = Some(Instant::now());
            }
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

    fn analyze_document(&self, content: &str, skip_expensive_analysis: bool) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let lexer = Lexer::new(content);
        let mut parser = Parser::new(lexer);

        match parser.parse_program() {
            Ok(program) => {
                if !skip_expensive_analysis {
                    diagnostics.extend(self.run_compiler_analysis(&program, content));

                    for decl in &program.declarations {
                        if let Declaration::Function(func) = decl {
                            self.check_allocator_usage(&func.body, &mut diagnostics, content);
                        }
                    }
                }
            }
            Err(err) => {
                diagnostics.push(self.error_to_diagnostic(err));
            }
        }

        diagnostics
    }

    fn run_compiler_analysis(&self, program: &Program, _content: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let mut type_checker = TypeChecker::new();

        if let Err(err) = type_checker.check_program(program) {
            diagnostics.push(self.error_to_diagnostic(err));
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
                    // TODO: Track variable initialization to avoid false positives
                    // For now, only warn if we can determine the object wasn't initialized with allocator
                    // This is too aggressive without flow analysis, so disable for now
                    let _should_warn = false;
                    if false { // Disabled until we have proper flow analysis
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
        compile_error_to_diagnostic(error)
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

    fn search_workspace_for_symbol(&self, symbol_name: &str) -> Option<(Url, SymbolInfo)> {
        use std::fs;
        use std::path::Path;

        let workspace_root = self.workspace_root.as_ref()?;
        let root_path = Path::new(workspace_root.path());

        self.search_directory_for_symbol(root_path, symbol_name)
    }

    fn search_directory_for_symbol(&self, dir: &std::path::Path, symbol_name: &str) -> Option<(Url, SymbolInfo)> {
        use std::fs;

        if !dir.is_dir() {
            return None;
        }

        let entries = fs::read_dir(dir).ok()?;

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_file() && path.extension().map_or(false, |e| e == "zen") {
                if let Ok(content) = fs::read_to_string(&path) {
                    let symbols = self.extract_symbols(&content);

                    if let Some(symbol_info) = symbols.get(symbol_name) {
                        if let Ok(uri) = Url::from_file_path(&path) {
                            let mut symbol = symbol_info.clone();
                            symbol.definition_uri = Some(uri.clone());
                            return Some((uri, symbol));
                        }
                    }
                }
            } else if path.is_dir() {
                let file_name = path.file_name()?.to_str()?;

                if file_name.starts_with('.') || file_name == "target" || file_name == "node_modules" {
                    continue;
                }

                if let Some(result) = self.search_directory_for_symbol(&path, symbol_name) {
                    return Some(result);
                }
            }
        }

        None
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

fn symbol_kind_to_completion_kind(kind: SymbolKind) -> CompletionItemKind {
    match kind {
        SymbolKind::FUNCTION | SymbolKind::METHOD => CompletionItemKind::FUNCTION,
        SymbolKind::STRUCT | SymbolKind::CLASS => CompletionItemKind::STRUCT,
        SymbolKind::ENUM => CompletionItemKind::ENUM,
        SymbolKind::ENUM_MEMBER => CompletionItemKind::ENUM_MEMBER,
        SymbolKind::VARIABLE => CompletionItemKind::VARIABLE,
        SymbolKind::CONSTANT => CompletionItemKind::CONSTANT,
        SymbolKind::FIELD | SymbolKind::PROPERTY => CompletionItemKind::FIELD,
        SymbolKind::INTERFACE => CompletionItemKind::INTERFACE,
        SymbolKind::MODULE | SymbolKind::NAMESPACE => CompletionItemKind::MODULE,
        SymbolKind::TYPE_PARAMETER => CompletionItemKind::TYPE_PARAMETER,
        SymbolKind::CONSTRUCTOR => CompletionItemKind::CONSTRUCTOR,
        SymbolKind::EVENT => CompletionItemKind::EVENT,
        SymbolKind::OPERATOR => CompletionItemKind::OPERATOR,
        _ => CompletionItemKind::TEXT,
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
        AstType::StaticLiteral => "str".to_string(),  // Internal string literal type
        AstType::StaticString => "StaticString".to_string(),
        AstType::String => "String".to_string(),
        AstType::Void => "void".to_string(),
        AstType::Ptr(inner) => format!("Ptr<{}>", format_type(inner)),
        AstType::MutPtr(inner) => format!("MutPtr<{}>", format_type(inner)),
        AstType::RawPtr(inner) => format!("RawPtr<{}>", format_type(inner)),
        AstType::Ref(inner) => format!("&{}", format_type(inner)),
        AstType::Range { start_type, end_type, inclusive } => {
            if *inclusive {
                format!("{}..={}", format_type(start_type), format_type(end_type))
            } else {
                format!("{}..{}", format_type(start_type), format_type(end_type))
            }
        }
        AstType::FunctionPointer { param_types, return_type } => {
            format!("fn({}) {}",
                param_types.iter().map(|p| format_type(p)).collect::<Vec<_>>().join(", "),
                format_type(return_type))
        }
        AstType::EnumType { name } => name.clone(),
        AstType::StdModule => "module".to_string(),
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
            if let Some(root_uri) = params.root_uri {
                // Setting workspace root
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
        use inkwell::context::Context;
        let context = Context::create();
        let compiler = Compiler::new(&context);

        while let Ok(job) = job_rx.recv() {
            // Analyzing document

            let start = Instant::now();
            let errors = compiler.analyze_for_diagnostics(&job.program);
            let duration = start.elapsed();

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
        use std::sync::mpsc::TryRecvError;

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

            if let Ok(msg) = self.connection.receiver.recv_timeout(timeout) {
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
            "textDocument/references" => self.handle_references(req.clone()),
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
                // Check if we're hovering over a pattern match variable
                if let Some(pattern_hover) = self.get_pattern_match_hover(&doc.content, position, &symbol_name, &doc.symbols, &store.stdlib_symbols, &store.documents) {
                    let contents = HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: pattern_hover,
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

                // Check if we're hovering over an enum variant definition
                if let Some(enum_hover) = self.get_enum_variant_hover(&doc.content, position, &symbol_name) {
                    let contents = HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: enum_hover,
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

                // Check stdlib symbols
                if let Some(symbol_info) = store.stdlib_symbols.get(&symbol_name) {
                    let mut hover_content = Vec::new();

                    if let Some(detail) = &symbol_info.detail {
                        hover_content.push(format!("```zen\n{}\n```", detail));
                    }

                    if let Some(doc) = &symbol_info.documentation {
                        hover_content.push(doc.clone());
                    }

                    if let Some(type_info) = &symbol_info.type_info {
                        hover_content.push(format!("**Type:** `{}`", format_type(type_info)));
                    }

                    hover_content.push("**Source:** Standard Library".to_string());

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

                // Check other open documents
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

                // Try to infer type from variable assignment in the current file
                if let Some(inferred_type) = self.infer_variable_type(&doc.content, &symbol_name, &doc.symbols, &store.stdlib_symbols, &store.workspace_symbols) {
                    let contents = HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: inferred_type,
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

                // Provide hover for built-in types and keywords
                let hover_text = match symbol_name.as_str() {
                    // Primitive integer types
                    "i8" => "```zen\ni8\n```\n\n**Signed 8-bit integer**\n- Range: -128 to 127\n- Size: 1 byte",
                    "i16" => "```zen\ni16\n```\n\n**Signed 16-bit integer**\n- Range: -32,768 to 32,767\n- Size: 2 bytes",
                    "i32" => "```zen\ni32\n```\n\n**Signed 32-bit integer**\n- Range: -2,147,483,648 to 2,147,483,647\n- Size: 4 bytes",
                    "i64" => "```zen\ni64\n```\n\n**Signed 64-bit integer**\n- Range: -9,223,372,036,854,775,808 to 9,223,372,036,854,775,807\n- Size: 8 bytes",
                    "u8" => "```zen\nu8\n```\n\n**Unsigned 8-bit integer**\n- Range: 0 to 255\n- Size: 1 byte",
                    "u16" => "```zen\nu16\n```\n\n**Unsigned 16-bit integer**\n- Range: 0 to 65,535\n- Size: 2 bytes",
                    "u32" => "```zen\nu32\n```\n\n**Unsigned 32-bit integer**\n- Range: 0 to 4,294,967,295\n- Size: 4 bytes",
                    "u64" => "```zen\nu64\n```\n\n**Unsigned 64-bit integer**\n- Range: 0 to 18,446,744,073,709,551,615\n- Size: 8 bytes",
                    "usize" => "```zen\nusize\n```\n\n**Pointer-sized unsigned integer**\n- Size: Platform dependent (4 or 8 bytes)\n- Used for array indexing and memory offsets",

                    // Floating point types
                    "f32" => "```zen\nf32\n```\n\n**32-bit floating point**\n- Precision: ~7 decimal digits\n- Size: 4 bytes\n- IEEE 754 single precision",
                    "f64" => "```zen\nf64\n```\n\n**64-bit floating point**\n- Precision: ~15 decimal digits\n- Size: 8 bytes\n- IEEE 754 double precision",

                    // Boolean
                    "bool" => "```zen\nbool\n```\n\n**Boolean type**\n- Values: `true` or `false`\n- Size: 1 byte",

                    // Void
                    "void" => "```zen\nvoid\n```\n\n**Void type**\n- Represents the absence of a value\n- Used as return type for functions with no return",

                    // Option and Result
                    "Option" => "```zen\nOption<T>:\n    Some: T,\n    None\n```\n\n**Optional value type**\n- Represents a value that may or may not exist\n- No null/nil in Zen!",
                    "Result" => "```zen\nResult<T, E>:\n    Ok: T,\n    Err: E\n```\n\n**Result type for error handling**\n- Represents success (Ok) or failure (Err)\n- Use `.raise()` for error propagation",

                    // Collections
                    "HashMap" => "```zen\nHashMap<K, V>\n```\n\n**Hash map collection**\n- Key-value storage with O(1) average lookup\n- Requires allocator",
                    "DynVec" => "```zen\nDynVec<T>\n```\n\n**Dynamic vector**\n- Growable array\n- Requires allocator",
                    "Vec" => "```zen\nVec<T, size>\n```\n\n**Fixed-size vector**\n- Stack-allocated\n- Compile-time size\n- No allocator needed",
                    "Array" => "```zen\nArray<T>\n```\n\n**Dynamic array**\n- Requires allocator",

                    // String types
                    "String" => "```zen\nString\n```\n\n**Dynamic string type**\n- Mutable, heap-allocated\n- Requires allocator",
                    "StaticString" => "```zen\nStaticString\n```\n\n**Static string type**\n- Immutable, compile-time\n- No allocator needed",

                    // Keywords
                    "loop" => "```zen\nloop() { ... }\nloop((handle) { ... })\n(range).loop((i) { ... })\n```\n\n**Loop construct**\n- Internal state management\n- Can provide control handle or iteration values",
                    "return" => "```zen\nreturn expr\n```\n\n**Return statement**\n- Returns a value from a function",
                    "break" => "```zen\nbreak\n```\n\n**Break statement**\n- Exits the current loop",
                    "continue" => "```zen\ncontinue\n```\n\n**Continue statement**\n- Skips to the next loop iteration",

                    // Error handling
                    "raise" => "```zen\nexpr.raise()\n```\n\n**Error propagation**\n- Unwraps Result<T, E> or returns Err early\n- Equivalent to Rust's `?` operator",

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

    fn find_function_line(&self, content: &str, func_name: &str) -> Option<usize> {
        let lines: Vec<&str> = content.lines().collect();
        for (line_num, line) in lines.iter().enumerate() {
            // Look for function definition: "func_name = "
            if line.contains(func_name) && line.contains("=") && line.contains("(") {
                // Verify this is a function definition, not just usage
                if let Some(eq_pos) = line.find('=') {
                    if let Some(name_start) = line.find(func_name) {
                        // Check if function name comes before '=' and there's '(' after
                        if name_start < eq_pos && line[eq_pos..].contains('(') {
                            return Some(line_num);
                        }
                    }
                }
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
                eprintln!("[LSP] Find references for symbol: '{}'", symbol_name);

                // Determine the scope of the symbol
                let symbol_scope = self.determine_symbol_scope(&doc, &symbol_name, position);
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
        let params: DocumentFormattingParams = match serde_json::from_value(req.params) {
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

        // Format the document
        let formatted = self.format_document(&doc.content);

        // Create a single edit that replaces the entire document
        let line_count = doc.content.lines().count();
        let last_line_len = doc.content.lines().last().map(|l| l.len()).unwrap_or(0);

        let edit = TextEdit {
            range: Range {
                start: Position::new(0, 0),
                end: Position::new(line_count as u32, last_line_len as u32),
            },
            new_text: formatted,
        };

        Response {
            id: req.id,
            result: Some(serde_json::to_value(vec![edit]).unwrap_or(Value::Null)),
            error: None,
        }
    }

    fn format_document(&self, content: &str) -> String {
        let mut formatted = String::new();
        let mut indent_level: usize = 0;
        let indent_str = "    "; // 4 spaces

        for line in content.lines() {
            let trimmed = line.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                formatted.push('\n');
                continue;
            }

            // Decrease indent for closing braces
            if trimmed.starts_with('}') || trimmed.starts_with(']') {
                indent_level = indent_level.saturating_sub(1);
            }

            // Add indentation
            for _ in 0..indent_level {
                formatted.push_str(indent_str);
            }

            formatted.push_str(trimmed);
            formatted.push('\n');

            // Increase indent after opening braces
            if trimmed.ends_with('{') || trimmed.ends_with('[') {
                indent_level += 1;
            }
        }

        formatted
    }

    fn handle_rename(&self, req: Request) -> Response {
        let params: RenameParams = match serde_json::from_value(req.params) {
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
        let new_name = params.new_name;
        let uri = &params.text_document_position.text_document.uri;

        if let Some(doc) = store.documents.get(uri) {
            let position = params.text_document_position.position;

            if let Some(symbol_name) = self.find_symbol_at_position(&doc.content, position) {
                eprintln!("[LSP] Rename: symbol='{}' -> '{}' at {}:{}",
                    symbol_name, new_name, position.line, position.character);

                // Determine the scope of the symbol
                let symbol_scope = self.determine_symbol_scope(&doc, &symbol_name, position);

                eprintln!("[LSP] Symbol scope: {:?}", symbol_scope);

                let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();

                match symbol_scope {
                    SymbolScope::Local { function_name } => {
                        // Local variable or parameter - only rename in current file, within function
                        eprintln!("[LSP] Renaming local symbol in function '{}'", function_name);

                        if let Some(edits) = self.rename_local_symbol(
                            &doc.content,
                            &symbol_name,
                            &new_name,
                            &function_name
                        ) {
                            if !edits.is_empty() {
                                changes.insert(uri.clone(), edits);
                            }
                        }
                    }
                    SymbolScope::ModuleLevel => {
                        // Module-level symbol (function, struct, enum) - rename across workspace
                        eprintln!("[LSP] Renaming module-level symbol across workspace");

                        // Find all workspace files that might reference this symbol
                        let workspace_files = self.collect_workspace_files(&store);

                        eprintln!("[LSP] Scanning {} workspace files for references", workspace_files.len());

                        for (file_uri, file_content) in workspace_files {
                            if let Some(edits) = self.rename_in_file(&file_content, &symbol_name, &new_name) {
                                if !edits.is_empty() {
                                    eprintln!("[LSP] Found {} occurrences in {}", edits.len(), file_uri.path());
                                    changes.insert(file_uri, edits);
                                }
                            }
                        }
                    }
                    SymbolScope::Unknown => {
                        // Fallback: only rename in current file
                        eprintln!("[LSP] Unknown scope, renaming only in current file");

                        if let Some(edits) = self.rename_in_file(&doc.content, &symbol_name, &new_name) {
                            if !edits.is_empty() {
                                changes.insert(uri.clone(), edits);
                            }
                        }
                    }
                }

                eprintln!("[LSP] Rename will affect {} files with {} total edits",
                    changes.len(),
                    changes.values().map(|v| v.len()).sum::<usize>());

                let workspace_edit = WorkspaceEdit {
                    changes: Some(changes),
                    document_changes: None,
                    change_annotations: None,
                };

                return Response {
                    id: req.id,
                    result: Some(serde_json::to_value(workspace_edit).unwrap_or(Value::Null)),
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

    fn handle_signature_help(&self, req: Request) -> Response {
        let params: SignatureHelpParams = match serde_json::from_value(req.params) {
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
        let doc = match store.documents.get(&params.text_document_position_params.text_document.uri) {
            Some(d) => d,
            None => {
                return Response {
                    id: req.id,
                    result: Some(Value::Null),
                    error: None,
                };
            }
        };

        // Find function call at cursor position
        let position = params.text_document_position_params.position;
        let function_call = self.find_function_call_at_position(&doc.content, position);

        let signature_help = match function_call {
            Some((function_name, active_param)) => {
                // Look up function in symbols (document, stdlib, workspace)
                let mut signature_info = None;

                // Check document symbols first (highest priority)
                if let Some(symbol) = doc.symbols.get(&function_name) {
                    signature_info = Some(self.create_signature_info(symbol));
                }

                // Check stdlib symbols if not found
                if signature_info.is_none() {
                    if let Some(symbol) = store.stdlib_symbols.get(&function_name) {
                        signature_info = Some(self.create_signature_info(symbol));
                    }
                }

                // Check workspace symbols if not found
                if signature_info.is_none() {
                    if let Some(symbol) = store.workspace_symbols.get(&function_name) {
                        signature_info = Some(self.create_signature_info(symbol));
                    }
                }

                match signature_info {
                    Some(sig_info) => SignatureHelp {
                        signatures: vec![sig_info],
                        active_signature: Some(0),
                        active_parameter: Some(active_param as u32),
                    },
                    None => SignatureHelp {
                        signatures: vec![],
                        active_signature: None,
                        active_parameter: None,
                    },
                }
            }
            None => SignatureHelp {
                signatures: vec![],
                active_signature: None,
                active_parameter: None,
            },
        };

        Response {
            id: req.id,
            result: serde_json::to_value(signature_help).ok(),
            error: None,
        }
    }

    fn handle_inlay_hints(&self, req: Request) -> Response {
        let params: InlayHintParams = match serde_json::from_value(req.params) {
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
        let doc = match store.documents.get(&params.text_document.uri) {
            Some(d) => d,
            None => {
                return Response {
                    id: req.id,
                    result: Some(Value::Null),
                    error: None,
                };
            }
        };

        let mut hints = Vec::new();

        // Parse the AST and extract variable declarations with position tracking
        if let Some(ast) = &doc.ast {
            for decl in ast {
                if let Declaration::Function(func) = decl {
                    self.collect_hints_from_statements(&func.body, &doc.content, doc, &mut hints);
                }
            }
        }

        Response {
            id: req.id,
            result: serde_json::to_value(hints).ok(),
            error: None,
        }
    }

    fn handle_code_lens(&self, req: Request) -> Response {
        let params: CodeLensParams = match serde_json::from_value(req.params) {
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
        let doc = match store.documents.get(&params.text_document.uri) {
            Some(d) => d,
            None => {
                return Response {
                    id: req.id,
                    result: Some(Value::Null),
                    error: None,
                };
            }
        };

        let mut lenses = Vec::new();

        // Find test functions and add "Run Test" code lens
        if let Some(ast) = &doc.ast {
            for (idx, decl) in ast.iter().enumerate() {
                if let Declaration::Function(func) = decl {
                    let func_name = &func.name;

                    // Check if this is a test function (starts with test_ or ends with _test)
                    if func_name.starts_with("test_") || func_name.ends_with("_test") || func_name.contains("_test_") {
                        // Find the line number of this function
                        let line_num = self.find_function_line(&doc.content, func_name);

                        if let Some(line) = line_num {
                            lenses.push(CodeLens {
                                range: Range {
                                    start: Position {
                                        line: line as u32,
                                        character: 0,
                                    },
                                    end: Position {
                                        line: line as u32,
                                        character: 0,
                                    },
                                },
                                command: Some(Command {
                                    title: "▶ Run Test".to_string(),
                                    command: "zen.runTest".to_string(),
                                    arguments: Some(vec![
                                        serde_json::to_value(&params.text_document.uri).unwrap(),
                                        serde_json::to_value(func_name).unwrap(),
                                    ]),
                                }),
                                data: None,
                            });
                        }
                    }
                }
            }
        }

        Response {
            id: req.id,
            result: serde_json::to_value(lenses).ok(),
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

            // Add refactoring code actions (not tied to diagnostics)
            // Extract variable - only if there's a selection
            if params.range.start != params.range.end {
                if let Some(action) = self.create_extract_variable_action(&params.range, &params.text_document.uri, &doc.content) {
                    actions.push(action);
                }

                // Extract function - only if there's a multi-line selection or complex expression
                if let Some(action) = self.create_extract_function_action(&params.range, &params.text_document.uri, &doc.content) {
                    actions.push(action);
                }
            }

            // Add imports for common types if they're missing
            if let Some(action) = self.create_add_import_action(&params.range, &doc.content) {
                actions.push(action);
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

    fn find_function_call_at_position(&self, content: &str, position: Position) -> Option<(String, usize)> {
        let lines: Vec<&str> = content.lines().collect();
        if position.line as usize >= lines.len() {
            return None;
        }

        let line = lines[position.line as usize];
        let cursor_pos = position.character as usize;

        // Find the function call - look backwards from cursor for opening paren
        let mut paren_count = 0;
        let mut current_pos = cursor_pos.min(line.len());

        // Move to the nearest opening paren
        while current_pos > 0 {
            let ch = line.chars().nth(current_pos - 1)?;
            if ch == ')' {
                paren_count += 1;
            } else if ch == '(' {
                if paren_count == 0 {
                    break;
                }
                paren_count -= 1;
            }
            current_pos -= 1;
        }

        if current_pos == 0 {
            return None; // No opening paren found
        }

        // Extract function name before the opening paren
        let before_paren = &line[..current_pos - 1];
        let function_name = before_paren
            .split(|c: char| c.is_whitespace() || c == '=' || c == ',' || c == ';')
            .last()?
            .trim()
            .split('.')
            .last()?
            .to_string();

        // Count parameters by counting commas at paren_depth = 0
        let inside_parens = &line[current_pos..cursor_pos.min(line.len())];
        let mut active_param = 0;
        let mut depth = 0;

        for ch in inside_parens.chars() {
            match ch {
                '(' => depth += 1,
                ')' => depth -= 1,
                ',' if depth == 0 => active_param += 1,
                _ => {}
            }
        }

        Some((function_name, active_param))
    }

    fn create_signature_info(&self, symbol: &SymbolInfo) -> SignatureInformation {
        // Extract function signature from symbol detail
        let label = symbol.detail.clone().unwrap_or_else(|| {
            format!("{}(...)", symbol.name)
        });

        // Parse parameters from the function signature
        let parameters = self.parse_function_parameters(&label);

        SignatureInformation {
            label,
            documentation: symbol.documentation.as_ref().map(|doc| {
                Documentation::String(doc.clone())
            }),
            parameters: if parameters.is_empty() {
                None
            } else {
                Some(parameters)
            },
            active_parameter: None,
        }
    }

    fn parse_function_parameters(&self, signature: &str) -> Vec<ParameterInformation> {
        // Parse signature like "function_name = (param1: Type1, param2: Type2) ReturnType"
        let mut parameters = Vec::new();

        // Find the parameter section between ( and )
        if let Some(start) = signature.find('(') {
            if let Some(end) = signature[start..].find(')') {
                let params_str = &signature[start + 1..start + end];

                // Split by commas (simple for now, could be enhanced for nested types)
                for param in params_str.split(',') {
                    let param = param.trim();
                    if !param.is_empty() {
                        parameters.push(ParameterInformation {
                            label: lsp_types::ParameterLabel::Simple(param.to_string()),
                            documentation: None,
                        });
                    }
                }
            }
        }

        parameters
    }

    fn collect_hints_from_statements(&self, statements: &[Statement], content: &str, doc: &Document, hints: &mut Vec<InlayHint>) {
        use crate::ast::Statement;

        for stmt in statements {
            match stmt {
                Statement::VariableDeclaration { name, type_, initializer, .. } => {
                    // Only add hints for variables without explicit type annotations
                    if type_.is_none() {
                        if let Some(init) = initializer {
                            if let Some(inferred_type) = self.infer_expression_type(init, doc) {
                                // Find the position of this variable declaration in the source
                                if let Some(position) = self.find_variable_position(content, name) {
                                    hints.push(InlayHint {
                                        position,
                                        label: InlayHintLabel::String(format!(": {}", inferred_type)),
                                        kind: Some(InlayHintKind::TYPE),
                                        text_edits: None,
                                        tooltip: None,
                                        padding_left: None,
                                        padding_right: None,
                                        data: None,
                                    });
                                }
                            }

                            // Also collect parameter hints from function calls in initializer
                            self.collect_param_hints_from_expression(init, content, doc, hints);
                        }
                    }
                }
                Statement::Expression(expr) => {
                    // Collect parameter hints from standalone expressions
                    self.collect_param_hints_from_expression(expr, content, doc, hints);
                }
                Statement::Return(expr) => {
                    // Collect parameter hints from return expressions
                    self.collect_param_hints_from_expression(expr, content, doc, hints);
                }
                Statement::Loop { body, .. } => {
                    self.collect_hints_from_statements(body, content, doc, hints);
                }
                _ => {}
            }
        }
    }

    fn find_variable_position(&self, content: &str, var_name: &str) -> Option<Position> {
        let lines: Vec<&str> = content.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            // Look for Zen variable declaration patterns:
            // "var_name = " or "var_name := " or "var_name ::= " or "var_name : Type ="

            // First, check if line contains the variable name
            if let Some(name_pos) = line.find(var_name) {
                // Check if this is actually a variable declaration
                // Must be at start of line or after whitespace
                let before_ok = name_pos == 0 ||
                    line.chars().nth(name_pos - 1).map_or(true, |c| c.is_whitespace());

                if before_ok {
                    // Check what comes after the variable name
                    let after_name = &line[name_pos + var_name.len()..].trim_start();

                    // Check for Zen assignment patterns
                    if after_name.starts_with("=") ||
                       after_name.starts_with(":=") ||
                       after_name.starts_with("::=") ||
                       after_name.starts_with(":") {
                        // Position after the variable name (where type hint should go)
                        let char_pos = name_pos + var_name.len();
                        return Some(Position {
                            line: line_num as u32,
                            character: char_pos as u32,
                        });
                    }
                }
            }
        }

        None
    }

    fn infer_expression_type(&self, expr: &Expression, doc: &Document) -> Option<String> {
        use crate::ast::Expression;

        match expr {
            Expression::Integer32(_) => Some("i32".to_string()),
            Expression::Integer64(_) => Some("i64".to_string()),
            Expression::Float32(_) => Some("f32".to_string()),
            Expression::Float64(_) => Some("f64".to_string()),
            Expression::String(_) => Some("StaticString".to_string()),
            Expression::Boolean(_) => Some("bool".to_string()),
            Expression::BinaryOp { left, right, .. } => {
                // Simple type inference for binary operations
                let left_type = self.infer_expression_type(left, doc)?;
                let right_type = self.infer_expression_type(right, doc)?;

                if left_type == "f64" || right_type == "f64" {
                    Some("f64".to_string())
                } else if left_type == "i64" || right_type == "i64" {
                    Some("i64".to_string())
                } else {
                    Some("i32".to_string())
                }
            }
            Expression::FunctionCall { name, .. } => {
                // Look up function return type from document symbols
                if let Some(symbol) = doc.symbols.get(name) {
                    // Extract return type from function signature like "add = (a: i32, b: i32) i32"
                    if let Some(detail) = &symbol.detail {
                        return self.extract_return_type_from_signature(detail);
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn extract_return_type_from_signature(&self, signature: &str) -> Option<String> {
        // Parse signature like "function_name = (params) ReturnType"
        // Find the closing paren, then take everything after it
        if let Some(close_paren) = signature.rfind(')') {
            let return_type = signature[close_paren + 1..].trim();
            if !return_type.is_empty() {
                return Some(return_type.to_string());
            }
        }
        None
    }

    fn collect_param_hints_from_expression(&self, expr: &Expression, content: &str, doc: &Document, hints: &mut Vec<InlayHint>) {
        use crate::ast::Expression;

        match expr {
            Expression::FunctionCall { name, args } => {
                // Look up function signature to get parameter names
                if let Some(param_names) = self.get_function_param_names(name, doc) {
                    // Add parameter name hints for each argument (if we have enough names)
                    for (idx, arg) in args.iter().enumerate() {
                        if let Some(param_name) = param_names.get(idx) {
                            // Find the position of this argument in the source
                            if let Some(arg_pos) = self.find_function_arg_position(content, name, idx) {
                                hints.push(InlayHint {
                                    position: arg_pos,
                                    label: InlayHintLabel::String(format!("{}: ", param_name)),
                                    kind: Some(InlayHintKind::PARAMETER),
                                    text_edits: None,
                                    tooltip: None,
                                    padding_left: None,
                                    padding_right: Some(true),
                                    data: None,
                                });
                            }
                        }

                        // Recursively collect from nested function calls
                        self.collect_param_hints_from_expression(arg, content, doc, hints);
                    }
                }
            }
            Expression::BinaryOp { left, right, .. } => {
                self.collect_param_hints_from_expression(left, content, doc, hints);
                self.collect_param_hints_from_expression(right, content, doc, hints);
            }
            Expression::QuestionMatch { scrutinee, arms } => {
                self.collect_param_hints_from_expression(scrutinee, content, doc, hints);
                for arm in arms {
                    self.collect_param_hints_from_expression(&arm.body, content, doc, hints);
                }
            }
            Expression::StructLiteral { fields, .. } => {
                for (_, field_expr) in fields {
                    self.collect_param_hints_from_expression(field_expr, content, doc, hints);
                }
            }
            Expression::ArrayLiteral(elements) => {
                for elem in elements {
                    self.collect_param_hints_from_expression(elem, content, doc, hints);
                }
            }
            _ => {}
        }
    }

    fn get_function_param_names(&self, func_name: &str, doc: &Document) -> Option<Vec<String>> {
        // First, try to find function in the document's AST
        if let Some(ast) = &doc.ast {
            for decl in ast {
                if let Declaration::Function(func) = decl {
                    if &func.name == func_name {
                        return Some(func.args.iter().map(|(name, _)| name.clone()).collect());
                    }
                }
            }
        }

        // Check stdlib symbols
        let store = self.store.lock().unwrap();
        if let Some(symbol) = store.stdlib_symbols.get(func_name) {
            if let Some(detail) = &symbol.detail {
                return self.extract_param_names_from_signature(detail);
            }
        }

        // Check workspace symbols
        if let Some(symbol) = store.workspace_symbols.get(func_name) {
            if let Some(detail) = &symbol.detail {
                return self.extract_param_names_from_signature(detail);
            }
        }

        None
    }

    fn extract_param_names_from_signature(&self, signature: &str) -> Option<Vec<String>> {
        // Parse signature like "function_name = (a: i32, b: i32) ReturnType"
        let open_paren = signature.find('(')?;
        let close_paren = signature.find(')')?;

        let params_str = &signature[open_paren + 1..close_paren];
        if params_str.trim().is_empty() {
            return Some(vec![]);
        }

        let param_names: Vec<String> = params_str
            .split(',')
            .filter_map(|param| {
                // Each param is like "name: Type"
                let parts: Vec<&str> = param.trim().split(':').collect();
                if !parts.is_empty() {
                    Some(parts[0].trim().to_string())
                } else {
                    None
                }
            })
            .collect();

        Some(param_names)
    }

    fn find_function_arg_position(&self, content: &str, func_name: &str, arg_index: usize) -> Option<Position> {
        let lines: Vec<&str> = content.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            // Look for function call pattern: "func_name("
            if let Some(func_pos) = line.find(func_name) {
                // Check if this is actually a function call (followed by '(')
                let after_func = &line[func_pos + func_name.len()..].trim_start();
                if after_func.starts_with('(') {
                    // Find the opening paren position
                    let paren_pos = func_pos + func_name.len() + (line[func_pos + func_name.len()..].find('(').unwrap_or(0));

                    // Find the nth argument by counting commas
                    let mut current_arg = 0;
                    let mut depth = 0;
                    let mut i = paren_pos + 1;

                    while i < line.len() {
                        let c = line.chars().nth(i).unwrap_or('\0');

                        if c == '(' {
                            depth += 1;
                        } else if c == ')' {
                            if depth == 0 {
                                break;
                            }
                            depth -= 1;
                        } else if c == ',' && depth == 0 {
                            current_arg += 1;
                        }

                        if current_arg == arg_index && !c.is_whitespace() && c != '(' {
                            return Some(Position {
                                line: line_num as u32,
                                character: i as u32,
                            });
                        }

                        i += 1;
                    }

                    // If we're looking for arg 0, return position right after opening paren
                    if arg_index == 0 && current_arg == 0 {
                        let mut j = paren_pos + 1;
                        while j < line.len() {
                            let c = line.chars().nth(j).unwrap_or('\0');
                            if !c.is_whitespace() {
                                return Some(Position {
                                    line: line_num as u32,
                                    character: j as u32,
                                });
                            }
                            j += 1;
                        }
                    }
                }
            }
        }

        None
    }

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
        let params: CallHierarchyPrepareParams = match serde_json::from_value(req.params) {
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
        let doc = match store.documents.get(&params.text_document_position_params.text_document.uri) {
            Some(doc) => doc,
            None => {
                return Response {
                    id: req.id,
                    result: Some(Value::Null),
                    error: None,
                }
            }
        };

        // Find function at position
        let position = params.text_document_position_params.position;
        if let Some(symbol_name) = self.find_symbol_at_position(&doc.content, position) {
            if let Some(symbol_info) = doc.symbols.get(&symbol_name) {
                if symbol_info.kind == SymbolKind::FUNCTION || symbol_info.kind == SymbolKind::METHOD {
                    let call_item = CallHierarchyItem {
                        name: symbol_info.name.clone(),
                        kind: symbol_info.kind,
                        tags: None,
                        detail: symbol_info.detail.clone(),
                        uri: params.text_document_position_params.text_document.uri.clone(),
                        range: symbol_info.range,
                        selection_range: symbol_info.selection_range,
                        data: None,
                    };

                    return Response {
                        id: req.id,
                        result: Some(serde_json::to_value(vec![call_item]).unwrap_or(Value::Null)),
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

    fn handle_incoming_calls(&self, req: Request) -> Response {
        let params: CallHierarchyIncomingCallsParams = match serde_json::from_value(req.params) {
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
        let mut incoming_calls = Vec::new();

        // Find all call sites of this function across all documents
        let target_func_name = &params.item.name;

        for (uri, doc) in &store.documents {
            // Search for function calls in the AST
            if let Some(ast) = &doc.ast {
                for decl in ast {
                    if let Declaration::Function(func) = decl {
                        // Check if this function calls the target function
                        let calls_target = self.function_calls_target(&func.body, target_func_name);
                        if calls_target {
                            // Find the function symbol
                            if let Some(caller_info) = doc.symbols.get(&func.name) {
                                let caller_item = CallHierarchyItem {
                                    name: caller_info.name.clone(),
                                    kind: SymbolKind::FUNCTION,
                                    tags: None,
                                    detail: caller_info.detail.clone(),
                                    uri: uri.clone(),
                                    range: caller_info.range,
                                    selection_range: caller_info.selection_range,
                                    data: None,
                                };

                                // Find all call ranges (simplified - using caller function range)
                                incoming_calls.push(CallHierarchyIncomingCall {
                                    from: caller_item,
                                    from_ranges: vec![caller_info.range],
                                });
                            }
                        }
                    }
                }
            }
        }

        Response {
            id: req.id,
            result: Some(serde_json::to_value(incoming_calls).unwrap_or(Value::Null)),
            error: None,
        }
    }

    fn handle_outgoing_calls(&self, req: Request) -> Response {
        let params: CallHierarchyOutgoingCallsParams = match serde_json::from_value(req.params) {
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
        let mut outgoing_calls = Vec::new();

        // Find the function in the document
        let uri = &params.item.uri;
        if let Some(doc) = store.documents.get(uri) {
            if let Some(ast) = &doc.ast {
                for decl in ast {
                    if let Declaration::Function(func) = decl {
                        if &func.name == &params.item.name {
                            // Find all function calls in this function
                            let called_functions = self.find_called_functions(&func.body);

                            for called_func in called_functions {
                                // Try to find the called function in symbols
                                if let Some(callee_info) = doc.symbols.get(&called_func)
                                    .or_else(|| store.stdlib_symbols.get(&called_func)) {

                                    let callee_uri = callee_info.definition_uri.clone()
                                        .unwrap_or_else(|| uri.clone());

                                    let callee_item = CallHierarchyItem {
                                        name: callee_info.name.clone(),
                                        kind: SymbolKind::FUNCTION,
                                        tags: None,
                                        detail: callee_info.detail.clone(),
                                        uri: callee_uri,
                                        range: callee_info.range,
                                        selection_range: callee_info.selection_range,
                                        data: None,
                                    };

                                    outgoing_calls.push(CallHierarchyOutgoingCall {
                                        to: callee_item,
                                        from_ranges: vec![params.item.range],
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Response {
            id: req.id,
            result: Some(serde_json::to_value(outgoing_calls).unwrap_or(Value::Null)),
            error: None,
        }
    }

    // Helper: Check if a function body calls a target function
    fn function_calls_target(&self, statements: &[Statement], target: &str) -> bool {
        for stmt in statements {
            match stmt {
                Statement::Expression(expr) => {
                    if self.expression_calls_function(expr, target) {
                        return true;
                    }
                }
                Statement::Return(expr) => {
                    if self.expression_calls_function(expr, target) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    // Helper: Check if an expression calls a function
    fn expression_calls_function(&self, expr: &Expression, target: &str) -> bool {
        match expr {
            Expression::FunctionCall { name, args } => {
                if name == target {
                    return true;
                }
                // Check arguments recursively
                for arg in args {
                    if self.expression_calls_function(arg, target) {
                        return true;
                    }
                }
                false
            }
            Expression::MethodCall { object, method: _, args } => {
                if self.expression_calls_function(object, target) {
                    return true;
                }
                for arg in args {
                    if self.expression_calls_function(arg, target) {
                        return true;
                    }
                }
                false
            }
            Expression::BinaryOp { left, right, .. } => {
                self.expression_calls_function(left, target) ||
                self.expression_calls_function(right, target)
            }
            _ => false,
        }
    }

    // Helper: Find all functions called in a function body
    fn find_called_functions(&self, statements: &[Statement]) -> Vec<String> {
        let mut functions = Vec::new();
        for stmt in statements {
            match stmt {
                Statement::Expression(expr) => {
                    self.collect_called_functions(expr, &mut functions);
                }
                Statement::Return(expr) => {
                    self.collect_called_functions(expr, &mut functions);
                }
                _ => {}
            }
        }
        functions
    }

    // Helper: Collect function names from an expression
    fn collect_called_functions(&self, expr: &Expression, functions: &mut Vec<String>) {
        match expr {
            Expression::FunctionCall { name, args } => {
                functions.push(name.clone());
                for arg in args {
                    self.collect_called_functions(arg, functions);
                }
            }
            Expression::MethodCall { object, method: _, args } => {
                self.collect_called_functions(object, functions);
                for arg in args {
                    self.collect_called_functions(arg, functions);
                }
            }
            Expression::BinaryOp { left, right, .. } => {
                self.collect_called_functions(left, functions);
                self.collect_called_functions(right, functions);
            }
            _ => {}
        }
    }

    fn create_extract_variable_action(&self, range: &Range, uri: &Url, content: &str) -> Option<CodeAction> {
        // Extract the selected text
        let lines: Vec<&str> = content.lines().collect();
        let mut selected_text = String::new();

        if range.start.line == range.end.line {
            // Single line selection
            if let Some(line) = lines.get(range.start.line as usize) {
                let start_char = range.start.character as usize;
                let end_char = range.end.character as usize;
                if start_char < line.len() && end_char <= line.len() {
                    selected_text = line[start_char..end_char].to_string();
                }
            }
        } else {
            // Multi-line selection
            for line_idx in range.start.line..=range.end.line {
                if let Some(line) = lines.get(line_idx as usize) {
                    if line_idx == range.start.line {
                        selected_text.push_str(&line[range.start.character as usize..]);
                    } else if line_idx == range.end.line {
                        selected_text.push_str(&line[..range.end.character as usize]);
                    } else {
                        selected_text.push_str(line);
                    }
                    if line_idx < range.end.line {
                        selected_text.push('\n');
                    }
                }
            }
        }

        // Skip if selection is empty or just whitespace
        if selected_text.trim().is_empty() {
            return None;
        }

        // Skip if selection looks like a variable name already (simple heuristic)
        if selected_text.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return None;
        }

        // Generate variable name based on selection
        let var_name = self.generate_variable_name(&selected_text);

        // Find the beginning of the current statement to insert the variable declaration
        let insert_line = range.start.line;
        let indent = if let Some(line) = lines.get(insert_line as usize) {
            line.chars().take_while(|c| c.is_whitespace()).collect::<String>()
        } else {
            "    ".to_string()
        };

        // Create two edits:
        // 1. Insert variable declaration before the current line
        // 2. Replace selected expression with variable name
        let declaration = format!("{}{} = {};\n", indent, var_name, selected_text.trim());

        let mut changes = Vec::new();

        // Insert variable declaration
        changes.push(TextEdit {
            range: Range {
                start: Position {
                    line: insert_line,
                    character: 0,
                },
                end: Position {
                    line: insert_line,
                    character: 0,
                },
            },
            new_text: declaration,
        });

        // Replace selected expression with variable name
        changes.push(TextEdit {
            range: range.clone(),
            new_text: var_name.clone(),
        });

        let workspace_edit = WorkspaceEdit {
            changes: Some({
                let mut change_map = HashMap::new();
                change_map.insert(uri.clone(), changes);
                change_map
            }),
            document_changes: None,
            change_annotations: None,
        };

        Some(CodeAction {
            title: format!("Extract to variable '{}'", var_name),
            kind: Some(CodeActionKind::REFACTOR_EXTRACT),
            diagnostics: None,
            edit: Some(workspace_edit),
            command: None,
            is_preferred: Some(false),
            disabled: None,
            data: None,
        })
    }

    fn create_extract_function_action(&self, range: &Range, uri: &Url, content: &str) -> Option<CodeAction> {
        // Extract the selected text
        let lines: Vec<&str> = content.lines().collect();
        let mut selected_text = String::new();

        if range.start.line == range.end.line {
            // Single line selection - only extract if it's a substantial expression
            if let Some(line) = lines.get(range.start.line as usize) {
                let start_char = range.start.character as usize;
                let end_char = range.end.character as usize;
                if start_char < line.len() && end_char <= line.len() {
                    selected_text = line[start_char..end_char].to_string();
                }
            }
            // For single-line, only suggest if it's a complex expression (contains operators or calls)
            if !selected_text.contains('(') && !selected_text.contains('+') &&
               !selected_text.contains('-') && !selected_text.contains('*') {
                return None;
            }
        } else {
            // Multi-line selection
            for line_idx in range.start.line..=range.end.line {
                if let Some(line) = lines.get(line_idx as usize) {
                    if line_idx == range.start.line {
                        selected_text.push_str(&line[range.start.character as usize..]);
                    } else if line_idx == range.end.line {
                        selected_text.push_str(&line[..range.end.character as usize]);
                    } else {
                        selected_text.push_str(line);
                    }
                    if line_idx < range.end.line {
                        selected_text.push('\n');
                    }
                }
            }
        }

        // Skip if selection is empty or just whitespace
        if selected_text.trim().is_empty() {
            return None;
        }

        // Generate function name based on selected code
        let func_name = self.generate_function_name(&selected_text);

        // Find appropriate indentation
        let base_indent = if let Some(line) = lines.get(range.start.line as usize) {
            line.chars().take_while(|c| c.is_whitespace()).collect::<String>()
        } else {
            "".to_string()
        };

        // Create function with proper Zen formatting (name = () type { body })
        let func_body_indent = format!("{}    ", base_indent);
        let formatted_body: Vec<String> = selected_text
            .lines()
            .map(|line| {
                if line.trim().is_empty() {
                    String::new()
                } else {
                    format!("{}{}", func_body_indent, line.trim())
                }
            })
            .collect();

        // Detect if code has a return statement to infer return type
        let return_type = if selected_text.contains("return ") {
            "void"  // Placeholder - could be smarter with AST analysis
        } else {
            "void"
        };

        let new_function = format!(
            "{}{} = () {} {{\n{}\n{}}}\n\n",
            base_indent,
            func_name,
            return_type,
            formatted_body.join("\n"),
            base_indent
        );

        // Find where to insert the new function (before the current function)
        let insert_line = self.find_function_start(content, range.start.line);

        let mut changes = Vec::new();

        // Insert new function
        changes.push(TextEdit {
            range: Range {
                start: Position {
                    line: insert_line,
                    character: 0,
                },
                end: Position {
                    line: insert_line,
                    character: 0,
                },
            },
            new_text: new_function,
        });

        // Replace selected code with function call
        changes.push(TextEdit {
            range: range.clone(),
            new_text: format!("{}()", func_name),
        });

        let workspace_edit = WorkspaceEdit {
            changes: Some({
                let mut change_map = HashMap::new();
                change_map.insert(uri.clone(), changes);
                change_map
            }),
            document_changes: None,
            change_annotations: None,
        };

        Some(CodeAction {
            title: format!("Extract to function '{}'", func_name),
            kind: Some(CodeActionKind::REFACTOR_EXTRACT),
            diagnostics: None,
            edit: Some(workspace_edit),
            command: None,
            is_preferred: Some(false),
            disabled: None,
            data: None,
        })
    }

    fn find_function_start(&self, content: &str, from_line: u32) -> u32 {
        // Find the start of the enclosing function by looking backwards
        // Zen uses: name = (params) return_type { }
        let lines: Vec<&str> = content.lines().collect();
        for i in (0..=from_line).rev() {
            if let Some(line) = lines.get(i as usize) {
                let trimmed = line.trim_start();
                // Match Zen function syntax: identifier = (...) type {
                if trimmed.contains(" = (") && trimmed.contains('{') {
                    return i;
                }
            }
        }
        // If no function found, insert at the beginning
        0
    }

    fn generate_function_name(&self, code: &str) -> String {
        // Generate a descriptive function name based on the code content
        let code_trimmed = code.trim();

        // If it contains a method call, use that as a hint
        if let Some(dot_pos) = code_trimmed.find('.') {
            if let Some(paren_pos) = code_trimmed[dot_pos..].find('(') {
                let method_part = &code_trimmed[dot_pos+1..dot_pos+paren_pos];
                if !method_part.is_empty() && method_part.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    return format!("do_{}", method_part);
                }
            }
        }

        // If it contains specific keywords, use them as hints
        if code_trimmed.contains("loop") {
            return "process_loop".to_string();
        }
        if code_trimmed.contains("println") || code_trimmed.contains("print") {
            return "print_output".to_string();
        }
        if code_trimmed.contains("push") {
            return "add_items".to_string();
        }
        if code_trimmed.contains("get") {
            return "get_value".to_string();
        }

        // Default name
        "extracted_fn".to_string()
    }

    fn generate_variable_name(&self, expression: &str) -> String {
        // Simple heuristic to generate a variable name from an expression
        let expr_trimmed = expression.trim();

        // If it's a method call, use the method name
        if let Some(dot_pos) = expr_trimmed.rfind('.') {
            if let Some(method_end) = expr_trimmed[dot_pos+1..].find('(') {
                let method_name = &expr_trimmed[dot_pos+1..dot_pos+1+method_end];
                return format!("{}_result", method_name);
            }
        }

        // If it's a function call, use the function name
        if let Some(paren_pos) = expr_trimmed.find('(') {
            let func_name = expr_trimmed[..paren_pos].trim();
            if !func_name.is_empty() && func_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return format!("{}_result", func_name);
            }
        }

        // If it's a binary operation, try to infer from operands
        for op in ["==", "!=", "<=", ">=", "<", ">", "+", "-", "*", "/", "%"] {
            if expr_trimmed.contains(op) {
                return "result".to_string();
            }
        }

        // Default fallback
        "extracted_value".to_string()
    }

    fn create_add_import_action(&self, _range: &Range, content: &str) -> Option<CodeAction> {
        // Check if common types are used but not imported
        let needs_io = content.contains("io.") && !content.contains("{ io }");
        let needs_allocator = (content.contains("get_default_allocator") ||
                               content.contains("GPA") ||
                               content.contains("AsyncPool")) &&
                              !content.contains("@std");

        if !needs_io && !needs_allocator {
            return None;
        }

        // Determine what to import
        let import_statement = if needs_io && needs_allocator {
            "{ io, GPA, AsyncPool } = @std\n"
        } else if needs_io {
            "{ io } = @std\n"
        } else {
            "{ GPA, AsyncPool } = @std\n"
        };

        // Insert at the top of the file
        let workspace_edit = WorkspaceEdit {
            changes: None,  // Would need URI context
            document_changes: None,
            change_annotations: None,
        };

        Some(CodeAction {
            title: "Add missing import from @std".to_string(),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: None,
            edit: Some(workspace_edit),
            command: None,
            is_preferred: Some(false),
            disabled: Some(lsp_types::CodeActionDisabled {
                reason: "Needs implementation".to_string(),
            }),
            data: None,
        })
    }

    fn infer_function_return_types(
        &self,
        local_symbols: &HashMap<String, SymbolInfo>,
        stdlib_symbols: &HashMap<String, SymbolInfo>,
        func_name: &str,
        all_docs: &HashMap<Url, Document>
    ) -> (Option<String>, Option<String>) {
        use crate::ast::Declaration;

        // Try AST first (most accurate)
        for (_uri, doc) in all_docs {
            if let Some(ast) = &doc.ast {
                for decl in ast {
                    if let Declaration::Function(func) = decl {
                        if func.name == func_name {
                            // Found it! Extract types from the return type
                            let result = Self::extract_generic_types(&func.return_type);
                            if result.0.is_some() {
                                return result;
                            }
                        }
                    }
                }
            }
        }

        // Fallback: parse from source code if AST unavailable (e.g., parse errors)
        for (_uri, doc) in all_docs {
            let result = Self::parse_function_from_source(&doc.content, func_name);
            if result.0.is_some() && result.1.is_some() {
                return result;
            }
        }

        (None, None)
    }

    fn parse_function_from_source(content: &str, func_name: &str) -> (Option<String>, Option<String>) {
        // Find the function definition line
        // Example: "divide = (a: f64, b: f64) Result<f64, StaticString> {"

        for line in content.lines() {
            if line.contains(&format!("{} =", func_name)) || line.contains(&format!("{}=", func_name)) {
                // Found the function definition
                if let Some(result_pos) = line.find("Result<") {
                    // Extract Result<T, E> by finding the matching >
                    let after_result = &line[result_pos..];
                    if let Some(start) = after_result.find('<') {
                        // Find the matching closing >
                        let mut depth = 0;
                        let mut end_pos = start;
                        for (i, ch) in after_result[start..].chars().enumerate() {
                            match ch {
                                '<' => depth += 1,
                                '>' => {
                                    depth -= 1;
                                    if depth == 0 {
                                        end_pos = start + i;
                                        break;
                                    }
                                }
                                _ => {}
                            }
                        }

                        if end_pos > start {
                            let generics = &after_result[start + 1..end_pos];
                            let parts = Self::split_generic_args(generics);
                            if parts.len() >= 2 {
                                return (Some(parts[0].trim().to_string()), Some(parts[1].trim().to_string()));
                            } else if parts.len() == 1 {
                                // Only one part found - maybe parsing error
                                return (Some(parts[0].trim().to_string()), Some("E".to_string()));
                            }
                        }
                    }
                } else if let Some(option_pos) = line.find("Option<") {
                    // Extract Option<T>
                    if let Some(start) = line[option_pos..].find('<') {
                        if let Some(end) = line[option_pos..].rfind('>') {
                            let inner = line[option_pos + start + 1..option_pos + end].trim();
                            return (Some(inner.to_string()), None);
                        }
                    }
                }
            }
        }

        (None, None)
    }

    fn extract_generic_types(ast_type: &AstType) -> (Option<String>, Option<String>) {
        match ast_type {
            // Result<T, E>
            AstType::Generic { name, type_args } if name == "Result" && type_args.len() == 2 => {
                let ok_type = format_type(&type_args[0]);
                let err_type = format_type(&type_args[1]);
                (Some(ok_type), Some(err_type))
            }
            // Option<T>
            AstType::Generic { name, type_args } if name == "Option" && type_args.len() == 1 => {
                let inner_type = format_type(&type_args[0]);
                (Some(inner_type), None)
            }
            _ => (None, None)
        }
    }

    fn parse_return_type_generics(signature: &str) -> (Option<String>, Option<String>) {
        // Parse function signature to extract Result<T, E> or Option<T>
        // Example: "divide = (a: f64, b: f64) Result<f64, StaticString>"

        // Find the return type (after the closing paren)
        if let Some(paren_pos) = signature.rfind(')') {
            let after_paren = signature[paren_pos+1..].trim();

            // Check for Result<T, E>
            if after_paren.starts_with("Result<") {
                if let Some(start) = after_paren.find('<') {
                    if let Some(end) = after_paren.rfind('>') {
                        let generics = &after_paren[start+1..end];

                        // Smart split by comma - handle nested generics
                        let parts = Self::split_generic_args(generics);
                        if parts.len() == 2 {
                            return (Some(parts[0].to_string()), Some(parts[1].to_string()));
                        } else if parts.len() == 1 {
                            // Single type param (shouldn't happen for Result)
                            return (Some(parts[0].to_string()), Some("unknown".to_string()));
                        }
                    }
                }
            }
            // Check for Option<T>
            else if after_paren.starts_with("Option<") {
                if let Some(start) = after_paren.find('<') {
                    if let Some(end) = after_paren.rfind('>') {
                        let inner_type = after_paren[start+1..end].trim();
                        return (Some(inner_type.to_string()), None);
                    }
                }
            }
        }

        (None, None)
    }

    fn split_generic_args(args: &str) -> Vec<String> {
        // Split generic arguments by comma, respecting nested <>
        let mut result = Vec::new();
        let mut current = String::new();
        let mut depth = 0;

        for ch in args.chars() {
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
                    result.push(current.trim().to_string());
                    current.clear();
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        if !current.trim().is_empty() {
            result.push(current.trim().to_string());
        }

        result
    }

    fn get_pattern_match_hover(
        &self,
        content: &str,
        position: Position,
        symbol_name: &str,
        local_symbols: &HashMap<String, SymbolInfo>,
        stdlib_symbols: &HashMap<String, SymbolInfo>,
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
                                let (concrete_ok_type, concrete_err_type) = self.infer_function_return_types(local_symbols, stdlib_symbols, func_name, all_docs);

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

    fn determine_symbol_scope(&self, doc: &Document, symbol_name: &str, position: Position) -> SymbolScope {
        if let Some(ast) = &doc.ast {
            for decl in ast {
                if let Declaration::Function(func) = decl {
                    if let Some(func_range) = self.find_function_range(&doc.content, &func.name) {
                        if position.line >= func_range.start.line && position.line <= func_range.end.line {
                            if self.is_local_symbol_in_function(func, symbol_name) {
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

    fn is_local_symbol_in_function(&self, func: &crate::ast::Function, symbol_name: &str) -> bool {
        for (param_name, _param_type) in &func.args {
            if param_name == symbol_name {
                return true;
            }
        }

        self.is_symbol_in_statements(&func.body, symbol_name)
    }

    fn is_symbol_in_statements(&self, statements: &[Statement], symbol_name: &str) -> bool {
        for stmt in statements {
            match stmt {
                Statement::VariableDeclaration { name, .. } if name == symbol_name => {
                    return true;
                }
                Statement::Loop { body, .. } => {
                    if self.is_symbol_in_statements(body, symbol_name) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn find_function_range(&self, content: &str, func_name: &str) -> Option<Range> {
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
            let mut end_line = start;

            for (line_num, line) in lines.iter().enumerate().skip(start) {
                for ch in line.chars() {
                    if ch == '{' {
                        brace_depth += 1;
                        found_opening = true;
                    } else if ch == '}' {
                        brace_depth -= 1;
                        if found_opening && brace_depth == 0 {
                            end_line = line_num;
                            return Some(Range {
                                start: Position { line: start as u32, character: 0 },
                                end: Position { line: end_line as u32, character: line.len() as u32 }
                            });
                        }
                    }
                }
            }
        }

        None
    }

    fn rename_local_symbol(
        &self,
        content: &str,
        symbol_name: &str,
        new_name: &str,
        function_name: &str
    ) -> Option<Vec<TextEdit>> {
        let func_range = self.find_function_range(content, function_name)?;
        let mut edits = Vec::new();
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

                if before_ok && after_ok {
                    edits.push(TextEdit {
                        range: Range {
                            start: Position {
                                line: line_num,
                                character: actual_col as u32,
                            },
                            end: Position {
                                line: line_num,
                                character: (actual_col + symbol_name.len()) as u32,
                            },
                        },
                        new_text: new_name.to_string(),
                    });
                }

                start_col = actual_col + 1;
            }
        }

        Some(edits)
    }

    fn collect_workspace_files(&self, store: &DocumentStore) -> Vec<(Url, String)> {
        use std::path::PathBuf;

        fn collect_zen_files_recursive(dir: &std::path::Path, files: &mut Vec<PathBuf>, max_depth: usize) {
            if max_depth == 0 {
                return;
            }

            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();

                    // Skip hidden directories and target/
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with('.') || name == "target" {
                            continue;
                        }
                    }

                    if path.is_dir() {
                        collect_zen_files_recursive(&path, files, max_depth - 1);
                    } else if path.extension().and_then(|e| e.to_str()) == Some("zen") {
                        if let Ok(canonical) = path.canonicalize() {
                            files.push(canonical);
                        }
                    }
                }
            }
        }

        let mut result = Vec::new();

        // Add all open documents
        for (uri, doc) in &store.documents {
            result.push((uri.clone(), doc.content.clone()));
        }

        // Add all workspace files recursively
        if let Some(workspace_root) = &store.workspace_root {
            if let Ok(root_path) = workspace_root.to_file_path() {
                let mut zen_files = Vec::new();
                collect_zen_files_recursive(&root_path, &mut zen_files, 5);

                for path in zen_files {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(uri) = Url::from_file_path(&path) {
                            // Only add if not already in open documents
                            if !store.documents.contains_key(&uri) {
                                result.push((uri, content));
                            }
                        }
                    }
                }
            }
        }

        result
    }

    fn rename_in_file(&self, content: &str, symbol_name: &str, new_name: &str) -> Option<Vec<TextEdit>> {
        let mut edits = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            let mut start_col = 0;

            while let Some(col) = line[start_col..].find(symbol_name) {
                let actual_col = start_col + col;

                let before_ok = actual_col == 0 ||
                    !line.chars().nth(actual_col - 1).unwrap_or(' ').is_alphanumeric();
                let after_ok = actual_col + symbol_name.len() >= line.len() ||
                    !line.chars().nth(actual_col + symbol_name.len()).unwrap_or(' ').is_alphanumeric();

                if before_ok && after_ok {
                    edits.push(TextEdit {
                        range: Range {
                            start: Position {
                                line: line_num as u32,
                                character: actual_col as u32,
                            },
                            end: Position {
                                line: line_num as u32,
                                character: (actual_col + symbol_name.len()) as u32,
                            },
                        },
                        new_text: new_name.to_string(),
                    });
                }

                start_col = actual_col + 1;
            }
        }

        Some(edits)
    }

    fn find_local_references(&self, content: &str, symbol_name: &str, function_name: &str) -> Option<Vec<Range>> {
        let func_range = self.find_function_range(content, function_name)?;
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
        use std::fs;

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