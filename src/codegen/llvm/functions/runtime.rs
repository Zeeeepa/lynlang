use super::super::LLVMCompiler;
use crate::error::CompileError;
use inkwell::values::FunctionValue;

pub fn get_or_create_runtime_function<'ctx>(
    _compiler: &mut LLVMCompiler<'ctx>,
    _name: &str,
) -> Result<FunctionValue<'ctx>, CompileError> {
    // TODO: Extract from functions_old.rs (1909 lines!)
    Err(CompileError::InternalError(
        "Not yet implemented".to_string(),
        None,
    ))
}
