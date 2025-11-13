use super::super::LLVMCompiler;
use crate::ast::Expression;
use crate::error::CompileError;
use inkwell::values::BasicValueEnum;

pub fn compile_pattern_match<'ctx>(
    _compiler: &mut LLVMCompiler<'ctx>,
    _expr: &Expression,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // TODO: Extract from expressions_old.rs
    Err(CompileError::InternalError("Not yet implemented".to_string(), None))
}

