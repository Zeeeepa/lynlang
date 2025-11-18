// Navigation module - LSP navigation handlers (go-to-definition, references, etc.)

pub mod utils;
pub mod imports;
pub mod highlight;

// Re-export handlers from legacy file for now (will be moved incrementally)
pub use super::navigation_legacy::{handle_definition, handle_type_definition, handle_references};
pub use highlight::handle_document_highlight;

