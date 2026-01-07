//! Helper functions for stdlib codegen
//! - Result creation helpers
//! - Memory allocation helpers (DRY: used by arrays, collections, strings)

use crate::codegen::llvm::LLVMCompiler;
use crate::error::CompileError;
use inkwell::module::Linkage;
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};

/// Helper function to create Result.Ok with a value
pub fn create_result_ok<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    value: BasicValueEnum<'ctx>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // Create Result struct {discriminant: 0, payload: value}
    let result_type = compiler.context.struct_type(
        &[
            compiler.context.i64_type().into(),
            compiler
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into(),
        ],
        false,
    );

    let mut result = result_type.get_undef();
    result = compiler
        .builder
        .build_insert_value(
            result,
            compiler.context.i64_type().const_int(0, false),
            0,
            "set_ok",
        )?
        .into_struct_value();
    result = compiler
        .builder
        .build_insert_value(result, value, 1, "set_payload")?
        .into_struct_value();

    Ok(result.into())
}

/// Helper function to create Result.Ok(void)
pub fn create_result_ok_void<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // Create Result struct {discriminant: 0, payload: null}
    let result_type = compiler.context.struct_type(
        &[
            compiler.context.i64_type().into(),
            compiler
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into(),
        ],
        false,
    );

    let mut result = result_type.get_undef();
    result = compiler
        .builder
        .build_insert_value(
            result,
            compiler.context.i64_type().const_int(0, false),
            0,
            "set_ok",
        )?
        .into_struct_value();
    let null_ptr = compiler
        .context
        .ptr_type(inkwell::AddressSpace::default())
        .const_null();
    result = compiler
        .builder
        .build_insert_value(result, null_ptr, 1, "set_payload")?
        .into_struct_value();

    Ok(result.into())
}

/// Helper function to create Result.Err with an error message
pub fn create_result_err<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    error: BasicValueEnum<'ctx>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // Create Result struct {discriminant: 1, payload: error}
    let result_type = compiler.context.struct_type(
        &[
            compiler.context.i64_type().into(),
            compiler
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into(),
        ],
        false,
    );

    let mut result = result_type.get_undef();
    result = compiler
        .builder
        .build_insert_value(
            result,
            compiler.context.i64_type().const_int(1, false),
            0,
            "set_err",
        )?
        .into_struct_value();
    result = compiler
        .builder
        .build_insert_value(result, error, 1, "set_error")?
        .into_struct_value();

    Ok(result.into())
}

// ============================================================================
// Memory Allocation Helpers (DRY: consolidates malloc/free/realloc patterns)
// ============================================================================

/// Get or declare malloc function
pub fn get_malloc<'ctx>(compiler: &LLVMCompiler<'ctx>) -> FunctionValue<'ctx> {
    compiler.module.get_function("malloc").unwrap_or_else(|| {
        let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
        let i64_type = compiler.context.i64_type();
        let fn_type = ptr_type.fn_type(&[i64_type.into()], false);
        compiler.module.add_function("malloc", fn_type, Some(Linkage::External))
    })
}

/// Get or declare free function
pub fn get_free<'ctx>(compiler: &LLVMCompiler<'ctx>) -> FunctionValue<'ctx> {
    compiler.module.get_function("free").unwrap_or_else(|| {
        let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
        let void_type = compiler.context.void_type();
        let fn_type = void_type.fn_type(&[ptr_type.into()], false);
        compiler.module.add_function("free", fn_type, Some(Linkage::External))
    })
}

/// Get or declare realloc function
pub fn get_realloc<'ctx>(compiler: &LLVMCompiler<'ctx>) -> FunctionValue<'ctx> {
    compiler.module.get_function("realloc").unwrap_or_else(|| {
        let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
        let i64_type = compiler.context.i64_type();
        let fn_type = ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false);
        compiler.module.add_function("realloc", fn_type, Some(Linkage::External))
    })
}

/// Allocate memory using malloc
pub fn call_malloc<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    size: IntValue<'ctx>,
    name: &str,
) -> Result<PointerValue<'ctx>, CompileError> {
    let malloc_fn = get_malloc(compiler);
    let result = compiler
        .builder
        .build_call(malloc_fn, &[size.into()], name)?
        .try_as_basic_value()
        .left()
        .ok_or_else(|| CompileError::InternalError("malloc returned void".to_string(), None))?;
    Ok(result.into_pointer_value())
}

/// Free memory
pub fn call_free<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    ptr: PointerValue<'ctx>,
) -> Result<(), CompileError> {
    let free_fn = get_free(compiler);
    compiler.builder.build_call(free_fn, &[ptr.into()], "")?;
    Ok(())
}

/// Reallocate memory
pub fn call_realloc<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    ptr: PointerValue<'ctx>,
    new_size: IntValue<'ctx>,
    name: &str,
) -> Result<PointerValue<'ctx>, CompileError> {
    let realloc_fn = get_realloc(compiler);
    let result = compiler
        .builder
        .build_call(realloc_fn, &[ptr.into(), new_size.into()], name)?
        .try_as_basic_value()
        .left()
        .ok_or_else(|| CompileError::InternalError("realloc returned void".to_string(), None))?;
    Ok(result.into_pointer_value())
}
