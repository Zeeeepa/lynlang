use super::super::LLVMCompiler;
use crate::ast::Expression;
use crate::error::CompileError;
use inkwell::values::BasicValueEnum;

pub fn compile_binary_operation<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    expr: &Expression,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    match expr {
        Expression::BinaryOp { op, left, right } => {
            compiler.compile_binary_operation(op, left, right)
        }
        _ => Err(CompileError::InternalError(
            format!("Expected BinaryOp, got {:?}", expr),
            None,
        )),
    }
}

pub fn compile_type_cast<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    expr: &Expression,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    match expr {
        Expression::TypeCast {
            expr: inner_expr,
            target_type: _,
        } => {
            // For now, just compile the inner expression
            // TODO: Implement actual type casting
            compiler.compile_expression(inner_expr)
        }
        _ => Err(CompileError::InternalError(
            format!("Expected TypeCast, got {:?}", expr),
            None,
        )),
    }
}
