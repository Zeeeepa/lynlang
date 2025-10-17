// LSP Types and Data Structures

use lsp_types::*;
use std::time::Instant;
use std::collections::HashMap;
use crate::ast::{Declaration, AstType};
use crate::lexer::Token;
use crate::ast::Program;

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
    pub enum_variants: Option<Vec<String>>,  // For enums: list of variant names
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
