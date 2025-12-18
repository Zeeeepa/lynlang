//! Compiler intrinsics type checking
//! Uses stdlib_metadata as the single source of truth for intrinsic types

use crate::ast::AstType;
use crate::error::Result;
use crate::stdlib_metadata::compiler as compiler_intrinsics;
use crate::well_known::well_known;

/// Check compiler intrinsic function calls and return their type
/// Returns None if not a compiler intrinsic, otherwise returns Ok(type) or error
pub fn check_compiler_intrinsic(
    module: &str,
    func: &str,
    args_len: usize,
) -> Option<Result<AstType>> {
    if module != "compiler" {
        return None;
    }

    compiler_intrinsics::check_intrinsic_call(func, args_len)
}

/// Check stdlib function calls and return their type
/// Returns None if not a known stdlib function, otherwise returns the type
pub fn check_stdlib_function(module: &str, func: &str) -> Option<AstType> {
    match (module, func) {
        // Core library functions
        ("core", "assert" | "panic") => Some(AstType::Void),

        // IO functions
        ("io", "print" | "println" | "print_int" | "print_float" | "eprint" | "eprintln") => {
            Some(AstType::Void)
        }
        ("io", "read_line" | "read_input") => Some(crate::ast::resolve_string_struct_type()),

        // Math functions
        ("math", "abs") => Some(AstType::I32),
        ("math", "sqrt") => Some(AstType::F64),
        ("math", "sin" | "cos" | "tan") => Some(AstType::F64),
        ("math", "floor" | "ceil") => Some(AstType::I32),
        ("math", "pow") => Some(AstType::F64),
        ("math", "min" | "max") => Some(AstType::I32),

        // String functions
        ("string", "len") => Some(AstType::I32),
        ("string", "concat") => Some(crate::ast::resolve_string_struct_type()),

        // Memory functions
        ("mem", "alloc") => Some(AstType::Ptr(Box::new(AstType::U8))),
        ("mem", "free") => Some(AstType::Void),

        // Filesystem functions
        ("fs", "read_file") => {
            let wk = well_known();
            let string_type = crate::ast::resolve_string_struct_type();
            Some(AstType::Generic {
                name: wk.result_name().to_string(),
                type_args: vec![string_type.clone(), string_type],
            })
        }
        ("fs", "write_file") => {
            let wk = well_known();
            Some(AstType::Generic {
                name: wk.result_name().to_string(),
                type_args: vec![AstType::Void, crate::ast::resolve_string_struct_type()],
            })
        }
        ("fs", "exists") => Some(AstType::Bool),
        ("fs", "remove_file") => {
            let wk = well_known();
            Some(AstType::Generic {
                name: wk.result_name().to_string(),
                type_args: vec![AstType::Void, crate::ast::resolve_string_struct_type()],
            })
        }
        ("fs", "create_dir") => {
            let wk = well_known();
            Some(AstType::Generic {
                name: wk.result_name().to_string(),
                type_args: vec![AstType::Void, crate::ast::resolve_string_struct_type()],
            })
        }

        _ => None,
    }
}
