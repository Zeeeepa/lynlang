use super::super::LLVMCompiler;
use crate::ast::Expression;
use crate::error::CompileError;
use inkwell::values::PointerValue;

/// Compile array index to get the address (for pointer arithmetic)
/// Note: General array/vec operations now use stdlib/vec.zen
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
            compiler.get_current_span(),
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
