//! Compiler intrinsics type checking
//! Handles type checking for compiler.* functions and built-in operations

use crate::ast::AstType;
use crate::error::{CompileError, Result};

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

    let result = match func {
        "inline_c" => {
            if args_len != 1 {
                Err(CompileError::TypeError(
                    format!(
                        "compiler.inline_c() expects exactly 1 argument (string literal), got {}",
                        args_len
                    ),
                    None,
                ))
            } else {
                Ok(AstType::Void)
            }
        }
        "raw_allocate" => {
            if args_len != 1 {
                Err(CompileError::TypeError(
                    format!(
                        "compiler.raw_allocate() expects 1 argument (size: usize), got {}",
                        args_len
                    ),
                    None,
                ))
            } else {
                Ok(AstType::Ptr(Box::new(AstType::U8)))
            }
        }
        "raw_deallocate" => {
            if args_len != 2 {
                Err(CompileError::TypeError(
                    format!(
                        "compiler.raw_deallocate() expects 2 arguments (ptr, size), got {}",
                        args_len
                    ),
                    None,
                ))
            } else {
                Ok(AstType::Void)
            }
        }
        "raw_reallocate" => {
            if args_len != 3 {
                Err(CompileError::TypeError(
                    format!(
                        "compiler.raw_reallocate() expects 3 arguments (ptr, old_size, new_size), got {}",
                        args_len
                    ),
                    None,
                ))
            } else {
                Ok(AstType::Ptr(Box::new(AstType::U8)))
            }
        }
        "raw_ptr_offset" => {
            if args_len != 2 {
                Err(CompileError::TypeError(
                    format!(
                        "compiler.raw_ptr_offset() expects 2 arguments (ptr, offset), got {}",
                        args_len
                    ),
                    None,
                ))
            } else {
                Ok(AstType::RawPtr(Box::new(AstType::U8)))
            }
        }
        "raw_ptr_cast" => {
            if args_len != 1 {
                Err(CompileError::TypeError(
                    format!(
                        "compiler.raw_ptr_cast() expects 1 argument (ptr), got {}",
                        args_len
                    ),
                    None,
                ))
            } else {
                Ok(AstType::RawPtr(Box::new(AstType::U8)))
            }
        }
        "call_external" => {
            if args_len != 2 {
                Err(CompileError::TypeError(
                    format!(
                        "compiler.call_external() expects 2 arguments (func_ptr, args), got {}",
                        args_len
                    ),
                    None,
                ))
            } else {
                Ok(AstType::RawPtr(Box::new(AstType::U8)))
            }
        }
        "load_library" => {
            if args_len != 1 {
                Err(CompileError::TypeError(
                    format!(
                        "compiler.load_library() expects 1 argument (path: string), got {}",
                        args_len
                    ),
                    None,
                ))
            } else {
                Ok(AstType::RawPtr(Box::new(AstType::U8)))
            }
        }
        "get_symbol" => {
            if args_len != 2 {
                Err(CompileError::TypeError(
                    format!(
                        "compiler.get_symbol() expects 2 arguments (lib_handle, symbol_name), got {}",
                        args_len
                    ),
                    None,
                ))
            } else {
                Ok(AstType::RawPtr(Box::new(AstType::U8)))
            }
        }
        "unload_library" => {
            if args_len != 1 {
                Err(CompileError::TypeError(
                    format!(
                        "compiler.unload_library() expects 1 argument (lib_handle), got {}",
                        args_len
                    ),
                    None,
                ))
            } else {
                Ok(AstType::Void)
            }
        }
        "null_ptr" => {
            if args_len != 0 {
                Err(CompileError::TypeError(
                    format!("compiler.null_ptr() expects 0 arguments, got {}", args_len),
                    None,
                ))
            } else {
                Ok(AstType::Ptr(Box::new(AstType::U8)))
            }
        }
        "int_to_ptr" => {
            if args_len != 1 {
                Err(CompileError::TypeError(
                    format!(
                        "compiler.int_to_ptr() expects 1 argument (addr: i64), got {}",
                        args_len
                    ),
                    None,
                ))
            } else {
                Ok(AstType::RawPtr(Box::new(AstType::U8)))
            }
        }
        "ptr_to_int" => {
            if args_len != 1 {
                Err(CompileError::TypeError(
                    format!(
                        "compiler.ptr_to_int() expects 1 argument (ptr), got {}",
                        args_len
                    ),
                    None,
                ))
            } else {
                Ok(AstType::I64)
            }
        }
        "load" => {
            if args_len != 1 {
                Err(CompileError::TypeError(
                    format!("compiler.load() expects 1 argument (ptr), got {}", args_len),
                    None,
                ))
            } else {
                // Returns generic T - use I32 as default for type inference
                Ok(AstType::I32)
            }
        }
        "store" => {
            if args_len != 2 {
                Err(CompileError::TypeError(
                    format!(
                        "compiler.store() expects 2 arguments (ptr, value), got {}",
                        args_len
                    ),
                    None,
                ))
            } else {
                Ok(AstType::Void)
            }
        }
        // GEP intrinsics
        "gep" => {
            if args_len != 2 {
                Err(CompileError::TypeError(
                    format!(
                        "compiler.gep() expects 2 arguments (ptr, offset), got {}",
                        args_len
                    ),
                    None,
                ))
            } else {
                Ok(AstType::Ptr(Box::new(AstType::U8)))
            }
        }
        "gep_struct" => {
            if args_len != 2 {
                Err(CompileError::TypeError(
                    format!(
                        "compiler.gep_struct() expects 2 arguments (ptr, field_index), got {}",
                        args_len
                    ),
                    None,
                ))
            } else {
                Ok(AstType::Ptr(Box::new(AstType::U8)))
            }
        }
        // Enum intrinsics
        "discriminant" => {
            if args_len != 1 {
                Err(CompileError::TypeError(
                    format!(
                        "compiler.discriminant() expects 1 argument, got {}",
                        args_len
                    ),
                    None,
                ))
            } else {
                Ok(AstType::I64)
            }
        }
        "set_discriminant" => {
            if args_len != 2 {
                Err(CompileError::TypeError(
                    format!(
                        "compiler.set_discriminant() expects 2 arguments, got {}",
                        args_len
                    ),
                    None,
                ))
            } else {
                Ok(AstType::Void)
            }
        }
        "get_payload" => {
            if args_len != 1 {
                Err(CompileError::TypeError(
                    format!(
                        "compiler.get_payload() expects 1 argument, got {}",
                        args_len
                    ),
                    None,
                ))
            } else {
                Ok(AstType::Ptr(Box::new(AstType::U8)))
            }
        }
        "set_payload" => {
            if args_len != 2 {
                Err(CompileError::TypeError(
                    format!(
                        "compiler.set_payload() expects 2 arguments, got {}",
                        args_len
                    ),
                    None,
                ))
            } else {
                Ok(AstType::Void)
            }
        }
        _ => return None,
    };

    Some(result)
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
            let string_type = crate::ast::resolve_string_struct_type();
            Some(AstType::Generic {
                name: "Result".to_string(),
                type_args: vec![string_type.clone(), string_type],
            })
        }
        ("fs", "write_file") => Some(AstType::Generic {
            name: "Result".to_string(),
            type_args: vec![AstType::Void, crate::ast::resolve_string_struct_type()],
        }),
        ("fs", "exists") => Some(AstType::Bool),
        ("fs", "remove_file") => Some(AstType::Generic {
            name: "Result".to_string(),
            type_args: vec![AstType::Void, crate::ast::resolve_string_struct_type()],
        }),
        ("fs", "create_dir") => Some(AstType::Generic {
            name: "Result".to_string(),
            type_args: vec![AstType::Void, crate::ast::resolve_string_struct_type()],
        }),

        _ => None,
    }
}
