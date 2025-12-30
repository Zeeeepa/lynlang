use super::analyzer;
use super::compiler_integration::CompilerIntegration;
use super::indexing::{find_stdlib_path, index_stdlib_directory, index_workspace_files_recursive};
use super::stdlib_resolver::StdlibResolver;
use super::types::{AnalysisJob, Document, SymbolInfo};
use super::utils::format_type;
use crate::ast::*;
use crate::stdlib_metadata::StdModuleTrait;
use crate::lexer::{Lexer, Token};
use crate::parser::Parser;
use crate::well_known::well_known;
use lsp_types::*;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::time::Instant;

pub struct DocumentStore {
    pub documents: HashMap<Url, Document>,
    pub stdlib_symbols: HashMap<String, SymbolInfo>,
    pub workspace_symbols: HashMap<String, SymbolInfo>, // Indexed workspace symbols
    pub workspace_root: Option<Url>,
    pub analysis_sender: Option<Sender<AnalysisJob>>,
    pub compiler: CompilerIntegration,
    pub stdlib_resolver: StdlibResolver,
}

impl DocumentStore {
    pub fn new() -> Self {
        let workspace_root_path = None::<&std::path::Path>;
        let stdlib_resolver = StdlibResolver::new(workspace_root_path);
        let compiler = CompilerIntegration::new();

        let mut store = Self {
            documents: HashMap::new(),
            stdlib_symbols: HashMap::new(),
            workspace_symbols: HashMap::new(),
            workspace_root: None,
            analysis_sender: None,
            compiler,
            stdlib_resolver,
        };

        store.register_builtin_types();
        store
    }
    
    pub fn index_stdlib_deferred(&mut self) {
        self.index_stdlib();
    }

    /// Register built-in primitive types that are always available
    fn register_builtin_types(&mut self) {
        use super::types::SymbolInfo;
        use crate::ast::AstType;
        use lsp_types::{Position, Range, SymbolKind};

        // Create a dummy range for built-in types (they don't have a source location)
        let dummy_range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 0,
            },
        };

        // Register all primitive types
        let builtin_types = vec![
            ("i8", AstType::I8),
            ("i16", AstType::I16),
            ("i32", AstType::I32),
            ("i64", AstType::I64),
            ("u8", AstType::U8),
            ("u16", AstType::U16),
            ("u32", AstType::U32),
            ("u64", AstType::U64),
            ("usize", AstType::Usize),
            ("f32", AstType::F32),
            ("f64", AstType::F64),
            ("bool", AstType::Bool),
            ("StaticString", AstType::StaticString), // Built-in primitive type
            ("void", AstType::Void),
        ];

        for (name, type_) in builtin_types {
            self.stdlib_symbols.insert(
                name.to_string(),
                SymbolInfo {
                    name: name.to_string(),
                    kind: SymbolKind::TYPE_PARAMETER, // Use TYPE_PARAMETER for primitive types
                    range: dummy_range.clone(),
                    selection_range: dummy_range.clone(),
                    detail: Some(format!("{} - Built-in primitive type", name)),
                    documentation: Some(format!(
                        "Built-in primitive type `{}`. Always available, no import needed.",
                        name
                    )),
                    type_info: Some(type_),
                    definition_uri: None,
                    references: Vec::new(),
                    enum_variants: None,
                },
            );
        }

        // Also register built-in generic types (Option, Result)
        let wk = well_known();
        let builtin_generics = vec![
            (
                wk.option_name(),
                AstType::Generic {
                    name: wk.option_name().to_string(),
                    type_args: vec![],
                },
            ),
            (
                wk.result_name(),
                AstType::Generic {
                    name: wk.result_name().to_string(),
                    type_args: vec![],
                },
            ),
        ];

        for (name, type_) in builtin_generics {
            self.stdlib_symbols.insert(
                name.to_string(),
                SymbolInfo {
                    name: name.to_string(),
                    kind: SymbolKind::ENUM,
                    range: dummy_range.clone(),
                    selection_range: dummy_range.clone(),
                    detail: Some(format!("{}<T> - Built-in generic type", name)),
                    documentation: Some(format!(
                        "Built-in generic type `{}`. Always available, no import needed.",
                        name
                    )),
                    type_info: Some(type_),
                    definition_uri: None,
                    references: Vec::new(),
                    enum_variants: None,
                },
            );
        }

        self.register_compiler_intrinsics(&dummy_range);
    }

    fn register_compiler_intrinsics(&mut self, dummy_range: &Range) {
        use crate::stdlib_metadata::compiler::get_compiler_module;
        use super::utils::format_type;

        let compiler_module = get_compiler_module();
        
        let intrinsics = vec![
            ("raw_allocate", "Allocates raw memory using malloc", "Memory allocation"),
            ("raw_deallocate", "Deallocates memory previously allocated with raw_allocate", "Memory deallocation"),
            ("raw_reallocate", "Reallocates memory to a new size", "Memory reallocation"),
            ("raw_ptr_offset", "Offset a pointer by byte count (deprecated - use gep)", "Pointer arithmetic"),
            ("raw_ptr_cast", "Reinterprets a pointer type (no runtime cost)", "Type coercion"),
            ("gep", "GetElementPointer - byte-level pointer arithmetic", "Pointer arithmetic"),
            ("gep_struct", "Struct field access using GetElementPointer", "Field access"),
            ("null_ptr", "Returns a null pointer", "Null pointer constant"),
            ("nullptr", "Alias for null_ptr - returns a null pointer", "Null pointer constant"),
            ("sizeof", "Returns the size of a type in bytes", "Type introspection"),
            ("alignof", "Returns the alignment of a type in bytes", "Type introspection"),
            ("discriminant", "Reads the discriminant (variant tag) from an enum", "Enum introspection"),
            ("set_discriminant", "Sets the discriminant (variant tag) of an enum", "Enum manipulation"),
            ("get_payload", "Returns a pointer to the payload data within an enum", "Enum access"),
            ("set_payload", "Copies payload data into an enum's payload field", "Enum manipulation"),
            ("load", "Load a value from a pointer", "Memory access"),
            ("store", "Store a value to a pointer", "Memory access"),
            ("memcpy", "Copy bytes from source to destination (non-overlapping)", "Memory operations"),
            ("memmove", "Copy bytes from source to destination (overlapping safe)", "Memory operations"),
            ("memset", "Set all bytes in memory to a value", "Memory operations"),
            ("memcmp", "Compare bytes in memory", "Memory operations"),
            ("ptr_to_int", "Convert a pointer to an integer address", "Type conversion"),
            ("int_to_ptr", "Convert an integer address to a pointer", "Type conversion"),
            ("trunc_f64_i64", "Truncate f64 to i64", "Type conversion"),
            ("trunc_f32_i32", "Truncate f32 to i32", "Type conversion"),
            ("sitofp_i64_f64", "Convert signed i64 to f64", "Type conversion"),
            ("uitofp_u64_f64", "Convert unsigned u64 to f64", "Type conversion"),
            ("bswap16", "Byte-swap a 16-bit value (endian conversion)", "Bitwise"),
            ("bswap32", "Byte-swap a 32-bit value (endian conversion)", "Bitwise"),
            ("bswap64", "Byte-swap a 64-bit value (endian conversion)", "Bitwise"),
            ("ctlz", "Count leading zeros", "Bitwise"),
            ("cttz", "Count trailing zeros", "Bitwise"),
            ("ctpop", "Population count (count set bits)", "Bitwise"),
            ("atomic_load", "Atomically load a value", "Atomic"),
            ("atomic_store", "Atomically store a value", "Atomic"),
            ("atomic_add", "Atomically add and return old value", "Atomic"),
            ("atomic_sub", "Atomically subtract and return old value", "Atomic"),
            ("atomic_cas", "Compare-and-swap, returns success", "Atomic"),
            ("atomic_xchg", "Atomically exchange and return old value", "Atomic"),
            ("fence", "Memory fence for synchronization", "Atomic"),
            ("add_overflow", "Add with overflow detection", "Overflow arithmetic"),
            ("sub_overflow", "Subtract with overflow detection", "Overflow arithmetic"),
            ("mul_overflow", "Multiply with overflow detection", "Overflow arithmetic"),
            ("unreachable", "Mark code as unreachable (UB if reached)", "Debug"),
            ("trap", "Trigger a trap/abort", "Debug"),
            ("debugtrap", "Trigger a debug trap (breakpoint)", "Debug"),
            ("inline_c", "Inline C code compilation (placeholder)", "FFI"),
            ("load_library", "Load a dynamic library (placeholder)", "FFI"),
            ("get_symbol", "Get a symbol from a loaded library (placeholder)", "FFI"),
            ("unload_library", "Unload a dynamic library (placeholder)", "FFI"),
            ("call_external", "Call an external function via pointer (placeholder)", "FFI"),
        ];

        for (name, doc, category) in intrinsics {
            if let Some(func) = compiler_module.get_function(name) {
                let params_str = func.params
                    .iter()
                    .map(|(pname, ptype)| format!("{}: {}", pname, format_type(ptype)))
                    .collect::<Vec<_>>()
                    .join(", ");
                
                let detail = format!(
                    "@std.compiler.{}({}) -> {}",
                    name,
                    params_str,
                    format_type(&func.return_type)
                );

                let full_doc = format!(
                    "{}\n\n**Category:** {}\n\n**Module:** `@std.compiler`\n\n**Signature:**\n```zen\n{}\n```",
                    doc, category, detail
                );

                self.stdlib_symbols.insert(
                    format!("compiler.{}", name),
                    SymbolInfo {
                        name: name.to_string(),
                        kind: SymbolKind::FUNCTION,
                        range: dummy_range.clone(),
                        selection_range: dummy_range.clone(),
                        detail: Some(detail.clone()),
                        documentation: Some(full_doc.clone()),
                        type_info: Some(func.return_type.clone()),
                        definition_uri: None,
                        references: Vec::new(),
                        enum_variants: None,
                    },
                );

                self.stdlib_symbols.insert(
                    format!("@std.compiler.{}", name),
                    SymbolInfo {
                        name: name.to_string(),
                        kind: SymbolKind::FUNCTION,
                        range: dummy_range.clone(),
                        selection_range: dummy_range.clone(),
                        detail: Some(detail),
                        documentation: Some(full_doc),
                        type_info: Some(func.return_type),
                        definition_uri: None,
                        references: Vec::new(),
                        enum_variants: None,
                    },
                );
            }
        }
    }

    pub fn set_analysis_sender(&mut self, sender: Sender<AnalysisJob>) {
        self.analysis_sender = Some(sender);
    }

    pub fn set_workspace_root(&mut self, root_uri: Url) {
        self.workspace_root = Some(root_uri.clone());

        // Update stdlib resolver with workspace root
        if let Ok(workspace_path) = root_uri.to_file_path() {
            self.stdlib_resolver = StdlibResolver::new(Some(&workspace_path));
        }

        // Note: Workspace indexing is now done asynchronously after initialization
        // to avoid blocking the main thread and holding locks for extended periods
    }

    pub fn index_workspace(&mut self, root_uri: &Url) {
        if let Ok(root_path) = root_uri.to_file_path() {
            log::debug!("[LSP] Indexing workspace: {}", root_path.display());
            let start = Instant::now();

            let count = self.index_workspace_directory(&root_path);

            let duration = start.elapsed();
            log::debug!(
                "[LSP] Indexed {} symbols from workspace in {:?}",
                count, duration
            );
        }
    }

    // Static method for background workspace indexing (doesn't hold locks)
    pub fn index_workspace_files(root_path: &std::path::Path) -> HashMap<String, SymbolInfo> {
        let mut workspace_symbols = HashMap::new();
        index_workspace_files_recursive(root_path, &mut workspace_symbols);
        workspace_symbols
    }

    fn index_workspace_directory(&mut self, path: &std::path::Path) -> usize {
        use std::fs;

        let mut symbol_count = 0;

        // Skip common directories we don't want to index
        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
            if dir_name == "target"
                || dir_name == "node_modules"
                || dir_name == ".git"
                || dir_name == "tests"
                || dir_name.starts_with('.')
            {
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
        if let Some(stdlib_path) = find_stdlib_path() {
            index_stdlib_directory(&stdlib_path, &mut self.stdlib_symbols);
            log::debug!("[LSP] Indexed {} stdlib symbols from: {}", 
                     self.stdlib_symbols.len(), stdlib_path.display());
        }
    }

    pub fn open(&mut self, uri: Url, version: i32, content: String) -> Vec<Diagnostic> {
        let tokens = self.tokenize(&content);
        let ast = self.parse(&content);
        
        let symbols = if let Some(ref ast_decls) = ast {
            self.extract_symbols_from_ast(ast_decls, &content)
        } else {
            HashMap::new()
        };

        let doc = Document {
            uri: uri.clone(),
            version,
            content: content.clone(),
            tokens,
            ast: ast.clone(),
            diagnostics: Vec::new(),
            symbols,
            last_analysis: Some(Instant::now()),
            cached_lines: None,
        };

        self.documents.insert(uri.clone(), doc);
        
        if let Some(ast_decls) = ast {
            if let Some(sender) = &self.analysis_sender {
                let job = AnalysisJob {
                    uri,
                    version,
                    content,
                    program: Program {
                        declarations: ast_decls,
                        statements: vec![],
                    },
                };
                let _ = sender.send(job);
            }
        }
        
        Vec::new()
    }

    pub fn update(&mut self, uri: Url, version: i32, content: String) -> Vec<Diagnostic> {
        const DEBOUNCE_MS: u128 = 300;

        let should_run_analysis = self
            .documents
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
            doc.content = content; // Move instead of clone - we own content
            doc.tokens = tokens;
            doc.ast = ast;
            doc.diagnostics = diagnostics.clone(); // Need clone for return value
            doc.symbols = symbols;
            doc.cached_lines = None; // Invalidate cache on content change
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
                    log::debug!("[LSP] Parse error in {}: {:?}", path, e);
                } else {
                    log::debug!("[LSP] Parse error: {:?}", e);
                }
                None
            }
        }
    }

    fn parse(&self, content: &str) -> Option<Vec<Declaration>> {
        self.parse_with_path(content, None)
    }

    fn analyze_document(&self, content: &str, skip_expensive_analysis: bool) -> Vec<Diagnostic> {
        analyzer::analyze_document(
            content,
            skip_expensive_analysis,
            &self.documents,
            &self.workspace_symbols,
            &self.stdlib_symbols,
        )
    }

    fn infer_type_from_expression(&self, expr: &Expression) -> Option<String> {
        analyzer::infer_type_from_expression(expr, &self.documents, &self.compiler)
    }

    fn extract_symbols(&self, content: &str) -> HashMap<String, SymbolInfo> {
        let mut symbols = HashMap::new();

        // Try to parse AST first
        if let Some(ast) = self.parse(content) {
            // First pass: Extract symbol definitions
            for (decl_index, decl) in ast.iter().enumerate() {
                let (line, char_pos) = self.find_declaration_position(content, &decl, decl_index);
                let symbol_name = match decl {
                    Declaration::Function(f) => &f.name,
                    Declaration::Struct(s) => &s.name,
                    Declaration::Enum(e) => &e.name,
                    Declaration::Constant { name, .. } => name,
                    _ => continue,
                };
                let name_end = char_pos + symbol_name.len();
                let range = Range {
                    start: Position {
                        line: line as u32,
                        character: char_pos as u32,
                    },
                    end: Position {
                        line: line as u32,
                        character: name_end as u32,
                    },
                };

                match decl {
                    Declaration::Function(func) => {
                        let detail = format!(
                            "{} = ({}) {}",
                            func.name,
                            func.args
                                .iter()
                                .map(|(name, ty)| format!("{}: {}", name, format_type(ty)))
                                .collect::<Vec<_>>()
                                .join(", "),
                            format_type(&func.return_type)
                        );

                        symbols.insert(
                            func.name.clone(),
                            SymbolInfo {
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
                            },
                        );
                    }
                    Declaration::Struct(struct_def) => {
                        let detail = format!(
                            "{} struct with {} fields",
                            struct_def.name,
                            struct_def.fields.len()
                        );

                        symbols.insert(
                            struct_def.name.clone(),
                            SymbolInfo {
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
                            },
                        );
                    }
                    Declaration::Enum(enum_def) => {
                        let detail = format!(
                            "{} enum with {} variants",
                            enum_def.name,
                            enum_def.variants.len()
                        );

                        let variant_names: Vec<String> =
                            enum_def.variants.iter().map(|v| v.name.clone()).collect();

                        symbols.insert(
                            enum_def.name.clone(),
                            SymbolInfo {
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
                            },
                        );

                        // Add enum variants as symbols
                        for variant in &enum_def.variants {
                            let variant_name = format!("{}::{}", enum_def.name, variant.name);
                            symbols.insert(
                                variant_name.clone(),
                                SymbolInfo {
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
                                },
                            );
                        }
                    }
                    Declaration::Constant { name, type_, .. } => {
                        symbols.insert(
                            name.clone(),
                            SymbolInfo {
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
                            },
                        );
                    }
                    _ => {}
                }
            }

            // Second pass: Find references, extract variables, and handle impl blocks
            for decl in ast {
                match decl {
                    Declaration::Function(func) => {
                        self.find_references_in_statements(&func.body, &mut symbols);
                        self.extract_variables_from_statements(&func.body, content, &mut symbols);
                    }
                    Declaration::TraitImplementation(impl_block) => {
                        let impl_range = self.find_impl_block_range(content, &impl_block.type_name);
                        for method in &impl_block.methods {
                            let method_name =
                                format!("{}.{}", impl_block.type_name, method.name);
                            let detail = format!(
                                "{}.{} = ({}) {}",
                                impl_block.type_name,
                                method.name,
                                method
                                    .args
                                    .iter()
                                    .map(|(name, ty)| format!("{}: {}", name, format_type(ty)))
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                format_type(&method.return_type)
                            );

                            symbols.insert(
                                method_name.clone(),
                                SymbolInfo {
                                    name: method.name.clone(),
                                    kind: SymbolKind::METHOD,
                                    range: impl_range,
                                    selection_range: impl_range,
                                    detail: Some(detail),
                                    documentation: Some(format!(
                                        "Method from {}.implements({})",
                                        impl_block.type_name, impl_block.trait_name
                                    )),
                                    type_info: Some(method.return_type.clone()),
                                    definition_uri: None,
                                    references: Vec::new(),
                                    enum_variants: None,
                                },
                            );
                        }
                    }
                    _ => {}
                }
            }
        } else {
            // Fallback: If parsing fails, try text-based extraction for basic symbols
            // This helps when there are syntax errors but we still want goto-definition to work
            log::debug!("[LSP] Parse failed, using text-based symbol extraction fallback");
            for (line_num, line) in content.lines().enumerate() {
                let trimmed = line.trim();
                // Skip comments
                if trimmed.starts_with("//") {
                    continue;
                }

                // Look for struct definitions: Name: {
                if let Some(colon_pos) = trimmed.find(':') {
                    let before_colon = trimmed[..colon_pos].trim();
                    if !before_colon.is_empty()
                        && before_colon
                            .chars()
                            .all(|c| c.is_alphanumeric() || c == '_')
                    {
                        if trimmed[colon_pos + 1..].trim().starts_with('{') {
                            // Found struct definition
                            let char_pos = line.find(before_colon).unwrap_or(0);
                            let range = Range {
                                start: Position {
                                    line: line_num as u32,
                                    character: char_pos as u32,
                                },
                                end: Position {
                                    line: line_num as u32,
                                    character: (char_pos + before_colon.len()) as u32,
                                },
                            };
                            symbols.insert(
                                before_colon.to_string(),
                                SymbolInfo {
                                    name: before_colon.to_string(),
                                    kind: SymbolKind::STRUCT,
                                    range: range.clone(),
                                    selection_range: range,
                                    detail: Some(format!("{} struct", before_colon)),
                                    documentation: None,
                                    type_info: None,
                                    definition_uri: None,
                                    references: Vec::new(),
                                    enum_variants: None,
                                },
                            );
                        }
                    }
                }

                // Look for function definitions: name = (
                if let Some(eq_pos) = trimmed.find('=') {
                    let before_eq = trimmed[..eq_pos].trim();
                    if !before_eq.is_empty()
                        && before_eq.chars().all(|c| c.is_alphanumeric() || c == '_')
                    {
                        if trimmed[eq_pos + 1..].trim().starts_with('(') {
                            // Found function definition
                            let char_pos = line.find(before_eq).unwrap_or(0);
                            let range = Range {
                                start: Position {
                                    line: line_num as u32,
                                    character: char_pos as u32,
                                },
                                end: Position {
                                    line: line_num as u32,
                                    character: (char_pos + before_eq.len()) as u32,
                                },
                            };
                            symbols.insert(
                                before_eq.to_string(),
                                SymbolInfo {
                                    name: before_eq.to_string(),
                                    kind: SymbolKind::FUNCTION,
                                    range: range.clone(),
                                    selection_range: range,
                                    detail: Some(format!("{} = (...) ...", before_eq)),
                                    documentation: None,
                                    type_info: None,
                                    definition_uri: None,
                                    references: Vec::new(),
                                    enum_variants: None,
                                },
                            );
                        }
                    }
                }
            }
        }

        symbols
    }

    fn extract_symbols_from_ast(
        &self,
        ast: &[Declaration],
        content: &str,
    ) -> HashMap<String, SymbolInfo> {
        let mut symbols = HashMap::new();

        for (decl_index, decl) in ast.iter().enumerate() {
            let (line, char_pos) = self.find_declaration_position(content, decl, decl_index);
            let symbol_name = match decl {
                Declaration::Function(f) => &f.name,
                Declaration::Struct(s) => &s.name,
                Declaration::Enum(e) => &e.name,
                Declaration::Constant { name, .. } => name,
                _ => continue,
            };
            let name_end = char_pos + symbol_name.len();
            let range = Range {
                start: Position {
                    line: line as u32,
                    character: char_pos as u32,
                },
                end: Position {
                    line: line as u32,
                    character: name_end as u32,
                },
            };

            match decl {
                Declaration::Function(func) => {
                    let detail = format!(
                        "{} = ({}) {}",
                        func.name,
                        func.args
                            .iter()
                            .map(|(name, ty)| format!("{}: {}", name, format_type(ty)))
                            .collect::<Vec<_>>()
                            .join(", "),
                        format_type(&func.return_type)
                    );

                    symbols.insert(
                        func.name.clone(),
                        SymbolInfo {
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
                        },
                    );
                }
                Declaration::Struct(struct_def) => {
                    let detail = format!(
                        "{} struct with {} fields",
                        struct_def.name,
                        struct_def.fields.len()
                    );

                    symbols.insert(
                        struct_def.name.clone(),
                        SymbolInfo {
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
                        },
                    );
                }
                Declaration::Enum(enum_def) => {
                    let detail = format!(
                        "{} enum with {} variants",
                        enum_def.name,
                        enum_def.variants.len()
                    );

                    let variant_names: Vec<String> =
                        enum_def.variants.iter().map(|v| v.name.clone()).collect();

                    symbols.insert(
                        enum_def.name.clone(),
                        SymbolInfo {
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
                        },
                    );

                    for variant in &enum_def.variants {
                        let variant_name = format!("{}::{}", enum_def.name, variant.name);
                        symbols.insert(
                            variant_name.clone(),
                            SymbolInfo {
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
                            },
                        );
                    }
                }
                Declaration::Constant { name, type_, .. } => {
                    symbols.insert(
                        name.clone(),
                        SymbolInfo {
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
                        },
                    );
                }
                _ => {}
            }
        }

        for decl in ast {
            match decl {
                Declaration::Function(func) => {
                    self.find_references_in_statements(&func.body, &mut symbols);
                    self.extract_variables_from_statements(&func.body, content, &mut symbols);
                }
                Declaration::TraitImplementation(impl_block) => {
                    let impl_range = self.find_impl_block_range(content, &impl_block.type_name);
                    for method in &impl_block.methods {
                        let method_name = format!("{}.{}", impl_block.type_name, method.name);
                        let detail = format!(
                            "{}.{} = ({}) {}",
                            impl_block.type_name,
                            method.name,
                            method
                                .args
                                .iter()
                                .map(|(name, ty)| format!("{}: {}", name, format_type(ty)))
                                .collect::<Vec<_>>()
                                .join(", "),
                            format_type(&method.return_type)
                        );

                        symbols.insert(
                            method_name.clone(),
                            SymbolInfo {
                                name: method.name.clone(),
                                kind: SymbolKind::METHOD,
                                range: impl_range,
                                selection_range: impl_range,
                                detail: Some(detail),
                                documentation: Some(format!(
                                    "Method from {}.implements({})",
                                    impl_block.type_name, impl_block.trait_name
                                )),
                                type_info: Some(method.return_type.clone()),
                                definition_uri: None,
                                references: Vec::new(),
                                enum_variants: None,
                            },
                        );
                    }
                }
                _ => {}
            }
        }

        symbols
    }

    fn find_declaration_position(
        &self,
        content: &str,
        decl: &Declaration,
        _index: usize,
    ) -> (usize, usize) {
        // Find the line number and character position where the declaration starts
        let search_str = match decl {
            Declaration::Function(f) => &f.name,
            Declaration::Struct(s) => &s.name,
            Declaration::Enum(e) => &e.name,
            Declaration::Constant { name, .. } => name,
            _ => return (0, 0),
        };

        let lines: Vec<&str> = content.lines().collect();
        for (line_num, line) in lines.iter().enumerate() {
            // Look for the symbol name at word boundaries followed by = or :
            if let Some(pos) = self.find_word_in_line_for_symbol(line, search_str) {
                // Check if this looks like a definition (has = or : after the name)
                let after_symbol = &line[pos + search_str.len()..].trim();
                if after_symbol.starts_with('=')
                    || after_symbol.starts_with(':')
                    || after_symbol.starts_with('(')
                {
                    return (line_num, pos);
                }
            }
        }
        (0, 0)
    }

    fn find_impl_block_range(&self, content: &str, type_name: &str) -> Range {
        let pattern = format!("{}.implements", type_name);
        let lines: Vec<&str> = content.lines().collect();
        for (line_num, line) in lines.iter().enumerate() {
            if let Some(pos) = line.find(&pattern) {
                return Range {
                    start: Position {
                        line: line_num as u32,
                        character: pos as u32,
                    },
                    end: Position {
                        line: line_num as u32,
                        character: (pos + pattern.len()) as u32,
                    },
                };
            }
        }
        Range::default()
    }

    fn find_word_in_line_for_symbol(&self, line: &str, symbol: &str) -> Option<usize> {
        let mut search_pos = 0;
        loop {
            if let Some(pos) = line[search_pos..].find(symbol) {
                let actual_pos = search_pos + pos;

                // Check word boundaries
                let before_ok = actual_pos == 0 || {
                    let before = line.chars().nth(actual_pos - 1).unwrap_or(' ');
                    !before.is_alphanumeric() && before != '_'
                };
                let after_pos = actual_pos + symbol.len();
                let after_ok = after_pos >= line.len() || {
                    let after = line.chars().nth(after_pos).unwrap_or(' ');
                    !after.is_alphanumeric() && after != '_'
                };

                if before_ok && after_ok {
                    return Some(actual_pos);
                }

                search_pos = actual_pos + 1;
            } else {
                break;
            }
        }
        None
    }

    pub fn search_workspace_for_symbol(&self, symbol_name: &str) -> Option<(Url, SymbolInfo)> {
        use std::path::Path;

        let workspace_root = self.workspace_root.as_ref()?;
        let root_path = Path::new(workspace_root.path());

        self.search_directory_for_symbol(root_path, symbol_name)
    }

    fn search_directory_for_symbol(
        &self,
        dir: &std::path::Path,
        symbol_name: &str,
    ) -> Option<(Url, SymbolInfo)> {
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

                if file_name.starts_with('.')
                    || file_name == "target"
                    || file_name == "node_modules"
                {
                    continue;
                }

                if let Some(result) = self.search_directory_for_symbol(&path, symbol_name) {
                    return Some(result);
                }
            }
        }

        None
    }

    fn find_references_in_statements(
        &self,
        statements: &[Statement],
        symbols: &mut HashMap<String, SymbolInfo>,
    ) {
        for stmt in statements {
            match stmt {
                Statement::Expression { expr, .. } => {
                    self.find_references_in_expression(expr, symbols)
                }
                Statement::Return { expr, .. } => self.find_references_in_expression(expr, symbols),
                Statement::VariableDeclaration {
                    initializer: Some(expr),
                    ..
                } => {
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

    fn find_references_in_expression(
        &self,
        expr: &Expression,
        symbols: &mut HashMap<String, SymbolInfo>,
    ) {
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
            Expression::MethodCall {
                object,
                method: _,
                args,
            } => {
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

    fn extract_variables_from_statements(
        &self,
        statements: &[Statement],
        content: &str,
        symbols: &mut HashMap<String, SymbolInfo>,
    ) {
        for stmt in statements {
            match stmt {
                Statement::VariableDeclaration {
                    name,
                    type_,
                    initializer,
                    ..
                } => {
                    // Find the position of this variable in the content
                    if let Some((line, char_pos)) = self.find_variable_position(content, name) {
                        let name_end = char_pos + name.len();
                        let range = Range {
                            start: Position {
                                line: line as u32,
                                character: char_pos as u32,
                            },
                            end: Position {
                                line: line as u32,
                                character: name_end as u32,
                            },
                        };

                        // Determine type information
                        let type_info = if let Some(ref _t) = type_ {
                            type_.clone()
                        } else if let Some(ref init) = initializer {
                            // Try to infer type from initializer using compiler integration
                            let mut inferred_type: Option<AstType> = None;
                            for doc in self.documents.values().take(5) {
                                if let Some(ast) = &doc.ast {
                                    let program = Program {
                                        declarations: ast.clone(),
                                        statements: vec![],
                                    };
                                    let mut compiler_integration = CompilerIntegration::new();
                                    if let Ok(ast_type) =
                                        compiler_integration.infer_expression_type(&program, init)
                                    {
                                        inferred_type = Some(ast_type);
                                        break;
                                    }
                                }
                            }
                            inferred_type
                        } else {
                            None
                        };

                        let detail = if let Some(ref t) = &type_info {
                            Some(format!("{}: {}", name, format_type(t)))
                        } else if let Some(ref init) = initializer {
                            // Try to infer type from initializer for display
                            if let Some(inferred) = self.infer_type_from_expression(init) {
                                Some(format!("{}: {}", name, inferred))
                            } else {
                                Some(name.clone())
                            }
                        } else {
                            Some(name.clone())
                        };

                        symbols.insert(
                            name.clone(),
                            SymbolInfo {
                                name: name.clone(),
                                kind: SymbolKind::VARIABLE,
                                range: range.clone(),
                                selection_range: range,
                                detail,
                                documentation: None,
                                type_info,
                                definition_uri: None,
                                references: Vec::new(),
                                enum_variants: None,
                            },
                        );
                    }
                }
                Statement::Loop { body, .. } => {
                    self.extract_variables_from_statements(body, content, symbols);
                }
                _ => {}
            }
        }
    }

    fn find_variable_position(&self, content: &str, var_name: &str) -> Option<(usize, usize)> {
        for (line_num, line) in content.lines().enumerate() {
            // Look for variable declaration pattern: name = or name: Type =
            if let Some(eq_pos) = line.find('=') {
                let before_eq = line[..eq_pos].trim();
                // Check if it matches our variable name
                if before_eq == var_name || before_eq.ends_with(&format!(" {}", var_name)) {
                    if let Some(char_pos) = line.find(var_name) {
                        return Some((line_num, char_pos));
                    }
                }
            }
            // Also check for name: Type = pattern
            if let Some(colon_pos) = line.find(':') {
                let before_colon = line[..colon_pos].trim();
                if before_colon == var_name {
                    if let Some(char_pos) = line.find(var_name) {
                        return Some((line_num, char_pos));
                    }
                }
            }
        }
        None
    }
}
