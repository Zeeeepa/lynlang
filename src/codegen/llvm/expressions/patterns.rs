use super::super::LLVMCompiler;
use crate::ast::Expression;
use crate::error::CompileError;
use inkwell::values::BasicValueEnum;

pub fn compile_pattern_match<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    expr: &Expression,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    match expr {
        Expression::QuestionMatch { scrutinee, arms } => {
            // Compile the scrutinee expression
            let scrutinee_val = compiler.compile_expression(scrutinee)?;
            
            // Get the current function and create blocks for pattern matching
            let current_fn = compiler.current_function.ok_or_else(|| {
                CompileError::InternalError("Pattern matching outside function".to_string(), None)
            })?;
            
            // Create blocks for each arm and a merge block
            let mut arm_blocks = Vec::new();
            let merge_block = compiler.context.append_basic_block(current_fn, "pattern_merge");
            
            // Compile each arm
            for (i, arm) in arms.iter().enumerate() {
                let arm_block = compiler.context.append_basic_block(current_fn, &format!("arm_{}", i));
                arm_blocks.push(arm_block);
            }
            
            // Test patterns and branch to appropriate arms
            let mut current_block = compiler.builder.get_insert_block().unwrap();
            
            for (i, arm) in arms.iter().enumerate() {
                // Test the pattern
                let (matches, bindings) = compiler.compile_pattern_test(&scrutinee_val, &arm.pattern)?;
                
                // Create a block for this arm's body
                let body_block = compiler.context.append_basic_block(current_fn, &format!("arm_{}_body", i));
                let next_block = if i < arms.len() - 1 {
                    Some(arm_blocks[i + 1])
                } else {
                    Some(merge_block)
                };
                
                // Branch based on pattern match
                compiler.builder.position_at_end(current_block);
                compiler.builder.build_conditional_branch(
                    matches,
                    body_block,
                    next_block.unwrap(),
                )?;
                
                // Compile the arm body
                compiler.builder.position_at_end(body_block);
                
                // Apply pattern bindings
                compiler.apply_pattern_bindings(&bindings);
                
                // Compile the body expression - handle Block expressions specially
                match &arm.body {
                    Expression::Block(stmts) => {
                        // Compile statements in the block
                        for stmt in stmts {
                            compiler.compile_statement(stmt)?;
                        }
                    }
                    _ => {
                        compiler.compile_expression(&arm.body)?;
                    }
                }
                
                // Branch to merge block
                compiler.builder.build_unconditional_branch(merge_block)?;
                
                current_block = next_block.unwrap();
            }
            
            // Position builder at merge block
            compiler.builder.position_at_end(merge_block);
            
            // For now, return void - in a full implementation, we'd need to phi nodes
            // to merge values from different arms
            // Void expressions don't produce a value, so we return a dummy i32
            Ok(compiler.context.i32_type().const_int(0, false).into())
        }
        Expression::Conditional { scrutinee, arms } => {
            // Similar to QuestionMatch but simpler - just boolean check
            let scrutinee_val = compiler.compile_expression(scrutinee)?;
            
            let current_fn = compiler.current_function.ok_or_else(|| {
                CompileError::InternalError("Conditional outside function".to_string(), None)
            })?;
            
            let then_block = compiler.context.append_basic_block(current_fn, "then");
            let else_block = compiler.context.append_basic_block(current_fn, "else");
            let merge_block = compiler.context.append_basic_block(current_fn, "cond_merge");
            
            // Branch based on scrutinee
            compiler.builder.build_conditional_branch(
                scrutinee_val.into_int_value(),
                then_block,
                else_block,
            )?;
            
            // Compile then branch
            compiler.builder.position_at_end(then_block);
            if let Some(then_arm) = arms.first() {
                match &then_arm.body {
                    Expression::Block(stmts) => {
                        for stmt in stmts {
                            compiler.compile_statement(stmt)?;
                        }
                    }
                    _ => {
                        compiler.compile_expression(&then_arm.body)?;
                    }
                }
            }
            compiler.builder.build_unconditional_branch(merge_block)?;
            
            // Compile else branch
            compiler.builder.position_at_end(else_block);
            if arms.len() > 1 {
                if let Some(else_arm) = arms.get(1) {
                    match &else_arm.body {
                        Expression::Block(stmts) => {
                            for stmt in stmts {
                                compiler.compile_statement(stmt)?;
                            }
                        }
                        _ => {
                            compiler.compile_expression(&else_arm.body)?;
                        }
                    }
                }
            }
            compiler.builder.build_unconditional_branch(merge_block)?;
            
            // Position at merge
            compiler.builder.position_at_end(merge_block);
            // Void expressions don't produce a value, so we return a dummy i32
            Ok(compiler.context.i32_type().const_int(0, false).into())
        }
        _ => Err(CompileError::InternalError(
            format!("Expected QuestionMatch or Conditional, got {:?}", expr),
            None,
        )),
    }
}

