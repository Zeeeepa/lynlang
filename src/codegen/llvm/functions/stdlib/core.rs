//! Core module codegen - assert, panic

use super::super::super::LLVMCompiler;
use crate::ast;
use crate::error::CompileError;
use inkwell::values::BasicValueEnum;

/// Compile core.assert function call
pub fn compile_core_assert<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 1 {
        return Err(CompileError::TypeError(
            format!("core.assert expects 1 argument, got {}", args.len()),
            None,
        ));
    }

    let condition = compiler.compile_expression(&args[0])?;
    let condition = if condition.is_int_value() {
        condition.into_int_value()
    } else {
        return Err(CompileError::TypeError(
            "core.assert requires a boolean condition".to_string(),
            None,
        ));
    };

    // Create basic blocks for assertion
    let current_fn = compiler
        .current_function
        .ok_or_else(|| CompileError::InternalError("No current function".to_string(), None))?;

    let then_block = compiler.context.append_basic_block(current_fn, "assert_pass");
    let else_block = compiler.context.append_basic_block(current_fn, "assert_fail");

    // Check condition
    compiler.builder
        .build_conditional_branch(condition, then_block, else_block)?;

    // Assert fail block - call abort() or exit(1)
    compiler.builder.position_at_end(else_block);

    // Get or declare exit function
    let exit_fn = compiler.module.get_function("exit").unwrap_or_else(|| {
        let i32_type = compiler.context.i32_type();
        let fn_type = compiler.context.void_type().fn_type(&[i32_type.into()], false);
        compiler.module
            .add_function("exit", fn_type, Some(inkwell::module::Linkage::External))
    });

    // Call exit(1)
    let exit_code = compiler.context.i32_type().const_int(1, false);
    compiler.builder
        .build_call(exit_fn, &[exit_code.into()], "exit_call")?;
    compiler.builder.build_unreachable()?;

    // Continue in pass block
    compiler.builder.position_at_end(then_block);

    // Return void value
    Ok(compiler.context.i32_type().const_int(0, false).into())
}

/// Compile core.panic function call
pub fn compile_core_panic<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 1 {
        return Err(CompileError::TypeError(
            format!("core.panic expects 1 argument, got {}", args.len()),
            None,
        ));
    }

    // First print the panic message if it's a string
    if let ast::Expression::String(msg) = &args[0] {
        // Get or declare puts function
        let puts = compiler.module.get_function("puts").unwrap_or_else(|| {
            let i8_ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
            let fn_type = compiler
                .context
                .i32_type()
                .fn_type(&[i8_ptr_type.into()], false);
            compiler.module
                .add_function("puts", fn_type, Some(inkwell::module::Linkage::External))
        });

        // Create panic message with "panic: " prefix
        let panic_msg = format!("panic: {}", msg);
        let string_value = compiler
            .builder
            .build_global_string_ptr(&panic_msg, "panic_msg")?;
        compiler.builder.build_call(
            puts,
            &[string_value.as_pointer_value().into()],
            "puts_call",
        )?;
    }

    // Get or declare exit function
    let exit_fn = compiler.module.get_function("exit").unwrap_or_else(|| {
        let i32_type = compiler.context.i32_type();
        let fn_type = compiler.context.void_type().fn_type(&[i32_type.into()], false);
        compiler.module
            .add_function("exit", fn_type, Some(inkwell::module::Linkage::External))
    });

    // Call exit(1)
    let exit_code = compiler.context.i32_type().const_int(1, false);
    compiler.builder
        .build_call(exit_fn, &[exit_code.into()], "exit_call")?;
    compiler.builder.build_unreachable()?;

    // Create a new unreachable block to satisfy type system
    let current_fn = compiler
        .current_function
        .ok_or_else(|| CompileError::InternalError("No current function".to_string(), None))?;
    let unreachable_block = compiler.context.append_basic_block(current_fn, "after_panic");
    compiler.builder.position_at_end(unreachable_block);

    // Return a dummy value (this code is unreachable)
    Ok(compiler.context.i32_type().const_int(0, false).into())
}
