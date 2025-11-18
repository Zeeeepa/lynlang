// Navigation module - LSP navigation handlers (go-to-definition, references, etc.)

pub mod utils;

// Re-export handlers for backward compatibility
pub use super::navigation::definition::handle_definition;
pub use super::navigation::type_definition::handle_type_definition;
pub use super::navigation::references::handle_references;
pub use super::navigation::highlight::handle_document_highlight;

