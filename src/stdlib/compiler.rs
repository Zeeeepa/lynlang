use super::{StdFunction, StdModuleTrait};
use crate::ast::AstType;
use std::collections::HashMap;

/// The @std.compiler module provides low-level compiler intrinsics
/// These are the ONLY primitives exposed - everything else is built in Zen
pub struct CompilerModule {
    functions: HashMap<String, StdFunction>,
    types: HashMap<String, AstType>,
}

impl CompilerModule {
    pub fn new() -> Self {
        let mut functions = HashMap::new();
        let mut types = HashMap::new();

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
                return_type: AstType::Ptr(Box::new(AstType::U8)),
                is_builtin: true,
            },
        );

        functions.insert(
            "raw_deallocate".to_string(),
            StdFunction {
                name: "raw_deallocate".to_string(),
                params: vec![
                    ("ptr".to_string(), AstType::Ptr(Box::new(AstType::U8))),
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
                    ("ptr".to_string(), AstType::Ptr(Box::new(AstType::U8))),
                    ("old_size".to_string(), AstType::Usize),
                    ("new_size".to_string(), AstType::Usize),
                ],
                return_type: AstType::Ptr(Box::new(AstType::U8)),
                is_builtin: true,
            },
        );

        // Raw pointer operations
        functions.insert(
            "raw_ptr_offset".to_string(),
            StdFunction {
                name: "raw_ptr_offset".to_string(),
                params: vec![
                    ("ptr".to_string(), AstType::RawPtr(Box::new(AstType::U8))),
                    ("offset".to_string(), AstType::I64), // Signed offset for pointer arithmetic
                ],
                return_type: AstType::RawPtr(Box::new(AstType::U8)),
                is_builtin: true,
            },
        );

        functions.insert(
            "raw_ptr_cast".to_string(),
            StdFunction {
                name: "raw_ptr_cast".to_string(),
                params: vec![
                    ("ptr".to_string(), AstType::RawPtr(Box::new(AstType::U8))),
                ],
                return_type: AstType::RawPtr(Box::new(AstType::U8)), // Generic would be better
                is_builtin: true,
            },
        );

        // Function calling primitives
        functions.insert(
            "call_external".to_string(),
            StdFunction {
                name: "call_external".to_string(),
                params: vec![
                    ("func_ptr".to_string(), AstType::RawPtr(Box::new(AstType::U8))),
                    ("args".to_string(), AstType::RawPtr(Box::new(AstType::U8))), // Args array as raw pointer
                ],
                return_type: AstType::RawPtr(Box::new(AstType::U8)),
                is_builtin: true,
            },
        );

        // Library loading primitives
        functions.insert(
            "load_library".to_string(),
            StdFunction {
                name: "load_library".to_string(),
                params: vec![("path".to_string(), AstType::StaticString)],
                return_type: AstType::RawPtr(Box::new(AstType::U8)), // Library handle
                is_builtin: true,
            },
        );

        functions.insert(
            "get_symbol".to_string(),
            StdFunction {
                name: "get_symbol".to_string(),
                params: vec![
                    ("lib_handle".to_string(), AstType::RawPtr(Box::new(AstType::U8))),
                    ("symbol_name".to_string(), AstType::StaticString),
                ],
                return_type: AstType::RawPtr(Box::new(AstType::U8)), // Function pointer
                is_builtin: true,
            },
        );

        functions.insert(
            "unload_library".to_string(),
            StdFunction {
                name: "unload_library".to_string(),
                params: vec![("lib_handle".to_string(), AstType::RawPtr(Box::new(AstType::U8)))],
                return_type: AstType::Void,
                is_builtin: true,
            },
        );

        // Enum intrinsics - exposed for pattern matching and enum manipulation
        functions.insert(
            "discriminant".to_string(),
            StdFunction {
                name: "discriminant".to_string(),
                params: vec![("enum_value".to_string(), AstType::RawPtr(Box::new(AstType::U8)))],
                return_type: AstType::I32,
                is_builtin: true,
            },
        );

        functions.insert(
            "set_discriminant".to_string(),
            StdFunction {
                name: "set_discriminant".to_string(),
                params: vec![
                    ("enum_ptr".to_string(), AstType::RawPtr(Box::new(AstType::U8))),
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
                params: vec![("enum_value".to_string(), AstType::RawPtr(Box::new(AstType::U8)))],
                return_type: AstType::RawPtr(Box::new(AstType::U8)),
                is_builtin: true,
            },
        );

        functions.insert(
            "set_payload".to_string(),
            StdFunction {
                name: "set_payload".to_string(),
                params: vec![
                    ("enum_ptr".to_string(), AstType::RawPtr(Box::new(AstType::U8))),
                    ("payload".to_string(), AstType::RawPtr(Box::new(AstType::U8))),
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
                    ("base_ptr".to_string(), AstType::RawPtr(Box::new(AstType::U8))),
                    ("offset".to_string(), AstType::I64),
                ],
                return_type: AstType::RawPtr(Box::new(AstType::U8)),
                is_builtin: true,
            },
        );

        // GEP for struct field access - variant that operates on typed pointers
        functions.insert(
            "gep_struct".to_string(),
            StdFunction {
                name: "gep_struct".to_string(),
                params: vec![
                    ("struct_ptr".to_string(), AstType::RawPtr(Box::new(AstType::U8))),
                    ("field_index".to_string(), AstType::I32),
                ],
                return_type: AstType::RawPtr(Box::new(AstType::U8)),
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

