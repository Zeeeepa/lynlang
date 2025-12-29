pub mod behaviors;
pub mod comptime;
pub mod core;
pub mod enums;
pub mod expressions;
pub mod external;
pub mod functions;
pub mod patterns;
pub mod program;
pub mod statements;
pub mod structs;
pub mod types;

pub use core::Parser;

use crate::ast::AstType;
use crate::error::CompileError;

/// Parse a type from a string (used for parsing type arguments like sizeof<i32>)
pub fn parse_type_from_string(s: &str) -> Result<AstType, CompileError> {
    match s.trim() {
        "i8" => Ok(AstType::I8),
        "i16" => Ok(AstType::I16),
        "i32" => Ok(AstType::I32),
        "i64" => Ok(AstType::I64),
        "u8" => Ok(AstType::U8),
        "u16" => Ok(AstType::U16),
        "u32" => Ok(AstType::U32),
        "u64" => Ok(AstType::U64),
        "usize" => Ok(AstType::Usize),
        "f32" => Ok(AstType::F32),
        "f64" => Ok(AstType::F64),
        "bool" => Ok(AstType::Bool),
        "void" => Ok(AstType::Void),
        s if s.starts_with("RawPtr<") && s.ends_with(">") => {
            let inner = &s[7..s.len() - 1];
            let inner_type = parse_type_from_string(inner)?;
            Ok(AstType::raw_ptr(inner_type))
        }
        s if s.starts_with("Ptr<") && s.ends_with(">") => {
            let inner = &s[4..s.len() - 1];
            let inner_type = parse_type_from_string(inner)?;
            Ok(AstType::ptr(inner_type))
        }
        _ => {
            // For unknown types, return as a generic type name
            Ok(AstType::Generic {
                name: s.to_string(),
                type_args: vec![],
            })
        }
    }
}
