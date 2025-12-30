// LSP module for Zen Language Server

// Submodules
pub mod analyzer;
pub mod helpers;
pub mod call_hierarchy;
pub mod code_action;
pub mod code_lens;
pub mod compiler_integration;
pub mod completion;
pub mod document_store;
pub mod formatting;
pub mod hover;
pub mod indexing;
pub mod inlay_hints;
pub mod navigation;
pub mod pattern_checking;
pub mod rename;
pub mod semantic_tokens;
pub mod server;
pub mod signature_help;
pub mod stdlib_resolver;
pub mod symbol_extraction;
pub mod symbols;
pub mod type_inference;
pub mod types;
pub mod utils;
pub mod workspace;

pub use server::ZenLanguageServer;

// Re-export commonly used types
pub use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionResponse, Diagnostic, DiagnosticSeverity,
    DocumentSymbol, Hover, HoverContents, Location, MarkedString, Position, Range,
    SymbolInformation, SymbolKind,
};

// Re-export internal types for convenience
pub use document_store::DocumentStore;
pub use types::{AnalysisJob, AnalysisResult, Document, SymbolInfo, ZenCompletionContext};
