//! Compiler intrinsics type checking
//! Uses crate::intrinsics as the single source of truth for intrinsic types

use crate::ast::AstType;
use crate::error::Result;
use crate::intrinsics as compiler_intrinsics;

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
