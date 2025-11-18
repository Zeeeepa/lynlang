//! IO module codegen - print, println, read_line, etc.

use super::super::super::LLVMCompiler;
use crate::ast;
use crate::error::CompileError;
use inkwell::values::BasicValueEnum;

pub fn compile_io_print<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 1 {
        return Err(CompileError::TypeError(
            format!("io.print expects 1 argument, got {}", args.len()),
            None,
        ));
    }

    // Get or declare printf
    let printf_fn = compiler.module.get_function("printf").unwrap_or_else(|| {
        let i32_type = compiler.context.i32_type();
        let i8_ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
        let fn_type = i32_type.fn_type(&[i8_ptr_type.into()], true);
        compiler.module.add_function("printf", fn_type, None)
    });

    // Compile the string argument
    let arg_value = compiler.compile_expression(&args[0])?;

    // Call printf
    let _call = compiler
        .builder
        .build_call(printf_fn, &[arg_value.into()], "printf_call")?;

    // Return void as unit value
    Ok(compiler.context.i32_type().const_zero().into())
}

/// Compile io.println function call
pub fn compile_io_println<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 1 {
        return Err(CompileError::TypeError(
            format!("io.println expects 1 argument, got {}", args.len()),
            None,
        ));
    }

    // Check if the expression is a boolean identifier or literal
    let is_bool_expr = match &args[0] {
        ast::Expression::Boolean(_) => true,
        ast::Expression::Identifier(name) => {
            if let Ok((_, ast_type)) = compiler.get_variable(name) {
                matches!(ast_type, crate::ast::AstType::Bool)
            } else {
                false
            }
        }
        _ => false,
    };

    // Compile the argument
    let arg_value = compiler.compile_expression(&args[0])?;

    // Convert to string based on type
    let string_ptr = match arg_value {
        BasicValueEnum::IntValue(int_val) => {
            // Check if it's a boolean by checking the bit width OR if we know it's a bool expression
            let bit_width = int_val.get_type().get_bit_width();
            // Check for i1 values (booleans) or known boolean expressions
            if bit_width == 1 || is_bool_expr {
                // Boolean value
                let true_str = compiler.builder.build_global_string_ptr("true", "true_str")?;
                let false_str = compiler.builder.build_global_string_ptr("false", "false_str")?;

                // If it's not already i1, truncate it
                let bool_val = if bit_width != 1 {
                    compiler.builder.build_int_truncate(
                        int_val,
                        compiler.context.bool_type(),
                        "bool_trunc",
                    )?
                } else {
                    int_val
                };

                // Create a select instruction to choose between "true" and "false"
                let str_ptr = compiler.builder.build_select(
                    bool_val,
                    true_str.as_pointer_value(),
                    false_str.as_pointer_value(),
                    "bool_str",
                )?;

                // Use puts to print the boolean string
                let puts_fn = compiler.module.get_function("puts").unwrap_or_else(|| {
                    let i32_type = compiler.context.i32_type();
                    let i8_ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
                    let fn_type = i32_type.fn_type(&[i8_ptr_type.into()], false);
                    compiler.module.add_function("puts", fn_type, None)
                });

                compiler.builder
                    .build_call(puts_fn, &[str_ptr.into()], "puts_bool")?;

                return Ok(compiler.context.i32_type().const_zero().into());
            } else {
                // Regular integer
                let printf_fn = compiler.module.get_function("printf").unwrap_or_else(|| {
                    let i32_type = compiler.context.i32_type();
                    let i8_ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
                    let fn_type = i32_type.fn_type(&[i8_ptr_type.into()], true);
                    compiler.module.add_function("printf", fn_type, None)
                });

                // Use appropriate format specifier based on bit width
                let format_str = match int_val.get_type().get_bit_width() {
                    64 => compiler
                        .builder
                        .build_global_string_ptr("%lld\n", "int64_format")?,
                    _ => compiler.builder.build_global_string_ptr("%d\n", "int_format")?,
                };

                compiler.builder.build_call(
                    printf_fn,
                    &[format_str.as_pointer_value().into(), int_val.into()],
                    "printf_int",
                )?;

                // Return early since printf handles the newline
                return Ok(compiler.context.i32_type().const_zero().into());
            }
        }
        BasicValueEnum::FloatValue(float_val) => {
            // For floats, use printf to format
            let printf_fn = compiler.module.get_function("printf").unwrap_or_else(|| {
                let i32_type = compiler.context.i32_type();
                let i8_ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
                let fn_type = i32_type.fn_type(&[i8_ptr_type.into()], true);
                compiler.module.add_function("printf", fn_type, None)
            });

            let format_str = compiler
                .builder
                .build_global_string_ptr("%f\n", "float_format")?;
            compiler.builder.build_call(
                printf_fn,
                &[format_str.as_pointer_value().into(), float_val.into()],
                "printf_float",
            )?;

            // Return early since printf handles the newline
            return Ok(compiler.context.i32_type().const_zero().into());
        }
        BasicValueEnum::PointerValue(ptr_val) => {
            // Assume it's a string pointer
            ptr_val
        }
        _ => {
            return Err(CompileError::TypeError(
                format!("io.println cannot print this type"),
                None,
            ));
        }
    };

    // For strings, use puts
    let puts_fn = compiler.module.get_function("puts").unwrap_or_else(|| {
        let i32_type = compiler.context.i32_type();
        let i8_ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
        let fn_type = i32_type.fn_type(&[i8_ptr_type.into()], false);
        compiler.module.add_function("puts", fn_type, None)
    });

    compiler.builder
        .build_call(puts_fn, &[string_ptr.into()], "puts_call")?;

    // Return void as unit value
    Ok(compiler.context.i32_type().const_zero().into())
}

/// Compile io.print_int function call
pub fn compile_io_print_int<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 1 {
        return Err(CompileError::TypeError(
            format!("io.print_int expects 1 argument, got {}", args.len()),
            None,
        ));
    }

    // Get or declare printf
    let printf_fn = compiler.module.get_function("printf").unwrap_or_else(|| {
        let i32_type = compiler.context.i32_type();
        let i8_ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
        let fn_type = i32_type.fn_type(&[i8_ptr_type.into()], true);
        compiler.module.add_function("printf", fn_type, None)
    });

    // Create format string for integer
    let format_str = compiler.builder.build_global_string_ptr("%d\n", "int_format")?;

    // Compile the integer argument
    let arg_value = compiler.compile_expression(&args[0])?;

    // Call printf
    let _call = compiler.builder.build_call(
        printf_fn,
        &[format_str.as_pointer_value().into(), arg_value.into()],
        "printf_int_call",
    )?;

    // Return void as unit value
    Ok(compiler.context.i32_type().const_zero().into())
}

/// Compile io.print_float function call
pub fn compile_io_print_float<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 1 {
        return Err(CompileError::TypeError(
            format!("io.print_float expects 1 argument, got {}", args.len()),
            None,
        ));
    }

    // Get or declare printf
    let printf_fn = compiler.module.get_function("printf").unwrap_or_else(|| {
        let i32_type = compiler.context.i32_type();
        let i8_ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
        let fn_type = i32_type.fn_type(&[i8_ptr_type.into()], true);
        compiler.module.add_function("printf", fn_type, None)
    });

    // Create format string for float
    let format_str = compiler
        .builder
        .build_global_string_ptr("%.6f\n", "float_format")?;

    // Compile the float argument
    let arg_value = compiler.compile_expression(&args[0])?;

    // Convert to f64 if needed
    let float_value = match arg_value {
        BasicValueEnum::FloatValue(f) => f,
        BasicValueEnum::IntValue(i) => {
            // Convert int to float
            compiler.builder.build_signed_int_to_float(
                i,
                compiler.context.f64_type(),
                "int_to_float",
            )?
        }
        _ => {
            return Err(CompileError::TypeError(
                "io.print_float expects a numeric argument".to_string(),
                None,
            ));
        }
    };

    // Call printf
    let _call = compiler.builder.build_call(
        printf_fn,
        &[format_str.as_pointer_value().into(), float_value.into()],
        "printf_float_call",
    )?;

    // Return void as unit value
    Ok(compiler.context.i32_type().const_zero().into())
}

