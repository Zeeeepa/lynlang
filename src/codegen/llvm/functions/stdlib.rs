use super::super::LLVMCompiler;
use crate::ast;
use crate::error::CompileError;
use inkwell::module::Linkage;
use inkwell::values::BasicValueEnum;

pub fn compile_io_print<'ctx>(
        compiler: &mut LLVMCompiler<'ctx>,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if args.len() != 1 {
            // eprintln!("DEBUG: io.print called with {} arguments", args.len());
            // for (i, arg) in args.iter().enumerate() {
            //     eprintln!("  Arg {}: {:?}", i, arg);
            // }
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

    /// Compile math module function calls
pub fn compile_math_function<'ctx>(
        compiler: &mut LLVMCompiler<'ctx>,
        func_name: &str,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Handle min and max functions
        if func_name == "min" || func_name == "max" {
            if args.len() != 2 {
                return Err(CompileError::TypeError(
                    format!("math.{} expects 2 arguments, got {}", func_name, args.len()),
                    None,
                ));
            }

            let left = compiler.compile_expression(&args[0])?;
            let right = compiler.compile_expression(&args[1])?;

            // Handle integer min/max
            if left.is_int_value() && right.is_int_value() {
                let left_int = left.into_int_value();
                let right_int = right.into_int_value();

                // Make sure both integers are the same type
                let (left_int, right_int) = if left_int.get_type() != right_int.get_type() {
                    // Promote to i64 if types differ
                    let i64_type = compiler.context.i64_type();
                    let left_promoted = if left_int.get_type().get_bit_width() < 64 {
                        compiler.builder
                            .build_int_s_extend(left_int, i64_type, "extend_left")?
                    } else {
                        left_int
                    };
                    let right_promoted = if right_int.get_type().get_bit_width() < 64 {
                        compiler.builder
                            .build_int_s_extend(right_int, i64_type, "extend_right")?
                    } else {
                        right_int
                    };
                    (left_promoted, right_promoted)
                } else {
                    (left_int, right_int)
                };

                let cmp = if func_name == "min" {
                    compiler.builder.build_int_compare(
                        inkwell::IntPredicate::SLT,
                        left_int,
                        right_int,
                        "lt",
                    )?
                } else {
                    compiler.builder.build_int_compare(
                        inkwell::IntPredicate::SGT,
                        left_int,
                        right_int,
                        "gt",
                    )?
                };

                let result = compiler
                    .builder
                    .build_select(cmp, left_int, right_int, func_name)?;
                return Ok(result.try_into().unwrap());
            }

            // Handle float min/max or mixed types
            let left_float = match left {
                BasicValueEnum::FloatValue(f) => f,
                BasicValueEnum::IntValue(i) => compiler.builder.build_signed_int_to_float(
                    i,
                    compiler.context.f64_type(),
                    "int_to_float",
                )?,
                _ => {
                    return Err(CompileError::TypeError(
                        format!("math.{} expects numeric arguments", func_name),
                        None,
                    ));
                }
            };

            let right_float = match right {
                BasicValueEnum::FloatValue(f) => f,
                BasicValueEnum::IntValue(i) => compiler.builder.build_signed_int_to_float(
                    i,
                    compiler.context.f64_type(),
                    "int_to_float",
                )?,
                _ => {
                    return Err(CompileError::TypeError(
                        format!("math.{} expects numeric arguments", func_name),
                        None,
                    ));
                }
            };

            // Use fmin/fmax intrinsics for floats
            let intrinsic_name = if func_name == "min" {
                "llvm.minnum.f64"
            } else {
                "llvm.maxnum.f64"
            };
            let intrinsic = compiler.module.get_function(intrinsic_name).unwrap_or_else(|| {
                let f64_type = compiler.context.f64_type();
                let fn_type = f64_type.fn_type(&[f64_type.into(), f64_type.into()], false);
                compiler.module.add_function(intrinsic_name, fn_type, None)
            });

            let result = compiler.builder.build_call(
                intrinsic,
                &[left_float.into(), right_float.into()],
                func_name,
            )?;

            return Ok(result.try_as_basic_value().left().unwrap());
        }

        // Handle abs specially for integer types
        if func_name == "abs" {
            if args.len() != 1 {
                return Err(CompileError::TypeError(
                    format!("math.abs expects 1 argument, got {}", args.len()),
                    None,
                ));
            }

            let val = compiler.compile_expression(&args[0])?;
            return match val {
                BasicValueEnum::IntValue(i) => {
                    // For integers, generate abs using conditional
                    let zero = i.get_type().const_zero();
                    let is_negative = compiler.builder.build_int_compare(
                        inkwell::IntPredicate::SLT,
                        i,
                        zero,
                        "is_negative",
                    )?;
                    let neg = compiler.builder.build_int_neg(i, "neg")?;
                    let abs_val = compiler.builder.build_select(is_negative, neg, i, "abs")?;
                    Ok(abs_val.try_into().unwrap())
                }
                BasicValueEnum::FloatValue(f) => {
                    // For floats, use fabs
                    let fabs_fn = compiler.module.get_function("fabs").unwrap_or_else(|| {
                        let fn_type = compiler
                            .context
                            .f64_type()
                            .fn_type(&[compiler.context.f64_type().into()], false);
                        compiler.module.add_function("fabs", fn_type, None)
                    });
                    let call = compiler.builder.build_call(fabs_fn, &[f.into()], "fabs_call")?;
                    Ok(call.try_as_basic_value().left().unwrap())
                }
                _ => Err(CompileError::TypeError(
                    "math.abs expects a numeric argument".to_string(),
                    None,
                )),
            };
        }

        // Map math function names to their C math library equivalents
        let c_func_name = match func_name {
            "sqrt" => "sqrt",
            "sin" => "sin",
            "cos" => "cos",
            "tan" => "tan",
            "asin" => "asin",
            "acos" => "acos",
            "atan" => "atan",
            "exp" => "exp",
            "log" => "log",
            "log10" => "log10",
            "log2" => "log2",
            "pow" => "pow",
            "floor" => "floor",
            "ceil" => "ceil",
            "round" => "round",
            "fabs" => "fabs",
            _ => {
                return Err(CompileError::UndeclaredFunction(
                    format!("math.{}", func_name),
                    None,
                ));
            }
        };

        // Determine function signature based on the function
        let (expected_args, fn_type) = match func_name {
            "pow" | "atan2" | "fmod" | "hypot" => {
                // Two-argument functions
                (
                    2,
                    compiler.context.f64_type().fn_type(
                        &[
                            compiler.context.f64_type().into(),
                            compiler.context.f64_type().into(),
                        ],
                        false,
                    ),
                )
            }
            _ => {
                // Single-argument functions
                (
                    1,
                    compiler.context
                        .f64_type()
                        .fn_type(&[compiler.context.f64_type().into()], false),
                )
            }
        };

        // Check argument count
        if args.len() != expected_args {
            return Err(CompileError::TypeError(
                format!(
                    "math.{} expects {} argument(s), got {}",
                    func_name,
                    expected_args,
                    args.len()
                ),
                None,
            ));
        }

        // Get or declare the math function
        let math_fn = compiler
            .module
            .get_function(c_func_name)
            .unwrap_or_else(|| compiler.module.add_function(c_func_name, fn_type, None));

        // Compile arguments
        let mut compiled_args = Vec::new();
        for arg in args {
            let val = compiler.compile_expression(arg)?;
            // Convert to f64 if needed
            let f64_val = match val {
                BasicValueEnum::FloatValue(f) => f,
                BasicValueEnum::IntValue(i) => {
                    // Cast int to float
                    compiler.builder.build_signed_int_to_float(
                        i,
                        compiler.context.f64_type(),
                        "int_to_float",
                    )?
                }
                _ => {
                    return Err(CompileError::TypeError(
                        format!("math.{} expects numeric arguments", func_name),
                        None,
                    ));
                }
            };
            compiled_args.push(f64_val.into());
        }

        // Call the math function
        let call_result =
            compiler.builder
                .build_call(math_fn, &compiled_args, &format!("{}_call", c_func_name))?;

        // Return the result
        Ok(call_result
            .try_as_basic_value()
            .left()
            .unwrap_or_else(|| compiler.context.f64_type().const_zero().into()))
    }

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

    /// Compile fs.read_file function call
pub fn compile_fs_read_file<'ctx>(
        compiler: &mut LLVMCompiler<'ctx>,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if args.len() != 1 {
            return Err(CompileError::TypeError(
                format!("fs.read_file expects 1 argument, got {}", args.len()),
                None,
            ));
        }

        // Get or declare fopen
        let fopen_fn = compiler.module.get_function("fopen").unwrap_or_else(|| {
            let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
            let fn_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
            compiler.module.add_function("fopen", fn_type, None)
        });

        // Get or declare fclose
        let fclose_fn = compiler.module.get_function("fclose").unwrap_or_else(|| {
            let i32_type = compiler.context.i32_type();
            let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
            let fn_type = i32_type.fn_type(&[ptr_type.into()], false);
            compiler.module.add_function("fclose", fn_type, None)
        });

        // Get or declare fseek
        let fseek_fn = compiler.module.get_function("fseek").unwrap_or_else(|| {
            let i32_type = compiler.context.i32_type();
            let i64_type = compiler.context.i64_type();
            let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
            let fn_type =
                i32_type.fn_type(&[ptr_type.into(), i64_type.into(), i32_type.into()], false);
            compiler.module.add_function("fseek", fn_type, None)
        });

        // Get or declare ftell
        let ftell_fn = compiler.module.get_function("ftell").unwrap_or_else(|| {
            let i64_type = compiler.context.i64_type();
            let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
            let fn_type = i64_type.fn_type(&[ptr_type.into()], false);
            compiler.module.add_function("ftell", fn_type, None)
        });

        // Get or declare fread
        let fread_fn = compiler.module.get_function("fread").unwrap_or_else(|| {
            let i64_type = compiler.context.i64_type();
            let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
            let fn_type = i64_type.fn_type(
                &[
                    ptr_type.into(),
                    i64_type.into(),
                    i64_type.into(),
                    ptr_type.into(),
                ],
                false,
            );
            compiler.module.add_function("fread", fn_type, None)
        });

        // Compile the path argument
        let path_value = compiler.compile_expression(&args[0])?;
        let path_ptr = path_value.into_pointer_value();

        // Create mode string "r"
        let mode_str = compiler.builder.build_global_string_ptr("r", "read_mode")?;

        // Call fopen
        let file_ptr = compiler
            .builder
            .build_call(
                fopen_fn,
                &[path_ptr.into(), mode_str.as_pointer_value().into()],
                "fopen_call",
            )?
            .try_as_basic_value()
            .left()
            .unwrap();

        // Check if file opened successfully
        let is_null = compiler
            .builder
            .build_is_null(file_ptr.into_pointer_value(), "is_null")?;
        let current_fn = compiler.current_function.unwrap();
        let success_block = compiler.context.append_basic_block(current_fn, "file_opened");
        let error_block = compiler.context.append_basic_block(current_fn, "file_error");
        let merge_block = compiler.context.append_basic_block(current_fn, "merge");

        compiler.builder
            .build_conditional_branch(is_null, error_block, success_block)?;

        // Success block: read file
        compiler.builder.position_at_end(success_block);

        // Seek to end to get file size
        let seek_end = compiler.context.i32_type().const_int(2, false); // SEEK_END
        compiler.builder.build_call(
            fseek_fn,
            &[
                file_ptr.into(),
                compiler.context.i64_type().const_zero().into(),
                seek_end.into(),
            ],
            "fseek_end",
        )?;

        // Get file size
        let file_size = compiler
            .builder
            .build_call(ftell_fn, &[file_ptr.into()], "ftell_call")?
            .try_as_basic_value()
            .left()
            .unwrap();

        // Seek back to beginning
        let seek_set = compiler.context.i32_type().const_int(0, false); // SEEK_SET
        compiler.builder.build_call(
            fseek_fn,
            &[
                file_ptr.into(),
                compiler.context.i64_type().const_zero().into(),
                seek_set.into(),
            ],
            "fseek_start",
        )?;

        // Allocate buffer for file contents
        let malloc_fn = compiler.module.get_function("malloc").unwrap();
        let buffer = compiler
            .builder
            .build_call(malloc_fn, &[file_size.into()], "malloc_buffer")?
            .try_as_basic_value()
            .left()
            .unwrap();

        // Read file contents
        compiler.builder.build_call(
            fread_fn,
            &[
                buffer.into(),
                compiler.context.i64_type().const_int(1, false).into(),
                file_size.into(),
                file_ptr.into(),
            ],
            "fread_call",
        )?;

        // Close file
        compiler.builder
            .build_call(fclose_fn, &[file_ptr.into()], "fclose_call")?;

        // Create Result.Ok with the buffer
        let result_ok = compiler.create_result_ok(buffer)?;
        compiler.builder.build_unconditional_branch(merge_block)?;
        let success_value = result_ok;

        // Error block: return Result.Err
        compiler.builder.position_at_end(error_block);
        let error_msg = compiler
            .builder
            .build_global_string_ptr("Failed to open file", "file_error_msg")?;
        let result_err = compiler.create_result_err(error_msg.as_pointer_value().into())?;
        compiler.builder.build_unconditional_branch(merge_block)?;
        let error_value = result_err;

        // Merge block
        compiler.builder.position_at_end(merge_block);
        let phi = compiler
            .builder
            .build_phi(success_value.get_type(), "result_phi")?;
        phi.add_incoming(&[(&success_value, success_block), (&error_value, error_block)]);

        Ok(phi.as_basic_value())
    }

    /// Compile fs.write_file function call
pub fn compile_fs_write_file<'ctx>(
        compiler: &mut LLVMCompiler<'ctx>,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if args.len() != 2 {
            return Err(CompileError::TypeError(
                format!("fs.write_file expects 2 arguments, got {}", args.len()),
                None,
            ));
        }

        // Get or declare fopen
        let fopen_fn = compiler.module.get_function("fopen").unwrap_or_else(|| {
            let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
            let fn_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
            compiler.module.add_function("fopen", fn_type, None)
        });

        // Get or declare fwrite
        let fwrite_fn = compiler.module.get_function("fwrite").unwrap_or_else(|| {
            let i64_type = compiler.context.i64_type();
            let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
            let fn_type = i64_type.fn_type(
                &[
                    ptr_type.into(),
                    i64_type.into(),
                    i64_type.into(),
                    ptr_type.into(),
                ],
                false,
            );
            compiler.module.add_function("fwrite", fn_type, None)
        });

        // Get or declare fclose
        let fclose_fn = compiler.module.get_function("fclose").unwrap_or_else(|| {
            let i32_type = compiler.context.i32_type();
            let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
            let fn_type = i32_type.fn_type(&[ptr_type.into()], false);
            compiler.module.add_function("fclose", fn_type, None)
        });

        // Get or declare strlen
        let strlen_fn = compiler.module.get_function("strlen").unwrap_or_else(|| {
            let i64_type = compiler.context.i64_type();
            let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
            let fn_type = i64_type.fn_type(&[ptr_type.into()], false);
            compiler.module.add_function("strlen", fn_type, None)
        });

        // Compile arguments
        let path_value = compiler.compile_expression(&args[0])?;
        let path_ptr = path_value.into_pointer_value();
        let content_value = compiler.compile_expression(&args[1])?;
        let content_ptr = content_value.into_pointer_value();

        // Create mode string "w"
        let mode_str = compiler.builder.build_global_string_ptr("w", "write_mode")?;

        // Call fopen
        let file_ptr = compiler
            .builder
            .build_call(
                fopen_fn,
                &[path_ptr.into(), mode_str.as_pointer_value().into()],
                "fopen_call",
            )?
            .try_as_basic_value()
            .left()
            .unwrap();

        // Check if file opened successfully
        let is_null = compiler
            .builder
            .build_is_null(file_ptr.into_pointer_value(), "is_null")?;
        let current_fn = compiler.current_function.unwrap();
        let success_block = compiler.context.append_basic_block(current_fn, "file_opened");
        let error_block = compiler.context.append_basic_block(current_fn, "file_error");
        let merge_block = compiler.context.append_basic_block(current_fn, "merge");

        compiler.builder
            .build_conditional_branch(is_null, error_block, success_block)?;

        // Success block: write file
        compiler.builder.position_at_end(success_block);

        // Get content length
        let content_len = compiler
            .builder
            .build_call(strlen_fn, &[content_ptr.into()], "strlen_call")?
            .try_as_basic_value()
            .left()
            .unwrap();

        // Write content
        compiler.builder.build_call(
            fwrite_fn,
            &[
                content_ptr.into(),
                compiler.context.i64_type().const_int(1, false).into(),
                content_len.into(),
                file_ptr.into(),
            ],
            "fwrite_call",
        )?;

        // Close file
        compiler.builder
            .build_call(fclose_fn, &[file_ptr.into()], "fclose_call")?;

        // Create Result.Ok(void)
        let result_ok = compiler.create_result_ok_void()?;
        compiler.builder.build_unconditional_branch(merge_block)?;
        let success_value = result_ok;

        // Error block: return Result.Err
        compiler.builder.position_at_end(error_block);
        let error_msg = compiler
            .builder
            .build_global_string_ptr("Failed to write file", "write_error_msg")?;
        let result_err = compiler.create_result_err(error_msg.as_pointer_value().into())?;
        compiler.builder.build_unconditional_branch(merge_block)?;
        let error_value = result_err;

        // Merge block
        compiler.builder.position_at_end(merge_block);
        let phi = compiler
            .builder
            .build_phi(success_value.get_type(), "result_phi")?;
        phi.add_incoming(&[(&success_value, success_block), (&error_value, error_block)]);

        Ok(phi.as_basic_value())
    }

    /// Compile fs.exists function call
pub fn compile_fs_exists<'ctx>(
        compiler: &mut LLVMCompiler<'ctx>,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if args.len() != 1 {
            return Err(CompileError::TypeError(
                format!("fs.exists expects 1 argument, got {}", args.len()),
                None,
            ));
        }

        // Get or declare access
        let access_fn = compiler.module.get_function("access").unwrap_or_else(|| {
            let i32_type = compiler.context.i32_type();
            let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
            let fn_type = i32_type.fn_type(&[ptr_type.into(), i32_type.into()], false);
            compiler.module.add_function("access", fn_type, None)
        });

        // Compile the path argument
        let path_value = compiler.compile_expression(&args[0])?;
        let path_ptr = path_value.into_pointer_value();

        // F_OK = 0 (check for existence)
        let f_ok = compiler.context.i32_type().const_int(0, false);

        // Call access
        let result = compiler
            .builder
            .build_call(access_fn, &[path_ptr.into(), f_ok.into()], "access_call")?
            .try_as_basic_value()
            .left()
            .unwrap();

        // Compare result with 0 (success)
        let zero = compiler.context.i32_type().const_int(0, false);
        let exists = compiler.builder.build_int_compare(
            inkwell::IntPredicate::EQ,
            result.into_int_value(),
            zero,
            "exists",
        )?;

        Ok(exists.into())
    }

    /// Compile fs.remove_file function call
pub fn compile_fs_remove_file<'ctx>(
        compiler: &mut LLVMCompiler<'ctx>,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if args.len() != 1 {
            return Err(CompileError::TypeError(
                format!("fs.remove_file expects 1 argument, got {}", args.len()),
                None,
            ));
        }

        // Get or declare unlink
        let unlink_fn = compiler.module.get_function("unlink").unwrap_or_else(|| {
            let i32_type = compiler.context.i32_type();
            let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
            let fn_type = i32_type.fn_type(&[ptr_type.into()], false);
            compiler.module.add_function("unlink", fn_type, None)
        });

        // Compile the path argument
        let path_value = compiler.compile_expression(&args[0])?;
        let path_ptr = path_value.into_pointer_value();

        // Call unlink
        let result = compiler
            .builder
            .build_call(unlink_fn, &[path_ptr.into()], "unlink_call")?
            .try_as_basic_value()
            .left()
            .unwrap();

        // Check if successful (result == 0)
        let zero = compiler.context.i32_type().const_int(0, false);
        let is_success = compiler.builder.build_int_compare(
            inkwell::IntPredicate::EQ,
            result.into_int_value(),
            zero,
            "is_success",
        )?;

        let current_fn = compiler.current_function.unwrap();
        let success_block = compiler
            .context
            .append_basic_block(current_fn, "remove_success");
        let error_block = compiler.context.append_basic_block(current_fn, "remove_error");
        let merge_block = compiler.context.append_basic_block(current_fn, "merge");

        compiler.builder
            .build_conditional_branch(is_success, success_block, error_block)?;

        // Success block
        compiler.builder.position_at_end(success_block);
        let result_ok = compiler.create_result_ok_void()?;
        compiler.builder.build_unconditional_branch(merge_block)?;
        let success_value = result_ok;

        // Error block
        compiler.builder.position_at_end(error_block);
        let error_msg = compiler
            .builder
            .build_global_string_ptr("Failed to remove file", "remove_error_msg")?;
        let result_err = compiler.create_result_err(error_msg.as_pointer_value().into())?;
        compiler.builder.build_unconditional_branch(merge_block)?;
        let error_value = result_err;

        // Merge block
        compiler.builder.position_at_end(merge_block);
        let phi = compiler
            .builder
            .build_phi(success_value.get_type(), "result_phi")?;
        phi.add_incoming(&[(&success_value, success_block), (&error_value, error_block)]);

        Ok(phi.as_basic_value())
    }

    /// Compile fs.create_dir function call
pub fn compile_fs_create_dir<'ctx>(
        compiler: &mut LLVMCompiler<'ctx>,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if args.len() != 1 {
            return Err(CompileError::TypeError(
                format!("fs.create_dir expects 1 argument, got {}", args.len()),
                None,
            ));
        }

        // Get or declare mkdir
        let mkdir_fn = compiler.module.get_function("mkdir").unwrap_or_else(|| {
            let i32_type = compiler.context.i32_type();
            let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
            let fn_type = i32_type.fn_type(&[ptr_type.into(), i32_type.into()], false);
            compiler.module.add_function("mkdir", fn_type, None)
        });

        // Compile the path argument
        let path_value = compiler.compile_expression(&args[0])?;
        let path_ptr = path_value.into_pointer_value();

        // Mode = 0755 (rwxr-xr-x)
        let mode = compiler.context.i32_type().const_int(0o755, false);

        // Call mkdir
        let result = compiler
            .builder
            .build_call(mkdir_fn, &[path_ptr.into(), mode.into()], "mkdir_call")?
            .try_as_basic_value()
            .left()
            .unwrap();

        // Check if successful (result == 0)
        let zero = compiler.context.i32_type().const_int(0, false);
        let is_success = compiler.builder.build_int_compare(
            inkwell::IntPredicate::EQ,
            result.into_int_value(),
            zero,
            "is_success",
        )?;

        let current_fn = compiler.current_function.unwrap();
        let success_block = compiler.context.append_basic_block(current_fn, "mkdir_success");
        let error_block = compiler.context.append_basic_block(current_fn, "mkdir_error");
        let merge_block = compiler.context.append_basic_block(current_fn, "merge");

        compiler.builder
            .build_conditional_branch(is_success, success_block, error_block)?;

        // Success block
        compiler.builder.position_at_end(success_block);
        let result_ok = compiler.create_result_ok_void()?;
        compiler.builder.build_unconditional_branch(merge_block)?;
        let success_value = result_ok;

        // Error block
        compiler.builder.position_at_end(error_block);
        let error_msg = compiler
            .builder
            .build_global_string_ptr("Failed to create directory", "mkdir_error_msg")?;
        let result_err = compiler.create_result_err(error_msg.as_pointer_value().into())?;
        compiler.builder.build_unconditional_branch(merge_block)?;
        let error_value = result_err;

        // Merge block
        compiler.builder.position_at_end(merge_block);
        let phi = compiler
            .builder
            .build_phi(success_value.get_type(), "result_phi")?;
        phi.add_incoming(&[(&success_value, success_block), (&error_value, error_block)]);

        Ok(phi.as_basic_value())
    }

    /// Helper function to create Result.Ok with a value
pub fn create_result_ok<'ctx>(
        compiler: &mut LLVMCompiler<'ctx>,
        value: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Create Result struct {discriminant: 0, payload: value}
        let result_type = compiler.context.struct_type(
            &[
                compiler.context.i64_type().into(),
                compiler.context
                    .ptr_type(inkwell::AddressSpace::default())
                    .into(),
            ],
            false,
        );

        let mut result = result_type.get_undef();
        result = compiler
            .builder
            .build_insert_value(
                result,
                compiler.context.i64_type().const_int(0, false),
                0,
                "set_ok",
            )?
            .into_struct_value();
        result = compiler
            .builder
            .build_insert_value(result, value, 1, "set_payload")?
            .into_struct_value();

        Ok(result.into())
    }

    /// Helper function to create Result.Ok(void)
pub fn create_result_ok_void<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Create Result struct {discriminant: 0, payload: null}
        let result_type = compiler.context.struct_type(
            &[
                compiler.context.i64_type().into(),
                compiler.context
                    .ptr_type(inkwell::AddressSpace::default())
                    .into(),
            ],
            false,
        );

        let mut result = result_type.get_undef();
        result = compiler
            .builder
            .build_insert_value(
                result,
                compiler.context.i64_type().const_int(0, false),
                0,
                "set_ok",
            )?
            .into_struct_value();
        let null_ptr = compiler
            .context
            .ptr_type(inkwell::AddressSpace::default())
            .const_null();
        result = compiler
            .builder
            .build_insert_value(result, null_ptr, 1, "set_payload")?
            .into_struct_value();

        Ok(result.into())
    }

    /// Helper function to create Result.Err with an error message
pub fn create_result_err<'ctx>(
        compiler: &mut LLVMCompiler<'ctx>,
        error: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Create Result struct {discriminant: 1, payload: error}
        let result_type = compiler.context.struct_type(
            &[
                compiler.context.i64_type().into(),
                compiler.context
                    .ptr_type(inkwell::AddressSpace::default())
                    .into(),
            ],
            false,
        );

        let mut result = result_type.get_undef();
        result = compiler
            .builder
            .build_insert_value(
                result,
                compiler.context.i64_type().const_int(1, false),
                0,
                "set_err",
            )?
            .into_struct_value();
        result = compiler
            .builder
            .build_insert_value(result, error, 1, "set_error")?
            .into_struct_value();

        Ok(result.into())
    }
    
    /// Compile HashMap.new() - creates a new HashMap with allocator
    pub fn compile_hashmap_new<'ctx>(
        compiler: &mut LLVMCompiler<'ctx>,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // HashMap REQUIRES an allocator per NO-GC design
        if args.is_empty() {
            return Err(CompileError::TypeError(
                "HashMap.new() requires an allocator argument for NO-GC memory management".to_string(),
                None,
            ));
        }
        
        // HashMap struct: { buckets_ptr, size, capacity, allocator_ptr }
        let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
        let hashmap_struct_type = compiler.context.struct_type(
            &[
                ptr_type.into(),                 // buckets pointer
                compiler.context.i64_type().into(),  // size
                compiler.context.i64_type().into(),  // capacity
                ptr_type.into(),                 // allocator pointer
            ],
            false,
        );
        
        // Use provided allocator
        let allocator_ptr = compiler.compile_expression(&args[0])?;
        
        // Initial capacity
        let initial_capacity = compiler.context.i64_type().const_int(16, false);
        
        // Allocate buckets array
        let bucket_size = compiler.context.i64_type().const_int(32, false); // Each bucket is 32 bytes (for chaining)
        let total_size = compiler.builder.build_int_mul(initial_capacity, bucket_size, "total_size")?;
        
        // For now, always use malloc (allocator is just stored for future use)
        // TODO: Implement proper allocator interface
        let malloc_fn = compiler.module.get_function("malloc").unwrap_or_else(|| {
            let i64_type = compiler.context.i64_type();
            let fn_type = ptr_type.fn_type(&[i64_type.into()], false);
            compiler.module.add_function("malloc", fn_type, Some(Linkage::External))
        });
        let buckets_ptr = compiler.builder.build_call(malloc_fn, &[total_size.into()], "buckets")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| CompileError::InternalError(
                "malloc should return a pointer".to_string(),
                None,
            ))?;
        
        // Initialize buckets to zero
        let memset_fn = compiler.module.get_function("memset").unwrap_or_else(|| {
            let i32_type = compiler.context.i32_type();
            let i64_type = compiler.context.i64_type();
            let fn_type = ptr_type.fn_type(&[ptr_type.into(), i32_type.into(), i64_type.into()], false);
            compiler.module.add_function("memset", fn_type, Some(Linkage::External))
        });
        
        compiler.builder.build_call(
            memset_fn,
            &[
                buckets_ptr.into(),
                compiler.context.i32_type().const_int(0, false).into(),
                total_size.into(),
            ],
            "memset_call",
        )?;
        
        // Create the HashMap struct
        let hashmap_alloca = compiler.builder.build_alloca(hashmap_struct_type, "hashmap")?;
        
        // Store buckets pointer
        let buckets_field = compiler.builder.build_struct_gep(
            hashmap_struct_type,
            hashmap_alloca,
            0,
            "buckets_field",
        )?;
        compiler.builder.build_store(buckets_field, buckets_ptr)?;
        
        // Store size (initially 0)
        let size_field = compiler.builder.build_struct_gep(
            hashmap_struct_type,
            hashmap_alloca,
            1,
            "size_field",
        )?;
        compiler.builder.build_store(size_field, compiler.context.i64_type().const_int(0, false))?;
        
        // Store capacity
        let capacity_field = compiler.builder.build_struct_gep(
            hashmap_struct_type,
            hashmap_alloca,
            2,
            "capacity_field",
        )?;
        compiler.builder.build_store(capacity_field, initial_capacity)?;
        
        // Load and return the hashmap struct
        // Store allocator pointer
        let allocator_field = compiler.builder.build_struct_gep(
            hashmap_struct_type,
            hashmap_alloca,
            3,
            "allocator_field",
        )?;
        compiler.builder.build_store(allocator_field, allocator_ptr)?;
        
        let result = compiler.builder.build_load(hashmap_struct_type, hashmap_alloca, "hashmap_value")?;
        Ok(result)
    }
    
    /// Compile HashSet.new() - creates a new HashSet
    pub fn compile_hashset_new<'ctx>(
        compiler: &mut LLVMCompiler<'ctx>,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // HashSet REQUIRES an allocator per NO-GC design
        if args.is_empty() {
            return Err(CompileError::TypeError(
                "HashSet.new() requires an allocator argument for NO-GC memory management".to_string(),
                None,
            ));
        }
        
        // HashSet uses the same structure as HashMap (but without values)
        // HashSet struct: { buckets_ptr, size, capacity, allocator_ptr }
        let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
        let hashset_struct_type = compiler.context.struct_type(
            &[
                ptr_type.into(),                 // buckets pointer
                compiler.context.i64_type().into(),  // size
                compiler.context.i64_type().into(),  // capacity
                ptr_type.into(),                 // allocator pointer
            ],
            false,
        );
        
        // Use provided allocator
        let allocator_ptr = compiler.compile_expression(&args[0])?;
        
        // Initial capacity
        let initial_capacity = compiler.context.i64_type().const_int(16, false);
        
        // Allocate buckets array
        let bucket_size = compiler.context.i64_type().const_int(16, false); // Each bucket is 16 bytes (just key + next pointer)
        let total_size = compiler.builder.build_int_mul(initial_capacity, bucket_size, "total_size")?;
        
        // Call malloc
        let malloc_fn = compiler.module.get_function("malloc").unwrap_or_else(|| {
            let i64_type = compiler.context.i64_type();
            let fn_type = ptr_type.fn_type(&[i64_type.into()], false);
            compiler.module.add_function("malloc", fn_type, Some(Linkage::External))
        });
        
        let buckets_ptr = compiler.builder.build_call(malloc_fn, &[total_size.into()], "buckets")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| CompileError::InternalError(
                "malloc should return a pointer".to_string(),
                None,
            ))?;
        
        // Initialize buckets to zero
        let memset_fn = compiler.module.get_function("memset").unwrap_or_else(|| {
            let i32_type = compiler.context.i32_type();
            let i64_type = compiler.context.i64_type();
            let fn_type = ptr_type.fn_type(&[ptr_type.into(), i32_type.into(), i64_type.into()], false);
            compiler.module.add_function("memset", fn_type, Some(Linkage::External))
        });
        
        compiler.builder.build_call(
            memset_fn,
            &[
                buckets_ptr.into(),
                compiler.context.i32_type().const_int(0, false).into(),
                total_size.into(),
            ],
            "memset_call",
        )?;
        
        // Create the HashSet struct
        let hashset_alloca = compiler.builder.build_alloca(hashset_struct_type, "hashset")?;
        
        // Store buckets pointer
        let buckets_field = compiler.builder.build_struct_gep(
            hashset_struct_type,
            hashset_alloca,
            0,
            "buckets_field",
        )?;
        compiler.builder.build_store(buckets_field, buckets_ptr)?;
        
        // Store size (initially 0)
        let size_field = compiler.builder.build_struct_gep(
            hashset_struct_type,
            hashset_alloca,
            1,
            "size_field",
        )?;
        compiler.builder.build_store(size_field, compiler.context.i64_type().const_int(0, false))?;
        
        // Store capacity
        let capacity_field = compiler.builder.build_struct_gep(
            hashset_struct_type,
            hashset_alloca,
            2,
            "capacity_field",
        )?;
        compiler.builder.build_store(capacity_field, initial_capacity)?;

        // Store allocator pointer
        let allocator_field = compiler.builder.build_struct_gep(
            hashset_struct_type,
            hashset_alloca,
            3,
            "allocator_field",
        )?;
        compiler.builder.build_store(allocator_field, allocator_ptr)?;

        // Load and return the hashset struct
        let result = compiler.builder.build_load(hashset_struct_type, hashset_alloca, "hashset_value")?;
        Ok(result)
    }
    
    /// Compile DynVec.new() - creates a new DynVec with allocator
    pub fn compile_dynvec_new<'ctx>(
        compiler: &mut LLVMCompiler<'ctx>,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // DynVec REQUIRES an allocator per NO-GC design
        if args.is_empty() {
            return Err(CompileError::TypeError(
                "DynVec.new() requires an allocator argument for NO-GC memory management".to_string(),
                None,
            ));
        }
        
        // DynVec struct: { ptr, length, capacity, allocator }
        let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
        let dynvec_struct_type = compiler.context.struct_type(
            &[
                ptr_type.into(),                 // data pointer
                compiler.context.i64_type().into(),  // length
                compiler.context.i64_type().into(),  // capacity
                ptr_type.into(),                 // allocator pointer
            ],
            false,
        );
        
        // Use provided allocator
        let allocator_ptr = compiler.compile_expression(&args[0])?;
        
        // Initial capacity
        let initial_capacity = compiler.context.i64_type().const_int(10, false);
        
        // Allocate memory for initial capacity (8 bytes per element for i64)
        let element_size = compiler.context.i64_type().const_int(8, false);
        let total_size = compiler.builder.build_int_mul(initial_capacity, element_size, "total_size")?;
        
        // For now, always use malloc (allocator is just stored for future use)
        // TODO: Implement proper allocator interface
        let malloc_fn = compiler.module.get_function("malloc").unwrap_or_else(|| {
            let i64_type = compiler.context.i64_type();
            let fn_type = ptr_type.fn_type(&[i64_type.into()], false);
            compiler.module.add_function("malloc", fn_type, Some(Linkage::External))
        });
        let data_ptr = compiler.builder.build_call(malloc_fn, &[total_size.into()], "dynvec_data")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| CompileError::InternalError(
                "malloc should return a pointer".to_string(),
                None,
            ))?;
        
        // Create the DynVec struct
        let dynvec_alloca = compiler.builder.build_alloca(dynvec_struct_type, "dynvec")?;
        
        // Store data pointer
        let data_field = compiler.builder.build_struct_gep(
            dynvec_struct_type,
            dynvec_alloca,
            0,
            "data_field",
        )?;
        compiler.builder.build_store(data_field, data_ptr)?;
        
        // Store length (initially 0)
        let length_field = compiler.builder.build_struct_gep(
            dynvec_struct_type,
            dynvec_alloca,
            1,
            "length_field",
        )?;
        compiler.builder.build_store(length_field, compiler.context.i64_type().const_int(0, false))?;
        
        // Store capacity
        let capacity_field = compiler.builder.build_struct_gep(
            dynvec_struct_type,
            dynvec_alloca,
            2,
            "capacity_field",
        )?;
        compiler.builder.build_store(capacity_field, initial_capacity)?;
        
        // Store allocator pointer
        let allocator_field = compiler.builder.build_struct_gep(
            dynvec_struct_type,
            dynvec_alloca,
            3,
            "allocator_field",
        )?;
        compiler.builder.build_store(allocator_field, allocator_ptr)?;
        
        // Load and return the dynvec struct
        let result = compiler.builder.build_load(dynvec_struct_type, dynvec_alloca, "dynvec_value")?;
        Ok(result)
    }
