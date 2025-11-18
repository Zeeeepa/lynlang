//! Math module codegen - sqrt, sin, cos, etc.

use super::super::super::LLVMCompiler;
use crate::ast;
use crate::error::CompileError;
use inkwell::values::BasicValueEnum;

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

            // Make sure both integers have the same type
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
