use super::{StdFunction, StdModuleTrait};
use crate::ast::AstType;
use crate::error::CompileError;
use crate::register_fn;
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

        // Type aliases for readability
        let ptr_u8 = AstType::ptr(AstType::U8);
        let raw_ptr_u8 = AstType::raw_ptr(AstType::U8);
        let raw_ptr_u64 = AstType::raw_ptr(AstType::U64);
        let generic_t = AstType::Generic {
            name: "T".to_string(),
            type_args: vec![],
        };
        let overflow_result = AstType::Struct {
            name: "OverflowResult".to_string(),
            fields: vec![
                ("result".to_string(), AstType::I64),
                ("overflow".to_string(), AstType::Bool),
            ],
        };

        // Inline C
        register_fn!(functions, "inline_c" => (code: AstType::StaticString) -> AstType::Void);

        // Memory primitives
        register_fn!(functions, "raw_allocate" => (size: AstType::Usize) -> ptr_u8.clone());
        register_fn!(functions, "raw_deallocate" => (ptr: ptr_u8.clone(), size: AstType::Usize) -> AstType::Void);
        register_fn!(functions, "raw_reallocate" => (ptr: ptr_u8.clone(), old_size: AstType::Usize, new_size: AstType::Usize) -> ptr_u8.clone());

        // Raw pointer operations
        register_fn!(functions, "raw_ptr_offset" => (ptr: raw_ptr_u8.clone(), offset: AstType::I64) -> raw_ptr_u8.clone());
        register_fn!(functions, "raw_ptr_cast" => (ptr: raw_ptr_u8.clone()) -> raw_ptr_u8.clone());

        // Type introspection
        register_fn!(functions, "sizeof" => () -> AstType::Usize);
        register_fn!(functions, "alignof" => () -> AstType::Usize);

        // Function calling primitives
        register_fn!(functions, "call_external" => (func_ptr: raw_ptr_u8.clone(), args: raw_ptr_u8.clone()) -> raw_ptr_u8.clone());

        // Library loading primitives
        register_fn!(functions, "load_library" => (path: AstType::StaticString) -> raw_ptr_u8.clone());
        register_fn!(functions, "get_symbol" => (lib_handle: raw_ptr_u8.clone(), symbol_name: AstType::StaticString) -> raw_ptr_u8.clone());
        register_fn!(functions, "unload_library" => (lib_handle: raw_ptr_u8.clone()) -> AstType::Void);
        register_fn!(functions, "dlerror" => () -> raw_ptr_u8.clone());

        // Enum intrinsics
        register_fn!(functions, "discriminant" => (enum_value: raw_ptr_u8.clone()) -> AstType::I32);
        register_fn!(functions, "set_discriminant" => (enum_ptr: raw_ptr_u8.clone(), discriminant: AstType::I32) -> AstType::Void);
        register_fn!(functions, "get_payload" => (enum_value: raw_ptr_u8.clone()) -> raw_ptr_u8.clone());
        register_fn!(functions, "set_payload" => (enum_ptr: raw_ptr_u8.clone(), payload: raw_ptr_u8.clone()) -> AstType::Void);

        // Pointer arithmetic intrinsics (GEP)
        register_fn!(functions, "gep" => (base_ptr: raw_ptr_u8.clone(), offset: AstType::I64) -> raw_ptr_u8.clone());
        register_fn!(functions, "gep_struct" => (struct_ptr: raw_ptr_u8.clone(), field_index: AstType::I32) -> raw_ptr_u8.clone());

        // Memory load/store intrinsics (generic)
        register_fn!(functions, "load" => (ptr: raw_ptr_u8.clone()) -> generic_t.clone());
        register_fn!(functions, "store" => (ptr: raw_ptr_u8.clone(), value: generic_t.clone()) -> AstType::Void);

        // Pointer <-> Integer conversion
        register_fn!(functions, "ptr_to_int" => (ptr: raw_ptr_u8.clone()) -> AstType::I64);
        register_fn!(functions, "int_to_ptr" => (addr: AstType::I64) -> raw_ptr_u8.clone());

        // Null pointer constants
        register_fn!(functions, "null_ptr" => () -> raw_ptr_u8.clone());
        register_fn!(functions, "nullptr" => () -> raw_ptr_u8.clone());

        // Memory operations
        register_fn!(functions, "memcpy" => (dest: raw_ptr_u8.clone(), src: raw_ptr_u8.clone(), size: AstType::Usize) -> AstType::Void);
        register_fn!(functions, "memmove" => (dest: raw_ptr_u8.clone(), src: raw_ptr_u8.clone(), size: AstType::Usize) -> AstType::Void);
        register_fn!(functions, "memset" => (dest: raw_ptr_u8.clone(), value: AstType::U8, size: AstType::Usize) -> AstType::Void);
        register_fn!(functions, "memcmp" => (ptr1: raw_ptr_u8.clone(), ptr2: raw_ptr_u8.clone(), size: AstType::Usize) -> AstType::I32);

        // Bitwise operations
        register_fn!(functions, "bswap16" => (value: AstType::U16) -> AstType::U16);
        register_fn!(functions, "bswap32" => (value: AstType::U32) -> AstType::U32);
        register_fn!(functions, "bswap64" => (value: AstType::U64) -> AstType::U64);
        register_fn!(functions, "ctlz" => (value: AstType::U64) -> AstType::U64);
        register_fn!(functions, "cttz" => (value: AstType::U64) -> AstType::U64);
        register_fn!(functions, "ctpop" => (value: AstType::U64) -> AstType::U64);

        // Atomic operations
        register_fn!(functions, "atomic_load" => (ptr: raw_ptr_u64.clone()) -> AstType::U64);
        register_fn!(functions, "atomic_store" => (ptr: raw_ptr_u64.clone(), value: AstType::U64) -> AstType::Void);
        register_fn!(functions, "atomic_add" => (ptr: raw_ptr_u64.clone(), value: AstType::U64) -> AstType::U64);
        register_fn!(functions, "atomic_sub" => (ptr: raw_ptr_u64.clone(), value: AstType::U64) -> AstType::U64);
        register_fn!(functions, "atomic_cas" => (ptr: raw_ptr_u64.clone(), expected: AstType::U64, new_value: AstType::U64) -> AstType::Bool);
        register_fn!(functions, "atomic_xchg" => (ptr: raw_ptr_u64.clone(), value: AstType::U64) -> AstType::U64);
        register_fn!(functions, "fence" => () -> AstType::Void);

        // Overflow-checked arithmetic
        register_fn!(functions, "add_overflow" => (a: AstType::I64, b: AstType::I64) -> overflow_result.clone());
        register_fn!(functions, "sub_overflow" => (a: AstType::I64, b: AstType::I64) -> overflow_result.clone());
        register_fn!(functions, "mul_overflow" => (a: AstType::I64, b: AstType::I64) -> overflow_result.clone());

        // Type conversion intrinsics
        register_fn!(functions, "trunc_f64_i64" => (value: AstType::F64) -> AstType::I64);
        register_fn!(functions, "trunc_f32_i32" => (value: AstType::F32) -> AstType::I32);
        register_fn!(functions, "sitofp_i64_f64" => (value: AstType::I64) -> AstType::F64);
        register_fn!(functions, "uitofp_u64_f64" => (value: AstType::U64) -> AstType::F64);

        // Debug/introspection
        register_fn!(functions, "unreachable" => () -> AstType::Void);
        register_fn!(functions, "trap" => () -> AstType::Void);
        register_fn!(functions, "debugtrap" => () -> AstType::Void);

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
