// Navigation module - LSP navigation handlers (go-to-definition, references, etc.)

pub mod utils;
pub mod imports;
pub mod highlight;
pub mod type_definition;
pub mod references;
pub mod scope;

// Re-export handlers from legacy file for now (definition handler still needs extraction)
pub use super::navigation_legacy::handle_definition;
pub use highlight::handle_document_highlight;
pub use type_definition::handle_type_definition;
pub use references::handle_references;

