// LSP Types and Data Structures

use crate::ast::Program;
use crate::ast::{AstType, Declaration};
use crate::lexer::Token;
use lsp_types::*;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct Document {
    pub uri: Url,
    pub version: i32,
    pub content: String,
    pub tokens: Vec<Token>,
    pub ast: Option<Vec<Declaration>>,
    pub diagnostics: Vec<Diagnostic>,
    pub symbols: HashMap<String, SymbolInfo>,
    pub last_analysis: Option<Instant>,
    // Cached lines vector to avoid repeated allocations
    pub(crate) cached_lines: Option<Vec<String>>,
}

impl Document {
    /// Get lines efficiently, using cache if available
    pub fn get_lines(&mut self) -> Vec<&str> {
        // For now, just return lines() iterator collected - we'll optimize caching later
        // The cache would need to be invalidated on content change anyway
        self.content.lines().collect()
    }

    /// Get a specific line by index
    pub fn get_line(&self, index: usize) -> Option<&str> {
        self.content.lines().nth(index)
    }
}

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub range: Range,
    pub selection_range: Range,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub type_info: Option<AstType>,
    pub definition_uri: Option<Url>,
    pub references: Vec<Range>,
    pub enum_variants: Option<Vec<String>>, // For enums: list of variant names
}

#[derive(Debug, Clone)]
pub struct UfcMethodInfo {
    pub receiver: String,
    pub method_name: String,
}

#[derive(Debug)]
pub enum ZenCompletionContext {
    General,
    UfcMethod { receiver_type: String },
    ModulePath { base: String },
}

#[derive(Debug)]
pub enum SymbolScope {
    Local { function_name: String },
    ModuleLevel,
    Unknown,
}

// Background analysis job
#[derive(Debug, Clone)]
pub struct AnalysisJob {
    pub uri: Url,
    pub version: i32,
    pub content: String,
    pub program: Program,
}

// Background analysis result
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub uri: Url,
    pub version: i32,
    pub diagnostics: Vec<Diagnostic>,
}
