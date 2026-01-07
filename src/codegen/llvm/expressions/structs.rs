use super::super::LLVMCompiler;
use crate::ast::Expression;
use crate::error::CompileError;
use inkwell::values::BasicValueEnum;

pub fn compile_struct_literal<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    expr: &Expression,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    match expr {
        Expression::StructLiteral { name, fields } => {
            let fields_vec: Vec<(String, Expression)> = fields.to_vec();
            compiler.compile_struct_literal(name, &fields_vec)
        }
        _ => Err(CompileError::InternalError(
            format!("Expected StructLiteral, got {:?}", expr),
            None,
        )),
    }
}

pub fn compile_struct_field<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    expr: &Expression,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    match expr {
        Expression::StructField { struct_, field } => compiler.compile_struct_field(struct_, field),
        _ => Err(CompileError::InternalError(
            format!("Expected StructField, got {:?}", expr),
            None,
        )),
    }
}

pub fn compile_member_access<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    expr: &Expression,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    match expr {
        Expression::MemberAccess { object, member } => {
            compiler.compile_struct_field(object, member)
        }
        _ => Err(CompileError::InternalError(
            format!("Expected MemberAccess, got {:?}", expr),
            None,
        )),
    }
}
