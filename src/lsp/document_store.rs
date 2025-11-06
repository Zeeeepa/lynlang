use lsp_types::*;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::time::Instant;
use crate::ast::{*, Pattern as AstPattern};
use crate::lexer::{Lexer, Token};
use crate::parser::Parser;
use crate::typechecker::TypeChecker;
use super::types::{Document, SymbolInfo, AnalysisJob};
use super::utils::{compile_error_to_diagnostic, format_type};

pub struct DocumentStore {
    pub documents: HashMap<Url, Document>,
    pub stdlib_symbols: HashMap<String, SymbolInfo>,
    pub workspace_symbols: HashMap<String, SymbolInfo>,  // Indexed workspace symbols
    pub workspace_root: Option<Url>,
    pub analysis_sender: Option<Sender<AnalysisJob>>,
}

impl DocumentStore {
    pub fn new() -> Self {
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

    pub fn set_analysis_sender(&mut self, sender: Sender<AnalysisJob>) {
        self.analysis_sender = Some(sender);
    }

    pub fn set_workspace_root(&mut self, root_uri: Url) {
        self.workspace_root = Some(root_uri.clone());
        // Note: Workspace indexing is now done asynchronously after initialization
        // to avoid blocking the main thread and holding locks for extended periods
    }

    pub fn index_workspace(&mut self, root_uri: &Url) {
        if let Ok(root_path) = root_uri.to_file_path() {
            eprintln!("[LSP] Indexing workspace: {}", root_path.display());
            let start = Instant::now();

            let count = self.index_workspace_directory(&root_path);

            let duration = start.elapsed();
            eprintln!("[LSP] Indexed {} symbols from workspace in {:?}", count, duration);
        }
    }

    // Static method for background workspace indexing (doesn't hold locks)
    pub fn index_workspace_files(root_path: &std::path::Path) -> HashMap<String, SymbolInfo> {
        let mut workspace_symbols = HashMap::new();
        Self::index_workspace_files_recursive(root_path, &mut workspace_symbols);
        workspace_symbols
    }

    fn index_workspace_files_recursive(path: &std::path::Path, symbols: &mut HashMap<String, SymbolInfo>) {
        use std::fs;

        // Skip common directories we don't want to index
        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
            if dir_name == "target" || dir_name == "node_modules" || dir_name == ".git"
                || dir_name == "tests" || dir_name.starts_with('.') {
                return;
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
                        // Extract symbols without creating a full DocumentStore
                        let file_path_str = entry_path.to_string_lossy();
                        let file_symbols = Self::extract_symbols_static(&content, Some(&file_path_str));

                        // Convert path to URI
                        if let Ok(uri) = Url::from_file_path(&entry_path) {
                            for (name, mut symbol) in file_symbols {
                                symbol.definition_uri = Some(uri.clone());
                                // Don't overwrite existing symbols (stdlib takes priority)
                                symbols.entry(name).or_insert(symbol);
                            }
                        }
                    }
                } else if entry_path.is_dir() {
                    // Recursively index subdirectories
                    Self::index_workspace_files_recursive(&entry_path, symbols);
                }
            }
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
                        let file_path_str = entry_path.to_string_lossy();
                        let symbols = self.extract_symbols_with_path(&content, Some(&file_path_str));

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

    pub fn open(&mut self, uri: Url, version: i32, content: String) -> Vec<Diagnostic> {
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

    pub fn update(&mut self, uri: Url, version: i32, content: String) -> Vec<Diagnostic> {
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

    fn parse_with_path(&self, content: &str, file_path: Option<&str>) -> Option<Vec<Declaration>> {
        let lexer = Lexer::new(content);
        let mut parser = Parser::new(lexer);

        match parser.parse_program() {
            Ok(program) => Some(program.declarations),
            Err(e) => {
                if let Some(path) = file_path {
                    eprintln!("[LSP] Parse error in {}: {:?}", path, e);
                } else {
                    eprintln!("[LSP] Parse error: {:?}", e);
                }
                None
            }
        }
    }

    fn parse(&self, content: &str) -> Option<Vec<Declaration>> {
        self.parse_with_path(content, None)
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
                            self.check_pattern_exhaustiveness(&func.body, &mut diagnostics, content);
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

    fn check_pattern_exhaustiveness(&self, statements: &[Statement], diagnostics: &mut Vec<Diagnostic>, content: &str) {
        for stmt in statements {
            match stmt {
                Statement::Expression(expr) | Statement::Return(expr) => {
                    self.check_exhaustiveness_in_expression(expr, diagnostics, content);
                }
                Statement::VariableDeclaration { initializer: Some(expr), .. } |
                Statement::VariableAssignment { value: expr, .. } => {
                    self.check_exhaustiveness_in_expression(expr, diagnostics, content);
                }
                _ => {}
            }
        }
    }

    fn check_exhaustiveness_in_expression(&self, expr: &Expression, diagnostics: &mut Vec<Diagnostic>, content: &str) {
        match expr {
            Expression::PatternMatch { scrutinee, arms } => {
                if let Some(scrutinee_type) = self.infer_expression_type_string(scrutinee) {
                    let missing_variants = self.find_missing_variants(&scrutinee_type, arms);

                    if !missing_variants.is_empty() {
                        if let Some(position) = self.find_pattern_match_position(content, scrutinee) {
                            let variant_list = missing_variants.join(", ");
                            diagnostics.push(Diagnostic {
                                range: Range {
                                    start: position,
                                    end: Position {
                                        line: position.line,
                                        character: position.character + 10,
                                    },
                                },
                                severity: Some(DiagnosticSeverity::WARNING),
                                code: Some(lsp_types::NumberOrString::String("non-exhaustive-match".to_string())),
                                source: Some("zen-lsp".to_string()),
                                message: format!("Non-exhaustive pattern match. Missing variants: {}", variant_list),
                                related_information: None,
                                tags: None,
                                code_description: None,
                                data: None,
                            });
                        }
                    }
                }

                self.check_exhaustiveness_in_expression(scrutinee, diagnostics, content);
                for arm in arms {
                    self.check_exhaustiveness_in_expression(&arm.body, diagnostics, content);
                }
            }
            Expression::Block(stmts) => {
                self.check_pattern_exhaustiveness(stmts, diagnostics, content);
            }
            Expression::Conditional { scrutinee, arms } => {
                self.check_exhaustiveness_in_expression(scrutinee, diagnostics, content);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        self.check_exhaustiveness_in_expression(guard, diagnostics, content);
                    }
                    self.check_exhaustiveness_in_expression(&arm.body, diagnostics, content);
                }
            }
            Expression::BinaryOp { left, right, .. } => {
                self.check_exhaustiveness_in_expression(left, diagnostics, content);
                self.check_exhaustiveness_in_expression(right, diagnostics, content);
            }
            _ => {}
        }
    }

    fn find_missing_variants(&self, scrutinee_type: &str, arms: &[PatternArm]) -> Vec<String> {
        // First check if it's a built-in enum type
        let known_enum_variants: Vec<String> = if scrutinee_type.starts_with("Option") {
            vec!["Some".to_string(), "None".to_string()]
        } else if scrutinee_type.starts_with("Result") {
            vec!["Ok".to_string(), "Err".to_string()]
        } else {
            // Try to look up custom enum from symbol tables
            // Extract just the enum name (before any :: or generic params)
            let enum_name = scrutinee_type.split("::").next()
                .unwrap_or(scrutinee_type)
                .split('<').next()
                .unwrap_or(scrutinee_type)
                .trim();

            // Search in all available symbol sources
            let mut found_variants: Option<Vec<String>> = None;

            // 1. Check current document symbols (limit search for performance)
            const MAX_DOCS_ENUM_SEARCH: usize = 30;
            for doc in self.documents.values().take(MAX_DOCS_ENUM_SEARCH) {
                if let Some(symbol) = doc.symbols.get(enum_name) {
                    if let Some(ref variants) = symbol.enum_variants {
                        found_variants = Some(variants.clone());
                        break;
                    }
                }
            }

            // 2. Check workspace symbols if not found
            if found_variants.is_none() {
                if let Some(symbol) = self.workspace_symbols.get(enum_name) {
                    if let Some(ref variants) = symbol.enum_variants {
                        found_variants = Some(variants.clone());
                    }
                }
            }

            // 3. Check stdlib symbols if not found
            if found_variants.is_none() {
                if let Some(symbol) = self.stdlib_symbols.get(enum_name) {
                    if let Some(ref variants) = symbol.enum_variants {
                        found_variants = Some(variants.clone());
                    }
                }
            }

            // If we found the enum, use its variants; otherwise return empty
            match found_variants {
                Some(variants) => variants,
                None => return Vec::new(),
            }
        };

        // Collect covered variants and check for wildcards
        let mut covered_variants = std::collections::HashSet::new();
        let mut has_wildcard = false;

        for arm in arms {
            match &arm.pattern {
                AstPattern::EnumVariant { variant, .. } => {
                    covered_variants.insert(variant.clone());
                }
                AstPattern::EnumLiteral { variant, .. } => {
                    covered_variants.insert(variant.clone());
                }
                AstPattern::Wildcard => {
                    has_wildcard = true;
                }
                _ => {}
            }
        }

        if has_wildcard {
            return Vec::new();
        }

        // Return missing variants
        known_enum_variants
            .into_iter()
            .filter(|v| !covered_variants.contains(v))
            .collect()
    }

    fn infer_expression_type_string(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Identifier(name) => {
                // Try to infer type from variable declarations in current documents (limit for performance)
                const MAX_DOCS_TYPE_SEARCH: usize = 30;
                for doc in self.documents.values().take(MAX_DOCS_TYPE_SEARCH) {
                    if let Some(ast) = &doc.ast {
                        if let Some(type_str) = self.find_variable_type_in_ast(name, ast) {
                            return Some(type_str);
                        }
                    }
                }
                // Fallback: just return the name (might be a type itself)
                Some(name.clone())
            }
            Expression::FunctionCall { name, .. } => {
                if name.contains("Result") || name.ends_with("_result") {
                    Some("Result<T, E>".to_string())
                } else if name.contains("Option") || name.ends_with("_option") {
                    Some("Option<T>".to_string())
                } else {
                    None
                }
            }
            _ => None
        }
    }

    fn find_variable_type_in_ast(&self, var_name: &str, ast: &[Declaration]) -> Option<String> {
        for decl in ast {
            match decl {
                Declaration::Function(func) => {
                    // Search in function body
                    if let Some(type_str) = self.find_variable_type_in_statements(var_name, &func.body) {
                        return Some(type_str);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn find_variable_type_in_statements(&self, var_name: &str, stmts: &[Statement]) -> Option<String> {
        for stmt in stmts {
            match stmt {
                Statement::VariableDeclaration { name, initializer, type_, .. } => {
                    if name == var_name {
                        // Use type annotation if available
                        if let Some(type_ann) = type_ {
                            return Some(format_type(type_ann));
                        }
                        // Otherwise try to infer from initializer
                        if let Some(init) = initializer {
                            return self.infer_type_from_expression(init);
                        }
                    }
                }
                Statement::Expression(expr) | Statement::Return(expr) => {
                    if let Some(type_str) = self.find_variable_in_expression(var_name, expr) {
                        return Some(type_str);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn find_variable_in_expression(&self, var_name: &str, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Block(stmts) => {
                self.find_variable_type_in_statements(var_name, stmts)
            }
            _ => None
        }
    }

    fn infer_type_from_expression(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::FunctionCall { name, .. } => {
                // Check if this is an enum constructor like TestEnum::Variant1
                if name.contains("::") {
                    let parts: Vec<&str> = name.split("::").collect();
                    if parts.len() == 2 {
                        return Some(parts[0].to_string());
                    }
                }
                None
            }
            _ => None
        }
    }

    fn find_pattern_match_position(&self, content: &str, scrutinee: &Expression) -> Option<Position> {
        if let Expression::Identifier(name) = scrutinee {
            return self.find_text_position(name, content);
        }
        None
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

    // Static version for background indexing (no reference tracking)
    fn extract_symbols_static(content: &str, file_path: Option<&str>) -> HashMap<String, SymbolInfo> {
        let mut symbols = HashMap::new();

        // Parse the content
        let lexer = Lexer::new(content);
        let mut parser = Parser::new(lexer);
        let ast = match parser.parse_program() {
            Ok(program) => program.declarations,
            Err(e) => {
                if let Some(path) = file_path {
                    eprintln!("[LSP] Parse error in {}: {:?}", path, e);
                } else {
                    eprintln!("[LSP] Parse error: {:?}", e);
                }
                return symbols;
            }
        };

        // Extract symbol definitions only (no reference tracking for performance)
        for decl in ast {
            let range = Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 100 },
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
                        enum_variants: None,
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
                        enum_variants: None,
                    });
                }
                Declaration::Enum(enum_def) => {
                    let detail = format!("{} enum with {} variants", enum_def.name, enum_def.variants.len());
                    let variant_names: Vec<String> = enum_def.variants.iter().map(|v| v.name.clone()).collect();

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
                        enum_variants: Some(variant_names.clone()),
                    });

                    // Add enum variants as symbols
                    for variant_name in variant_names {
                        let full_name = format!("{}::{}", enum_def.name, variant_name);
                        symbols.insert(full_name.clone(), SymbolInfo {
                            name: variant_name.clone(),
                            kind: SymbolKind::ENUM_MEMBER,
                            range: range.clone(),
                            selection_range: range.clone(),
                            detail: Some(full_name),
                            documentation: None,
                            type_info: None,
                            definition_uri: None,
                            references: Vec::new(),
                            enum_variants: None,
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
                        enum_variants: None,
                    });
                }
                _ => {}
            }
        }

        symbols
    }

    fn extract_symbols_with_path(&self, content: &str, file_path: Option<&str>) -> HashMap<String, SymbolInfo> {
        let mut symbols = HashMap::new();

        if let Some(ast) = self.parse_with_path(content, file_path) {
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
                            enum_variants: None,
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
                            enum_variants: None,
                        });
                    }
                    Declaration::Enum(enum_def) => {
                        let detail = format!("{} enum with {} variants", enum_def.name, enum_def.variants.len());

                        let variant_names: Vec<String> = enum_def.variants.iter()
                            .map(|v| v.name.clone())
                            .collect();

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
                            enum_variants: Some(variant_names),
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
                                enum_variants: None,
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
                            enum_variants: None,
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
                            enum_variants: None,
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
                            enum_variants: None,
                        });
                    }
                    Declaration::Enum(enum_def) => {
                        let detail = format!("{} enum with {} variants", enum_def.name, enum_def.variants.len());

                        let variant_names: Vec<String> = enum_def.variants.iter()
                            .map(|v| v.name.clone())
                            .collect();

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
                            enum_variants: Some(variant_names),
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
                                enum_variants: None,
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
                            enum_variants: None,
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

    pub fn search_workspace_for_symbol(&self, symbol_name: &str) -> Option<(Url, SymbolInfo)> {
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
                if let Some(_symbol) = symbols.get_mut(name) {
                    // Add reference location (would need position info)
                }
            }
            Expression::FunctionCall { name, args, .. } => {
                // Track function call reference
                if let Some(_symbol) = symbols.get_mut(name) {
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
