//! Standard library module registration for type checker
//! Handles registering @std module functions with their signatures

use super::{FunctionSignature, TypeChecker};
use crate::ast::AstType;
use crate::error::Result;

/// Register functions from a stdlib module
pub fn register_stdlib_module(checker: &mut TypeChecker, alias: &str, module_path: &str) -> Result<()> {
        match module_path {
            "math" => register_math_module(checker, alias),
            "io" => register_io_module(checker, alias),
            "core" => register_core_module(checker, alias),
            "compiler" => register_compiler_module(checker, alias),
            "get_default_allocator" => register_allocator_function(checker, alias),
            _ => {
                // Unknown stdlib module, but not an error
            }
        }
        Ok(())
    }

fn register_math_module(checker: &mut TypeChecker, alias: &str) {
        let math_funcs = vec![
            ("sqrt", vec![AstType::F64], AstType::F64),
            ("sin", vec![AstType::F64], AstType::F64),
            ("cos", vec![AstType::F64], AstType::F64),
            ("tan", vec![AstType::F64], AstType::F64),
            ("pow", vec![AstType::F64, AstType::F64], AstType::F64),
            ("exp", vec![AstType::F64], AstType::F64),
            ("log", vec![AstType::F64], AstType::F64),
            ("floor", vec![AstType::F64], AstType::F64),
            ("ceil", vec![AstType::F64], AstType::F64),
            ("round", vec![AstType::F64], AstType::F64),
            ("abs", vec![AstType::I64], AstType::I64), // For now, just i64 version
            ("min", vec![AstType::F64, AstType::F64], AstType::F64),
            ("max", vec![AstType::F64, AstType::F64], AstType::F64),
        ];

        for (name, args, ret) in math_funcs {
            register_function(checker, alias, name, args, ret);
        }
    }

fn register_io_module(checker: &mut TypeChecker, alias: &str) {
        let string_type = crate::ast::resolve_string_struct_type();
        let io_funcs = vec![
            ("print", vec![string_type.clone()], AstType::Void),
            ("print_int", vec![AstType::I64], AstType::Void),
            ("print_float", vec![AstType::F64], AstType::Void),
            ("println", vec![string_type.clone()], AstType::Void),
            ("read_line", vec![], string_type.clone()),
            ("read_file", vec![string_type.clone()], string_type.clone()),
            (
                "write_file",
                vec![string_type.clone(), string_type.clone()],
                AstType::Void,
            ),
        ];

        for (name, args, ret) in io_funcs {
            register_function(checker, alias, name, args, ret);
        }
    }

fn register_core_module(checker: &mut TypeChecker, alias: &str) {
        let core_funcs = vec![
            // sizeof and alignof are compile-time operations, skip for now
            ("assert", vec![AstType::Bool], AstType::Void),
            ("panic", vec![crate::ast::resolve_string_struct_type()], AstType::Void),
        ];

        for (name, args, ret) in core_funcs {
            register_function(checker, alias, name, args, ret);
        }
    }

fn register_compiler_module(checker: &mut TypeChecker, alias: &str) {
        // Register compiler intrinsics - these are handled specially in codegen
        // Using RawPtr(U8) for pointer types to match stdlib usage
        let raw_ptr_u8 = AstType::RawPtr(Box::new(AstType::U8));
        let ptr_u8 = AstType::Ptr(Box::new(AstType::U8));
        let compiler_funcs = vec![
            ("inline_c", vec![AstType::StaticString], AstType::Void),
            // Memory primitives
            ("raw_allocate", vec![AstType::Usize], raw_ptr_u8.clone()),
            ("raw_deallocate", vec![raw_ptr_u8.clone(), AstType::Usize], AstType::Void),
            ("raw_reallocate", vec![raw_ptr_u8.clone(), AstType::Usize, AstType::Usize], raw_ptr_u8.clone()),
            ("raw_ptr_offset", vec![raw_ptr_u8.clone(), AstType::I64], raw_ptr_u8.clone()),
            ("raw_ptr_cast", vec![raw_ptr_u8.clone()], raw_ptr_u8.clone()),
            ("call_external", vec![raw_ptr_u8.clone(), raw_ptr_u8.clone()], raw_ptr_u8.clone()),
            ("load_library", vec![AstType::StaticString], raw_ptr_u8.clone()),
            ("get_symbol", vec![raw_ptr_u8.clone(), AstType::StaticString], raw_ptr_u8.clone()),
            ("unload_library", vec![raw_ptr_u8.clone()], AstType::Void),
            ("null_ptr", vec![], raw_ptr_u8.clone()),
            // GEP intrinsics
            ("gep", vec![raw_ptr_u8.clone(), AstType::I64], raw_ptr_u8.clone()),
            ("gep_struct", vec![raw_ptr_u8.clone(), AstType::I32], raw_ptr_u8.clone()),
            // Enum intrinsics
            ("discriminant", vec![raw_ptr_u8.clone()], AstType::I32),
            ("set_discriminant", vec![raw_ptr_u8.clone(), AstType::I32], AstType::Void),
            ("get_payload", vec![raw_ptr_u8.clone()], raw_ptr_u8.clone()),
            ("set_payload", vec![raw_ptr_u8.clone(), raw_ptr_u8.clone()], AstType::Void),
            // Type introspection - sizeof takes a type parameter, returns usize
            // Note: sizeof(T) is special - it takes a type, not a value. Type checker handles this specially.
            ("sizeof", vec![], AstType::Usize),
            // Pointer <-> Integer conversion intrinsics
            ("ptr_to_int", vec![raw_ptr_u8.clone()], AstType::I64),
            ("int_to_ptr", vec![AstType::I64], raw_ptr_u8.clone()),
            // Memory load/store intrinsics (generic - actual type determined by usage)
            ("load", vec![raw_ptr_u8.clone()], AstType::I64), // Returns generic T, use I64 as placeholder
            ("store", vec![raw_ptr_u8.clone(), AstType::I64], AstType::Void), // Takes generic T
        ];

        for (name, args, ret) in compiler_funcs {
            register_function(checker, alias, name, args, ret);
        }
        
        // Also register with Ptr type for backwards compatibility
        let ptr_funcs = vec![
            ("raw_allocate", vec![AstType::Usize], ptr_u8.clone()),
            ("raw_deallocate", vec![ptr_u8.clone(), AstType::Usize], AstType::Void),
        ];
        
        for (name, args, ret) in ptr_funcs {
            let qualified_name = format!("{}.{}_ptr", alias, name);
            let params = args
                .into_iter()
                .enumerate()
                .map(|(i, t)| (format!("arg{}", i), t))
                .collect();
            checker.functions.insert(
                qualified_name,
                FunctionSignature {
                    params,
                    return_type: ret,
                    is_external: true,
                },
            );
        }
    }

fn register_allocator_function(checker: &mut TypeChecker, alias: &str) {
        // Special case: get_default_allocator is a function, not a module
        // Register it as a global function
        checker.functions.insert(
            alias.to_string(), // Use alias directly as the function name
            FunctionSignature {
                params: vec![], // No parameters
                return_type: AstType::Ptr(Box::new(AstType::Void)), // Returns opaque allocator pointer
                is_external: true,
            },
        );
    }

fn register_function(checker: &mut TypeChecker, alias: &str, name: &str, args: Vec<AstType>, ret: AstType) {
        let qualified_name = format!("{}.{}", alias, name);
        let params = args
            .into_iter()
            .enumerate()
            .map(|(i, t)| (format!("arg{}", i), t))
            .collect();
        checker.functions.insert(
            qualified_name,
            FunctionSignature {
                params,
                return_type: ret,
                is_external: true,
            },
        );
    }

