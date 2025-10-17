# Zen Language LSP Refactoring Guide

## Executive Summary

This document provides a comprehensive, step-by-step guide to refactor the Zen Language LSP server from a monolithic, string-based implementation to a modular, AST-based architecture that properly leverages the compiler infrastructure.

## Current State Analysis

### Problems Identified

1. **God File**: `src/lsp/server.rs` (~6000 lines) contains all LSP logic
2. **String-Based Analysis**: Features use string searching instead of AST traversal
3. **Hardcoded Knowledge**: Type methods are hardcoded in arrays instead of queried from compiler
4. **Semantic Checks in LSP**: Logic like allocator checking and exhaustiveness should be in the type checker
5. **Missing Span Information**: AST nodes don't track their source positions

### Current Module Structure

```
src/lsp/
├── mod.rs              # Module exports
├── server.rs           # Monolithic server (6000 lines)
├── types.rs            # Data structures
├── document_store.rs   # Document management + analysis
├── utils.rs            # Utility functions
├── symbols.rs          # (exists)
├── navigation.rs       # (exists)
├── completion.rs       # (exists)
├── formatting.rs       # (exists)
└── type_inference.rs   # (exists)
```

## Refactoring Plan

### Phase 1: Add Span Information to AST (FOUNDATIONAL)

**Goal**: Every AST node must know its source location.

#### Step 1.1: Enhance the Span Type

File: `src/ast/span.rs`

```rust
//! Source position and span tracking

use std::fmt;

/// Source position in the input text
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Position {
    pub line: usize,
    pub column: usize,
    pub offset: usize, // byte offset from start
}

impl Position {
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self { line, column, offset }
    }

    pub fn contains(&self, offset: usize) -> bool {
        self.offset == offset
    }
}

/// Source span representing a range in the input text
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    pub fn dummy() -> Self {
        Self::default()
    }

    /// Check if this span contains a given byte offset
    pub fn contains(&self, offset: usize) -> bool {
        offset >= self.start.offset && offset < self.end.offset
    }

    /// Merge two spans into one that encompasses both
    pub fn merge(self, other: Span) -> Span {
        Span {
            start: if self.start.offset < other.start.offset {
                self.start
            } else {
                other.start
            },
            end: if self.end.offset > other.end.offset {
                self.end
            } else {
                other.end
            },
        }
    }

    /// Convert to LSP Range
    pub fn to_lsp_range(&self) -> lsp_types::Range {
        lsp_types::Range {
            start: lsp_types::Position {
                line: self.start.line as u32,
                character: self.start.column as u32,
            },
            end: lsp_types::Position {
                line: self.end.line as u32,
                character: self.end.column as u32,
            },
        }
    }

    /// Create from LSP position and content
    pub fn from_lsp_position(content: &str, position: lsp_types::Position) -> Option<Span> {
        let target_line = position.line as usize;
        let target_col = position.character as usize;

        let mut offset = 0;
        for (line_idx, line) in content.lines().enumerate() {
            if line_idx == target_line {
                offset += target_col;
                let pos = Position::new(target_line, target_col, offset);
                return Some(Span::new(pos, pos));
            }
            offset += line.len() + 1; // +1 for newline
        }
        None
    }
}
```

#### Step 1.2: Add Span to Expression

**Challenge**: Adding spans to every Expression variant is a breaking change.

**Two Approaches**:

**Option A: Wrapper Pattern (Non-Breaking, Incremental)**

```rust
// In src/ast/expressions.rs

use super::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct SpannedExpression {
    pub expr: Expression,
    pub span: Span,
}

impl SpannedExpression {
    pub fn new(expr: Expression, span: Span) -> Self {
        Self { expr, span }
    }

    pub fn dummy(expr: Expression) -> Self {
        Self { expr, span: Span::dummy() }
    }
}

// Keep existing Expression enum unchanged for now
// Gradually migrate to SpannedExpression
```

**Option B: Add span to each variant (Breaking, but Complete)**

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Identifier { name: String, span: Span },
    FunctionCall { name: String, args: Vec<Expression>, span: Span },
    MethodCall { object: Box<Expression>, method: String, args: Vec<Expression>, span: Span },
    // ... all other variants get a span field
}

impl Expression {
    /// Get the span for this expression
    pub fn span(&self) -> Span {
        match self {
            Expression::Identifier { span, .. } => *span,
            Expression::FunctionCall { span, .. } => *span,
            // ... all variants
        }
    }
}
```

**Recommendation**: Use Option A initially, migrate to Option B in a separate PR.

#### Step 1.3: Update Lexer to Track Positions

The lexer must track byte offsets, line, and column.

```rust
// In src/lexer.rs

pub struct Lexer<'a> {
    input: &'a str,
    position: usize,  // current byte offset
    line: usize,
    column: usize,
    // ... existing fields
}

impl<'a> Lexer<'a> {
    pub fn current_position(&self) -> Position {
        Position::new(self.line, self.column, self.position)
    }

    // Update advance() to track line/column
    fn advance(&mut self) {
        if let Some(ch) = self.current_char() {
            if ch == '\n' {
                self.line += 1;
                self.column = 0;
            } else {
                self.column += 1;
            }
            self.position += ch.len_utf8();
        }
    }
}
```

#### Step 1.4: Update Parser to Capture Spans

```rust
// In src/parser/*.rs files

impl Parser {
    fn parse_identifier(&mut self) -> Result<Expression, CompileError> {
        let start = self.lexer.current_position();
        let name = self.expect_identifier()?;
        let end = self.lexer.current_position();

        Ok(Expression::Identifier {
            name,
            span: Span::new(start, end),
        })
    }

    // Apply to all parse methods...
}
```

---

### Phase 2: Create LSP Core Modules

#### Step 2.1: Create `src/lsp/state.rs`

This replaces/consolidates the state management.

```rust
//! LSP server state management

use lsp_types::*;
use std::collections::HashMap;
use std::sync::Arc;
use crate::ast::{Declaration, AstType, Span};
use crate::typechecker::TypeEnvironment;

/// Analysis cache - the result of a full compiler pass
#[derive(Debug, Clone)]
pub struct AnalysisCache {
    pub ast: Arc<Vec<Declaration>>,
    pub diagnostics: Vec<Diagnostic>,
    pub type_env: Arc<TypeEnvironment>,  // From type checker
    pub symbols: Arc<SymbolTable>,       // Resolved symbols
}

/// Symbol table with full type information
#[derive(Debug, Clone, Default)]
pub struct SymbolTable {
    /// Maps symbol names to their definitions
    pub definitions: HashMap<String, SymbolDef>,
    /// Maps byte offsets to symbols at that location
    pub by_position: HashMap<usize, String>,
}

#[derive(Debug, Clone)]
pub struct SymbolDef {
    pub name: String,
    pub kind: SymbolKind,
    pub span: Span,
    pub type_: Option<AstType>,
    pub documentation: Option<String>,
    pub definition_uri: Option<Url>,
    pub enum_variants: Option<Vec<String>>,
}

/// Represents a single document
#[derive(Debug, Clone)]
pub struct Document {
    pub uri: Url,
    pub version: i32,
    pub content: Arc<String>,
    pub cache: Option<Arc<AnalysisCache>>,
}

/// The global document store
#[derive(Debug)]
pub struct DocumentStore {
    pub documents: HashMap<Url, Document>,
    pub stdlib_symbols: Arc<SymbolTable>,
    pub workspace_symbols: Arc<SymbolTable>,
    pub workspace_root: Option<Url>,
}

impl DocumentStore {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            stdlib_symbols: Arc::new(SymbolTable::default()),
            workspace_symbols: Arc::new(SymbolTable::default()),
            workspace_root: None,
        }
    }

    pub fn get_document(&self, uri: &Url) -> Option<&Document> {
        self.documents.get(uri)
    }

    pub fn open(&mut self, uri: Url, version: i32, content: String) {
        let doc = Document {
            uri: uri.clone(),
            version,
            content: Arc::new(content),
            cache: None,  // Will be populated by background analysis
        };
        self.documents.insert(uri, doc);
    }

    pub fn update(&mut self, uri: Url, version: i32, content: String) {
        if let Some(doc) = self.documents.get_mut(&uri) {
            doc.version = version;
            doc.content = Arc::new(content);
            doc.cache = None;  // Invalidate cache
        }
    }

    pub fn update_cache(&mut self, uri: &Url, cache: AnalysisCache) {
        if let Some(doc) = self.documents.get_mut(uri) {
            doc.cache = Some(Arc::new(cache));
        }
    }
}
```

#### Step 2.2: Create `src/lsp/analysis.rs`

This is the heart of the refactoring: AST-based queries.

```rust
//! AST-based analysis for LSP features

use crate::ast::{Expression, Declaration, Statement, Span};
use crate::lsp::state::{AnalysisCache, SymbolTable, SymbolDef};

/// Find the most specific AST node at a given byte offset
pub fn find_node_at_position(
    ast: &[Declaration],
    offset: usize,
) -> Option<AstNode> {
    // Depth-first search to find the smallest node containing offset
    for decl in ast {
        if let Some(node) = find_in_declaration(decl, offset) {
            return Some(node);
        }
    }
    None
}

#[derive(Debug, Clone)]
pub enum AstNode {
    Identifier { name: String, span: Span },
    FunctionCall { name: String, span: Span },
    MethodCall { receiver_span: Span, method: String, method_span: Span },
    // ... other node types
}

fn find_in_declaration(decl: &Declaration, offset: usize) -> Option<AstNode> {
    match decl {
        Declaration::Function(func) => {
            // Check if offset is in function name
            if func.name_span.contains(offset) {
                return Some(AstNode::Identifier {
                    name: func.name.clone(),
                    span: func.name_span,
                });
            }
            // Recurse into function body
            for stmt in &func.body {
                if let Some(node) = find_in_statement(stmt, offset) {
                    return Some(node);
                }
            }
            None
        }
        // ... other declarations
        _ => None,
    }
}

fn find_in_expression(expr: &Expression, offset: usize) -> Option<AstNode> {
    let span = expr.span();
    if !span.contains(offset) {
        return None;
    }

    match expr {
        Expression::Identifier { name, span } => {
            Some(AstNode::Identifier { name: name.clone(), span: *span })
        }
        Expression::MethodCall { object, method, span, .. } => {
            // First check if we're on the method name itself
            // This requires the parser to track method_span separately
            // For now, return method call node
            Some(AstNode::MethodCall {
                receiver_span: object.span(),
                method: method.clone(),
                method_span: *span,  // Approximate
            })
        }
        // Recurse into sub-expressions...
        _ => None,
    }
}

/// Look up a symbol in the cache
pub fn lookup_symbol(
    cache: &AnalysisCache,
    name: &str,
) -> Option<&SymbolDef> {
    cache.symbols.definitions.get(name)
}

/// Get the type of an expression from the type environment
pub fn get_expression_type(
    cache: &AnalysisCache,
    expr_span: Span,
) -> Option<&crate::ast::AstType> {
    // Query the type environment for the type at this span
    cache.type_env.get_type_at_span(expr_span)
}
```

#### Step 2.3: Create `src/lsp/background.rs`

Move the background analysis worker here.

```rust
//! Background analysis worker thread

use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use lsp_server::{Connection, Message};
use lsp_types::*;

use crate::parser::Parser;
use crate::lexer::Lexer;
use crate::typechecker::TypeChecker;
use crate::ast::Program;
use crate::lsp::state::{AnalysisCache, SymbolTable, DocumentStore};
use crate::lsp::utils::compile_error_to_diagnostic;

pub struct AnalysisJob {
    pub uri: Url,
    pub version: i32,
    pub content: String,
}

pub struct AnalysisResult {
    pub uri: Url,
    pub version: i32,
    pub cache: AnalysisCache,
}

pub fn start_background_worker(
    job_rx: Receiver<AnalysisJob>,
    connection: Connection,
    store: Arc<Mutex<DocumentStore>>,
) {
    std::thread::spawn(move || {
        eprintln!("[LSP] Background analysis worker started");

        while let Ok(job) = job_rx.recv() {
            match analyze_document(&job) {
                Ok(cache) => {
                    // Update the document store
                    if let Ok(mut store) = store.lock() {
                        store.update_cache(&job.uri, cache.clone());
                    }

                    // Send diagnostics to client
                    let diagnostics = cache.diagnostics.clone();
                    let notification = lsp_server::Notification::new(
                        "textDocument/publishDiagnostics".to_string(),
                        PublishDiagnosticsParams {
                            uri: job.uri.clone(),
                            diagnostics,
                            version: Some(job.version),
                        },
                    );

                    let _ = connection.sender.send(Message::Notification(notification));
                }
                Err(e) => {
                    eprintln!("[LSP] Analysis error for {}: {:?}", job.uri, e);
                }
            }
        }

        eprintln!("[LSP] Background analysis worker stopped");
    });
}

fn analyze_document(job: &AnalysisJob) -> Result<AnalysisCache, String> {
    // Parse
    let lexer = Lexer::new(&job.content);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program()
        .map_err(|e| format!("Parse error: {:?}", e))?;

    // Type check
    let mut type_checker = TypeChecker::new();
    let mut diagnostics = Vec::new();

    if let Err(err) = type_checker.check_program(&program) {
        diagnostics.push(compile_error_to_diagnostic(err));
    }

    // Extract symbols
    let symbols = extract_symbols(&program);

    Ok(AnalysisCache {
        ast: Arc::new(program.declarations),
        diagnostics,
        type_env: Arc::new(type_checker.into_environment()),
        symbols: Arc::new(symbols),
    })
}

fn extract_symbols(program: &Program) -> SymbolTable {
    let mut table = SymbolTable::default();

    for decl in &program.declarations {
        // Extract symbol definitions from declarations
        // Use span information to populate by_position map
        // ...
    }

    table
}
```

#### Step 2.4: Create `src/lsp/handlers/` directory

Create a handlers module with individual files for each LSP feature.

```
src/lsp/handlers/
├── mod.rs
├── hover.rs
├── completion.rs
├── definition.rs
├── references.rs
├── symbols.rs
├── formatting.rs
└── semantic_tokens.rs
```

**Example: `src/lsp/handlers/hover.rs`**

```rust
//! Hover information handler

use lsp_types::*;
use crate::lsp::state::{DocumentStore, AnalysisCache};
use crate::lsp::analysis::{find_node_at_position, lookup_symbol, AstNode};
use crate::ast::Span;

pub fn handle_hover(
    store: &DocumentStore,
    params: HoverParams,
) -> Option<Hover> {
    let uri = &params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    // Get document
    let doc = store.get_document(uri)?;
    let cache = doc.cache.as_ref()?;

    // Convert LSP position to byte offset
    let span = Span::from_lsp_position(&doc.content, position)?;
    let offset = span.start.offset;

    // Find AST node at this position
    let node = find_node_at_position(&cache.ast, offset)?;

    // Look up symbol information
    match node {
        AstNode::Identifier { name, .. } => {
            let symbol = lookup_symbol(&cache, &name)?;

            let hover_text = format!(
                "```zen\n{}: {}\n```\n{}",
                symbol.name,
                symbol.type_.as_ref()
                    .map(|t| format!("{:?}", t))
                    .unwrap_or_else(|| "unknown".to_string()),
                symbol.documentation.as_deref().unwrap_or("")
            );

            Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: hover_text,
                }),
                range: Some(symbol.span.to_lsp_range()),
            })
        }
        _ => None,
    }
}
```

---

### Phase 3: Move Semantic Checks to Type Checker

#### Step 3.1: Allocator Checking

Move `check_allocator_usage` from DocumentStore to TypeChecker.

```rust
// In src/typechecker/mod.rs

impl TypeChecker {
    pub fn check_program(&mut self, program: &Program) -> Result<(), CompileError> {
        for decl in &program.declarations {
            self.check_declaration(decl)?;
        }

        // NEW: Check allocator usage
        for decl in &program.declarations {
            if let Declaration::Function(func) = decl {
                self.check_allocator_usage(&func.body)?;
            }
        }

        Ok(())
    }

    fn check_allocator_usage(&self, statements: &[Statement]) -> Result<(), CompileError> {
        for stmt in statements {
            // Check each expression
            // If HashMap::new() or similar without allocator, return error
            // Use span information for precise error location
        }
        Ok(())
    }
}
```

Update CompileError:

```rust
// In src/error.rs

pub enum CompileError {
    // ... existing errors
    MissingAllocator {
        type_name: String,
        span: Span,
        help: String,
    },
    NonExhaustiveMatch {
        span: Span,
        missing_variants: Vec<String>,
    },
}
```

#### Step 3.2: Exhaustiveness Checking

Move `check_pattern_exhaustiveness` to TypeChecker.

```rust
impl TypeChecker {
    fn check_pattern_match_exhaustiveness(
        &self,
        scrutinee_type: &AstType,
        arms: &[PatternArm],
        span: Span,
    ) -> Result<(), CompileError> {
        let required_variants = self.get_enum_variants(scrutinee_type)?;
        let covered_variants = self.extract_covered_variants(arms);

        let missing: Vec<String> = required_variants
            .into_iter()
            .filter(|v| !covered_variants.contains(v))
            .collect();

        if !missing.is_empty() && !self.has_wildcard(arms) {
            return Err(CompileError::NonExhaustiveMatch {
                span,
                missing_variants: missing,
            });
        }

        Ok(())
    }
}
```

---

### Phase 4: Type Checker API for LSP

#### Step 4.1: Add Method Lookup to TypeChecker

```rust
// In src/typechecker/mod.rs

impl TypeChecker {
    /// Get all methods available for a given type
    pub fn get_methods_for_type(&self, type_: &AstType) -> Vec<MethodInfo> {
        match type_ {
            AstType::Named(name) if name == "String" => {
                self.get_string_methods()
            }
            AstType::Generic { name, .. } if name == "HashMap" => {
                self.get_hashmap_methods()
            }
            AstType::Named(name) => {
                // Look up user-defined type
                self.get_user_defined_methods(name)
            }
            _ => Vec::new(),
        }
    }

    fn get_string_methods(&self) -> Vec<MethodInfo> {
        vec![
            MethodInfo {
                name: "len".to_string(),
                params: vec![],
                return_type: AstType::Named("i32".to_string()),
                documentation: Some("Returns the length of the string".to_string()),
            },
            MethodInfo {
                name: "split".to_string(),
                params: vec![("delimiter".to_string(), AstType::Named("String".to_string()))],
                return_type: AstType::Generic {
                    name: "Vec".to_string(),
                    type_params: vec![AstType::Named("String".to_string())],
                },
                documentation: Some("Splits the string by delimiter".to_string()),
            },
            // ... all String methods
        ]
    }

    fn get_user_defined_methods(&self, type_name: &str) -> Vec<MethodInfo> {
        // Search the environment for functions whose first parameter is type_name
        self.env.functions.values()
            .filter(|func| {
                func.args.first()
                    .map(|(_, ty)| matches!(ty, AstType::Named(n) if n == type_name))
                    .unwrap_or(false)
            })
            .map(|func| MethodInfo {
                name: func.name.clone(),
                params: func.args[1..].to_vec(),  // Skip self parameter
                return_type: func.return_type.clone(),
                documentation: None,
            })
            .collect()
    }
}

pub struct MethodInfo {
    pub name: String,
    pub params: Vec<(String, AstType)>,
    pub return_type: AstType,
    pub documentation: Option<String>,
}
```

---

### Phase 5: Refactor LSP Feature Handlers

#### Step 5.1: Update Completion Handler

```rust
// In src/lsp/handlers/completion.rs

use crate::lsp::analysis::{find_node_at_position, get_expression_type, AstNode};
use crate::typechecker::TypeChecker;

pub fn handle_completion(
    store: &DocumentStore,
    type_checker: &TypeChecker,
    params: CompletionParams,
) -> Option<CompletionResponse> {
    let uri = &params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;

    let doc = store.get_document(uri)?;
    let cache = doc.cache.as_ref()?;

    // Convert position to offset
    let span = Span::from_lsp_position(&doc.content, position)?;
    let offset = span.start.offset;

    // Find node at position
    let node = find_node_at_position(&cache.ast, offset)?;

    // Determine completion context
    match node {
        AstNode::MethodCall { receiver_span, .. } => {
            // Get type of receiver
            let receiver_type = get_expression_type(&cache, receiver_span)?;

            // Query type checker for methods
            let methods = type_checker.get_methods_for_type(receiver_type);

            let completions: Vec<CompletionItem> = methods
                .into_iter()
                .map(|method| CompletionItem {
                    label: method.name.clone(),
                    kind: Some(CompletionItemKind::METHOD),
                    detail: Some(format!(
                        "({}) -> {}",
                        method.params.iter()
                            .map(|(name, ty)| format!("{}: {:?}", name, ty))
                            .collect::<Vec<_>>()
                            .join(", "),
                        format!("{:?}", method.return_type)
                    )),
                    documentation: method.documentation.map(|doc| {
                        Documentation::MarkupContent(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: doc,
                        })
                    }),
                    ..Default::default()
                })
                .collect();

            Some(CompletionResponse::Array(completions))
        }
        _ => {
            // General completions: all symbols in scope
            Some(complete_all_symbols(&cache))
        }
    }
}
```

#### Step 5.2: Update Definition Handler

```rust
// In src/lsp/handlers/definition.rs

pub fn handle_definition(
    store: &DocumentStore,
    params: GotoDefinitionParams,
) -> Option<GotoDefinitionResponse> {
    let uri = &params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    let doc = store.get_document(uri)?;
    let cache = doc.cache.as_ref()?;

    let span = Span::from_lsp_position(&doc.content, position)?;
    let node = find_node_at_position(&cache.ast, span.start.offset)?;

    match node {
        AstNode::Identifier { name, .. } => {
            // Look up definition in symbol table
            let symbol = lookup_symbol(&cache, &name)?;

            Some(GotoDefinitionResponse::Scalar(Location {
                uri: symbol.definition_uri.clone()?,
                range: symbol.span.to_lsp_range(),
            }))
        }
        _ => None,
    }
}
```

---

### Phase 6: Update Server Main Loop

#### Step 6.1: Refactor `src/lsp/server.rs`

```rust
//! Zen Language Server main loop

use lsp_server::{Connection, Message, Request, Response};
use lsp_types::*;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;

use crate::lsp::state::DocumentStore;
use crate::lsp::background::{start_background_worker, AnalysisJob};
use crate::lsp::handlers;
use crate::typechecker::TypeChecker;

pub struct ZenLanguageServer {
    connection: Connection,
    store: Arc<Mutex<DocumentStore>>,
    job_sender: mpsc::Sender<AnalysisJob>,
    type_checker: Arc<TypeChecker>,  // Shared type checker for queries
}

impl ZenLanguageServer {
    pub fn new(connection: Connection) -> Self {
        let store = Arc::new(Mutex::new(DocumentStore::new()));
        let (job_tx, job_rx) = mpsc::channel();

        // Start background worker
        start_background_worker(job_rx, connection.clone(), Arc::clone(&store));

        Self {
            connection,
            store,
            job_sender: job_tx,
            type_checker: Arc::new(TypeChecker::new()),
        }
    }

    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        eprintln!("[LSP] Server starting main loop");

        for msg in &self.connection.receiver {
            match msg {
                Message::Request(req) => self.handle_request(req)?,
                Message::Notification(not) => self.handle_notification(not)?,
                Message::Response(_) => {}
            }
        }

        Ok(())
    }

    fn handle_request(&self, req: Request) -> Result<(), Box<dyn std::error::Error>> {
        match req.method.as_str() {
            "textDocument/hover" => {
                let params: HoverParams = serde_json::from_value(req.params)?;
                let store = self.store.lock().unwrap();
                let result = handlers::hover::handle_hover(&store, params);
                self.send_response(Response::new_ok(req.id, result))?;
            }
            "textDocument/completion" => {
                let params: CompletionParams = serde_json::from_value(req.params)?;
                let store = self.store.lock().unwrap();
                let result = handlers::completion::handle_completion(
                    &store,
                    &self.type_checker,
                    params,
                );
                self.send_response(Response::new_ok(req.id, result))?;
            }
            "textDocument/definition" => {
                let params: GotoDefinitionParams = serde_json::from_value(req.params)?;
                let store = self.store.lock().unwrap();
                let result = handlers::definition::handle_definition(&store, params);
                self.send_response(Response::new_ok(req.id, result))?;
            }
            // ... other handlers
            _ => {
                eprintln!("[LSP] Unhandled request: {}", req.method);
            }
        }
        Ok(())
    }

    fn handle_notification(&self, not: lsp_server::Notification) -> Result<(), Box<dyn std::error::Error>> {
        match not.method.as_str() {
            "textDocument/didOpen" => {
                let params: DidOpenTextDocumentParams = serde_json::from_value(not.params)?;
                let mut store = self.store.lock().unwrap();
                store.open(
                    params.text_document.uri.clone(),
                    params.text_document.version,
                    params.text_document.text,
                );

                // Send analysis job
                self.job_sender.send(AnalysisJob {
                    uri: params.text_document.uri,
                    version: params.text_document.version,
                    content: params.text_document.text,
                })?;
            }
            "textDocument/didChange" => {
                let params: DidChangeTextDocumentParams = serde_json::from_value(not.params)?;
                let content = params.content_changes[0].text.clone();

                let mut store = self.store.lock().unwrap();
                store.update(
                    params.text_document.uri.clone(),
                    params.text_document.version,
                    content.clone(),
                );

                // Send analysis job
                self.job_sender.send(AnalysisJob {
                    uri: params.text_document.uri,
                    version: params.text_document.version,
                    content,
                })?;
            }
            _ => {}
        }
        Ok(())
    }

    fn send_response(&self, response: Response) -> Result<(), Box<dyn std::error::Error>> {
        self.connection.sender.send(Message::Response(response))?;
        Ok(())
    }
}
```

---

## Migration Strategy

### Recommended Order

1. **Week 1**: Implement Span tracking (Phase 1)
   - Add span to Lexer
   - Use wrapper pattern for Expression
   - Update a few key parser functions

2. **Week 2**: Create new modules (Phase 2)
   - Create state.rs, analysis.rs, background.rs
   - Implement find_node_at_position

3. **Week 3**: Move semantic checks (Phase 3)
   - Refactor TypeChecker
   - Update CompileError

4. **Week 4**: Type checker API (Phase 4)
   - Implement get_methods_for_type

5. **Week 5**: Refactor handlers (Phase 5)
   - Update completion, hover, definition

6. **Week 6**: Server refactoring (Phase 6)
   - Simplify main loop
   - Remove old code

### Testing Strategy

1. Create integration tests for each handler
2. Test with real Zen code samples
3. Benchmark performance (should be faster after refactoring)
4. Ensure all LSP features still work

---

## Benefits After Refactoring

1. **Correctness**: No more false positives from string matching
2. **Performance**: Pre-computed analysis cache, no repeated parsing
3. **Maintainability**: Each handler is ~50-100 lines instead of mixed into 6000-line file
4. **Extensibility**: Easy to add new LSP features
5. **Consistency**: Compiler and LSP always report the same errors
6. **Type Safety**: Full use of Rust's type system instead of stringly-typed code

---

## File Structure After Refactoring

```
src/lsp/
├── mod.rs                  # Module exports (~30 lines)
├── server.rs               # Main loop (~200 lines)
├── state.rs                # State management (~300 lines)
├── analysis.rs             # AST queries (~400 lines)
├── background.rs           # Background worker (~200 lines)
├── utils.rs                # Utilities (~100 lines)
└── handlers/
    ├── mod.rs              # Handler exports (~20 lines)
    ├── hover.rs            # Hover handler (~100 lines)
    ├── completion.rs       # Completion handler (~200 lines)
    ├── definition.rs       # Definition handler (~80 lines)
    ├── references.rs       # References handler (~100 lines)
    ├── symbols.rs          # Document symbols (~100 lines)
    ├── formatting.rs       # Formatting (~100 lines)
    └── semantic_tokens.rs  # Semantic tokens (~150 lines)
```

**Total: ~2000 lines** (vs. 6000 lines monolithic)

---

## Conclusion

This refactoring transforms the LSP from a "performative" string-based implementation to a proper compiler-integrated LSP server. The key insight is: **the AST is the single source of truth**. All IDE features should query the AST, not search strings.

By following this guide step-by-step, you'll end up with a maintainable, correct, and fast LSP server that scales with your language's growth.
