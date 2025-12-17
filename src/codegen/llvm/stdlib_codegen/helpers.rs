//! Helper functions for stdlib codegen - Result creation helpers

use super::super::LLVMCompiler;
use crate::error::CompileError;
use inkwell::values::BasicValueEnum;

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
