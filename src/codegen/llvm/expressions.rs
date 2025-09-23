use super::{LLVMCompiler, symbols};
use crate::ast::Expression;
use crate::error::CompileError;
use inkwell::values::{BasicValueEnum, BasicValue, PointerValue};

impl<'ctx> LLVMCompiler<'ctx> {
    pub fn compile_expression(&mut self, expr: &Expression) -> Result<BasicValueEnum<'ctx>, CompileError> {
        match expr {
            Expression::Integer8(value) => {
                Ok(self.context.i8_type().const_int(*value as u64, true).into())
            }
            Expression::Integer16(value) => {
                Ok(self.context.i16_type().const_int(*value as u64, true).into())
            }
            Expression::Integer32(value) => {
                Ok(self.context.i32_type().const_int(*value as u64, true).into())
            }
            Expression::Integer64(value) => {
                Ok(self.context.i64_type().const_int(*value as u64, true).into())
            }
            Expression::Unsigned8(value) => {
                Ok(self.context.i8_type().const_int(*value as u64, false).into())
            }
            Expression::Unsigned16(value) => {
                Ok(self.context.i16_type().const_int(*value as u64, false).into())
            }
            Expression::Unsigned32(value) => {
                Ok(self.context.i32_type().const_int(*value as u64, false).into())
            }
            Expression::Unsigned64(value) => {
                Ok(self.context.i64_type().const_int(*value, false).into())
            }
            Expression::Float32(value) => {
                self.compile_float_literal(*value as f64)
            }
            Expression::Float64(value) => {
                self.compile_float_literal(*value)
            }
            Expression::Boolean(value) => {
                Ok(self.context.bool_type().const_int(*value as u64, false).into())
            }
            Expression::String(value) => {
                self.compile_string_literal(value)
            }
            Expression::Identifier(name) => {
                self.compile_identifier(name)
            }
            Expression::BinaryOp { left, op, right } => {
                self.compile_binary_operation(op, left, right)
            }
            Expression::FunctionCall { name, args } => {
                self.compile_function_call(name, args)
            }
            Expression::QuestionMatch { scrutinee, arms } => {
                self.compile_conditional_expression(scrutinee, arms)
            }
            Expression::Conditional { scrutinee, arms } => {
                // Convert ConditionalArm to MatchArm for compilation
                let match_arms: Vec<crate::ast::MatchArm> = arms.iter().map(|arm| {
                    crate::ast::MatchArm {
                        pattern: arm.pattern.clone(),
                        guard: arm.guard.clone(),
                        body: arm.body.clone(),
                    }
                }).collect();
                self.compile_conditional_expression(scrutinee, &match_arms)
            }
            Expression::AddressOf(expr) => {
                self.compile_address_of(expr)
            }
            Expression::Dereference(expr) => {
                self.compile_dereference(expr)
            }
            Expression::PointerOffset { pointer, offset } => {
                self.compile_pointer_offset(pointer, offset)
            }
            // Zen spec pointer operations
            Expression::PointerDereference(expr) => {
                // Same as regular dereference, but more explicit in intent
                self.compile_dereference(expr)
            }
            Expression::PointerAddress(expr) => {
                // Get the numeric address value from a pointer (ptr.addr returns usize)
                self.compile_pointer_to_int(expr)
            }
            Expression::CreateReference(expr) => {
                // Create an immutable reference (Ptr<T>)
                self.compile_address_of(expr)
            }
            Expression::CreateMutableReference(expr) => {
                // Create a mutable reference (MutPtr<T>)
                self.compile_address_of(expr)
            }
            Expression::StructLiteral { name, fields } => {
                self.compile_struct_literal(name, fields)
            }
            Expression::StructField { struct_, field } => {
                self.compile_struct_field(struct_, field)
            }
            Expression::ArrayLiteral(elements) => {
                self.compile_array_literal(elements)
            }
            Expression::ArrayIndex { array, index } => {
                self.compile_array_index(array, index)
            }
            Expression::EnumVariant { enum_name, variant, payload } => {
                self.compile_enum_variant(enum_name, variant, payload)
            }
            Expression::EnumLiteral { variant, payload } => {
                // For enum literals, infer the enum type from context
                // For now, handle Option and Result types specially
                if variant == "Some" || variant == "None" {
                    self.compile_enum_variant("Option", variant, payload)
                } else if variant == "Ok" || variant == "Err" {
                    self.compile_enum_variant("Result", variant, payload)
                } else {
                    // Try to infer from context or error
                    return Err(CompileError::TypeMismatch {
                        expected: "known enum type".to_string(),
                        found: format!("enum literal .{}", variant),
                        span: None,
                    });
                }
            }
            Expression::MemberAccess { object, member } => {
                self.compile_member_access(object, member)
            }
            Expression::StringLength(expr) => {
                self.compile_string_length(expr)
            }
            Expression::StringInterpolation { parts } => {
                self.compile_string_interpolation(parts)
            }
            Expression::Comptime(expr) => {
                self.compile_comptime_expression(expr)
            }
            Expression::Range { start, end, inclusive } => {
                self.compile_range_expression(start, end, *inclusive)
            }
            Expression::PatternMatch { scrutinee, arms } => {
                self.compile_pattern_match(scrutinee, arms)
            }
            Expression::StdReference => {
                // For now, std modules return a placeholder value
                // This will be expanded when we implement the module system
                Ok(self.context.i32_type().const_int(0, false).into())
            }
            Expression::ThisReference => {
                // For now, modules return a placeholder value
                // This will be expanded when we implement the module system  
                Ok(self.context.i32_type().const_int(0, false).into())
            }
            Expression::Block(statements) => {
                // Compile block expression - evaluates to last expression or void
                for stmt in statements {
                    // Note: compile_statement returns (), we need to handle this differently
                    self.compile_statement(stmt)?;
                    // For now, blocks evaluate to void (i32 0)
                }
                // For now, blocks always return void
                Ok(self.context.i32_type().const_int(0, false).into())
            }
            Expression::Return(expr) => {
                // Compile return expression 
                let return_val = self.compile_expression(expr)?;
                // Generate return instruction
                self.builder.build_return(Some(&return_val))
                    .map_err(|e| CompileError::InternalError(format!("Failed to build return: {:?}", e), None))?;
                // Return expressions don't actually return a value in the normal sense,
                // but we need to return something for the type system
                Ok(return_val)
            }
            Expression::TypeCast { expr, target_type } => {
                self.compile_type_cast(expr, target_type)
            }
            Expression::MethodCall { object, method, args } => {
                // Special handling for range.loop()
                if method == "loop" {
                    if let Expression::Range { start, end, inclusive } = object.as_ref() {
                        // Implement range iteration
                        return self.compile_range_loop(start, end, *inclusive, args);
                    }
                }
                
                // Implement UFC (Uniform Function Call)
                // object.method(args) becomes method(object, args)
                
                // First compile the object
                let object_value = self.compile_expression(object)?;
                
                // Special handling for pointer methods
                if method == "val" {
                    // Dereference pointer
                    if object_value.is_pointer_value() {
                        let deref = self.builder.build_load(
                            object_value.get_type(),
                            object_value.into_pointer_value(),
                            "ptr_deref"
                        ).map_err(|e| CompileError::from(e))?;
                        return Ok(deref);
                    }
                } else if method == "addr" {
                    // Get pointer address as integer
                    if object_value.is_pointer_value() {
                        let addr = self.builder.build_ptr_to_int(
                            object_value.into_pointer_value(),
                            self.context.i64_type(),
                            "ptr_addr"
                        ).map_err(|e| CompileError::from(e))?;
                        return Ok(addr.into());
                    }
                } else if method == "raise" {
                    // Handle Result.raise() - for now just return the value
                    // TODO: Implement proper error propagation
                    return Ok(object_value);
                }
                
                // Try trait method resolution first
                if let Ok(result) = self.compile_method_call(object, method, args) {
                    return Ok(result);
                }
                
                // For regular UFC calls, prepend object to arguments
                let mut ufc_args = vec![*object.clone()];
                ufc_args.extend(args.clone());
                
                // Try to call as a regular function with object as first argument
                self.compile_function_call(method, &ufc_args)
            }
            Expression::Loop { body } => {
                // Loop expression: loop(() { body })
                // This creates an infinite loop that must be exited with break
                
                let loop_body_block = self.context.append_basic_block(self.current_function.unwrap(), "loop_expr_body");
                let after_loop_block = self.context.append_basic_block(self.current_function.unwrap(), "after_loop_expr");
                
                // Push loop context for break/continue
                self.loop_stack.push((loop_body_block, after_loop_block));
                
                // Jump to loop body
                self.builder.build_unconditional_branch(loop_body_block).map_err(|e| CompileError::from(e))?;
                self.builder.position_at_end(loop_body_block);
                
                // Compile the loop body based on its type
                match body.as_ref() {
                    Expression::Closure { params: _, body: closure_body } => {
                        // If the closure body is a Block, compile its statements
                        if let Expression::Block(statements) = closure_body.as_ref() {
                            for stmt in statements {
                                self.compile_statement(stmt)?;
                            }
                        } else {
                            // Otherwise compile the expression
                            self.compile_expression(closure_body)?;
                        }
                    }
                    Expression::Block(statements) => {
                        // Direct block in loop
                        for stmt in statements {
                            self.compile_statement(stmt)?;
                        }
                    }
                    _ => {
                        // Otherwise just compile the expression
                        self.compile_expression(body)?;
                    }
                }
                
                // Loop back if no terminator (no break was executed)
                let current_block = self.builder.get_insert_block().unwrap();
                if current_block.get_terminator().is_none() {
                    self.builder.build_unconditional_branch(loop_body_block).map_err(|e| CompileError::from(e))?;
                }
                
                self.loop_stack.pop();
                self.builder.position_at_end(after_loop_block);
                
                // Return void value
                Ok(self.context.i32_type().const_int(0, false).into())
            }
            Expression::Closure { params: _, body: _ } => {
                // TODO: Implement closures
                Ok(self.context.i32_type().const_int(0, false).into())
            }
            Expression::Raise(expr) => {
                // .raise() propagates errors early by transforming to pattern matching:
                // expr.raise() becomes:
                // expr ? 
                //     | Ok(val) { val }
                //     | Err(e) { return Err(e) }
                self.compile_raise_expression(expr)
            }
            Expression::Break { label: _ } => {
                // Break out of the current loop
                if let Some((_, after_loop)) = self.loop_stack.last() {
                    self.builder.build_unconditional_branch(*after_loop).map_err(|e| CompileError::from(e))?;
                    // Create a new block after the break (unreachable but needed for LLVM)
                    let unreachable_block = self.context.append_basic_block(self.current_function.unwrap(), "after_break");
                    self.builder.position_at_end(unreachable_block);
                    // Return a dummy value
                    Ok(self.context.i32_type().const_int(0, false).into())
                } else {
                    Err(CompileError::InternalError("Break outside of loop".to_string(), None))
                }
            }
            Expression::Continue { label: _ } => {
                // Continue to the next iteration of the current loop
                if let Some((loop_header, _)) = self.loop_stack.last() {
                    self.builder.build_unconditional_branch(*loop_header).map_err(|e| CompileError::from(e))?;
                    // Create a new block after the continue (unreachable but needed for LLVM)
                    let unreachable_block = self.context.append_basic_block(self.current_function.unwrap(), "after_continue");
                    self.builder.position_at_end(unreachable_block);
                    // Return a dummy value
                    Ok(self.context.i32_type().const_int(0, false).into())
                } else {
                    Err(CompileError::InternalError("Continue outside of loop".to_string(), None))
                }
            }
            Expression::VecConstructor { element_type, size, initial_values } => {
                self.compile_vec_constructor(element_type, *size, initial_values.as_ref())
            }
            Expression::DynVecConstructor { element_types, allocator, initial_capacity } => {
                self.compile_dynvec_constructor(element_types, allocator, initial_capacity.as_ref().map(|v| &**v))
            }
        }
    }

    fn compile_conditional_expression(&mut self, scrutinee: &Expression, arms: &[crate::ast::MatchArm]) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Generate a unique ID for this conditional to avoid block name collisions
        static mut CONDITIONAL_ID: u32 = 0;
        let cond_id = unsafe {
            CONDITIONAL_ID += 1;
            CONDITIONAL_ID
        };
        
        let parent_function = self.current_function
            .ok_or_else(|| CompileError::InternalError("No current function for conditional".to_string(), None))?;
        
        // Compile the scrutinee expression
        let scrutinee_val = self.compile_expression(scrutinee)?;
        
        // Create the merge block where all arms will jump to
        let merge_bb = self.context.append_basic_block(parent_function, &format!("match_merge_{}", cond_id));
        
        // We'll collect the values and blocks for the phi node
        let mut phi_values = Vec::new();
        
        // Track the current "next" block for fallthrough
        let mut _current_block = self.builder.get_insert_block().unwrap();
        let mut unmatched_block = None;
        
        // Check if we have exhaustive boolean patterns (true and false) for the entire conditional
        let _is_exhaustive_boolean = arms.len() == 2 && {
            let has_true = arms.iter().any(|a| {
                if let crate::ast::Pattern::Literal(expr) = &a.pattern {
                    matches!(expr, crate::ast::Expression::Boolean(true))
                } else {
                    false
                }
            });
            let has_false = arms.iter().any(|a| {
                if let crate::ast::Pattern::Literal(expr) = &a.pattern {
                    matches!(expr, crate::ast::Expression::Boolean(false))
                } else {
                    false
                }
            });
            has_true && has_false
        };
        
        for (i, arm) in arms.iter().enumerate() {
            let is_last = i == arms.len() - 1;
            
            // Test the pattern
            let (matches, bindings) = self.compile_pattern_test(&scrutinee_val, &arm.pattern)?;
            
            // Check guard if present
            let final_condition = if let Some(guard_expr) = &arm.guard {
                // Save current variables before applying bindings
                let saved_vars = self.apply_pattern_bindings(&bindings);
                
                // Compile the guard expression
                let guard_val = self.compile_expression(guard_expr)?;
                
                // Restore variables
                self.restore_variables(saved_vars);
                
                // The final condition is: pattern matches AND guard is true
                if !guard_val.is_int_value() {
                    return Err(CompileError::TypeMismatch {
                        expected: "boolean for guard expression".to_string(),
                        found: format!("{:?}", guard_val.get_type()),
                        span: None,
                    });
                }
                
                self.builder.build_and(matches, guard_val.into_int_value(), "guard_and_pattern")?   
            } else {
                matches
            };
            
            // Create blocks for this arm
            let match_bb = self.context.append_basic_block(parent_function, &format!("match_{}_{}", cond_id, i));
            
            // For boolean patterns (true/false), we always need a next block for the false case
            // Check if this is a wildcard pattern which is always exhaustive
            let _is_wildcard = matches!(arm.pattern, crate::ast::Pattern::Wildcard);
            
            let next_bb = if !is_last {
                self.context.append_basic_block(parent_function, &format!("test_{}_{}", cond_id, i + 1))
            } else {
                // For the last arm, create an unmatched block
                // This will be used if the pattern doesn't match
                // Even for exhaustive patterns, we need this for correct control flow
                let block = self.context.append_basic_block(parent_function, &format!("pattern_unmatched_{}", cond_id));
                unmatched_block = Some(block);
                block
            };
            
            // Branch based on the condition
            self.builder.build_conditional_branch(final_condition, match_bb, next_bb)?;
            
            // Generate code for the match block
            self.builder.position_at_end(match_bb);
            
            // Apply pattern bindings
            let saved_vars = self.apply_pattern_bindings(&bindings);
            
            // Compile the arm body
            let arm_val = self.compile_expression(&arm.body)?;
            
            // Restore variables
            self.restore_variables(saved_vars);
            
            // Jump to merge block
            self.builder.build_unconditional_branch(merge_bb)?;
            let match_bb_end = self.builder.get_insert_block().unwrap();
            
            // Save value and block for phi node
            phi_values.push((arm_val, match_bb_end));
            
            // Position at the next test block for the next iteration
            if !is_last {
                self.builder.position_at_end(next_bb);
                _current_block = next_bb;
            }
        }
        
        // Handle unmatched pattern block if we created one
        if let Some(unmatched_bb) = unmatched_block {
            self.builder.position_at_end(unmatched_bb);
            // For now, just return a default value (0) and branch to merge
            // In a complete implementation, this would be a runtime error
            // Use the same type as the first arm to ensure type consistency
            let default_val = if !phi_values.is_empty() {
                // Create a zero value of the same type as the first arm
                match phi_values[0].0.get_type() {
                    inkwell::types::BasicTypeEnum::IntType(int_type) => {
                        int_type.const_int(0, false).into()
                    }
                    inkwell::types::BasicTypeEnum::FloatType(float_type) => {
                        float_type.const_float(0.0).into()
                    }
                    _ => {
                        // For other types, use a null pointer or similar default
                        self.context.i32_type().const_int(0, false).into()
                    }
                }
            } else {
                self.context.i32_type().const_int(0, false).into()
            };
            self.builder.build_unconditional_branch(merge_bb)?;
            phi_values.push((default_val, unmatched_bb));
        }
        
        // Position at merge block and create phi node
        self.builder.position_at_end(merge_bb);
        
        if phi_values.is_empty() {
            return Err(CompileError::InternalError("No arms in conditional expression".to_string(), None));
        }
        
        // All values should have the same type
        let result_type = phi_values[0].0.get_type();
        let phi = self.builder.build_phi(result_type, "match_result")?;
        
        // Add all incoming values
        for (value, block) in &phi_values {
            phi.add_incoming(&[(value, *block)]);
        }
        
        Ok(phi.as_basic_value())
    }

    fn compile_array_literal(&mut self, elements: &[Expression]) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Infer type from first element or default to i32
        let element_type = if !elements.is_empty() {
            // Compile first element to get its type
            let first_val = self.compile_expression(&elements[0])?;
            match first_val.get_type() {
                inkwell::types::BasicTypeEnum::IntType(_) => self.context.i32_type(),
                _ => self.context.i32_type() // Default to i32 for other types too
            }
        } else {
            self.context.i32_type()
        };
        
        let array_len = elements.len() as u32;
        let _array_type = element_type.array_type(array_len);

        // Allocate the array on the heap (malloc)
        let elem_size = element_type.size_of();
        let i64_type = self.context.i64_type();
        let total_size = i64_type.const_int(array_len as u64, false);
        let malloc_fn = self.module.get_function("malloc").ok_or_else(|| CompileError::InternalError("No malloc function declared".to_string(), None))?;
        let size = self.builder.build_int_mul(elem_size, total_size, "arraysize");
        let raw_ptr = self.builder.build_call(malloc_fn, &[size?.into()], "arraymalloc")?.try_as_basic_value().left().unwrap().into_pointer_value();
        let array_ptr = self.builder.build_pointer_cast(raw_ptr, self.context.ptr_type(inkwell::AddressSpace::default()), "arrayptr")?;

        // Store each element
        for (i, expr) in elements.iter().enumerate() {
            let value = self.compile_expression(expr)?;
            let gep = unsafe {
                self.builder.build_gep(element_type, array_ptr, &[element_type.const_int(i as u64, false)], &format!("arrayidx{}", i))?
            };
            self.builder.build_store(gep, value)?;
        }
        Ok(array_ptr.as_basic_value_enum())
    }

    fn compile_array_index(&mut self, array: &Expression, index: &Expression) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Get the address of the indexed element
        let gep = self.compile_array_index_address(array, index)?;
        
        // Try to infer element type from context. Default to i32 for compatibility with tests
        // TODO: Proper type inference for array elements from declaration
        let element_type = self.context.i32_type();
        
        // Load the value from the address
        let loaded = self.builder.build_load(element_type, gep, "arrayload")?;
        Ok(loaded)
    }
    
    pub fn compile_array_index_address(&mut self, array: &Expression, index: &Expression) -> Result<PointerValue<'ctx>, CompileError> {
        // Compile array expression - should be a pointer
        let array_val = self.compile_expression(array)?;
        
        // Get the actual pointer value
        let array_ptr = if array_val.is_pointer_value() {
            array_val.into_pointer_value()
        } else {
            return Err(CompileError::TypeError(
                format!("Array indexing requires pointer type, got {:?}", array_val.get_type()),
                None
            ));
        };
        
        // Try to infer element type from context. Default to i32 for compatibility with tests
        // TODO: Proper type inference for array elements from declaration
        let element_type = self.context.i32_type();
        
        let index_val = self.compile_expression(index)?;
        let gep = unsafe {
            self.builder.build_gep(element_type, array_ptr, &[index_val.into_int_value()], "arrayidx")?
        };
        Ok(gep)
    }

    pub(super) fn compile_enum_variant(&mut self, enum_name: &str, variant: &str, payload: &Option<Box<Expression>>) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Look up the enum info from the symbol table
        let enum_info = if enum_name.is_empty() {
            // If enum_name is empty, we need to search for an enum that has this variant
            // This happens when we use .Some(x) without specifying the enum type
            let mut found_enum = None;
            for symbol in self.symbols.all_symbols() {
                if let symbols::Symbol::EnumType(info) = symbol {
                    if info.variant_indices.contains_key(variant) {
                        found_enum = Some(info.clone());
                        break;
                    }
                }
            }
            match found_enum {
                Some(info) => info,
                None => {
                    // Fallback to basic representation if enum not found in symbol table
                    // This maintains backward compatibility
                    let tag = 0;
                    let tag_val = self.context.i64_type().const_int(tag, false);
                    let payload_val = if let Some(expr) = payload {
                        let compiled = self.compile_expression(expr)?;
                        if compiled.is_pointer_value() {
                            self.builder.build_ptr_to_int(
                                compiled.into_pointer_value(),
                                self.context.i64_type(),
                                "ptr_as_int"
                            )?.into()
                        } else {
                            compiled
                        }
                    } else {
                        self.context.i64_type().const_int(0, false).into()
                    };
                    let enum_struct_type = self.context.struct_type(&[
                        self.context.i64_type().into(),
                        self.context.i64_type().into(),
                    ], false);
                    let alloca = self.builder.build_alloca(enum_struct_type, &format!("{}_{}_enum_tmp", enum_name, variant))?;
                    let tag_ptr = self.builder.build_struct_gep(enum_struct_type, alloca, 0, "tag_ptr")?;
                    self.builder.build_store(tag_ptr, tag_val)?;
                    let payload_ptr = self.builder.build_struct_gep(enum_struct_type, alloca, 1, "payload_ptr")?;
                    self.builder.build_store(payload_ptr, payload_val)?;
                    let loaded = self.builder.build_load(enum_struct_type, alloca, &format!("{}_{}_enum_val", enum_name, variant))?;
                    return Ok(loaded);
                }
            }
        } else {
            match self.symbols.lookup(enum_name) {
                Some(symbols::Symbol::EnumType(info)) => info.clone(),
                _ => {
                    // Fallback to basic representation if enum not found in symbol table
                    // This maintains backward compatibility
                    let tag = 0;
                    let tag_val = self.context.i64_type().const_int(tag, false);
                    let payload_val = if let Some(expr) = payload {
                        let compiled = self.compile_expression(expr)?;
                        if compiled.is_pointer_value() {
                            self.builder.build_ptr_to_int(
                                compiled.into_pointer_value(),
                                self.context.i64_type(),
                                "ptr_as_int"
                            )?.into()
                        } else {
                            compiled
                        }
                    } else {
                        self.context.i64_type().const_int(0, false).into()
                    };
                    let enum_struct_type = self.context.struct_type(&[
                        self.context.i64_type().into(),
                        self.context.i64_type().into(),
                    ], false);
                    let alloca = self.builder.build_alloca(enum_struct_type, &format!("{}_{}_enum_tmp", enum_name, variant))?;
                    let tag_ptr = self.builder.build_struct_gep(enum_struct_type, alloca, 0, "tag_ptr")?;
                    self.builder.build_store(tag_ptr, tag_val)?;
                    let payload_ptr = self.builder.build_struct_gep(enum_struct_type, alloca, 1, "payload_ptr")?;
                    self.builder.build_store(payload_ptr, payload_val)?;
                    let loaded = self.builder.build_load(enum_struct_type, alloca, &format!("{}_{}_enum_val", enum_name, variant))?;
                    return Ok(loaded);
                }
            }
        };
        
        // Look up the variant index
        let tag = enum_info.variant_indices.get(variant)
            .copied()
            .ok_or_else(|| CompileError::UndeclaredVariable(
                format!("Unknown variant '{}' for enum '{}'", variant, enum_name),
                None
            ))?;
        
        let tag_val = self.context.i64_type().const_int(tag, false);
        let payload_val = if let Some(expr) = payload {
            let compiled = self.compile_expression(expr)?;
            // Convert pointer values to i64 for storage in enum
            if compiled.is_pointer_value() {
                self.builder.build_ptr_to_int(
                    compiled.into_pointer_value(),
                    self.context.i64_type(),
                    "ptr_as_int"
                )?.into()
            } else {
                compiled
            }
        } else {
            self.context.i64_type().const_int(0, false).into()
        };
        
        // Use the enum's LLVM type
        let enum_struct_type = enum_info.llvm_type;
        let alloca = self.builder.build_alloca(enum_struct_type, &format!("{}_{}_enum_tmp", enum_name, variant))?;
        let tag_ptr = self.builder.build_struct_gep(enum_struct_type, alloca, 0, "tag_ptr")?;
        self.builder.build_store(tag_ptr, tag_val)?;
        let payload_ptr = self.builder.build_struct_gep(enum_struct_type, alloca, 1, "payload_ptr")?;
        self.builder.build_store(payload_ptr, payload_val)?;
        let loaded = self.builder.build_load(enum_struct_type, alloca, &format!("{}_{}_enum_val", enum_name, variant))?;
        Ok(loaded)
    }

    fn compile_member_access(&mut self, object: &Expression, member: &str) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Check if this is a stdlib module constant access (e.g., math.pi)
        if let Expression::Identifier(module_name) = object {
            // Check if this is a known stdlib module with constants
            if module_name == "math" && member == "pi" {
                // Return the value of pi as a float constant
                return Ok(self.context.f64_type().const_float(std::f64::consts::PI).into());
            }
        }
        
        // Check if this is an enum variant access (GameEntity.Player syntax)
        if let Expression::Identifier(enum_name) = object {
            // Check if this identifier is an enum type
            if let Some(symbols::Symbol::EnumType(enum_info)) = self.symbols.lookup(enum_name) {
                // This is an enum variant creation, not a member access
                // Check if the member is a valid variant
                if enum_info.variant_indices.contains_key(member) {
                    // Create the enum variant
                    return self.compile_enum_variant(enum_name, member, &None);
                }
            }
        }
        
        // Not an enum or stdlib constant, delegate to the struct field access logic
        self.compile_struct_field(object, member)
    }

    fn compile_comptime_expression(&mut self, expr: &Expression) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Evaluate the expression at compile time using the persistent evaluator
        match self.comptime_evaluator.evaluate_expression(expr) {
            Ok(value) => {
                // Convert the comptime value to a constant expression and compile it
                let const_expr = value.to_expression()?;
                self.compile_expression(&const_expr)
            }
            Err(e) => {
                return Err(CompileError::InternalError(
                    format!("Comptime evaluation error: {}", e),
                    None
                ));
            }
        }
    }

    fn compile_pattern_match(&mut self, scrutinee: &Expression, arms: &[crate::ast::PatternArm]) -> Result<BasicValueEnum<'ctx>, CompileError> {
        let parent_function = self.current_function
            .ok_or_else(|| CompileError::InternalError("No current function for pattern match".to_string(), None))?;
        
        // Compile the scrutinee expression
        let scrutinee_val = self.compile_expression(scrutinee)?;
        
        // Create the merge block where all arms will jump to
        let merge_bb = self.context.append_basic_block(parent_function, "match_merge");
        
        // We'll collect the values and blocks for the phi node
        let mut phi_values = Vec::new();
        
        // Track the current "next" block for fallthrough
        let mut _current_block = self.builder.get_insert_block().unwrap();
        let mut unmatched_block = None;
        
        for (i, arm) in arms.iter().enumerate() {
            let is_last = i == arms.len() - 1;
            
            // Test the pattern
            let (matches, bindings) = self.compile_pattern_test(&scrutinee_val, &arm.pattern)?;
            
            // Check guard if present
            let final_condition = if let Some(guard_expr) = &arm.guard {
                // Save current variables before applying bindings
                let saved_vars = self.apply_pattern_bindings(&bindings);
                
                // Compile the guard expression
                let guard_val = self.compile_expression(guard_expr)?;
                
                // Restore variables
                self.restore_variables(saved_vars);
                
                // The final condition is: pattern matches AND guard is true
                if !guard_val.is_int_value() {
                    return Err(CompileError::TypeMismatch {
                        expected: "boolean for guard expression".to_string(),
                        found: format!("{:?}", guard_val.get_type()),
                        span: None,
                    });
                }
                
                self.builder.build_and(matches, guard_val.into_int_value(), "guard_and_pattern")?   
            } else {
                matches
            };
            
            // Create blocks for this arm
            let match_bb = self.context.append_basic_block(parent_function, &format!("match_{}", i));
            
            // Determine the next block - if this arm doesn't match, where do we go?
            let next_bb = if !is_last {
                // Not the last arm - go to test the next pattern
                self.context.append_basic_block(parent_function, &format!("test_{}", i + 1))
            } else {
                // Last arm - create an unmatched block
                let block = self.context.append_basic_block(parent_function, "pattern_unmatched");
                unmatched_block = Some(block);
                block
            };
            
            // Branch based on the condition
            self.builder.build_conditional_branch(final_condition, match_bb, next_bb)?;
            
            // Generate code for the match block
            self.builder.position_at_end(match_bb);
            
            // Apply pattern bindings
            let saved_vars = self.apply_pattern_bindings(&bindings);
            
            // Compile the arm body
            let arm_val = self.compile_expression(&arm.body)?;
            
            // Restore variables
            self.restore_variables(saved_vars);
            
            // Jump to merge block
            self.builder.build_unconditional_branch(merge_bb)?;
            let match_bb_end = self.builder.get_insert_block().unwrap();
            
            // Save value and block for phi node
            phi_values.push((arm_val, match_bb_end));
            
            // Position at the next test block for the next iteration
            if !is_last {
                self.builder.position_at_end(next_bb);
                _current_block = next_bb;
            }
        }
        
        // Handle unmatched pattern block if we created one
        if let Some(unmatched_bb) = unmatched_block {
            self.builder.position_at_end(unmatched_bb);
            // For now, just return a default value (0) and branch to merge
            // In a complete implementation, this would be a runtime error
            // Use the same type as the first arm to ensure type consistency
            let default_val = if !phi_values.is_empty() {
                // Create a zero value of the same type as the first arm
                match phi_values[0].0.get_type() {
                    inkwell::types::BasicTypeEnum::IntType(int_type) => {
                        int_type.const_int(0, false).into()
                    }
                    inkwell::types::BasicTypeEnum::FloatType(float_type) => {
                        float_type.const_float(0.0).into()
                    }
                    _ => {
                        // For other types, use a null pointer or similar default
                        self.context.i32_type().const_int(0, false).into()
                    }
                }
            } else {
                self.context.i32_type().const_int(0, false).into()
            };
            self.builder.build_unconditional_branch(merge_bb)?;
            phi_values.push((default_val, unmatched_bb));
        }
        
        // Position at merge block and create phi node
        self.builder.position_at_end(merge_bb);
        
        if phi_values.is_empty() {
            return Err(CompileError::InternalError("No arms in pattern match expression".to_string(), None));
        }
        
        // All values should have the same type
        let result_type = phi_values[0].0.get_type();
        let phi = self.builder.build_phi(result_type, "match_result")?;
        
        // Add all incoming values
        for (value, block) in &phi_values {
            phi.add_incoming(&[(value, *block)]);
        }
        
        Ok(phi.as_basic_value())
    }

    fn compile_range_expression(&mut self, start: &Expression, end: &Expression, inclusive: bool) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // For now, represent ranges as a simple struct { start: i64, end: i64, inclusive: bool }
        let start_val = self.compile_expression(start)?;
        let end_val = self.compile_expression(end)?;
        
        // Create a simple struct type for the range
        let _range_struct_type = self.context.struct_type(&[
            start_val.get_type(),
            end_val.get_type(),
            self.context.bool_type().into(),
        ], false);
        
        // Create the range struct value
        let range_struct = self.context.const_struct(&[
            start_val,
            end_val,
            self.context.bool_type().const_int(inclusive as u64, false).into(),
        ], false);
        
        Ok(range_struct.as_basic_value_enum())
    }
    
    fn compile_type_cast(&mut self, expr: &Expression, target_type: &crate::ast::AstType) -> Result<BasicValueEnum<'ctx>, CompileError> {
        use inkwell::values::{IntValue, PointerValue};
        use crate::ast::AstType;
        
        let value = self.compile_expression(expr)?;
        let _target_llvm_type = self.to_llvm_type(target_type)?;
        
        // Handle pointer casts
        if matches!(target_type, AstType::Ptr(_)) {
            // Cast to pointer type
            if let Ok(ptr_val) = value.try_into() {
                let ptr_val: PointerValue = ptr_val;
                let casted = self.builder.build_pointer_cast(
                    ptr_val,
                    self.context.ptr_type(inkwell::AddressSpace::default()),
                    "cast"
                )?;
                return Ok(casted.as_basic_value_enum());
            } else if let Ok(int_val) = value.try_into() {
                // Integer to pointer cast
                let int_val: IntValue = int_val;
                let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                let casted = self.builder.build_int_to_ptr(int_val, ptr_type, "inttoptr")?;
                return Ok(casted.as_basic_value_enum());
            }
        }
        
        // Handle integer casts
        match (value, target_type) {
            (BasicValueEnum::IntValue(int_val), AstType::I8 | AstType::U8) => {
                let target = self.context.i8_type();
                let casted = self.builder.build_int_cast(int_val, target, "cast")?;
                Ok(casted.as_basic_value_enum())
            }
            (BasicValueEnum::IntValue(int_val), AstType::I16 | AstType::U16) => {
                let target = self.context.i16_type();
                let casted = self.builder.build_int_cast(int_val, target, "cast")?;
                Ok(casted.as_basic_value_enum())
            }
            (BasicValueEnum::IntValue(int_val), AstType::I32 | AstType::U32) => {
                let target = self.context.i32_type();
                let casted = self.builder.build_int_cast(int_val, target, "cast")?;
                Ok(casted.as_basic_value_enum())
            }
            (BasicValueEnum::IntValue(int_val), AstType::I64 | AstType::U64) => {
                let target = self.context.i64_type();
                let casted = self.builder.build_int_cast(int_val, target, "cast")?;
                Ok(casted.as_basic_value_enum())
            }
            // Float to int casts
            (BasicValueEnum::FloatValue(float_val), AstType::I32 | AstType::U32) => {
                let target = self.context.i32_type();
                let casted = self.builder.build_float_to_signed_int(float_val, target, "fptosi")?;
                Ok(casted.as_basic_value_enum())
            }
            // Int to float casts
            (BasicValueEnum::IntValue(int_val), AstType::F32) => {
                let target = self.context.f32_type();
                let casted = self.builder.build_signed_int_to_float(int_val, target, "sitofp")?;
                Ok(casted.as_basic_value_enum())
            }
            (BasicValueEnum::IntValue(int_val), AstType::F64) => {
                let target = self.context.f64_type();
                let casted = self.builder.build_signed_int_to_float(int_val, target, "sitofp")?;
                Ok(casted.as_basic_value_enum())
            }
            _ => {
                // For unhandled casts, just return the value as-is
                // This could be improved with better type checking
                Ok(value)
            }
        }
    }
    
    fn compile_range_loop(&mut self, start: &Expression, end: &Expression, inclusive: bool, args: &[Expression]) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Extract the loop variable and body from args
        // args[0] should be a closure with one parameter
        let closure = args.get(0).ok_or_else(|| 
            CompileError::InternalError("Range.loop() requires a closure argument".to_string(), None))?;
        
        let (param_name, loop_body) = match closure {
            Expression::Closure { params, body } => {
                let param = params.get(0).ok_or_else(||
                    CompileError::InternalError("Range.loop() closure must have one parameter".to_string(), None))?;
                (param.0.clone(), body.as_ref())
            }
            _ => return Err(CompileError::InternalError("Range.loop() requires a closure argument".to_string(), None))
        };
        
        // Compile start and end values
        let start_val = self.compile_expression(start)?;
        let end_val = self.compile_expression(end)?;
        
        // Ensure both are integers
        let (start_int, end_int) = match (start_val, end_val) {
            (BasicValueEnum::IntValue(s), BasicValueEnum::IntValue(e)) => (s, e),
            _ => return Err(CompileError::InternalError("Range bounds must be integers".to_string(), None))
        };
        
        // Create loop blocks
        let loop_header = self.context.append_basic_block(self.current_function.unwrap(), "range_header");
        let loop_body_block = self.context.append_basic_block(self.current_function.unwrap(), "range_body");
        let after_loop = self.context.append_basic_block(self.current_function.unwrap(), "after_range");
        
        // Allocate space for the loop variable
        let loop_var = self.builder.build_alloca(start_int.get_type(), &param_name)?;
        self.builder.build_store(loop_var, start_int)?;
        
        // Jump to loop header
        self.builder.build_unconditional_branch(loop_header)?;
        self.builder.position_at_end(loop_header);
        
        // Load current value
        let current = self.builder.build_load(start_int.get_type(), loop_var, &param_name)?;
        let current_int = current.into_int_value();
        
        // Check loop condition
        let condition = if inclusive {
            self.builder.build_int_compare(
                inkwell::IntPredicate::SLE,
                current_int,
                end_int,
                "range_check"
            )?
        } else {
            self.builder.build_int_compare(
                inkwell::IntPredicate::SLT,
                current_int,
                end_int,
                "range_check"
            )?
        };
        
        self.builder.build_conditional_branch(condition, loop_body_block, after_loop)?;
        self.builder.position_at_end(loop_body_block);
        
        // Push loop context for break/continue
        self.loop_stack.push((loop_header, after_loop));
        
        // Store the loop variable in symbols table
        use crate::codegen::llvm::symbols::Symbol;
        self.symbols.insert(&param_name, Symbol::Variable(loop_var));
        
        // Enter a new scope for the loop body
        self.symbols.enter_scope();
        
        // Also store in the variables map for compatibility
        self.variables.insert(param_name.clone(), super::VariableInfo {
            pointer: loop_var,
            ast_type: crate::ast::AstType::I32,
            is_mutable: true,  // Loop variables are mutable
            is_initialized: true,
        });
        
        // Compile the loop body
        match loop_body {
            Expression::Block(statements) => {
                for stmt in statements {
                    self.compile_statement(stmt)?;
                }
            }
            _ => {
                self.compile_expression(loop_body)?;
            }
        }
        
        // Exit the scope and remove from variables map
        self.symbols.exit_scope();
        self.variables.remove(&param_name);
        
        // Increment the loop variable
        let current = self.builder.build_load(start_int.get_type(), loop_var, &param_name)?;
        let one = start_int.get_type().const_int(1, false);
        let next = self.builder.build_int_add(current.into_int_value(), one, "next")?;
        self.builder.build_store(loop_var, next)?;
        
        // Jump back to header if no break was executed
        let current_block = self.builder.get_insert_block().unwrap();
        if current_block.get_terminator().is_none() {
            self.builder.build_unconditional_branch(loop_header)?;
        }
        
        self.loop_stack.pop();
        self.builder.position_at_end(after_loop);
        
        // Return void value
        Ok(self.context.i32_type().const_int(0, false).into())
    }
    
    /// Compile Vec<T, size>() constructor
    fn compile_vec_constructor(
        &mut self, 
        element_type: &crate::ast::AstType, 
        size: usize, 
        initial_values: Option<&Vec<Expression>>
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // For now, simplify Vec to just return a placeholder
        // TODO: Implement proper Vec with array storage
        
        // Get element LLVM type
        let _elem_llvm_type = self.to_llvm_type(element_type)?;
        
        // Create a simple struct type: { i64 capacity, i64 len }
        let i64_type = self.context.i64_type();
        let vec_struct_type = self.context.struct_type(&[
            i64_type.into(),  // capacity
            i64_type.into(),  // length
        ], false);
        
        // Allocate the Vec on the stack
        let vec_ptr = self.builder.build_alloca(vec_struct_type, "vec")?;
        
        // Initialize capacity field
        let cap_field_ptr = self.builder.build_struct_gep(vec_struct_type, vec_ptr, 0, "cap_field")?;
        let cap_value = i64_type.const_int(size as u64, false);
        self.builder.build_store(cap_field_ptr, cap_value)?;
        
        // Initialize length field
        let initial_len = if let Some(values) = initial_values {
            values.len() as u64
        } else {
            0
        };
        let len_field_ptr = self.builder.build_struct_gep(vec_struct_type, vec_ptr, 1, "len_field")?;
        let len_value = i64_type.const_int(initial_len, false);
        self.builder.build_store(len_field_ptr, len_value)?;
        
        // TODO: Actually allocate and initialize the data array
        
        // Load and return the Vec struct value
        Ok(self.builder.build_load(vec_struct_type, vec_ptr, "vec_value")?)
    }
    
    /// Compile DynVec<T>() or DynVec<T1, T2, ...>() constructor
    fn compile_dynvec_constructor(
        &mut self,
        element_types: &[crate::ast::AstType],
        allocator: &Expression,
        initial_capacity: Option<&Expression>
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Compile allocator expression
        let _allocator_value = self.compile_expression(allocator)?;
        
        // Get initial capacity (default to 8 if not specified)
        let capacity_value = if let Some(cap_expr) = initial_capacity {
            self.compile_expression(cap_expr)?
        } else {
            self.context.i64_type().const_int(8, false).into()
        };
        
        let capacity_int = match capacity_value {
            BasicValueEnum::IntValue(int_val) => int_val,
            _ => return Err(CompileError::TypeError("DynVec capacity must be integer".to_string(), None)),
        };
        
        // Create DynVec struct based on element types
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let len_type = self.context.i64_type();
        let cap_type = self.context.i64_type();
        
        let dynvec_struct_type = if element_types.len() == 1 {
            // Single type DynVec: { ptr, len, capacity }
            self.context.struct_type(&[
                ptr_type.into(),
                len_type.into(), 
                cap_type.into(),
            ], false)
        } else {
            // Mixed variant DynVec: { ptr, len, capacity, discriminants }
            let discriminant_ptr = self.context.ptr_type(inkwell::AddressSpace::default());
            self.context.struct_type(&[
                ptr_type.into(),
                len_type.into(),
                cap_type.into(),
                discriminant_ptr.into(),
            ], false)
        };
        
        // Allocate the DynVec on the stack
        let dynvec_ptr = self.builder.build_alloca(dynvec_struct_type, "dynvec")?;
        
        // Initialize data pointer to null initially
        let data_field_ptr = self.builder.build_struct_gep(dynvec_struct_type, dynvec_ptr, 0, "data_field")?;
        let null_ptr = ptr_type.const_null();
        self.builder.build_store(data_field_ptr, null_ptr)?;
        
        // Initialize length to 0
        let len_field_ptr = self.builder.build_struct_gep(dynvec_struct_type, dynvec_ptr, 1, "len_field")?;
        let len_zero = len_type.const_int(0, false);
        self.builder.build_store(len_field_ptr, len_zero)?;
        
        // Initialize capacity
        let cap_field_ptr = self.builder.build_struct_gep(dynvec_struct_type, dynvec_ptr, 2, "cap_field")?;
        self.builder.build_store(cap_field_ptr, capacity_int)?;
        
        // For mixed variant DynVec, initialize discriminants pointer to null
        if element_types.len() > 1 {
            let discriminant_field_ptr = self.builder.build_struct_gep(dynvec_struct_type, dynvec_ptr, 3, "discriminant_field")?;
            self.builder.build_store(discriminant_field_ptr, null_ptr)?;
        }
        
        // TODO: Actually allocate memory using the allocator
        // For now, we'll just create the struct with null pointers
        // In a complete implementation, we would:
        // 1. Call the allocator to get memory
        // 2. Store the pointer in the data field
        // 3. For mixed variants, allocate discriminant array
        
        // Load and return the DynVec struct value
        Ok(self.builder.build_load(dynvec_struct_type, dynvec_ptr, "dynvec_value")?)
    }

    /// Compile .raise() expression by transforming it into pattern matching
    fn compile_raise_expression(&mut self, expr: &Expression) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Generate a unique ID for this raise to avoid block name collisions
        static mut RAISE_ID: u32 = 0;
        let raise_id = unsafe {
            RAISE_ID += 1;
            RAISE_ID
        };

        let parent_function = self.current_function
            .ok_or_else(|| CompileError::InternalError("No current function for .raise()".to_string(), None))?;

        // Compile the expression that returns a Result
        let result_value = self.compile_expression(expr)?;

        // For simplicity, we'll assume Result is represented as a struct with:
        // - tag: i8 (0 = Ok, 1 = Err)  
        // - data: union of Ok and Err types
        // In a complete implementation, this would be determined by the type system
        
        // Create blocks for pattern matching
        let check_bb = self.context.append_basic_block(parent_function, &format!("raise_check_{}", raise_id));
        let ok_bb = self.context.append_basic_block(parent_function, &format!("raise_ok_{}", raise_id));
        let err_bb = self.context.append_basic_block(parent_function, &format!("raise_err_{}", raise_id));
        let continue_bb = self.context.append_basic_block(parent_function, &format!("raise_continue_{}", raise_id));
        
        // Jump to check block
        self.builder.build_unconditional_branch(check_bb)?;
        self.builder.position_at_end(check_bb);
        
        // Extract the tag from the Result struct
        // TODO: This is a simplified implementation. In practice, we'd need to:
        // 1. Get the actual Result type from the type system
        // 2. Handle the specific layout (enum discriminant + payload)
        // 3. Support different Result<T,E> instantiations
        
        // For now, assume a simple tagged union representation
        let result_struct_type = result_value.get_type();
        if let inkwell::types::BasicTypeEnum::StructType(struct_type) = result_struct_type {
            // Extract tag field (first field)
            let tag_ptr = self.builder.build_struct_gep(struct_type, result_value.into_pointer_value(), 0, "tag_ptr")?;
            let tag_value = self.builder.build_load(self.context.i8_type(), tag_ptr, "tag")?;
            
            // Compare tag with 0 (Ok)
            let is_ok = self.builder.build_int_compare(
                inkwell::IntPredicate::EQ,
                tag_value.into_int_value(),
                self.context.i8_type().const_int(0, false),
                "is_ok"
            )?;
            
            // Branch based on tag
            self.builder.build_conditional_branch(is_ok, ok_bb, err_bb)?;
            
            // Handle Ok case
            self.builder.position_at_end(ok_bb);
            // Extract the Ok value from the union (second field)
            let value_ptr = self.builder.build_struct_gep(struct_type, result_value.into_pointer_value(), 1, "value_ptr")?;
            // TODO: Load the correct type based on the Result<T,E> instantiation
            let ok_value = self.builder.build_load(self.context.i32_type(), value_ptr, "ok_value")?;
            self.builder.build_unconditional_branch(continue_bb)?;
            
            // Handle Err case - generate early return
            self.builder.position_at_end(err_bb);
            // Extract the Err value
            let err_value_ptr = self.builder.build_struct_gep(struct_type, result_value.into_pointer_value(), 1, "err_value_ptr")?;
            let err_value = self.builder.build_load(self.context.i32_type(), err_value_ptr, "err_value")?;
            
            // Create a new Result with Err tag for early return
            let return_result_ptr = self.builder.build_alloca(struct_type, "return_result")?;
            let return_tag_ptr = self.builder.build_struct_gep(struct_type, return_result_ptr, 0, "return_tag_ptr")?;
            let return_value_ptr = self.builder.build_struct_gep(struct_type, return_result_ptr, 1, "return_value_ptr")?;
            
            // Set tag to 1 (Err)
            self.builder.build_store(return_tag_ptr, self.context.i8_type().const_int(1, false))?;
            // Store the error value
            self.builder.build_store(return_value_ptr, err_value)?;
            
            // Load the complete Result and return it
            let return_result = self.builder.build_load(struct_type, return_result_ptr, "return_result")?;
            self.builder.build_return(Some(&return_result))?;
            
            // Continue with Ok case
            self.builder.position_at_end(continue_bb);
            Ok(ok_value)
        } else {
            // Fallback: if we can't determine the Result structure, just return the value
            // This should not happen with proper type checking
            Ok(result_value)
        }
    }
} 