use super::super::LLVMCompiler;
use crate::ast::{Expression, Statement};
use crate::error::CompileError;
use inkwell::values::BasicValueEnum;

pub fn compile_pattern_match<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    expr: &Expression,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    match expr {
        Expression::QuestionMatch { scrutinee, arms } => {
            let scrutinee_val = compiler.compile_expression(scrutinee)?;

            let current_fn = compiler.current_function.ok_or_else(|| {
                CompileError::InternalError(
                    "Pattern matching outside function".to_string(),
                    compiler.get_current_span(),
                )
            })?;

            let merge_block = compiler
                .context
                .append_basic_block(current_fn, "pattern_merge");
            let default_block = compiler
                .context
                .append_basic_block(current_fn, "pattern_default");

            let current_block = compiler.builder.get_insert_block().unwrap();
            let scrutinee_type = compiler.infer_expression_type(scrutinee).ok();

            if arms.is_empty() {
                return Err(CompileError::InternalError(
                    "Pattern match must have at least one arm".to_string(),
                    None,
                ));
            }

            let mut test_blocks = Vec::new();
            let mut body_blocks = Vec::new();

            for i in 0..arms.len() {
                let test_block = compiler
                    .context
                    .append_basic_block(current_fn, &format!("arm_{}_test", i));
                let body_block = compiler
                    .context
                    .append_basic_block(current_fn, &format!("arm_{}_body", i));
                test_blocks.push(test_block);
                body_blocks.push(body_block);
            }

            compiler.builder.position_at_end(current_block);
            compiler
                .builder
                .build_unconditional_branch(test_blocks[0])?;

            let mut incoming_values: Vec<(BasicValueEnum<'ctx>, inkwell::basic_block::BasicBlock<'ctx>)> = Vec::new();
            let mut first_arm_value: Option<BasicValueEnum<'ctx>> = None;

            for (i, arm) in arms.iter().enumerate() {
                compiler.builder.position_at_end(test_blocks[i]);

                let (matches, bindings) = compiler.compile_pattern_test_with_type(
                    &scrutinee_val,
                    &arm.pattern,
                    scrutinee_type.as_ref(),
                )?;

                let next_test_block = if i < arms.len() - 1 {
                    test_blocks[i + 1]
                } else {
                    default_block
                };

                compiler.builder.build_conditional_branch(
                    matches,
                    body_blocks[i],
                    next_test_block,
                )?;

                compiler.builder.position_at_end(body_blocks[i]);
                compiler.apply_pattern_bindings(&bindings);

                let arm_value = match &arm.body {
                    Expression::Block(stmts) => {
                        if stmts.is_empty() {
                            compiler.context.i32_type().const_int(0, false).into()
                        } else {
                            for stmt in &stmts[..stmts.len() - 1] {
                                compiler.compile_statement(stmt)?;
                            }
                            let last = &stmts[stmts.len() - 1];
                            if let Statement::Expression { expr, .. } = last {
                                compiler.compile_expression(expr)?
                            } else {
                                compiler.compile_statement(last)?;
                                compiler.context.i32_type().const_int(0, false).into()
                            }
                        }
                    }
                    _ => {
                        compiler.compile_expression(&arm.body)?
                    }
                };

                if first_arm_value.is_none() {
                    first_arm_value = Some(arm_value);
                }

                let arm_end_block = compiler.builder.get_insert_block().unwrap();
                if arm_end_block.get_terminator().is_none() {
                    incoming_values.push((arm_value, arm_end_block));
                    compiler.builder.build_unconditional_branch(merge_block)?;
                }
            }

            compiler.builder.position_at_end(default_block);
            let default_value: BasicValueEnum = if let Some(first_val) = first_arm_value {
                if first_val.is_int_value() {
                    first_val.into_int_value().get_type().const_zero().into()
                } else if first_val.is_float_value() {
                    first_val.into_float_value().get_type().const_zero().into()
                } else if first_val.is_pointer_value() {
                    first_val.into_pointer_value().get_type().const_null().into()
                } else {
                    compiler.context.i32_type().const_int(0, false).into()
                }
            } else {
                compiler.context.i32_type().const_int(0, false).into()
            };
            incoming_values.push((default_value, default_block));
            compiler.builder.build_unconditional_branch(merge_block)?;

            compiler.builder.position_at_end(merge_block);

            if incoming_values.is_empty() {
                Ok(compiler.context.i32_type().const_int(0, false).into())
            } else if incoming_values.len() == 1 {
                Ok(incoming_values[0].0)
            } else {
                let first_val = incoming_values[0].0;
                let phi = if first_val.is_int_value() {
                    compiler.builder.build_phi(first_val.into_int_value().get_type(), "match_result")?
                } else if first_val.is_float_value() {
                    compiler.builder.build_phi(first_val.into_float_value().get_type(), "match_result")?
                } else if first_val.is_pointer_value() {
                    compiler.builder.build_phi(first_val.into_pointer_value().get_type(), "match_result")?
                } else if first_val.is_struct_value() {
                    compiler.builder.build_phi(first_val.into_struct_value().get_type(), "match_result")?
                } else {
                    return Ok(compiler.context.i32_type().const_int(0, false).into());
                };

                for (val, block) in &incoming_values {
                    phi.add_incoming(&[(val, *block)]);
                }

                Ok(phi.as_basic_value())
            }
        }
        Expression::Conditional { scrutinee, arms } => {
            let scrutinee_val = compiler.compile_expression(scrutinee)?;

            let current_fn = compiler.current_function.ok_or_else(|| {
                CompileError::InternalError(
                    "Conditional outside function".to_string(),
                    compiler.get_current_span(),
                )
            })?;

            let then_block = compiler.context.append_basic_block(current_fn, "then");
            let else_block = compiler.context.append_basic_block(current_fn, "else");
            let merge_block = compiler
                .context
                .append_basic_block(current_fn, "cond_merge");

            compiler.builder.build_conditional_branch(
                scrutinee_val.into_int_value(),
                then_block,
                else_block,
            )?;

            let mut incoming_values: Vec<(BasicValueEnum<'ctx>, inkwell::basic_block::BasicBlock<'ctx>)> = Vec::new();

            compiler.builder.position_at_end(then_block);
            let then_value = if let Some(then_arm) = arms.first() {
                match &then_arm.body {
                    Expression::Block(stmts) => {
                        if stmts.is_empty() {
                            compiler.context.i32_type().const_int(0, false).into()
                        } else {
                            for stmt in &stmts[..stmts.len() - 1] {
                                compiler.compile_statement(stmt)?;
                            }
                            let last = &stmts[stmts.len() - 1];
                            if let Statement::Expression { expr, .. } = last {
                                compiler.compile_expression(expr)?
                            } else {
                                compiler.compile_statement(last)?;
                                compiler.context.i32_type().const_int(0, false).into()
                            }
                        }
                    }
                    _ => {
                        compiler.compile_expression(&then_arm.body)?
                    }
                }
            } else {
                compiler.context.i32_type().const_int(0, false).into()
            };

            let then_end_block = compiler.builder.get_insert_block().unwrap();
            if then_end_block.get_terminator().is_none() {
                incoming_values.push((then_value, then_end_block));
                compiler.builder.build_unconditional_branch(merge_block)?;
            }

            compiler.builder.position_at_end(else_block);
            let else_value = if arms.len() > 1 {
                if let Some(else_arm) = arms.get(1) {
                    match &else_arm.body {
                        Expression::Block(stmts) => {
                            if stmts.is_empty() {
                                compiler.context.i32_type().const_int(0, false).into()
                            } else {
                                for stmt in &stmts[..stmts.len() - 1] {
                                    compiler.compile_statement(stmt)?;
                                }
                                let last = &stmts[stmts.len() - 1];
                                if let Statement::Expression { expr, .. } = last {
                                    compiler.compile_expression(expr)?
                                } else {
                                    compiler.compile_statement(last)?;
                                    compiler.context.i32_type().const_int(0, false).into()
                                }
                            }
                        }
                        _ => {
                            compiler.compile_expression(&else_arm.body)?
                        }
                    }
                } else {
                    compiler.context.i32_type().const_int(0, false).into()
                }
            } else {
                compiler.context.i32_type().const_int(0, false).into()
            };

            let else_end_block = compiler.builder.get_insert_block().unwrap();
            if else_end_block.get_terminator().is_none() {
                incoming_values.push((else_value, else_end_block));
                compiler.builder.build_unconditional_branch(merge_block)?;
            }

            compiler.builder.position_at_end(merge_block);

            if incoming_values.is_empty() {
                Ok(compiler.context.i32_type().const_int(0, false).into())
            } else if incoming_values.len() == 1 {
                Ok(incoming_values[0].0)
            } else {
                let first_val = incoming_values[0].0;
                let phi = if first_val.is_int_value() {
                    compiler.builder.build_phi(first_val.into_int_value().get_type(), "cond_result")?
                } else if first_val.is_float_value() {
                    compiler.builder.build_phi(first_val.into_float_value().get_type(), "cond_result")?
                } else if first_val.is_pointer_value() {
                    compiler.builder.build_phi(first_val.into_pointer_value().get_type(), "cond_result")?
                } else if first_val.is_struct_value() {
                    compiler.builder.build_phi(first_val.into_struct_value().get_type(), "cond_result")?
                } else {
                    return Ok(compiler.context.i32_type().const_int(0, false).into());
                };

                for (val, block) in &incoming_values {
                    phi.add_incoming(&[(val, *block)]);
                }

                Ok(phi.as_basic_value())
            }
        }
        _ => Err(CompileError::InternalError(
            format!("Expected QuestionMatch or Conditional, got {:?}", expr),
            None,
        )),
    }
}
