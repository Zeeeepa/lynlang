use super::super::LLVMCompiler;
use crate::ast::Expression;
use crate::error::CompileError;
use inkwell::values::BasicValueEnum;

pub fn compile_enum_variant<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    enum_name: &str,
    variant: &str,
    payload: &Option<Box<Expression>>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // Delegate to the existing implementation in enums_variant.rs
    super::enums_variant::compile_enum_variant(compiler, enum_name, variant, payload)
}

pub fn compile_enum_literal<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    expr: &Expression,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // Delegate to the existing implementation
    if let Expression::EnumLiteral { variant, payload } = expr {
        // Try to infer the enum name from context or use Option as default
        compile_enum_variant(compiler, "Option", variant, payload)
    } else {
        Err(CompileError::InternalError(
            "Expected EnumLiteral".to_string(),
            None,
        ))
    }
}

pub fn compile_some<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    value: &Box<Expression>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // Some(value) is Option::Some(value)
    compile_enum_variant(compiler, "Option", "Some", &Some(value.clone()))
}

pub fn compile_none<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // None (or null) is Option::None
    compile_enum_variant(compiler, "Option", "None", &None)
}
