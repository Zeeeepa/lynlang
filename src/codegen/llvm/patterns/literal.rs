//! Literal pattern compilation - literals, ranges, wildcards, identifiers

use super::super::LLVMCompiler;
use crate::error::CompileError;
use inkwell::values::{BasicValueEnum, IntValue};
use inkwell::IntPredicate;

/// Compile literal pattern (including range literals)
pub fn compile_literal_pattern<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    scrutinee_val: &BasicValueEnum<'ctx>,
    lit_expr: &crate::ast::Expression,
) -> Result<IntValue<'ctx>, CompileError> {
        // Check if the literal expression is a range
        if let crate::ast::Expression::Range {
            start,
            end,
            inclusive,
        } = lit_expr
        {
            // Handle range pattern directly
            if !scrutinee_val.is_int_value() {
                return Err(CompileError::TypeMismatch {
                    expected: "integer for range pattern".to_string(),
                    found: format!("{:?}", scrutinee_val.get_type()),
                    span: None,
                });
            }

            let start_val = compiler.compile_expression(start)?;
            let end_val = compiler.compile_expression(end)?;

            let scrutinee_int = scrutinee_val.into_int_value();

            // Ensure start and end are int values
            if !start_val.is_int_value() || !end_val.is_int_value() {
                return Err(CompileError::TypeMismatch {
                    expected: "integer values for range bounds".to_string(),
                    found: format!(
                        "start: {:?}, end: {:?}",
                        start_val.get_type(),
                        end_val.get_type()
                    ),
                    span: None,
                });
            }

            let start_int = start_val.into_int_value();
            let end_int = end_val.into_int_value();

            // Ensure all integers have the same type
            let scrutinee_type = scrutinee_int.get_type();
            let start_type = start_int.get_type();
            let end_type = end_int.get_type();

            // Cast if needed to match scrutinee type
            let start_int = if start_type != scrutinee_type {
                compiler.builder
                    .build_int_cast(start_int, scrutinee_type, "start_cast")?
            } else {
                start_int
            };

            let end_int = if end_type != scrutinee_type {
                compiler.builder
                    .build_int_cast(end_int, scrutinee_type, "end_cast")?
            } else {
                end_int
            };

            let ge_start = compiler.builder.build_int_compare(
                IntPredicate::SGE,
                scrutinee_int,
                start_int,
                "range_ge",
            )?;

            let le_end = if *inclusive {
                compiler.builder.build_int_compare(
                    IntPredicate::SLE,
                    scrutinee_int,
                    end_int,
                    "range_le",
                )?
            } else {
                compiler.builder.build_int_compare(
                    IntPredicate::SLT,
                    scrutinee_int,
                    end_int,
                    "range_lt",
                )?
            };

            compiler.builder.build_and(ge_start, le_end, "range_match")
        } else {
            // Regular literal pattern
            let pattern_val = compiler.compile_expression(lit_expr)?;
            compiler.values_equal(scrutinee_val, &pattern_val)
        }
}

/// Compile range pattern
pub fn compile_range_pattern<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    scrutinee_val: &BasicValueEnum<'ctx>,
    start: &crate::ast::Expression,
    end: &crate::ast::Expression,
    inclusive: &bool,
) -> Result<IntValue<'ctx>, CompileError> {
    let start_val = compiler.compile_expression(start)?;
    let end_val = compiler.compile_expression(end)?;

    if !scrutinee_val.is_int_value() {
        return Err(CompileError::TypeMismatch {
            expected: "integer for range pattern".to_string(),
            found: format!("{:?}", scrutinee_val.get_type()),
            span: None,
        });
    }

    let scrutinee_int = scrutinee_val.into_int_value();
    let start_int = start_val.into_int_value();
    let end_int = end_val.into_int_value();

    let ge_start = compiler.builder.build_int_compare(
        IntPredicate::SGE,
        scrutinee_int,
        start_int,
        "range_ge",
    )?;

    let le_end = if *inclusive {
        compiler.builder.build_int_compare(
            IntPredicate::SLE,
            scrutinee_int,
            end_int,
            "range_le",
        )?
    } else {
        compiler.builder.build_int_compare(
            IntPredicate::SLT,
            scrutinee_int,
            end_int,
            "range_lt",
        )?
    };

    compiler.builder.build_and(ge_start, le_end, "range_match")
}

/// Compile wildcard pattern (always matches)
pub fn compile_wildcard_pattern<'ctx>(compiler: &LLVMCompiler<'ctx>) -> IntValue<'ctx> {
    compiler.context.bool_type().const_int(1, false)
}

/// Compile identifier pattern (binds the value)
pub fn compile_identifier_pattern<'ctx>(compiler: &LLVMCompiler<'ctx>, _name: &str) -> IntValue<'ctx> {
    compiler.context.bool_type().const_int(1, false)
}

