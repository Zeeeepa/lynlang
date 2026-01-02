use super::super::LLVMCompiler;
use crate::ast::Expression;
use crate::error::CompileError;
use inkwell::values::{BasicValueEnum, PointerValue};

/// NOTE: Legacy collection constructors
/// Vec, DynVec, Array are now implemented in Zen stdlib using compiler intrinsics.
/// Use: `Vec.new(allocator)` instead of `Vec<T, size>()`
/// See: stdlib/vec.zen, stdlib/compiler/compiler.zen
pub fn compile_array_literal<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    _expr: &Expression,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    Err(CompileError::InternalError(
        "Array literals are deprecated. Use Vec.new(allocator) from stdlib/vec.zen".to_string(),
        compiler.get_current_span(),
    ))
}

pub fn compile_array_index<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    _expr: &Expression,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    Err(CompileError::InternalError(
        "Use Vec.get(index) from stdlib/vec.zen for array indexing".to_string(),
        compiler.get_current_span(),
    ))
}

pub fn compile_array_index_address<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    array: &Expression,
    index: &Expression,
) -> Result<PointerValue<'ctx>, CompileError> {
    let array_val = compiler.compile_expression(array)?;

    let array_ptr = if array_val.is_pointer_value() {
        array_val.into_pointer_value()
    } else {
        return Err(CompileError::TypeError(
            format!(
                "Array indexing requires pointer type, got {:?}",
                array_val.get_type()
            ),
            None,
        ));
    };

    let element_type = compiler.context.i32_type();
    let index_val = compiler.compile_expression(index)?;
    let gep = unsafe {
        compiler.builder.build_gep(
            element_type,
            array_ptr,
            &[index_val.into_int_value()],
            "arrayidx",
        )?
    };
    Ok(gep)
}

pub fn compile_vec_constructor<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    _expr: &Expression,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    Err(CompileError::InternalError(
        "Vec<T, size>() syntax is deprecated. Use Vec.new(allocator) from stdlib/vec.zen".to_string(),
        compiler.get_current_span(),
    ))
}

pub fn compile_dynvec_constructor<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    _expr: &Expression,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    Err(CompileError::InternalError(
        "DynVec<T>() syntax is deprecated. Use Vec.new(allocator) from stdlib/vec.zen".to_string(),
        compiler.get_current_span(),
    ))
}

pub fn compile_array_constructor<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    _expr: &Expression,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    Err(CompileError::InternalError(
        "Array<T>() syntax is deprecated. Use Vec.new(allocator) from stdlib/vec.zen".to_string(),
        compiler.get_current_span(),
    ))
}
