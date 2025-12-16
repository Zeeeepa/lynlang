//! Standard library function codegen
//! Split into modules by functionality

pub mod io;
pub mod math;
pub mod core;
pub mod fs;
pub mod compiler;
pub mod collections;
pub mod helpers;

// Re-export io functions for backward compatibility
pub use io::*;

// Note: Other modules (math, core, helpers, fs, compiler, collections) are 
// available via their module paths when needed

// All functions are now delegated to their respective modules
use super::LLVMCompiler;
use crate::ast;
use crate::error::CompileError;
use inkwell::values::BasicValueEnum;

// Delegate to math module
pub fn compile_math_function<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    func_name: &str,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    math::compile_math_function(compiler, func_name, args)
}

// Delegate to core module
pub fn compile_core_assert<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    core::compile_core_assert(compiler, args)
}

// Delegate to core module
pub fn compile_core_panic<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    core::compile_core_panic(compiler, args)
}

// Delegate to fs module
pub fn compile_fs_read_file<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    fs::compile_fs_read_file(compiler, args)
}

// Delegate to fs module
pub fn compile_fs_write_file<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    fs::compile_fs_write_file(compiler, args)
}

// Delegate to fs module
pub fn compile_fs_exists<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    fs::compile_fs_exists(compiler, args)
}

// Delegate to fs module
pub fn compile_fs_remove_file<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    fs::compile_fs_remove_file(compiler, args)
}

// Delegate to fs module
pub fn compile_fs_create_dir<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    fs::compile_fs_create_dir(compiler, args)
}

// Delegate to helpers module
pub fn create_result_ok<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    value: BasicValueEnum<'ctx>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    helpers::create_result_ok(compiler, value)
}

pub fn create_result_ok_void<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    helpers::create_result_ok_void(compiler)
}

pub fn create_result_err<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    error: BasicValueEnum<'ctx>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    helpers::create_result_err(compiler, error)
}

// Delegate to collections module
pub fn compile_hashmap_new<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    collections::compile_hashmap_new(compiler, args)
}

// Delegate to collections module
pub fn compile_hashset_new<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    collections::compile_hashset_new(compiler, args)
}

// Delegate to collections module
pub fn compile_dynvec_new<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    collections::compile_dynvec_new(compiler, args)
}

// Delegate to compiler module
pub fn compile_inline_c<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compiler::compile_inline_c(compiler, args)
}

// Delegate to compiler module
pub fn compile_raw_allocate<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compiler::compile_raw_allocate(compiler, args)
}

// Delegate to compiler module
pub fn compile_raw_deallocate<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compiler::compile_raw_deallocate(compiler, args)
}

// Delegate to compiler module
pub fn compile_raw_reallocate<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compiler::compile_raw_reallocate(compiler, args)
}

// Delegate to compiler module
pub fn compile_raw_ptr_offset<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compiler::compile_raw_ptr_offset(compiler, args)
}

// Delegate to compiler module
pub fn compile_raw_ptr_cast<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compiler::compile_raw_ptr_cast(compiler, args)
}

// Delegate to compiler module
pub fn compile_call_external<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compiler::compile_call_external(compiler, args)
}

// Delegate to compiler module
pub fn compile_load_library<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compiler::compile_load_library(compiler, args)
}

// Delegate to compiler module
pub fn compile_get_symbol<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compiler::compile_get_symbol(compiler, args)
}

// Delegate to compiler module
pub fn compile_unload_library<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compiler::compile_unload_library(compiler, args)
}

// Delegate to compiler module
pub fn compile_null_ptr<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compiler::compile_null_ptr(compiler, args)
}

// Delegate to compiler module - enum intrinsics
pub fn compile_discriminant<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compiler::compile_discriminant(compiler, args)
}

pub fn compile_set_discriminant<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compiler::compile_set_discriminant(compiler, args)
}

pub fn compile_get_payload<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compiler::compile_get_payload(compiler, args)
}

pub fn compile_set_payload<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compiler::compile_set_payload(compiler, args)
}

// Delegate to compiler module - GEP intrinsics
pub fn compile_gep<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compiler::compile_gep(compiler, args)
}

pub fn compile_gep_struct<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compiler::compile_gep_struct(compiler, args)
}

// Delegate to compiler module - load/store intrinsics
pub fn compile_load<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
    type_arg: Option<&ast::AstType>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compiler::compile_load(compiler, args, type_arg)
}

pub fn compile_store<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
    type_arg: Option<&ast::AstType>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compiler::compile_store(compiler, args, type_arg)
}

// Delegate to compiler module - pointer conversion intrinsics
pub fn compile_ptr_to_int<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compiler::compile_ptr_to_int(compiler, args)
}

pub fn compile_int_to_ptr<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compiler::compile_int_to_ptr(compiler, args)
}
