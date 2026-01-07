//! Compiler intrinsics type checking
//! Uses stdlib_metadata as the single source of truth for intrinsic types

use crate::ast::AstType;
use crate::error::Result;
use crate::stdlib_metadata::compiler as compiler_intrinsics;
use crate::stdlib_metadata::{
    core::CoreModule, fs::FsModule, io::IOModule, math::MathModule, StdModuleTrait,
};
use std::sync::OnceLock;

/// Global singleton for core module
static CORE_MODULE: OnceLock<CoreModule> = OnceLock::new();
/// Global singleton for IO module
static IO_MODULE: OnceLock<IOModule> = OnceLock::new();
/// Global singleton for math module
static MATH_MODULE: OnceLock<MathModule> = OnceLock::new();
/// Global singleton for fs module
static FS_MODULE: OnceLock<FsModule> = OnceLock::new();

fn get_core_module() -> &'static CoreModule {
    CORE_MODULE.get_or_init(CoreModule::new)
}

fn get_io_module() -> &'static IOModule {
    IO_MODULE.get_or_init(IOModule::new)
}

fn get_math_module() -> &'static MathModule {
    MATH_MODULE.get_or_init(MathModule::new)
}

fn get_fs_module() -> &'static FsModule {
    FS_MODULE.get_or_init(FsModule::new)
}

/// Check compiler intrinsic function calls and return their type
/// Returns None if not a compiler intrinsic, otherwise returns Ok(type) or error
/// Accepts both "compiler" (via @std.compiler) and "builtin"/"@builtin" modules
pub fn check_compiler_intrinsic(
    module: &str,
    func: &str,
    args_len: usize,
) -> Option<Result<AstType>> {
    // Both @std.compiler and @builtin route to the same intrinsics
    // Handle both "builtin" and "@builtin" (with @ prefix from parsing)
    let is_compiler = module == "compiler";
    let is_builtin = module == "builtin" || module == "@builtin";

    if !is_compiler && !is_builtin {
        return None;
    }

    compiler_intrinsics::check_intrinsic_call(func, args_len)
}

/// Check stdlib function calls and return their type
/// Uses stdlib_metadata modules as the single source of truth
/// Returns None if not a known stdlib function, otherwise returns the type
pub fn check_stdlib_function(module: &str, func: &str) -> Option<AstType> {
    match module {
        "core" => get_core_module()
            .get_function(func)
            .map(|f| f.return_type),
        "io" => get_io_module().get_function(func).map(|f| f.return_type),
        "math" => get_math_module()
            .get_function(func)
            .map(|f| f.return_type),
        "fs" => get_fs_module().get_function(func).map(|f| f.return_type),
        _ => None,
    }
}
