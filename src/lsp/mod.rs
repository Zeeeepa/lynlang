// LSP module for Zen Language Server

pub mod enhanced_server;

pub use enhanced_server::ZenLanguageServer;

// Re-export commonly used types
pub use lsp_types::{
    Diagnostic, DiagnosticSeverity, Position, Range, Location,
    CompletionItem, CompletionItemKind, CompletionResponse,
    Hover, HoverContents, MarkedString,
    DocumentSymbol, SymbolKind, SymbolInformation,
};