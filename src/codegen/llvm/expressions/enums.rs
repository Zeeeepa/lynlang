use super::super::LLVMCompiler;
use crate::ast::Expression;
use crate::error::CompileError;
use inkwell::values::BasicValueEnum;

pub fn compile_enum_variant<'ctx>(
    _compiler: &mut LLVMCompiler<'ctx>,
    _enum_name: &str,
    _variant: &str,
    _payload: &Option<Box<Expression>>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // TODO: Extract from expressions_old.rs
    Err(CompileError::InternalError("Not yet implemented".to_string(), None))
}

pub fn compile_enum_literal<'ctx>(
    _compiler: &mut LLVMCompiler<'ctx>,
    _expr: &Expression,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // TODO: Extract from expressions_old.rs
    Err(CompileError::InternalError("Not yet implemented".to_string(), None))
}

