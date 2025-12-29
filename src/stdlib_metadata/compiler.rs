use super::{StdFunction, StdModuleTrait};
use crate::ast::AstType;
use crate::error::CompileError;
use std::collections::HashMap;
use std::sync::OnceLock;

/// The @std.compiler module provides low-level compiler intrinsics
/// These are the ONLY primitives exposed - everything else is built in Zen
///
/// This is the SINGLE SOURCE OF TRUTH for all compiler intrinsic type information.
/// Both the typechecker and codegen should use these definitions.
pub struct CompilerModule {
    functions: HashMap<String, StdFunction>,
    types: HashMap<String, AstType>,
}

/// Global singleton for compiler intrinsics - avoids repeated HashMap construction
static COMPILER_MODULE: OnceLock<CompilerModule> = OnceLock::new();

/// Get the global compiler module instance
pub fn get_compiler_module() -> &'static CompilerModule {
    COMPILER_MODULE.get_or_init(CompilerModule::new)
}

/// Quick lookup for compiler intrinsic return type
/// Returns None if not a compiler intrinsic
pub fn get_intrinsic_return_type(func_name: &str) -> Option<AstType> {
    get_compiler_module()
        .get_function(func_name)
        .map(|f| f.return_type)
}

/// Get full intrinsic function info (params + return type)
/// Returns None if not a compiler intrinsic
pub fn get_intrinsic_info(func_name: &str) -> Option<StdFunction> {
    get_compiler_module().get_function(func_name)
}

/// Check if a function name is a compiler intrinsic
pub fn is_compiler_intrinsic(func_name: &str) -> bool {
    get_compiler_module().functions.contains_key(func_name)
}

/// Validate intrinsic call and return its type
/// Returns Err if wrong number of arguments, Ok(type) if valid, None if not an intrinsic
pub fn check_intrinsic_call(
    func_name: &str,
    args_len: usize,
) -> Option<Result<AstType, CompileError>> {
    let func = get_compiler_module().get_function(func_name)?;
    let expected = func.params.len();

    if args_len != expected {
        Some(Err(CompileError::TypeError(
            format!(
                "compiler.{}() expects {} argument(s), got {}",
                func_name, expected, args_len
            ),
            None,
        )))
    } else {
        Some(Ok(func.return_type))
    }
}

impl CompilerModule {
    pub fn new() -> Self {
        let mut functions = HashMap::new();
        let types = HashMap::new();

        // inline.c() - Inline C code compilation
        functions.insert(
            "inline_c".to_string(),
            StdFunction {
                name: "inline_c".to_string(),
                params: vec![("code".to_string(), AstType::StaticString)],
                return_type: AstType::Void,
                is_builtin: true,
            },
        );

        // Memory primitives
        functions.insert(
            "raw_allocate".to_string(),
            StdFunction {
                name: "raw_allocate".to_string(),
                params: vec![("size".to_string(), AstType::Usize)],
                return_type: AstType::ptr(AstType::U8),
                is_builtin: true,
            },
        );

        functions.insert(
            "raw_deallocate".to_string(),
            StdFunction {
                name: "raw_deallocate".to_string(),
                params: vec![
                    ("ptr".to_string(), AstType::ptr(AstType::U8)),
                    ("size".to_string(), AstType::Usize),
                ],
                return_type: AstType::Void,
                is_builtin: true,
            },
        );

        functions.insert(
            "raw_reallocate".to_string(),
            StdFunction {
                name: "raw_reallocate".to_string(),
                params: vec![
                    ("ptr".to_string(), AstType::ptr(AstType::U8)),
                    ("old_size".to_string(), AstType::Usize),
                    ("new_size".to_string(), AstType::Usize),
                ],
                return_type: AstType::ptr(AstType::U8),
                is_builtin: true,
            },
        );

        // Raw pointer operations
        functions.insert(
            "raw_ptr_offset".to_string(),
            StdFunction {
                name: "raw_ptr_offset".to_string(),
                params: vec![
                    ("ptr".to_string(), AstType::raw_ptr(AstType::U8)),
                    ("offset".to_string(), AstType::I64), // Signed offset for pointer arithmetic
                ],
                return_type: AstType::raw_ptr(AstType::U8),
                is_builtin: true,
            },
        );

        functions.insert(
            "raw_ptr_cast".to_string(),
            StdFunction {
                name: "raw_ptr_cast".to_string(),
                params: vec![("ptr".to_string(), AstType::raw_ptr(AstType::U8))],
                return_type: AstType::raw_ptr(AstType::U8), // Generic would be better
                is_builtin: true,
            },
        );

        // Type introspection - size of type in bytes
        functions.insert(
            "sizeof".to_string(),
            StdFunction {
                name: "sizeof".to_string(),
                params: vec![],
                return_type: AstType::Usize,
                is_builtin: true,
            },
        );

        // Function calling primitives
        functions.insert(
            "call_external".to_string(),
            StdFunction {
                name: "call_external".to_string(),
                params: vec![
                    (
                        "func_ptr".to_string(),
                        AstType::raw_ptr(AstType::U8),
                    ),
                    ("args".to_string(), AstType::raw_ptr(AstType::U8)), // Args array as raw pointer
                ],
                return_type: AstType::raw_ptr(AstType::U8),
                is_builtin: true,
            },
        );

        // Library loading primitives
        functions.insert(
            "load_library".to_string(),
            StdFunction {
                name: "load_library".to_string(),
                params: vec![("path".to_string(), AstType::StaticString)],
                return_type: AstType::raw_ptr(AstType::U8), // Library handle
                is_builtin: true,
            },
        );

        functions.insert(
            "get_symbol".to_string(),
            StdFunction {
                name: "get_symbol".to_string(),
                params: vec![
                    (
                        "lib_handle".to_string(),
                        AstType::raw_ptr(AstType::U8),
                    ),
                    ("symbol_name".to_string(), AstType::StaticString),
                ],
                return_type: AstType::raw_ptr(AstType::U8), // Function pointer
                is_builtin: true,
            },
        );

        functions.insert(
            "unload_library".to_string(),
            StdFunction {
                name: "unload_library".to_string(),
                params: vec![(
                    "lib_handle".to_string(),
                    AstType::raw_ptr(AstType::U8),
                )],
                return_type: AstType::Void,
                is_builtin: true,
            },
        );

        // Enum intrinsics - exposed for pattern matching and enum manipulation
        functions.insert(
            "discriminant".to_string(),
            StdFunction {
                name: "discriminant".to_string(),
                params: vec![(
                    "enum_value".to_string(),
                    AstType::raw_ptr(AstType::U8),
                )],
                return_type: AstType::I32,
                is_builtin: true,
            },
        );

        functions.insert(
            "set_discriminant".to_string(),
            StdFunction {
                name: "set_discriminant".to_string(),
                params: vec![
                    (
                        "enum_ptr".to_string(),
                        AstType::raw_ptr(AstType::U8),
                    ),
                    ("discriminant".to_string(), AstType::I32),
                ],
                return_type: AstType::Void,
                is_builtin: true,
            },
        );

        functions.insert(
            "get_payload".to_string(),
            StdFunction {
                name: "get_payload".to_string(),
                params: vec![(
                    "enum_value".to_string(),
                    AstType::raw_ptr(AstType::U8),
                )],
                return_type: AstType::raw_ptr(AstType::U8),
                is_builtin: true,
            },
        );

        functions.insert(
            "set_payload".to_string(),
            StdFunction {
                name: "set_payload".to_string(),
                params: vec![
                    (
                        "enum_ptr".to_string(),
                        AstType::raw_ptr(AstType::U8),
                    ),
                    (
                        "payload".to_string(),
                        AstType::raw_ptr(AstType::U8),
                    ),
                ],
                return_type: AstType::Void,
                is_builtin: true,
            },
        );

        // Pointer arithmetic intrinsic - GEP (GetElementPointer)
        functions.insert(
            "gep".to_string(),
            StdFunction {
                name: "gep".to_string(),
                params: vec![
                    (
                        "base_ptr".to_string(),
                        AstType::raw_ptr(AstType::U8),
                    ),
                    ("offset".to_string(), AstType::I64),
                ],
                return_type: AstType::raw_ptr(AstType::U8),
                is_builtin: true,
            },
        );

        // Memory load/store intrinsics - generic functions
        functions.insert(
            "load".to_string(),
            StdFunction {
                name: "load".to_string(),
                params: vec![("ptr".to_string(), AstType::raw_ptr(AstType::U8))],
                return_type: AstType::Generic {
                    name: "T".to_string(),
                    type_args: vec![],
                },
                is_builtin: true,
            },
        );

        functions.insert(
            "store".to_string(),
            StdFunction {
                name: "store".to_string(),
                params: vec![
                    ("ptr".to_string(), AstType::raw_ptr(AstType::U8)),
                    (
                        "value".to_string(),
                        AstType::Generic {
                            name: "T".to_string(),
                            type_args: vec![],
                        },
                    ),
                ],
                return_type: AstType::Void,
                is_builtin: true,
            },
        );

        // Pointer <-> Integer conversion intrinsics (replaces 'as' keyword)
        functions.insert(
            "ptr_to_int".to_string(),
            StdFunction {
                name: "ptr_to_int".to_string(),
                params: vec![("ptr".to_string(), AstType::raw_ptr(AstType::U8))],
                return_type: AstType::I64,
                is_builtin: true,
            },
        );

        functions.insert(
            "int_to_ptr".to_string(),
            StdFunction {
                name: "int_to_ptr".to_string(),
                params: vec![("addr".to_string(), AstType::I64)],
                return_type: AstType::raw_ptr(AstType::U8),
                is_builtin: true,
            },
        );

        // Null pointer constant
        functions.insert(
            "null_ptr".to_string(),
            StdFunction {
                name: "null_ptr".to_string(),
                params: vec![],
                return_type: AstType::raw_ptr(AstType::U8),
                is_builtin: true,
            },
        );

        // Alias for null_ptr (common naming convention)
        functions.insert(
            "nullptr".to_string(),
            StdFunction {
                name: "nullptr".to_string(),
                params: vec![],
                return_type: AstType::raw_ptr(AstType::U8),
                is_builtin: true,
            },
        );

        functions.insert(
            "gep_struct".to_string(),
            StdFunction {
                name: "gep_struct".to_string(),
                params: vec![
                    (
                        "struct_ptr".to_string(),
                        AstType::raw_ptr(AstType::U8),
                    ),
                    ("field_index".to_string(), AstType::I32),
                ],
                return_type: AstType::raw_ptr(AstType::U8),
                is_builtin: true,
            },
        );

        // Memory operations
        functions.insert(
            "memcpy".to_string(),
            StdFunction {
                name: "memcpy".to_string(),
                params: vec![
                    ("dest".to_string(), AstType::raw_ptr(AstType::U8)),
                    ("src".to_string(), AstType::raw_ptr(AstType::U8)),
                    ("size".to_string(), AstType::Usize),
                ],
                return_type: AstType::Void,
                is_builtin: true,
            },
        );

        functions.insert(
            "memmove".to_string(),
            StdFunction {
                name: "memmove".to_string(),
                params: vec![
                    ("dest".to_string(), AstType::raw_ptr(AstType::U8)),
                    ("src".to_string(), AstType::raw_ptr(AstType::U8)),
                    ("size".to_string(), AstType::Usize),
                ],
                return_type: AstType::Void,
                is_builtin: true,
            },
        );

        functions.insert(
            "memset".to_string(),
            StdFunction {
                name: "memset".to_string(),
                params: vec![
                    ("dest".to_string(), AstType::raw_ptr(AstType::U8)),
                    ("value".to_string(), AstType::U8),
                    ("size".to_string(), AstType::Usize),
                ],
                return_type: AstType::Void,
                is_builtin: true,
            },
        );

        functions.insert(
            "memcmp".to_string(),
            StdFunction {
                name: "memcmp".to_string(),
                params: vec![
                    ("ptr1".to_string(), AstType::raw_ptr(AstType::U8)),
                    ("ptr2".to_string(), AstType::raw_ptr(AstType::U8)),
                    ("size".to_string(), AstType::Usize),
                ],
                return_type: AstType::I32,
                is_builtin: true,
            },
        );

        // Bitwise operations
        functions.insert(
            "bswap16".to_string(),
            StdFunction {
                name: "bswap16".to_string(),
                params: vec![("value".to_string(), AstType::U16)],
                return_type: AstType::U16,
                is_builtin: true,
            },
        );

        functions.insert(
            "bswap32".to_string(),
            StdFunction {
                name: "bswap32".to_string(),
                params: vec![("value".to_string(), AstType::U32)],
                return_type: AstType::U32,
                is_builtin: true,
            },
        );

        functions.insert(
            "bswap64".to_string(),
            StdFunction {
                name: "bswap64".to_string(),
                params: vec![("value".to_string(), AstType::U64)],
                return_type: AstType::U64,
                is_builtin: true,
            },
        );

        functions.insert(
            "ctlz".to_string(),
            StdFunction {
                name: "ctlz".to_string(),
                params: vec![("value".to_string(), AstType::U64)],
                return_type: AstType::U64,
                is_builtin: true,
            },
        );

        functions.insert(
            "cttz".to_string(),
            StdFunction {
                name: "cttz".to_string(),
                params: vec![("value".to_string(), AstType::U64)],
                return_type: AstType::U64,
                is_builtin: true,
            },
        );

        functions.insert(
            "ctpop".to_string(),
            StdFunction {
                name: "ctpop".to_string(),
                params: vec![("value".to_string(), AstType::U64)],
                return_type: AstType::U64,
                is_builtin: true,
            },
        );

        // Atomic operations
        functions.insert(
            "atomic_load".to_string(),
            StdFunction {
                name: "atomic_load".to_string(),
                params: vec![("ptr".to_string(), AstType::raw_ptr(AstType::U64))],
                return_type: AstType::U64,
                is_builtin: true,
            },
        );

        functions.insert(
            "atomic_store".to_string(),
            StdFunction {
                name: "atomic_store".to_string(),
                params: vec![
                    ("ptr".to_string(), AstType::raw_ptr(AstType::U64)),
                    ("value".to_string(), AstType::U64),
                ],
                return_type: AstType::Void,
                is_builtin: true,
            },
        );

        functions.insert(
            "atomic_add".to_string(),
            StdFunction {
                name: "atomic_add".to_string(),
                params: vec![
                    ("ptr".to_string(), AstType::raw_ptr(AstType::U64)),
                    ("value".to_string(), AstType::U64),
                ],
                return_type: AstType::U64,
                is_builtin: true,
            },
        );

        functions.insert(
            "atomic_sub".to_string(),
            StdFunction {
                name: "atomic_sub".to_string(),
                params: vec![
                    ("ptr".to_string(), AstType::raw_ptr(AstType::U64)),
                    ("value".to_string(), AstType::U64),
                ],
                return_type: AstType::U64,
                is_builtin: true,
            },
        );

        functions.insert(
            "atomic_cas".to_string(),
            StdFunction {
                name: "atomic_cas".to_string(),
                params: vec![
                    ("ptr".to_string(), AstType::raw_ptr(AstType::U64)),
                    ("expected".to_string(), AstType::U64),
                    ("new_value".to_string(), AstType::U64),
                ],
                return_type: AstType::Bool,
                is_builtin: true,
            },
        );

        functions.insert(
            "atomic_xchg".to_string(),
            StdFunction {
                name: "atomic_xchg".to_string(),
                params: vec![
                    ("ptr".to_string(), AstType::raw_ptr(AstType::U64)),
                    ("value".to_string(), AstType::U64),
                ],
                return_type: AstType::U64,
                is_builtin: true,
            },
        );

        functions.insert(
            "fence".to_string(),
            StdFunction {
                name: "fence".to_string(),
                params: vec![],
                return_type: AstType::Void,
                is_builtin: true,
            },
        );

        // Overflow-checked arithmetic
        functions.insert(
            "add_overflow".to_string(),
            StdFunction {
                name: "add_overflow".to_string(),
                params: vec![
                    ("a".to_string(), AstType::I64),
                    ("b".to_string(), AstType::I64),
                ],
                return_type: AstType::Struct {
                    name: "OverflowResult".to_string(),
                    fields: vec![
                        ("result".to_string(), AstType::I64),
                        ("overflow".to_string(), AstType::Bool),
                    ],
                },
                is_builtin: true,
            },
        );

        functions.insert(
            "sub_overflow".to_string(),
            StdFunction {
                name: "sub_overflow".to_string(),
                params: vec![
                    ("a".to_string(), AstType::I64),
                    ("b".to_string(), AstType::I64),
                ],
                return_type: AstType::Struct {
                    name: "OverflowResult".to_string(),
                    fields: vec![
                        ("result".to_string(), AstType::I64),
                        ("overflow".to_string(), AstType::Bool),
                    ],
                },
                is_builtin: true,
            },
        );

        functions.insert(
            "mul_overflow".to_string(),
            StdFunction {
                name: "mul_overflow".to_string(),
                params: vec![
                    ("a".to_string(), AstType::I64),
                    ("b".to_string(), AstType::I64),
                ],
                return_type: AstType::Struct {
                    name: "OverflowResult".to_string(),
                    fields: vec![
                        ("result".to_string(), AstType::I64),
                        ("overflow".to_string(), AstType::Bool),
                    ],
                },
                is_builtin: true,
            },
        );

        // Type conversion intrinsics
        functions.insert(
            "trunc_f64_i64".to_string(),
            StdFunction {
                name: "trunc_f64_i64".to_string(),
                params: vec![("value".to_string(), AstType::F64)],
                return_type: AstType::I64,
                is_builtin: true,
            },
        );

        functions.insert(
            "trunc_f32_i32".to_string(),
            StdFunction {
                name: "trunc_f32_i32".to_string(),
                params: vec![("value".to_string(), AstType::F32)],
                return_type: AstType::I32,
                is_builtin: true,
            },
        );

        functions.insert(
            "sitofp_i64_f64".to_string(),
            StdFunction {
                name: "sitofp_i64_f64".to_string(),
                params: vec![("value".to_string(), AstType::I64)],
                return_type: AstType::F64,
                is_builtin: true,
            },
        );

        functions.insert(
            "uitofp_u64_f64".to_string(),
            StdFunction {
                name: "uitofp_u64_f64".to_string(),
                params: vec![("value".to_string(), AstType::U64)],
                return_type: AstType::F64,
                is_builtin: true,
            },
        );

        // Debug/introspection
        functions.insert(
            "alignof".to_string(),
            StdFunction {
                name: "alignof".to_string(),
                params: vec![],
                return_type: AstType::Usize,
                is_builtin: true,
            },
        );

        functions.insert(
            "unreachable".to_string(),
            StdFunction {
                name: "unreachable".to_string(),
                params: vec![],
                return_type: AstType::Void,
                is_builtin: true,
            },
        );

        functions.insert(
            "trap".to_string(),
            StdFunction {
                name: "trap".to_string(),
                params: vec![],
                return_type: AstType::Void,
                is_builtin: true,
            },
        );

        functions.insert(
            "debugtrap".to_string(),
            StdFunction {
                name: "debugtrap".to_string(),
                params: vec![],
                return_type: AstType::Void,
                is_builtin: true,
            },
        );

        CompilerModule { functions, types }
    }
}

impl StdModuleTrait for CompilerModule {
    fn name(&self) -> &str {
        "compiler"
    }

    fn get_function(&self, name: &str) -> Option<StdFunction> {
        self.functions.get(name).cloned()
    }

    fn get_type(&self, name: &str) -> Option<AstType> {
        self.types.get(name).cloned()
    }
}
