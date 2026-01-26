//! Helper functions for type inference

use crate::ast::AstType;
use crate::stdlib_types::StdlibTypeRegistry;

/// Extract the type name from common AstType variants (Struct, Generic, Enum)
pub fn extract_type_name(ast_type: &AstType) -> Option<&str> {
    match ast_type {
        AstType::Struct { name, .. } => Some(name.as_str()),
        AstType::Generic { name, .. } => Some(name.as_str()),
        AstType::Enum { name, .. } => Some(name.as_str()),
        _ => None,
    }
}

/// Check if a type is any string type (static or dynamic)
pub fn is_string_type(ast_type: &AstType) -> bool {
    matches!(ast_type, AstType::StaticString | AstType::StaticLiteral)
        || matches!(ast_type, AstType::Struct { name, .. } if StdlibTypeRegistry::is_string_type(name))
}
