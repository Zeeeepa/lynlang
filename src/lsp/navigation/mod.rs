// Navigation module - LSP navigation handlers (go-to-definition, references, etc.)

pub mod utils;
pub mod imports;
pub mod highlight;
pub mod type_definition;
pub mod references;
pub mod scope;
pub mod ufc;
pub mod definition;

// Re-export all handlers
pub use definition::handle_definition;
pub use highlight::handle_document_highlight;
pub use type_definition::handle_type_definition;
pub use references::handle_references;

// Re-export utility functions for other modules
pub use utils::{find_symbol_at_position, find_symbol_definition_in_content};

