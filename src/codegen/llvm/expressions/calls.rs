use super::super::LLVMCompiler;
use crate::ast::Expression;
use crate::error::CompileError;
use inkwell::values::BasicValueEnum;
use super::super::functions::calls as function_calls;

pub fn compile_function_call<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    expr: &Expression,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    match expr {
        Expression::FunctionCall { name, args } => {
            function_calls::compile_function_call(compiler, name, args)
        }
        _ => Err(CompileError::InternalError(
            format!("Expected FunctionCall, got {:?}", expr),
            None,
        )),
    }
}

pub fn compile_method_call<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    expr: &Expression,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    match expr {
        Expression::MethodCall { object, method, args } => {
            // Use behaviors module to handle method calls
            compiler.compile_method_call(object, method, args)
        }
        _ => Err(CompileError::InternalError(
            format!("Expected MethodCall, got {:?}", expr),
            None,
        )),
    }
}

pub fn compile_closure<'ctx>(
    _compiler: &mut LLVMCompiler<'ctx>,
    expr: &Expression,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    match expr {
        Expression::Closure { params: _, body: _, return_type: _ } => {
            // TODO: Implement closure compilation
            Err(CompileError::InternalError("Closure compilation not yet implemented".to_string(), None))
        }
        _ => Err(CompileError::InternalError(
            format!("Expected Closure, got {:?}", expr),
            None,
        )),
    }
}

