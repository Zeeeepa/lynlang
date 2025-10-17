// LSP module for Zen Language Server

// Submodules
pub mod types;
pub mod utils;
pub mod document_store;
pub mod type_inference;
pub mod symbols;
pub mod navigation;
pub mod completion;
pub mod formatting;
pub mod server;

pub use server::ZenLanguageServer;

// Re-export commonly used types
pub use lsp_types::{
    Diagnostic, DiagnosticSeverity, Position, Range, Location,
    CompletionItem, CompletionItemKind, CompletionResponse,
    Hover, HoverContents, MarkedString,
    DocumentSymbol, SymbolKind, SymbolInformation,
};

// Re-export internal types for convenience
pub use types::{Document, SymbolInfo, ZenCompletionContext, AnalysisJob, AnalysisResult};
pub use document_store::DocumentStore;