use super::{symbols, LLVMCompiler, Type, VariableInfo};
use crate::ast::{AstType, Expression};
use crate::error::CompileError;
use inkwell::module::Linkage;
use inkwell::values::{BasicValue, BasicValueEnum, PointerValue};

impl<'ctx> LLVMCompiler<'ctx> {
    /// Infer the type of an expression for generic type tracking
    pub fn infer_expression_type(&self, expr: &Expression) -> Result<AstType, CompileError> {
        match expr {
            Expression::Integer8(_) => Ok(AstType::I8),
            Expression::Integer16(_) => Ok(AstType::I16),
            Expression::Integer32(_) => Ok(AstType::I32),
            Expression::Integer64(_) => Ok(AstType::I64),
            Expression::Unsigned8(_) => Ok(AstType::U8),
            Expression::Unsigned16(_) => Ok(AstType::U16),
            Expression::Unsigned32(_) => Ok(AstType::U32),
            Expression::Unsigned64(_) => Ok(AstType::U64),
            Expression::Float32(_) => Ok(AstType::F32),
            Expression::Float64(_) => Ok(AstType::F64),
            Expression::Boolean(_) => Ok(AstType::Bool),
            Expression::Unit => Ok(AstType::Void),
            Expression::String(_) => Ok(AstType::String),
            Expression::Identifier(name) => {
                // Look up variable type
                if let Some(var_info) = self.variables.get(name) {
                    Ok(var_info.ast_type.clone())
                } else {
                    Ok(AstType::Void)
                }
            }
            Expression::Range {
                start: _,
                end: _,
                inclusive,
            } => {
                // Range expressions have Range type
                Ok(AstType::Range {
                    start_type: Box::new(AstType::I32),
                    end_type: Box::new(AstType::I32),
                    inclusive: *inclusive,
                })
            }
            _ => Ok(AstType::Void),
        }
    }

    pub fn compile_expression(
        &mut self,
        expr: &Expression,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        match expr {
            Expression::Integer8(value) => {
                Ok(self.context.i8_type().const_int(*value as u64, true).into())
            }
            Expression::Integer16(value) => Ok(self
                .context
                .i16_type()
                .const_int(*value as u64, true)
                .into()),
            Expression::Integer32(value) => Ok(self
                .context
                .i32_type()
                .const_int(*value as u64, true)
                .into()),
            Expression::Integer64(value) => Ok(self
                .context
                .i64_type()
                .const_int(*value as u64, true)
                .into()),
            Expression::Unsigned8(value) => Ok(self
                .context
                .i8_type()
                .const_int(*value as u64, false)
                .into()),
            Expression::Unsigned16(value) => Ok(self
                .context
                .i16_type()
                .const_int(*value as u64, false)
                .into()),
            Expression::Unsigned32(value) => Ok(self
                .context
                .i32_type()
                .const_int(*value as u64, false)
                .into()),
            Expression::Unsigned64(value) => {
                Ok(self.context.i64_type().const_int(*value, false).into())
            }
            Expression::Float32(value) => self.compile_float_literal(*value as f64),
            Expression::Float64(value) => self.compile_float_literal(*value),
            Expression::Boolean(value) => Ok(self
                .context
                .bool_type()
                .const_int(*value as u64, false)
                .into()),
            Expression::Unit => {
                // Unit type is represented as an empty struct in LLVM
                // We use a null pointer to represent unit values
                Ok(self
                    .context
                    .ptr_type(inkwell::AddressSpace::default())
                    .const_null()
                    .into())
            }
            Expression::String(value) => self.compile_string_literal(value),
            Expression::Identifier(name) => self.compile_identifier(name),
            Expression::BinaryOp { left, op, right } => {
                self.compile_binary_operation(op, left, right)
            }
            Expression::FunctionCall { name, args } => self.compile_function_call(name, args),
            Expression::QuestionMatch { scrutinee, arms } => {
                // Convert MatchArm to PatternArm for compilation
                let pattern_arms: Vec<crate::ast::PatternArm> = arms
                    .iter()
                    .map(|arm| crate::ast::PatternArm {
                        pattern: arm.pattern.clone(),
                        guard: arm.guard.clone(),
                        body: arm.body.clone(),
                    })
                    .collect();
                self.compile_pattern_match(scrutinee, &pattern_arms)
            }
            Expression::Conditional { scrutinee, arms } => {
                // Convert ConditionalArm to PatternArm for compilation
                let pattern_arms: Vec<crate::ast::PatternArm> = arms
                    .iter()
                    .map(|arm| crate::ast::PatternArm {
                        pattern: arm.pattern.clone(),
                        guard: arm.guard.clone(),
                        body: arm.body.clone(),
                    })
                    .collect();
                self.compile_pattern_match(scrutinee, &pattern_arms)
            }
            Expression::AddressOf(expr) => self.compile_address_of(expr),
            Expression::Dereference(expr) => self.compile_dereference(expr),
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
            Expression::StructLiteral { name, fields } => self.compile_struct_literal(name, fields),
            Expression::StructField { struct_, field } => self.compile_struct_field(struct_, field),
            Expression::ArrayLiteral(elements) => self.compile_array_literal(elements),
            Expression::ArrayIndex { array, index } => self.compile_array_index(array, index),
            Expression::EnumVariant {
                enum_name,
                variant,
                payload,
            } => self.compile_enum_variant(enum_name, variant, payload),
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
            Expression::StringLength(expr) => self.compile_string_length(expr),
            Expression::StringInterpolation { parts } => self.compile_string_interpolation(parts),
            Expression::Comptime(expr) => self.compile_comptime_expression(expr),
            Expression::Range {
                start,
                end,
                inclusive,
            } => self.compile_range_expression(start, end, *inclusive),
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
                // Don't create a new scope here - blocks in Zen share the same scope
                // This allows variables declared in blocks to be accessible

                // Compile block expression - evaluates to last expression or void
                if statements.is_empty() {
                    // Empty block returns void
                    Ok(self.context.i32_type().const_int(0, false).into())
                } else {
                    // Compile all statements except the last one
                    for stmt in &statements[..statements.len() - 1] {
                        self.compile_statement(stmt)?;
                    }

                    // Check if the last statement is an expression statement
                    // If so, return its value
                    match &statements[statements.len() - 1] {
                        crate::ast::Statement::Expression(expr) => {
                            // The last expression is the block's value
                            self.compile_expression(expr)
                        }
                        _ => {
                            // Last statement is not an expression, return void
                            self.compile_statement(&statements[statements.len() - 1])?;
                            Ok(self.context.i32_type().const_int(0, false).into())
                        }
                    }
                }
            }
            Expression::Return(expr) => {
                // Compile return expression
                let return_val = self.compile_expression(expr)?;
                // Generate return instruction
                self.builder.build_return(Some(&return_val)).map_err(|e| {
                    CompileError::InternalError(format!("Failed to build return: {:?}", e), None)
                })?;
                // Return expressions don't actually return a value in the normal sense,
                // but we need to return something for the type system
                Ok(return_val)
            }
            Expression::TypeCast { expr, target_type } => self.compile_type_cast(expr, target_type),
            Expression::MethodCall {
                object,
                method,
                args,
            } => {
                // Special handling for Array.new() static method
                if let Expression::Identifier(name) = object.as_ref() {
                    if name == "Array" && method == "new" {
                        // This is Array.new() - a static method call
                        return self.compile_array_new(args);
                    }
                    
                    // Special handling for module method calls (GPA.init())
                    if let Some(var_info) = self.variables.get(name) {
                        if var_info.ast_type == AstType::StdModule {
                            // This is a module method call like GPA.init()
                            match method.as_str() {
                                "init" => {
                                    // For allocator modules, init() returns a dummy allocator
                                    // In a real implementation, this would create the actual allocator
                                    return Ok(self.context.i64_type().const_int(1, false).into());
                                }
                                _ => {
                                    return Err(CompileError::TypeError(
                                        format!(
                                            "Unknown method '{}' for module '{}'",
                                            method, name
                                        ),
                                        None,
                                    ));
                                }
                            }
                        }
                    }
                }

                // Special handling for .raise()
                if method == "raise" && args.is_empty() {
                    // Convert to Raise expression
                    return self.compile_raise_expression(object);
                }

                // Special handling for range.loop()
                if method == "loop" {
                    if let Expression::Range {
                        start,
                        end,
                        inclusive,
                    } = object.as_ref()
                    {
                        // Implement range iteration
                        return self.compile_range_loop(start, end, *inclusive, args);
                    }

                    // Check if object is an identifier holding a Range
                    if let Expression::Identifier(name) = object.as_ref() {
                        if let Some(var_info) = self.variables.get(name).cloned() {
                            if let AstType::Range { .. } = &var_info.ast_type {
                                // Load the range struct
                                let range_type = match self.to_llvm_type(&var_info.ast_type)? {
                                    super::Type::Struct(s) => s,
                                    _ => {
                                        return Err(CompileError::InternalError(
                                            "Expected struct type for Range".to_string(),
                                            None,
                                        ))
                                    }
                                };
                                let range_val =
                                    self.builder
                                        .build_load(range_type, var_info.pointer, name)?;

                                // Extract start, end, and inclusive from the struct
                                let range_struct = range_val.into_struct_value();
                                let start_val = self
                                    .builder
                                    .build_extract_value(range_struct, 0, "start")?
                                    .into_int_value();
                                let end_val = self
                                    .builder
                                    .build_extract_value(range_struct, 1, "end")?
                                    .into_int_value();
                                let inclusive_val = self
                                    .builder
                                    .build_extract_value(range_struct, 2, "inclusive")?
                                    .into_int_value();

                                // Check if inclusive (convert bool to actual value)
                                // Check if the constant is non-zero
                                let is_inclusive = if let Some(const_val) =
                                    inclusive_val.get_zero_extended_constant()
                                {
                                    const_val != 0
                                } else {
                                    false // Default to exclusive if we can't determine
                                };

                                // Get the closure from args
                                let closure = args.get(0).ok_or_else(|| {
                                    CompileError::InternalError(
                                        "Range.loop() requires a closure argument".to_string(),
                                        None,
                                    )
                                })?;

                                let (param_name, loop_body) = match closure {
                                    Expression::Closure { params, body } => {
                                        let param = params.get(0).ok_or_else(|| {
                                            CompileError::InternalError(
                                                "Range.loop() closure must have one parameter"
                                                    .to_string(),
                                                None,
                                            )
                                        })?;
                                        (param.0.clone(), body.as_ref())
                                    }
                                    _ => {
                                        return Err(CompileError::InternalError(
                                            "Range.loop() requires a closure argument".to_string(),
                                            None,
                                        ))
                                    }
                                };

                                // Call the range loop implementation
                                return self.compile_range_loop_from_values(
                                    start_val,
                                    end_val,
                                    is_inclusive,
                                    &param_name,
                                    loop_body,
                                );
                            }
                        }
                    }
                }

                // Implement UFC (Uniform Function Call)
                // object.method(args) becomes method(object, args)

                // First compile the object
                let object_value = self.compile_expression(object)?;

                // Special handling for string methods
                // Check if the object is a string value (pointer type)
                if object_value.is_pointer_value() {
                    // Check if this is a string method
                    match method.as_str() {
                        "to_f64" => {
                            // Call the runtime string_to_f64 function
                            let func = self.get_or_create_runtime_function("string_to_f64")?;
                            let result = self
                                .builder
                                .build_call(func, &[object_value.into()], "to_f64_result")?
                                .try_as_basic_value()
                                .left()
                                .ok_or_else(|| {
                                    CompileError::InternalError("string_to_f64 did not return a value".to_string(), None)
                                })?;
                            // Update generic type context for Option<f64>
                            self.generic_type_context
                                .insert("Option_Some_Type".to_string(), crate::ast::AstType::F64);
                            return Ok(result);
                        }
                        "to_i32" => {
                            let func = self.get_or_create_runtime_function("string_to_i32")?;
                            let result = self
                                .builder
                                .build_call(func, &[object_value.into()], "to_i32_result")?
                                .try_as_basic_value()
                                .left()
                                .ok_or_else(|| {
                                    CompileError::InternalError(
                                        "string_to_i32 should return a value".to_string(),
                                        None,
                                    )
                                })?;
                            return Ok(result);
                        }
                        "to_i64" => {
                            let func = self.get_or_create_runtime_function("string_to_i64")?;
                            let result = self
                                .builder
                                .build_call(func, &[object_value.into()], "to_i64_result")?
                                .try_as_basic_value()
                                .left()
                                .ok_or_else(|| {
                                    CompileError::InternalError(
                                        "string_to_i64 should return a value".to_string(),
                                        None,
                                    )
                                })?;
                            return Ok(result);
                        }
                        "to_f32" => {
                            let func = self.get_or_create_runtime_function("string_to_f32")?;
                            let result = self
                                .builder
                                .build_call(func, &[object_value.into()], "to_f32_result")?
                                .try_as_basic_value()
                                .left()
                                .ok_or_else(|| {
                                    CompileError::InternalError(
                                        "string_to_f32 should return a value".to_string(),
                                        None,
                                    )
                                })?;
                            return Ok(result);
                        }
                        _ => {}
                    }
                }

                // Special handling for pointer methods
                if method == "val" {
                    // Dereference pointer
                    if object_value.is_pointer_value() {
                        let deref = self
                            .builder
                            .build_load(
                                object_value.get_type(),
                                object_value.into_pointer_value(),
                                "ptr_deref",
                            )
                            .map_err(|e| CompileError::from(e))?;
                        return Ok(deref);
                    }
                } else if method == "addr" {
                    // Get pointer address as integer
                    if object_value.is_pointer_value() {
                        let addr = self
                            .builder
                            .build_ptr_to_int(
                                object_value.into_pointer_value(),
                                self.context.i64_type(),
                                "ptr_addr",
                            )
                            .map_err(|e| CompileError::from(e))?;
                        return Ok(addr.into());
                    }
                } else if method == "raise" {
                    // Handle Result.raise() - propagate errors
                    // If the Result is Err, return early with that error
                    // If Ok, extract and return the value

                    // For Result<T, E>, we need to check the discriminant
                    // and either return early with Err or continue with Ok value

                    // Get parent function for control flow
                    let parent_function = self.current_function.ok_or_else(|| {
                        CompileError::InternalError(
                            "No current function for raise".to_string(),
                            None,
                        )
                    })?;

                    // Create blocks for Ok and Err cases
                    let ok_block = self.context.append_basic_block(parent_function, "raise_ok");
                    let err_block = self
                        .context
                        .append_basic_block(parent_function, "raise_err");
                    let continue_block = self
                        .context
                        .append_basic_block(parent_function, "raise_continue");

                    // Extract discriminant (assuming Result is an enum with tag at index 0)
                    let discriminant = if object_value.is_pointer_value() {
                        // If it's a pointer, load the discriminant
                        let disc_ptr = self.builder.build_struct_gep(
                            self.context.struct_type(
                                &[
                                    self.context.i64_type().into(),
                                    self.context.i64_type().into(),
                                ],
                                false,
                            ),
                            object_value.into_pointer_value(),
                            0,
                            "disc_ptr",
                        )?;
                        self.builder
                            .build_load(self.context.i64_type(), disc_ptr, "disc")?
                            .into_int_value()
                    } else {
                        // Direct value - extract first element
                        self.builder
                            .build_extract_value(object_value.into_struct_value(), 0, "disc")?
                            .into_int_value()
                    };

                    // Check if it's Ok (0) or Err (1)
                    let is_ok = self.builder.build_int_compare(
                        inkwell::IntPredicate::EQ,
                        discriminant,
                        self.context.i64_type().const_int(0, false),
                        "is_ok",
                    )?;

                    self.builder
                        .build_conditional_branch(is_ok, ok_block, err_block)?;

                    // Err block: return the whole Result (propagate the error)
                    self.builder.position_at_end(err_block);
                    self.builder.build_return(Some(&object_value))?;

                    // Ok block: extract the value and continue
                    self.builder.position_at_end(ok_block);
                    let ok_value = if object_value.is_pointer_value() {
                        // Load from pointer
                        let payload_ptr = self.builder.build_struct_gep(
                            self.context.struct_type(
                                &[
                                    self.context.i64_type().into(),
                                    self.context.i64_type().into(),
                                ],
                                false,
                            ),
                            object_value.into_pointer_value(),
                            1,
                            "payload_ptr",
                        )?;
                        self.builder
                            .build_load(self.context.i64_type(), payload_ptr, "payload")?
                    } else {
                        // Extract from struct
                        self.builder.build_extract_value(
                            object_value.into_struct_value(),
                            1,
                            "payload",
                        )?
                    };
                    self.builder.build_unconditional_branch(continue_block)?;

                    // Continue block
                    self.builder.position_at_end(continue_block);

                    return Ok(ok_value);
                }

                // Special handling for DynVec methods
                if let Expression::Identifier(obj_name) = object.as_ref() {
                    // Check if this is actually a DynVec type
                    let is_dynvec = if let Some(var_info) = self.variables.get(obj_name) {
                        matches!(&var_info.ast_type, AstType::DynVec { .. })
                    } else {
                        false
                    };
                    
                    if is_dynvec {
                        // Only handle DynVec methods if it's actually a DynVec
                        match method.as_str() {
                        "push" => {
                            // DynVec.push(value) implementation
                            if args.len() != 1 {
                                return Err(CompileError::TypeError(
                                    "push expects exactly 1 argument".to_string(),
                                    None,
                                ));
                            }

                            // Get the value to push
                            let value = self.compile_expression(&args[0])?;

                            // For DynVec methods that mutate, we need the pointer to the original variable
                            // not a copy of the value
                            let (dynvec_ptr, _) = self.get_variable(obj_name)?;

                            // Load len and capacity
                            let len_ptr = self.builder.build_struct_gep(
                                object_value.get_type(),
                                dynvec_ptr,
                                1,
                                "len_ptr",
                            )?;
                            let capacity_ptr = self.builder.build_struct_gep(
                                object_value.get_type(),
                                dynvec_ptr,
                                2,
                                "capacity_ptr",
                            )?;

                            let len = self
                                .builder
                                .build_load(self.context.i64_type(), len_ptr, "len")?
                                .into_int_value();
                            let capacity = self
                                .builder
                                .build_load(self.context.i64_type(), capacity_ptr, "capacity")?
                                .into_int_value();

                            // Check if we need to grow
                            let need_grow = self.builder.build_int_compare(
                                inkwell::IntPredicate::UGE,
                                len,
                                capacity,
                                "need_grow",
                            )?;

                            let grow_block = self
                                .context
                                .append_basic_block(self.current_function.unwrap(), "dynvec_grow");
                            let store_block = self
                                .context
                                .append_basic_block(self.current_function.unwrap(), "dynvec_store");

                            self.builder.build_conditional_branch(
                                need_grow,
                                grow_block,
                                store_block,
                            )?;

                            // Grow block - double capacity and reallocate
                            self.builder.position_at_end(grow_block);
                            let new_capacity = self.builder.build_int_mul(
                                capacity,
                                self.context.i64_type().const_int(2, false),
                                "new_capacity",
                            )?;

                            // Calculate new size in bytes
                            let element_size = self.context.i64_type().const_int(8, false); // Assuming i64 elements for now
                            let new_size = self.builder.build_int_mul(
                                new_capacity,
                                element_size,
                                "new_size",
                            )?;

                            // Get data pointer
                            let data_ptr_ptr = self.builder.build_struct_gep(
                                object_value.get_type(),
                                dynvec_ptr,
                                0,
                                "data_ptr_ptr",
                            )?;
                            let old_data = self.builder.build_load(
                                self.context.ptr_type(inkwell::AddressSpace::default()),
                                data_ptr_ptr,
                                "old_data",
                            )?;

                            // Reallocate using realloc
                            let realloc_fn = self.get_or_create_runtime_function("realloc")?;
                            let new_data = self
                                .builder
                                .build_call(
                                    realloc_fn,
                                    &[old_data.into(), new_size.into()],
                                    "new_data",
                                )?
                                .try_as_basic_value()
                                .left()
                                .unwrap();

                            // Update data pointer and capacity
                            self.builder.build_store(data_ptr_ptr, new_data)?;
                            self.builder.build_store(capacity_ptr, new_capacity)?;

                            self.builder.build_unconditional_branch(store_block)?;

                            // Store block - store value at data[len]
                            self.builder.position_at_end(store_block);

                            // Re-load data pointer (might have changed)
                            let data_ptr_ptr = self.builder.build_struct_gep(
                                object_value.get_type(),
                                dynvec_ptr,
                                0,
                                "data_ptr_ptr",
                            )?;
                            let data_ptr = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(inkwell::AddressSpace::default()),
                                    data_ptr_ptr,
                                    "data_ptr",
                                )?
                                .into_pointer_value();

                            // Re-load len (for correct index)
                            let len = self
                                .builder
                                .build_load(self.context.i64_type(), len_ptr, "len")?
                                .into_int_value();

                            // Calculate element address
                            let element_ptr = unsafe {
                                self.builder.build_gep(
                                    value.get_type(),
                                    data_ptr,
                                    &[len],
                                    "element_ptr",
                                )
                            }?;

                            // Store the value
                            self.builder.build_store(element_ptr, value)?;

                            // Increment length
                            let new_len = self.builder.build_int_add(
                                len,
                                self.context.i64_type().const_int(1, false),
                                "new_len",
                            )?;
                            self.builder.build_store(len_ptr, new_len)?;

                            // Return void (unit type)
                            return Ok(self.context.struct_type(&[], false).const_zero().into());
                        }
                        "pop" => {
                            // DynVec.pop() -> Option<T>
                            // Returns Some(value) if vector is not empty, None otherwise

                            // For DynVec methods that mutate, we need the pointer to the original variable
                            let (dynvec_ptr, _) = self.get_variable(obj_name)?;

                            // Load len
                            let len_ptr = self.builder.build_struct_gep(
                                object_value.get_type(),
                                dynvec_ptr,
                                1,
                                "len_ptr",
                            )?;
                            let len = self
                                .builder
                                .build_load(self.context.i64_type(), len_ptr, "len")?
                                .into_int_value();

                            // Check if empty
                            let is_empty = self.builder.build_int_compare(
                                inkwell::IntPredicate::EQ,
                                len,
                                self.context.i64_type().const_int(0, false),
                                "is_empty",
                            )?;

                            let none_block = self
                                .context
                                .append_basic_block(self.current_function.unwrap(), "pop_none");
                            let some_block = self
                                .context
                                .append_basic_block(self.current_function.unwrap(), "pop_some");
                            let merge_block = self
                                .context
                                .append_basic_block(self.current_function.unwrap(), "pop_merge");

                            self.builder
                                .build_conditional_branch(is_empty, none_block, some_block)?;

                            // None block - return None
                            self.builder.position_at_end(none_block);
                            let option_type = self.context.struct_type(
                                &[
                                    self.context.i64_type().into(), // discriminant
                                    self.context.i64_type().into(), // payload
                                ],
                                false,
                            );
                            let none_value = option_type.const_named_struct(&[
                                self.context.i64_type().const_int(1, false).into(), // None = 1
                                self.context.i64_type().const_int(0, false).into(), // dummy payload
                            ]);
                            self.builder.build_unconditional_branch(merge_block)?;

                            // Some block - decrement len and return value
                            self.builder.position_at_end(some_block);

                            // Decrement len
                            let new_len = self.builder.build_int_sub(
                                len,
                                self.context.i64_type().const_int(1, false),
                                "new_len",
                            )?;
                            self.builder.build_store(len_ptr, new_len)?;

                            // Load data pointer
                            let data_ptr_ptr = self.builder.build_struct_gep(
                                object_value.get_type(),
                                dynvec_ptr,
                                0,
                                "data_ptr_ptr",
                            )?;
                            let data_ptr = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(inkwell::AddressSpace::default()),
                                    data_ptr_ptr,
                                    "data_ptr",
                                )?
                                .into_pointer_value();

                            // Get element at new_len (the last element)
                            let element_ptr = unsafe {
                                self.builder.build_gep(
                                    self.context.i64_type(),
                                    data_ptr,
                                    &[new_len],
                                    "element_ptr",
                                )
                            }?;

                            let value = self.builder.build_load(
                                self.context.i64_type(),
                                element_ptr,
                                "popped_value",
                            )?;

                            // Create Some(value)
                            let some_value = option_type.const_named_struct(&[
                                self.context.i64_type().const_int(0, false).into(), // Some = 0
                                value.into(),
                            ]);

                            self.builder.build_unconditional_branch(merge_block)?;

                            // Merge block - PHI node for result
                            self.builder.position_at_end(merge_block);
                            let phi = self.builder.build_phi(option_type, "pop_result")?;
                            phi.add_incoming(&[
                                (&none_value, none_block),
                                (&some_value, some_block),
                            ]);

                            return Ok(phi.as_basic_value());
                        }
                        "get" => {
                            // DynVec.get(index) -> Option<T>
                            if args.len() != 1 {
                                return Err(CompileError::TypeError(
                                    "get expects exactly 1 argument".to_string(),
                                    None,
                                ));
                            }

                            let index = self.compile_expression(&args[0])?;
                            let index = if index.is_int_value() {
                                let idx = index.into_int_value();
                                // Extend to i64 if needed
                                if idx.get_type().get_bit_width() < 64 {
                                    self.builder.build_int_z_extend(
                                        idx,
                                        self.context.i64_type(),
                                        "index_i64",
                                    )?
                                } else {
                                    idx
                                }
                            } else {
                                return Err(CompileError::TypeError(
                                    "Index must be an integer".to_string(),
                                    None,
                                ));
                            };

                            // Get the pointer to the original variable for consistency
                            let (dynvec_ptr, _) = self.get_variable(obj_name)?;

                            // Load len
                            let len_ptr = self.builder.build_struct_gep(
                                object_value.get_type(),
                                dynvec_ptr,
                                1,
                                "len_ptr",
                            )?;
                            let len = self
                                .builder
                                .build_load(self.context.i64_type(), len_ptr, "len")?
                                .into_int_value();

                            // Check if index is valid
                            let is_valid = self.builder.build_int_compare(
                                inkwell::IntPredicate::ULT,
                                index,
                                len,
                                "is_valid_index",
                            )?;

                            let none_block = self
                                .context
                                .append_basic_block(self.current_function.unwrap(), "get_none");
                            let some_block = self
                                .context
                                .append_basic_block(self.current_function.unwrap(), "get_some");
                            let merge_block = self
                                .context
                                .append_basic_block(self.current_function.unwrap(), "get_merge");

                            self.builder
                                .build_conditional_branch(is_valid, some_block, none_block)?;

                            // None block - return None
                            self.builder.position_at_end(none_block);
                            let option_type = self.context.struct_type(
                                &[
                                    self.context.i64_type().into(), // discriminant
                                    self.context.i64_type().into(), // payload
                                ],
                                false,
                            );
                            let none_value = option_type.const_named_struct(&[
                                self.context.i64_type().const_int(1, false).into(), // None = 1
                                self.context.i64_type().const_int(0, false).into(), // dummy payload
                            ]);
                            self.builder.build_unconditional_branch(merge_block)?;

                            // Some block - return value at index
                            self.builder.position_at_end(some_block);

                            // Load data pointer
                            let data_ptr_ptr = self.builder.build_struct_gep(
                                object_value.get_type(),
                                dynvec_ptr,
                                0,
                                "data_ptr_ptr",
                            )?;
                            let data_ptr = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(inkwell::AddressSpace::default()),
                                    data_ptr_ptr,
                                    "data_ptr",
                                )?
                                .into_pointer_value();

                            // Get element at index
                            let element_ptr = unsafe {
                                self.builder.build_gep(
                                    self.context.i64_type(),
                                    data_ptr,
                                    &[index],
                                    "element_ptr",
                                )
                            }?;

                            let value = self.builder.build_load(
                                self.context.i64_type(),
                                element_ptr,
                                "element_value",
                            )?;

                            // Create Some(value)
                            let some_value = option_type.const_named_struct(&[
                                self.context.i64_type().const_int(0, false).into(), // Some = 0
                                value.into(),
                            ]);

                            self.builder.build_unconditional_branch(merge_block)?;

                            // Merge block - PHI node for result
                            self.builder.position_at_end(merge_block);
                            let phi = self.builder.build_phi(option_type, "get_result")?;
                            phi.add_incoming(&[
                                (&none_value, none_block),
                                (&some_value, some_block),
                            ]);

                            return Ok(phi.as_basic_value());
                        }
                        "set" => {
                            // DynVec.set(index, value) -> bool
                            if args.len() != 2 {
                                return Err(CompileError::TypeError(
                                    "set expects exactly 2 arguments".to_string(),
                                    None,
                                ));
                            }

                            let index = self.compile_expression(&args[0])?;
                            let index = if index.is_int_value() {
                                let idx = index.into_int_value();
                                // Extend to i64 if needed
                                if idx.get_type().get_bit_width() < 64 {
                                    self.builder.build_int_z_extend(
                                        idx,
                                        self.context.i64_type(),
                                        "index_i64",
                                    )?
                                } else {
                                    idx
                                }
                            } else {
                                return Err(CompileError::TypeError(
                                    "Index must be an integer".to_string(),
                                    None,
                                ));
                            };

                            let value = self.compile_expression(&args[1])?;

                            // For DynVec methods that mutate, we need the pointer to the original variable
                            let (dynvec_ptr, _) = self.get_variable(obj_name)?;

                            // Load len
                            let len_ptr = self.builder.build_struct_gep(
                                object_value.get_type(),
                                dynvec_ptr,
                                1,
                                "len_ptr",
                            )?;
                            let len = self
                                .builder
                                .build_load(self.context.i64_type(), len_ptr, "len")?
                                .into_int_value();

                            // Check if index is valid
                            let is_valid = self.builder.build_int_compare(
                                inkwell::IntPredicate::ULT,
                                index,
                                len,
                                "is_valid_index",
                            )?;

                            let fail_block = self
                                .context
                                .append_basic_block(self.current_function.unwrap(), "set_fail");
                            let success_block = self
                                .context
                                .append_basic_block(self.current_function.unwrap(), "set_success");
                            let merge_block = self
                                .context
                                .append_basic_block(self.current_function.unwrap(), "set_merge");

                            self.builder.build_conditional_branch(
                                is_valid,
                                success_block,
                                fail_block,
                            )?;

                            // Fail block - return false
                            self.builder.position_at_end(fail_block);
                            let false_val = self.context.bool_type().const_int(0, false);
                            self.builder.build_unconditional_branch(merge_block)?;

                            // Success block - set value and return true
                            self.builder.position_at_end(success_block);

                            // Load data pointer
                            let data_ptr_ptr = self.builder.build_struct_gep(
                                object_value.get_type(),
                                dynvec_ptr,
                                0,
                                "data_ptr_ptr",
                            )?;
                            let data_ptr = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(inkwell::AddressSpace::default()),
                                    data_ptr_ptr,
                                    "data_ptr",
                                )?
                                .into_pointer_value();

                            // Get element at index
                            let element_ptr = unsafe {
                                self.builder.build_gep(
                                    value.get_type(),
                                    data_ptr,
                                    &[index],
                                    "element_ptr",
                                )
                            }?;

                            // Store the new value
                            self.builder.build_store(element_ptr, value)?;

                            let true_val = self.context.bool_type().const_int(1, false);
                            self.builder.build_unconditional_branch(merge_block)?;

                            // Merge block - PHI node for result
                            self.builder.position_at_end(merge_block);
                            let phi = self
                                .builder
                                .build_phi(self.context.bool_type(), "set_result")?;
                            phi.add_incoming(&[
                                (&false_val, fail_block),
                                (&true_val, success_block),
                            ]);

                            return Ok(phi.as_basic_value());
                        }
                        "len" => {
                            // DynVec.len() -> usize
                            // Get the pointer to the original variable for consistency
                            let (dynvec_ptr, _) = self.get_variable(obj_name)?;

                            // Load and return len
                            let len_ptr = self.builder.build_struct_gep(
                                object_value.get_type(),
                                dynvec_ptr,
                                1,
                                "len_ptr",
                            )?;
                            let len =
                                self.builder
                                    .build_load(self.context.i64_type(), len_ptr, "len")?;
                            return Ok(len);
                        }
                        "clear" => {
                            // DynVec.clear() - sets len to 0
                            // For DynVec methods that mutate, we need the pointer to the original variable
                            let (dynvec_ptr, _) = self.get_variable(obj_name)?;

                            // Set len to 0
                            let len_ptr = self.builder.build_struct_gep(
                                object_value.get_type(),
                                dynvec_ptr,
                                1,
                                "len_ptr",
                            )?;
                            self.builder.build_store(
                                len_ptr,
                                self.context.i64_type().const_int(0, false),
                            )?;

                            // Return void (unit type)
                            return Ok(self.context.struct_type(&[], false).const_zero().into());
                        }
                        _ => {}
                    }
                    }  // Close the is_dynvec check
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

                let loop_body_block = self
                    .context
                    .append_basic_block(self.current_function.unwrap(), "loop_expr_body");
                let after_loop_block = self
                    .context
                    .append_basic_block(self.current_function.unwrap(), "after_loop_expr");

                // Push loop context for break/continue
                self.loop_stack.push((loop_body_block, after_loop_block));

                // Jump to loop body
                self.builder
                    .build_unconditional_branch(loop_body_block)
                    .map_err(|e| CompileError::from(e))?;
                self.builder.position_at_end(loop_body_block);

                // Compile the loop body based on its type
                match body.as_ref() {
                    Expression::Closure {
                        params: _,
                        body: closure_body,
                    } => {
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
                    self.builder
                        .build_unconditional_branch(loop_body_block)
                        .map_err(|e| CompileError::from(e))?;
                }

                self.loop_stack.pop();
                self.builder.position_at_end(after_loop_block);

                // Return void value
                Ok(self.context.i32_type().const_int(0, false).into())
            }
            Expression::Closure { params, body } => {
                // Generate inline function
                let current_func = self.current_function.ok_or_else(|| {
                    CompileError::InternalError(
                        "Closure outside of function context".to_string(),
                        None,
                    )
                })?;

                // Create a unique name for the inline function
                let inline_func_name = format!(
                    "__inline_{}_{}",
                    current_func.get_name().to_str().unwrap_or("anon"),
                    self.inline_counter
                );
                self.inline_counter += 1;

                // Determine parameter types (default to i32 if not specified)
                let mut param_types = Vec::new();
                for (_, opt_type) in params {
                    let ast_type = opt_type.as_ref().unwrap_or(&AstType::I32);
                    match self.to_llvm_type(ast_type)? {
                        Type::Basic(ty) => param_types.push(ty.into()),
                        _ => {
                            return Err(CompileError::InternalError(
                                "Closure parameter must be basic type".to_string(),
                                None,
                            ))
                        }
                    }
                }

                // For now, assume i32 return type if body is not analyzed
                // TODO: Infer return type from body
                let return_type = self.context.i32_type();
                let fn_type = return_type.fn_type(&param_types, false);

                // Create the inline function
                let inline_func =
                    self.module
                        .add_function(&inline_func_name, fn_type, Some(Linkage::Internal));

                // Save current state
                let prev_func = self.current_function;
                let prev_block = self.builder.get_insert_block();
                let prev_vars = self.variables.clone();

                // Create entry block for inline function
                let entry_block = self.context.append_basic_block(inline_func, "entry");
                self.builder.position_at_end(entry_block);
                self.current_function = Some(inline_func);

                // Bind parameters to local variables
                for (i, (param_name, opt_type)) in params.iter().enumerate() {
                    let param_value = inline_func.get_nth_param(i as u32).unwrap();
                    let ast_type = opt_type.as_ref().unwrap_or(&AstType::I32);

                    // Create an alloca for the parameter and store its value
                    let alloca = self
                        .builder
                        .build_alloca(param_value.get_type(), &format!("param_{}", param_name))
                        .map_err(|e| CompileError::from(e))?;

                    self.builder
                        .build_store(alloca, param_value)
                        .map_err(|e| CompileError::from(e))?;

                    self.variables.insert(
                        param_name.clone(),
                        VariableInfo {
                            pointer: alloca,
                            ast_type: ast_type.clone(),
                            is_mutable: false,
                            is_initialized: true,
                        },
                    );
                }

                // Compile the body
                let result = self.compile_expression(body)?;

                // Add return if not already present
                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    self.builder
                        .build_return(Some(&result))
                        .map_err(|e| CompileError::from(e))?;
                }

                // Restore previous state
                self.current_function = prev_func;
                if let Some(block) = prev_block {
                    self.builder.position_at_end(block);
                }
                self.variables = prev_vars;

                // Return the function pointer
                Ok(inline_func.as_global_value().as_basic_value_enum())
            }
            Expression::Raise(expr) => {
                // .raise() propagates errors early by transforming to pattern matching:
                // expr.raise() becomes:
                // expr ?
                //     | Ok(val) { val }
                //     | Err(e) { return Err(e) }
                self.compile_raise_expression(expr)
            }
            Expression::Break { label: _, value: _ } => {
                // Break out of the current loop
                if let Some((_, after_loop)) = self.loop_stack.last() {
                    self.builder
                        .build_unconditional_branch(*after_loop)
                        .map_err(|e| CompileError::from(e))?;
                    // Create a new block after the break (unreachable but needed for LLVM)
                    let unreachable_block = self
                        .context
                        .append_basic_block(self.current_function.unwrap(), "after_break");
                    self.builder.position_at_end(unreachable_block);
                    // Return a dummy value
                    Ok(self.context.i32_type().const_int(0, false).into())
                } else {
                    Err(CompileError::InternalError(
                        "Break outside of loop".to_string(),
                        None,
                    ))
                }
            }
            Expression::Continue { label: _ } => {
                // Continue to the next iteration of the current loop
                if let Some((loop_header, _)) = self.loop_stack.last() {
                    self.builder
                        .build_unconditional_branch(*loop_header)
                        .map_err(|e| CompileError::from(e))?;
                    // Create a new block after the continue (unreachable but needed for LLVM)
                    let unreachable_block = self
                        .context
                        .append_basic_block(self.current_function.unwrap(), "after_continue");
                    self.builder.position_at_end(unreachable_block);
                    // Return a dummy value
                    Ok(self.context.i32_type().const_int(0, false).into())
                } else {
                    Err(CompileError::InternalError(
                        "Continue outside of loop".to_string(),
                        None,
                    ))
                }
            }
            Expression::VecConstructor {
                element_type,
                size,
                initial_values,
            } => self.compile_vec_constructor(element_type, *size, initial_values.as_ref()),
            Expression::DynVecConstructor {
                element_types,
                allocator,
                initial_capacity,
            } => self.compile_dynvec_constructor(
                element_types,
                allocator,
                initial_capacity.as_ref().map(|v| &**v),
            ),
            Expression::Some(value) => {
                // Compile Option::Some variant using the registered enum
                self.compile_enum_variant("Option", "Some", &Some(value.clone()))
            }
            Expression::None => {
                // Compile Option::None variant using the registered enum
                self.compile_enum_variant("Option", "None", &None)
            }
            Expression::CollectionLoop {
                collection,
                param,
                index_param,
                body,
            } => {
                // Compile collection.loop() expression
                self.compile_collection_loop(collection, param, index_param.as_deref(), body)
            }
            Expression::Defer(expr) => {
                // Register deferred expression to be executed at scope exit
                self.register_deferred_expression(expr)?;
                // Return unit/void value
                Ok(self.context.i32_type().const_int(0, false).into())
            }
        }
    }

    fn compile_conditional_expression(
        &mut self,
        scrutinee: &Expression,
        arms: &[crate::ast::MatchArm],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Generate a unique ID for this conditional to avoid block name collisions
        static mut CONDITIONAL_ID: u32 = 0;
        let cond_id = unsafe {
            CONDITIONAL_ID += 1;
            CONDITIONAL_ID
        };

        let parent_function = self.current_function.ok_or_else(|| {
            CompileError::InternalError("No current function for conditional".to_string(), None)
        })?;

        // Compile the scrutinee expression
        let scrutinee_val = self.compile_expression(scrutinee)?;

        // Create the merge block where all arms will jump to
        let merge_bb = self
            .context
            .append_basic_block(parent_function, &format!("match_merge_{}", cond_id));

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

                self.builder
                    .build_and(matches, guard_val.into_int_value(), "guard_and_pattern")?
            } else {
                matches
            };

            // Create blocks for this arm
            let match_bb = self
                .context
                .append_basic_block(parent_function, &format!("match_{}_{}", cond_id, i));

            // For boolean patterns (true/false), we always need a next block for the false case
            // Check if this is a wildcard pattern which is always exhaustive
            let _is_wildcard = matches!(arm.pattern, crate::ast::Pattern::Wildcard);

            let next_bb = if !is_last {
                self.context
                    .append_basic_block(parent_function, &format!("test_{}_{}", cond_id, i + 1))
            } else {
                // For the last arm, create an unmatched block
                // This will be used if the pattern doesn't match
                // Even for exhaustive patterns, we need this for correct control flow
                let block = self
                    .context
                    .append_basic_block(parent_function, &format!("pattern_unmatched_{}", cond_id));
                unmatched_block = Some(block);
                block
            };

            // Branch based on the condition
            self.builder
                .build_conditional_branch(final_condition, match_bb, next_bb)?;

            // Generate code for the match block
            self.builder.position_at_end(match_bb);

            // Apply pattern bindings
            let saved_vars = self.apply_pattern_bindings(&bindings);

            // Compile the arm body
            let arm_val = self.compile_expression(&arm.body)?;

            // Restore variables
            self.restore_variables(saved_vars);

            // Jump to merge block only if the block doesn't already have a terminator (e.g., return)
            let current_block = self.builder.get_insert_block().unwrap();
            if current_block.get_terminator().is_none() {
                self.builder.build_unconditional_branch(merge_bb)?;
                let match_bb_end = self.builder.get_insert_block().unwrap();
                // Save value and block for phi node only if we branch to merge
                // eprintln!("DEBUG: Adding PHI value with type: {:?}", arm_val.get_type());
                phi_values.push((arm_val, match_bb_end));
            } else {
                // eprintln!("DEBUG: Arm {} has terminator, not adding to phi_values", i);
            }

            // Position at the next test block for the next iteration
            if !is_last {
                self.builder.position_at_end(next_bb);
                _current_block = next_bb;
            }
        }

        // Handle unmatched pattern block if we created one
        if let Some(unmatched_bb) = unmatched_block {
            self.builder.position_at_end(unmatched_bb);

            // Only add to phi_values if other arms also produced values
            // If all arms had returns/breaks/continues, don't add to phi_values
            if !phi_values.is_empty() {
                // For now, just return a default value (0) and branch to merge
                // In a complete implementation, this would be a runtime error
                // Use the same type as the first arm to ensure type consistency
                let default_val = match phi_values[0].0.get_type() {
                    inkwell::types::BasicTypeEnum::IntType(int_type) => {
                        int_type.const_int(0, false).into()
                    }
                    inkwell::types::BasicTypeEnum::FloatType(float_type) => {
                        float_type.const_float(0.0).into()
                    }
                    inkwell::types::BasicTypeEnum::PointerType(ptr_type) => {
                        // For pointer types (like strings), use a null pointer
                        ptr_type.const_null().into()
                    }
                    inkwell::types::BasicTypeEnum::StructType(struct_type) => {
                        // For struct types (like enums), create a zero struct
                        struct_type.const_zero().into()
                    }
                    _ => {
                        // For other types, use a default value
                        self.context.i32_type().const_int(0, false).into()
                    }
                };
                // Only branch if there's no terminator
                let current_block = self.builder.get_insert_block().unwrap();
                if current_block.get_terminator().is_none() {
                    self.builder.build_unconditional_branch(merge_bb)?;
                    phi_values.push((default_val, unmatched_bb));
                }
            } else {
                // All arms had terminators, so this unmatched block is also unreachable
                // We still need to add some terminator to make LLVM happy
                // We'll add an unreachable instruction
                self.builder.build_unreachable()?;
            }
        }

        // Position at merge block and create phi node only if any arms branch to it
        if !phi_values.is_empty() {
            self.builder.position_at_end(merge_bb);

            // Normalize all values to the same type
            // If all values are integers, cast them to the widest type
            let mut max_int_width = 0;
            let mut has_non_int = false;

            for (value, _) in &phi_values {
                if value.is_int_value() {
                    let int_val = value.into_int_value();
                    let width = int_val.get_type().get_bit_width();
                    max_int_width = max_int_width.max(width);
                } else {
                    has_non_int = true;
                }
            }

            // Normalize integer values if needed
            let normalized_values: Vec<(
                BasicValueEnum<'ctx>,
                inkwell::basic_block::BasicBlock<'ctx>,
            )> = if !has_non_int && max_int_width > 0 {
                // All values are integers, cast them to the widest type
                let target_type = self.context.custom_width_int_type(max_int_width);
                phi_values
                    .into_iter()
                    .map(|(value, block)| {
                        if value.is_int_value() {
                            let int_val = value.into_int_value();
                            if int_val.get_type().get_bit_width() < max_int_width {
                                // Need to cast
                                let casted = self
                                    .builder
                                    .build_int_z_extend_or_bit_cast(
                                        int_val,
                                        target_type,
                                        "cast_to_common",
                                    )
                                    .unwrap();
                                (casted.into(), block)
                            } else {
                                (value, block)
                            }
                        } else {
                            (value, block)
                        }
                    })
                    .collect()
            } else {
                phi_values
            };

            // Use the normalized type for PHI node
            let phi_type = normalized_values[0].0.get_type();
            let phi = self.builder.build_phi(phi_type, "match_result")?;

            // Add all incoming values
            for (value, block) in &normalized_values {
                phi.add_incoming(&[(value, *block)]);
            }

            Ok(phi.as_basic_value())
        } else {
            // All arms had terminators (e.g., returns), so we don't need a merge block
            // Return a dummy value that won't be used
            Ok(self.context.i32_type().const_int(0, false).into())
        }
    }

    fn compile_array_literal(
        &mut self,
        elements: &[Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Infer type from first element or default to i32
        let element_type = if !elements.is_empty() {
            // Compile first element to get its type
            let first_val = self.compile_expression(&elements[0])?;
            match first_val.get_type() {
                inkwell::types::BasicTypeEnum::IntType(_) => self.context.i32_type(),
                _ => self.context.i32_type(), // Default to i32 for other types too
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
        let malloc_fn = self.module.get_function("malloc").ok_or_else(|| {
            CompileError::InternalError("No malloc function declared".to_string(), None)
        })?;
        let size = self
            .builder
            .build_int_mul(elem_size, total_size, "arraysize");
        let raw_ptr = self
            .builder
            .build_call(malloc_fn, &[size?.into()], "arraymalloc")?
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();
        let array_ptr = self.builder.build_pointer_cast(
            raw_ptr,
            self.context.ptr_type(inkwell::AddressSpace::default()),
            "arrayptr",
        )?;

        // Store each element
        for (i, expr) in elements.iter().enumerate() {
            let value = self.compile_expression(expr)?;
            let gep = unsafe {
                self.builder.build_gep(
                    element_type,
                    array_ptr,
                    &[element_type.const_int(i as u64, false)],
                    &format!("arrayidx{}", i),
                )?
            };
            self.builder.build_store(gep, value)?;
        }
        Ok(array_ptr.as_basic_value_enum())
    }

    fn compile_array_index(
        &mut self,
        array: &Expression,
        index: &Expression,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Get the address of the indexed element
        let gep = self.compile_array_index_address(array, index)?;

        // Try to infer element type from context. Default to i32 for compatibility with tests
        // TODO: Proper type inference for array elements from declaration
        let element_type = self.context.i32_type();

        // Load the value from the address
        let loaded = self.builder.build_load(element_type, gep, "arrayload")?;
        Ok(loaded)
    }

    pub fn compile_array_index_address(
        &mut self,
        array: &Expression,
        index: &Expression,
    ) -> Result<PointerValue<'ctx>, CompileError> {
        // Compile array expression - should be a pointer
        let array_val = self.compile_expression(array)?;

        // Get the actual pointer value
        let array_ptr = if array_val.is_pointer_value() {
            array_val.into_pointer_value()
        } else {
            return Err(CompileError::TypeError(
                format!(
                    "Array indexing requires pointer type, got {:?}",
                    array_val.get_type()
                ),
                None,
            ));
        };

        // Try to infer element type from context. Default to i32 for compatibility with tests
        // TODO: Proper type inference for array elements from declaration
        let element_type = self.context.i32_type();

        let index_val = self.compile_expression(index)?;
        let gep = unsafe {
            self.builder.build_gep(
                element_type,
                array_ptr,
                &[index_val.into_int_value()],
                "arrayidx",
            )?
        };
        Ok(gep)
    }

    pub(super) fn compile_enum_variant(
        &mut self,
        enum_name: &str,
        variant: &str,
        payload: &Option<Box<Expression>>,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Track Result<T, E> type information when compiling Result variants
        if enum_name == "Result" && payload.is_some() {
            if let Some(ref payload_expr) = payload {
                let payload_type = self.infer_expression_type(payload_expr);
                if let Ok(t) = payload_type {
                    let key = if variant == "Ok" {
                        "Result_Ok_Type".to_string()
                    } else {
                        "Result_Err_Type".to_string()
                    };
                    self.generic_type_context.insert(key, t);
                }
            }
        }

        // Track Option<T> type information when compiling Option variants
        if enum_name == "Option" && payload.is_some() {
            if let Some(ref payload_expr) = payload {
                let payload_type = self.infer_expression_type(payload_expr);
                if let Ok(t) = payload_type {
                    self.generic_type_context
                        .insert("Option_Some_Type".to_string(), t);
                }
            }
        }

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
                    // Special handling for Result type variants
                    let tag = if variant == "Ok" {
                        0
                    } else if variant == "Err" {
                        1
                    } else if variant == "Some" {
                        0
                    } else if variant == "None" {
                        1
                    } else {
                        0
                    };
                    let tag_val = self.context.i64_type().const_int(tag, false);

                    // Create proper enum struct type with pointer payload for genericity
                    let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                    let enum_struct_type = self
                        .context
                        .struct_type(&[self.context.i64_type().into(), ptr_type.into()], false);

                    let alloca = self.builder.build_alloca(
                        enum_struct_type,
                        &format!("{}_{}_enum_tmp", enum_name, variant),
                    )?;
                    let tag_ptr =
                        self.builder
                            .build_struct_gep(enum_struct_type, alloca, 0, "tag_ptr")?;
                    self.builder.build_store(tag_ptr, tag_val)?;

                    // Handle payload if present
                    if let Some(expr) = payload {
                        let compiled = self.compile_expression(expr)?;
                        let payload_ptr = self.builder.build_struct_gep(
                            enum_struct_type,
                            alloca,
                            1,
                            "payload_ptr",
                        )?;

                        // Store payload as pointer
                        let payload_value = if compiled.is_pointer_value() {
                            compiled
                        } else {
                            let value_alloca = self
                                .builder
                                .build_alloca(compiled.get_type(), "enum_payload_value")?;
                            self.builder.build_store(value_alloca, compiled)?;
                            value_alloca.into()
                        };
                        self.builder.build_store(payload_ptr, payload_value)?;
                    } else {
                        // For None variant, store null pointer
                        let payload_ptr = self.builder.build_struct_gep(
                            enum_struct_type,
                            alloca,
                            1,
                            "payload_ptr",
                        )?;
                        let null_ptr = ptr_type.const_null();
                        self.builder.build_store(payload_ptr, null_ptr)?;
                    }

                    let loaded = self.builder.build_load(
                        enum_struct_type,
                        alloca,
                        &format!("{}_{}_enum_val", enum_name, variant),
                    )?;
                    return Ok(loaded);
                }
            }
        } else {
            match self.symbols.lookup(enum_name) {
                Some(symbols::Symbol::EnumType(info)) => info.clone(),
                _ => {
                    // Fallback to basic representation if enum not found in symbol table
                    // This maintains backward compatibility
                    // Special handling for Result type variants
                    let tag = if variant == "Ok" {
                        0
                    } else if variant == "Err" {
                        1
                    } else if variant == "Some" {
                        0
                    } else if variant == "None" {
                        1
                    } else {
                        0
                    };
                    let tag_val = self.context.i64_type().const_int(tag, false);

                    // Create proper enum struct type with pointer payload for genericity
                    let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                    let enum_struct_type = self
                        .context
                        .struct_type(&[self.context.i64_type().into(), ptr_type.into()], false);

                    let alloca = self.builder.build_alloca(
                        enum_struct_type,
                        &format!("{}_{}_enum_tmp", enum_name, variant),
                    )?;
                    let tag_ptr =
                        self.builder
                            .build_struct_gep(enum_struct_type, alloca, 0, "tag_ptr")?;
                    self.builder.build_store(tag_ptr, tag_val)?;

                    // Handle payload if present
                    if let Some(expr) = payload {
                        let compiled = self.compile_expression(expr)?;
                        let payload_ptr = self.builder.build_struct_gep(
                            enum_struct_type,
                            alloca,
                            1,
                            "payload_ptr",
                        )?;

                        // Store payload as pointer
                        let payload_value = if compiled.is_pointer_value() {
                            compiled
                        } else {
                            let value_alloca = self
                                .builder
                                .build_alloca(compiled.get_type(), "enum_payload_value")?;
                            self.builder.build_store(value_alloca, compiled)?;
                            value_alloca.into()
                        };
                        self.builder.build_store(payload_ptr, payload_value)?;
                    } else {
                        // For None variant, store null pointer
                        let payload_ptr = self.builder.build_struct_gep(
                            enum_struct_type,
                            alloca,
                            1,
                            "payload_ptr",
                        )?;
                        let null_ptr = ptr_type.const_null();
                        self.builder.build_store(payload_ptr, null_ptr)?;
                    }

                    let loaded = self.builder.build_load(
                        enum_struct_type,
                        alloca,
                        &format!("{}_{}_enum_val", enum_name, variant),
                    )?;
                    return Ok(loaded);
                }
            }
        };

        // Look up the variant index
        let tag = enum_info
            .variant_indices
            .get(variant)
            .copied()
            .ok_or_else(|| {
                CompileError::UndeclaredVariable(
                    format!("Unknown variant '{}' for enum '{}'", variant, enum_name),
                    None,
                )
            })?;

        let tag_val = self.context.i64_type().const_int(tag, false);

        // Use the enum's LLVM type
        let enum_struct_type = enum_info.llvm_type;
        let alloca = self.builder.build_alloca(
            enum_struct_type,
            &format!("{}_{}_enum_tmp", enum_name, variant),
        )?;
        let tag_ptr = self
            .builder
            .build_struct_gep(enum_struct_type, alloca, 0, "tag_ptr")?;
        self.builder.build_store(tag_ptr, tag_val)?;

        // Handle payload if present - preserve the actual type!
        if let Some(expr) = payload {
            let compiled = self.compile_expression(expr)?;
            // Check if enum has payload field (index 1)
            if enum_struct_type.count_fields() > 1 {
                let payload_ptr =
                    self.builder
                        .build_struct_gep(enum_struct_type, alloca, 1, "payload_ptr")?;

                // Get the expected payload type from the enum struct
                let payload_field_type =
                    enum_struct_type.get_field_type_at_index(1).ok_or_else(|| {
                        CompileError::InternalError(
                            "Enum payload field not found".to_string(),
                            None,
                        )
                    })?;

                // If payload field is a pointer type, we need to handle it specially
                let payload_value = if payload_field_type.is_pointer_type() {
                    // If the compiled value is already a pointer, use it directly
                    if compiled.is_pointer_value() {
                        compiled
                    } else {
                        // Otherwise allocate space and store the value, then use the pointer
                        let value_alloca = self
                            .builder
                            .build_alloca(compiled.get_type(), "enum_payload_value")?;
                        self.builder.build_store(value_alloca, compiled)?;
                        value_alloca.into()
                    }
                } else {
                    // For non-pointer payload fields, use casting as before
                    self.cast_value_to_type(compiled, payload_field_type)?
                };

                // Store the properly typed value
                self.builder.build_store(payload_ptr, payload_value)?;
            }
        } else if enum_struct_type.count_fields() > 1 {
            // If no payload but enum expects one, store null pointer
            let payload_ptr =
                self.builder
                    .build_struct_gep(enum_struct_type, alloca, 1, "payload_ptr")?;
            let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
            let null_ptr = ptr_type.const_null();
            self.builder.build_store(payload_ptr, null_ptr)?;
        }
        let loaded = self.builder.build_load(
            enum_struct_type,
            alloca,
            &format!("{}_{}_enum_val", enum_name, variant),
        )?;
        Ok(loaded)
    }

    fn compile_member_access(
        &mut self,
        object: &Expression,
        member: &str,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Check if this is a stdlib module constant access (e.g., math.pi)
        if let Expression::Identifier(module_name) = object {
            // Check if this is a known stdlib module with constants
            if module_name == "math" && member == "pi" {
                // Return the value of pi as a float constant
                return Ok(self
                    .context
                    .f64_type()
                    .const_float(std::f64::consts::PI)
                    .into());
            }
        }

        // Check if this is Array.new() or other Array methods
        if let Expression::Identifier(type_name) = object {
            if type_name == "Array" && member == "new" {
                // Array.new() is a special static method
                // Return a marker that indicates this is Array.new
                // The actual implementation will be in the FunctionCall handler
                // For now, just return a dummy value that won't be used
                return Ok(self.context.i64_type().const_int(0xA77A9_0001, false).into());
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

    fn compile_comptime_expression(
        &mut self,
        expr: &Expression,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
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
                    None,
                ));
            }
        }
    }

    fn compile_pattern_match(
        &mut self,
        scrutinee: &Expression,
        arms: &[crate::ast::PatternArm],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        
        let parent_function = self.current_function.ok_or_else(|| {
            CompileError::InternalError("No current function for pattern match".to_string(), None)
        })?;

        // Compile the scrutinee expression
        let scrutinee_val = self.compile_expression(scrutinee)?;

        // Create the merge block where all arms will jump to
        let merge_bb = self
            .context
            .append_basic_block(parent_function, "match_merge");

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

                self.builder
                    .build_and(matches, guard_val.into_int_value(), "guard_and_pattern")?
            } else {
                matches
            };

            // Create blocks for this arm
            let match_bb = self
                .context
                .append_basic_block(parent_function, &format!("match_{}", i));

            // Determine the next block - if this arm doesn't match, where do we go?
            let next_bb = if !is_last {
                // Not the last arm - go to test the next pattern
                self.context
                    .append_basic_block(parent_function, &format!("test_{}", i + 1))
            } else {
                // Last arm - create an unmatched block
                let block = self
                    .context
                    .append_basic_block(parent_function, "pattern_unmatched");
                unmatched_block = Some(block);
                block
            };

            // Branch based on the condition
            self.builder
                .build_conditional_branch(final_condition, match_bb, next_bb)?;

            // Generate code for the match block
            self.builder.position_at_end(match_bb);

            // Apply pattern bindings
            let saved_vars = self.apply_pattern_bindings(&bindings);

            // Compile the arm body
            let arm_val = self.compile_expression(&arm.body)?;

            // Restore variables
            self.restore_variables(saved_vars);

            // Jump to merge block only if the block doesn't already have a terminator (e.g., return)
            let current_block = self.builder.get_insert_block().unwrap();
            if current_block.get_terminator().is_none() {
                self.builder.build_unconditional_branch(merge_bb)?;
                let match_bb_end = self.builder.get_insert_block().unwrap();
                // Save value and block for phi node only if we branch to merge
                // eprintln!("DEBUG: Adding PHI value with type: {:?}", arm_val.get_type());
                phi_values.push((arm_val, match_bb_end));
            } else {
                // eprintln!("DEBUG: Arm {} has terminator, not adding to phi_values", i);
            }

            // Position at the next test block for the next iteration
            if !is_last {
                self.builder.position_at_end(next_bb);
                _current_block = next_bb;
            }
        }

        // Handle unmatched pattern block if we created one
        if let Some(unmatched_bb) = unmatched_block {
            self.builder.position_at_end(unmatched_bb);

            // Only add to phi_values if other arms also produced values
            // If all arms had returns/breaks/continues, don't add to phi_values
            if !phi_values.is_empty() {
                // For now, just return a default value (0) and branch to merge
                // In a complete implementation, this would be a runtime error
                // Use the same type as the first arm to ensure type consistency
                let default_val = match phi_values[0].0.get_type() {
                    inkwell::types::BasicTypeEnum::IntType(int_type) => {
                        int_type.const_int(0, false).into()
                    }
                    inkwell::types::BasicTypeEnum::FloatType(float_type) => {
                        float_type.const_float(0.0).into()
                    }
                    inkwell::types::BasicTypeEnum::PointerType(ptr_type) => {
                        // For pointer types (like strings), use a null pointer
                        ptr_type.const_null().into()
                    }
                    inkwell::types::BasicTypeEnum::StructType(struct_type) => {
                        // For struct types (like enums), create a zero struct
                        struct_type.const_zero().into()
                    }
                    _ => {
                        // For other types, use a default value
                        self.context.i32_type().const_int(0, false).into()
                    }
                };
                // Only branch if there's no terminator
                let current_block = self.builder.get_insert_block().unwrap();
                if current_block.get_terminator().is_none() {
                    self.builder.build_unconditional_branch(merge_bb)?;
                    phi_values.push((default_val, unmatched_bb));
                }
            } else {
                // All arms had terminators, so this unmatched block is also unreachable
                // We still need to add some terminator to make LLVM happy
                // We'll add an unreachable instruction
                self.builder.build_unreachable()?;
            }
        }

        // Position at merge block
        self.builder.position_at_end(merge_bb);

        // Check if we need to create a phi node
        if phi_values.is_empty() {
            // If all arms returned/broke/continued, the pattern match doesn't produce a value
            // The merge block is unreachable, but we still need to add a terminator
            self.builder.build_unreachable()?;
            // Return a dummy value - it won't be used since all paths have terminators
            return Ok(self.context.i32_type().const_int(0, false).into());
        }

        // Normalize all values to the same type
        // If all values are integers, cast them to the widest type
        let mut max_int_width = 0;
        let mut has_non_int = false;
        let _result_type = if !phi_values.is_empty() {
            phi_values[0].0.get_type()
        } else {
            // This shouldn't happen but let's be safe
            // eprintln!("WARNING: phi_values is empty after non-empty check");
            return Ok(self.context.i32_type().const_int(0, false).into());
        };

        for (value, _) in &phi_values {
            // eprintln!("DEBUG: PHI value type: {:?}", value.get_type());
            if value.is_int_value() {
                let int_val = value.into_int_value();
                let width = int_val.get_type().get_bit_width();
                // eprintln!("DEBUG: Integer value with width: {}", width);
                max_int_width = max_int_width.max(width);
            } else {
                has_non_int = true;
            }
        }

        // Normalize integer values if needed
        // eprintln!("DEBUG: Normalizing values - has_non_int: {}, max_int_width: {}", has_non_int, max_int_width);
        let normalized_values: Vec<(BasicValueEnum<'ctx>, inkwell::basic_block::BasicBlock<'ctx>)> =
            if !has_non_int && max_int_width > 0 {
                // All values are integers, cast them to the widest type
                let target_type = self.context.custom_width_int_type(max_int_width);
                phi_values
                    .into_iter()
                    .map(|(value, block)| {
                        if value.is_int_value() {
                            let int_val = value.into_int_value();
                            if int_val.get_type().get_bit_width() < max_int_width {
                                // Need to cast
                                let casted = self
                                    .builder
                                    .build_int_z_extend_or_bit_cast(
                                        int_val,
                                        target_type,
                                        "cast_to_common",
                                    )
                                    .unwrap();
                                (casted.into(), block)
                            } else {
                                (value, block)
                            }
                        } else {
                            (value, block)
                        }
                    })
                    .collect()
            } else {
                phi_values
            };

        // Use the normalized type for PHI node
        if normalized_values.is_empty() {
            // eprintln!("WARNING: normalized_values is empty after transformation");
            return Ok(self.context.i32_type().const_int(0, false).into());
        }
        let phi_type = normalized_values[0].0.get_type();
        let phi = self.builder.build_phi(phi_type, "match_result")?;

        // Add all incoming values
        for (value, block) in &normalized_values {
            phi.add_incoming(&[(value, *block)]);
        }

        Ok(phi.as_basic_value())
    }

    fn compile_range_expression(
        &mut self,
        start: &Expression,
        end: &Expression,
        inclusive: bool,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Represent ranges as a struct { start: i64, end: i64, inclusive: bool }
        // Always cast to i64 to ensure consistent types
        let start_val = self.compile_expression(start)?;
        let end_val = self.compile_expression(end)?;

        // Cast to i64 if needed
        let start_i64 = if start_val.is_int_value() {
            let int_val = start_val.into_int_value();
            if int_val.get_type() != self.context.i64_type() {
                self.builder
                    .build_int_cast(int_val, self.context.i64_type(), "start_cast")?
                    .into()
            } else {
                start_val
            }
        } else {
            start_val
        };

        let end_i64 = if end_val.is_int_value() {
            let int_val = end_val.into_int_value();
            if int_val.get_type() != self.context.i64_type() {
                self.builder
                    .build_int_cast(int_val, self.context.i64_type(), "end_cast")?
                    .into()
            } else {
                end_val
            }
        } else {
            end_val
        };

        // Create a simple struct type for the range - always use i64 for consistency
        let _range_struct_type = self.context.struct_type(
            &[
                self.context.i64_type().into(),
                self.context.i64_type().into(),
                self.context.bool_type().into(),
            ],
            false,
        );

        // Create the range struct value
        let range_struct = self.context.const_struct(
            &[
                start_i64,
                end_i64,
                self.context
                    .bool_type()
                    .const_int(inclusive as u64, false)
                    .into(),
            ],
            false,
        );

        Ok(range_struct.as_basic_value_enum())
    }

    fn compile_type_cast(
        &mut self,
        expr: &Expression,
        target_type: &crate::ast::AstType,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        use crate::ast::AstType;
        use inkwell::values::{IntValue, PointerValue};

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
                    "cast",
                )?;
                return Ok(casted.as_basic_value_enum());
            } else if let Ok(int_val) = value.try_into() {
                // Integer to pointer cast
                let int_val: IntValue = int_val;
                let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                let casted = self
                    .builder
                    .build_int_to_ptr(int_val, ptr_type, "inttoptr")?;
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
                let casted = self
                    .builder
                    .build_float_to_signed_int(float_val, target, "fptosi")?;
                Ok(casted.as_basic_value_enum())
            }
            // Int to float casts
            (BasicValueEnum::IntValue(int_val), AstType::F32) => {
                let target = self.context.f32_type();
                let casted = self
                    .builder
                    .build_signed_int_to_float(int_val, target, "sitofp")?;
                Ok(casted.as_basic_value_enum())
            }
            (BasicValueEnum::IntValue(int_val), AstType::F64) => {
                let target = self.context.f64_type();
                let casted = self
                    .builder
                    .build_signed_int_to_float(int_val, target, "sitofp")?;
                Ok(casted.as_basic_value_enum())
            }
            _ => {
                // For unhandled casts, just return the value as-is
                // This could be improved with better type checking
                Ok(value)
            }
        }
    }

    fn compile_range_loop(
        &mut self,
        start: &Expression,
        end: &Expression,
        inclusive: bool,
        args: &[Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Extract the loop variable and body from args
        // args[0] should be a closure with one parameter
        let closure = args.get(0).ok_or_else(|| {
            CompileError::InternalError(
                "Range.loop() requires a closure argument".to_string(),
                None,
            )
        })?;

        let (param_name, loop_body) = match closure {
            Expression::Closure { params, body } => {
                let param = params.get(0).ok_or_else(|| {
                    CompileError::InternalError(
                        "Range.loop() closure must have one parameter".to_string(),
                        None,
                    )
                })?;
                (param.0.clone(), body.as_ref())
            }
            _ => {
                return Err(CompileError::InternalError(
                    "Range.loop() requires a closure argument".to_string(),
                    None,
                ))
            }
        };

        // Compile start and end values
        let start_val = self.compile_expression(start)?;
        let end_val = self.compile_expression(end)?;

        // Ensure both are integers
        let (start_int, end_int) = match (start_val, end_val) {
            (BasicValueEnum::IntValue(s), BasicValueEnum::IntValue(e)) => (s, e),
            _ => {
                return Err(CompileError::InternalError(
                    "Range bounds must be integers".to_string(),
                    None,
                ))
            }
        };

        // Create loop blocks
        let loop_header = self
            .context
            .append_basic_block(self.current_function.unwrap(), "range_header");
        let loop_body_block = self
            .context
            .append_basic_block(self.current_function.unwrap(), "range_body");
        let after_loop = self
            .context
            .append_basic_block(self.current_function.unwrap(), "after_range");

        // Allocate space for the loop variable
        let loop_var = self
            .builder
            .build_alloca(start_int.get_type(), &param_name)?;
        self.builder.build_store(loop_var, start_int)?;

        // Jump to loop header
        self.builder.build_unconditional_branch(loop_header)?;
        self.builder.position_at_end(loop_header);

        // Load current value
        let current = self
            .builder
            .build_load(start_int.get_type(), loop_var, &param_name)?;
        let current_int = current.into_int_value();

        // Check loop condition
        let condition = if inclusive {
            self.builder.build_int_compare(
                inkwell::IntPredicate::SLE,
                current_int,
                end_int,
                "range_check",
            )?
        } else {
            self.builder.build_int_compare(
                inkwell::IntPredicate::SLT,
                current_int,
                end_int,
                "range_check",
            )?
        };

        self.builder
            .build_conditional_branch(condition, loop_body_block, after_loop)?;
        self.builder.position_at_end(loop_body_block);

        // Push loop context for break/continue
        self.loop_stack.push((loop_header, after_loop));

        // Enter a new scope for the loop body
        self.symbols.enter_scope();

        // Store the loop variable in symbols table (AFTER entering scope)
        use crate::codegen::llvm::symbols::Symbol;
        self.symbols.insert(&param_name, Symbol::Variable(loop_var));

        // Also store in the variables map for compatibility
        self.variables.insert(
            param_name.clone(),
            super::VariableInfo {
                pointer: loop_var,
                ast_type: crate::ast::AstType::I32,
                is_mutable: true, // Loop variables are mutable
                is_initialized: true,
            },
        );

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

        // Exit the scope but DON'T remove from variables map yet
        self.symbols.exit_scope();

        // Increment the loop variable (still needs to access loop_var)
        let current = self
            .builder
            .build_load(start_int.get_type(), loop_var, &param_name)?;
        let one = start_int.get_type().const_int(1, false);
        let next = self
            .builder
            .build_int_add(current.into_int_value(), one, "next")?;
        self.builder.build_store(loop_var, next)?;

        // Jump back to header if no break was executed
        let current_block = self.builder.get_insert_block().unwrap();
        if current_block.get_terminator().is_none() {
            self.builder.build_unconditional_branch(loop_header)?;
        }

        self.loop_stack.pop();
        self.builder.position_at_end(after_loop);

        // NOW remove the loop variable from variables map
        self.variables.remove(&param_name);

        // Return void value
        Ok(self.context.i32_type().const_int(0, false).into())
    }

    /// Compile Vec<T, size>() constructor
    fn compile_vec_constructor(
        &mut self,
        element_type: &crate::ast::AstType,
        size: usize,
        initial_values: Option<&Vec<Expression>>,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // For now, simplify Vec to just return a placeholder
        // TODO: Implement proper Vec with array storage

        // Get element LLVM type
        let _elem_llvm_type = self.to_llvm_type(element_type)?;

        // Create a simple struct type: { i64 capacity, i64 len }
        let i64_type = self.context.i64_type();
        let vec_struct_type = self.context.struct_type(
            &[
                i64_type.into(), // capacity
                i64_type.into(), // length
            ],
            false,
        );

        // Allocate the Vec on the stack
        let vec_ptr = self.builder.build_alloca(vec_struct_type, "vec")?;

        // Initialize capacity field
        let cap_field_ptr =
            self.builder
                .build_struct_gep(vec_struct_type, vec_ptr, 0, "cap_field")?;
        let cap_value = i64_type.const_int(size as u64, false);
        self.builder.build_store(cap_field_ptr, cap_value)?;

        // Initialize length field
        let initial_len = if let Some(values) = initial_values {
            values.len() as u64
        } else {
            0
        };
        let len_field_ptr =
            self.builder
                .build_struct_gep(vec_struct_type, vec_ptr, 1, "len_field")?;
        let len_value = i64_type.const_int(initial_len, false);
        self.builder.build_store(len_field_ptr, len_value)?;

        // TODO: Actually allocate and initialize the data array

        // Load and return the Vec struct value
        Ok(self
            .builder
            .build_load(vec_struct_type, vec_ptr, "vec_value")?)
    }

    /// Compile DynVec<T>() or DynVec<T1, T2, ...>() constructor
    fn compile_dynvec_constructor(
        &mut self,
        element_types: &[crate::ast::AstType],
        allocator: &Expression,
        initial_capacity: Option<&Expression>,
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
            _ => {
                return Err(CompileError::TypeError(
                    "DynVec capacity must be integer".to_string(),
                    None,
                ))
            }
        };

        // Create DynVec struct based on element types
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let len_type = self.context.i64_type();
        let cap_type = self.context.i64_type();

        let dynvec_struct_type = if element_types.len() == 1 {
            // Single type DynVec: { ptr, len, capacity }
            self.context
                .struct_type(&[ptr_type.into(), len_type.into(), cap_type.into()], false)
        } else {
            // Mixed variant DynVec: { ptr, len, capacity, discriminants }
            let discriminant_ptr = self.context.ptr_type(inkwell::AddressSpace::default());
            self.context.struct_type(
                &[
                    ptr_type.into(),
                    len_type.into(),
                    cap_type.into(),
                    discriminant_ptr.into(),
                ],
                false,
            )
        };

        // Allocate the DynVec on the stack
        let dynvec_ptr = self.builder.build_alloca(dynvec_struct_type, "dynvec")?;

        // Initialize data pointer to null initially
        let data_field_ptr =
            self.builder
                .build_struct_gep(dynvec_struct_type, dynvec_ptr, 0, "data_field")?;
        let null_ptr = ptr_type.const_null();
        self.builder.build_store(data_field_ptr, null_ptr)?;

        // Initialize length to 0
        let len_field_ptr =
            self.builder
                .build_struct_gep(dynvec_struct_type, dynvec_ptr, 1, "len_field")?;
        let len_zero = len_type.const_int(0, false);
        self.builder.build_store(len_field_ptr, len_zero)?;

        // Initialize capacity
        let cap_field_ptr =
            self.builder
                .build_struct_gep(dynvec_struct_type, dynvec_ptr, 2, "cap_field")?;
        self.builder.build_store(cap_field_ptr, capacity_int)?;

        // Allocate memory for the data array if capacity > 0
        if capacity_int.get_zero_extended_constant().is_some() {
            let cap_value = capacity_int.get_zero_extended_constant().unwrap();
            if cap_value > 0 {
                // Calculate size needed for elements
                // For now, assume i32 elements (4 bytes each) - will need proper type size calculation
                let element_size = self.context.i64_type().const_int(8, false); // Default to 8 bytes per element
                                                                                // Ensure capacity_int is i64 for multiplication
                let capacity_i64 = if capacity_int.get_type().get_bit_width() == 64 {
                    capacity_int
                } else {
                    self.builder.build_int_z_extend(
                        capacity_int,
                        self.context.i64_type(),
                        "cap_i64",
                    )?
                };
                let total_size =
                    self.builder
                        .build_int_mul(capacity_i64, element_size, "total_size")?;

                // Call malloc to allocate memory
                let malloc_fn = self.module.get_function("malloc").ok_or_else(|| {
                    CompileError::InternalError("No malloc function declared".to_string(), None)
                })?;

                let allocated_ptr = self
                    .builder
                    .build_call(malloc_fn, &[total_size.into()], "dynvec_data_malloc")?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| {
                        CompileError::InternalError(
                            "malloc did not return a value".to_string(),
                            None,
                        )
                    })?
                    .into_pointer_value();

                // Store the allocated pointer in the data field
                self.builder.build_store(data_field_ptr, allocated_ptr)?;

                // For mixed variant DynVec, also allocate discriminant array
                if element_types.len() > 1 {
                    let discriminant_size = self.builder.build_int_mul(
                        capacity_i64,
                        self.context.i64_type().const_int(1, false),
                        "discriminant_size",
                    )?;

                    let discriminant_ptr = self
                        .builder
                        .build_call(
                            malloc_fn,
                            &[discriminant_size.into()],
                            "discriminant_malloc",
                        )?
                        .try_as_basic_value()
                        .left()
                        .ok_or_else(|| {
                            CompileError::InternalError(
                                "malloc did not return a value".to_string(),
                                None,
                            )
                        })?
                        .into_pointer_value();

                    let discriminant_field_ptr = self.builder.build_struct_gep(
                        dynvec_struct_type,
                        dynvec_ptr,
                        3,
                        "discriminant_field",
                    )?;
                    self.builder
                        .build_store(discriminant_field_ptr, discriminant_ptr)?;
                }
            }
        }

        // For mixed variant DynVec with no initial capacity, initialize discriminants pointer to null
        if element_types.len() > 1
            && capacity_int
                .get_zero_extended_constant()
                .map_or(true, |v| v == 0)
        {
            let discriminant_field_ptr = self.builder.build_struct_gep(
                dynvec_struct_type,
                dynvec_ptr,
                3,
                "discriminant_field",
            )?;
            self.builder.build_store(discriminant_field_ptr, null_ptr)?;
        }

        // Load and return the DynVec struct value
        Ok(self
            .builder
            .build_load(dynvec_struct_type, dynvec_ptr, "dynvec_value")?)
    }

    /// Compile .raise() expression by transforming it into pattern matching
    ///
    /// .raise() is syntactic sugar for early error propagation:
    /// expr.raise() transforms Result<T, E> by either unwrapping Ok(val) to val
    /// or early returning with Err(e)
    fn compile_raise_expression(
        &mut self,
        expr: &Expression,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Generate a unique ID for this raise to avoid block name collisions
        static mut RAISE_ID: u32 = 0;
        let raise_id = unsafe {
            RAISE_ID += 1;
            RAISE_ID
        };

        let parent_function = self.current_function.ok_or_else(|| {
            CompileError::InternalError("No current function for .raise()".to_string(), None)
        })?;

        // Get the current function's name to look up its return type
        let function_name = parent_function
            .get_name()
            .to_str()
            .unwrap_or("anon")
            .to_string();

        // Check if the function returns a Result type
        let returns_result = if let Some(return_type) = self.function_types.get(&function_name) {
            match return_type {
                AstType::Generic { name, .. } if name == "Result" => true,
                AstType::Result { .. } => true, // Also handle legacy Result type
                _ => false,
            }
        } else {
            false
        };

        // Compile the expression that should return a Result<T, E>
        let result_value = self.compile_expression(expr)?;

        // Create blocks for pattern matching on Result
        let check_bb = self
            .context
            .append_basic_block(parent_function, &format!("raise_check_{}", raise_id));
        let ok_bb = self
            .context
            .append_basic_block(parent_function, &format!("raise_ok_{}", raise_id));
        let err_bb = self
            .context
            .append_basic_block(parent_function, &format!("raise_err_{}", raise_id));
        let continue_bb = self
            .context
            .append_basic_block(parent_function, &format!("raise_continue_{}", raise_id));

        // Jump to check block
        self.builder.build_unconditional_branch(check_bb)?;
        self.builder.position_at_end(check_bb);

        // Handle the Result enum based on its actual representation
        // Result<T, E> is an enum with variants Ok(T) and Err(E)
        // This should work with the existing enum compilation system

        if result_value.is_struct_value() {
            // Result is represented as a struct with tag + payload
            let struct_val = result_value.into_struct_value();
            let struct_type = struct_val.get_type();

            // Create a temporary alloca to work with the struct
            let temp_alloca = self.builder.build_alloca(struct_type, "result_temp")?;
            self.builder.build_store(temp_alloca, struct_val)?;

            // Extract the tag (discriminant) from the first field
            let tag_ptr = self
                .builder
                .build_struct_gep(struct_type, temp_alloca, 0, "tag_ptr")?;
            let tag_value = self
                .builder
                .build_load(self.context.i64_type(), tag_ptr, "tag")?;

            // Check if tag == 0 (Ok variant)
            let is_ok = self.builder.build_int_compare(
                inkwell::IntPredicate::EQ,
                tag_value.into_int_value(),
                self.context.i64_type().const_int(0, false),
                "is_ok",
            )?;

            // Branch based on the tag
            self.builder
                .build_conditional_branch(is_ok, ok_bb, err_bb)?;

            // Handle Ok case - extract the Ok value
            self.builder.position_at_end(ok_bb);
            if struct_type.count_fields() > 1 {
                let payload_ptr =
                    self.builder
                        .build_struct_gep(struct_type, temp_alloca, 1, "payload_ptr")?;
                // Get the actual payload type from the struct
                let payload_field_type =
                    struct_type.get_field_type_at_index(1).ok_or_else(|| {
                        CompileError::InternalError(
                            "Result payload field not found".to_string(),
                            None,
                        )
                    })?;

                // Load the payload value (which is a pointer to the actual value)
                let ok_value_ptr =
                    self.builder
                        .build_load(payload_field_type, payload_ptr, "ok_value_ptr")?;

                // The payload is always stored as a pointer in our enum representation
                // We need to dereference it to get the actual value
                let ok_value = if ok_value_ptr.is_pointer_value() {
                    let ptr_val = ok_value_ptr.into_pointer_value();

                    // Use the tracked generic type information to determine the correct type to load
                    let load_type: inkwell::types::BasicTypeEnum =
                        if let Some(ast_type) = self.generic_type_context.get("Result_Ok_Type") {
                            match ast_type {
                                AstType::I8 => self.context.i8_type().into(),
                                AstType::I16 => self.context.i16_type().into(),
                                AstType::I32 => self.context.i32_type().into(),
                                AstType::I64 => self.context.i64_type().into(),
                                AstType::U8 => self.context.i8_type().into(),
                                AstType::U16 => self.context.i16_type().into(),
                                AstType::U32 => self.context.i32_type().into(),
                                AstType::U64 => self.context.i64_type().into(),
                                AstType::F32 => self.context.f32_type().into(),
                                AstType::F64 => self.context.f64_type().into(),
                                AstType::Bool => self.context.bool_type().into(),
                                _ => self.context.i32_type().into(), // Default fallback
                            }
                        } else {
                            // Default to i32 for backward compatibility
                            self.context.i32_type().into()
                        };
                    let loaded_value =
                        self.builder
                            .build_load(load_type, ptr_val, "ok_value_deref")?;

                    // The loaded value should be the correct type
                    loaded_value
                } else {
                    // If it's not a pointer, it might be an integer that looks like a pointer address
                    // This can happen if the payload is stored incorrectly
                    ok_value_ptr
                };
                self.builder.build_unconditional_branch(continue_bb)?;

                // Handle Err case - propagate the error by returning early
                self.builder.position_at_end(err_bb);

                if returns_result {
                    // Function returns Result<T,E> - propagate the entire Result with Err variant
                    let err_payload_ptr = self.builder.build_struct_gep(
                        struct_type,
                        temp_alloca,
                        1,
                        "err_payload_ptr",
                    )?;

                    // Get the actual payload type from the struct
                    let payload_field_type =
                        struct_type.get_field_type_at_index(1).ok_or_else(|| {
                            CompileError::InternalError(
                                "Result payload field not found".to_string(),
                                None,
                            )
                        })?;

                    // Load the error payload with the correct type
                    let err_value = self.builder.build_load(
                        payload_field_type,
                        err_payload_ptr,
                        "err_value",
                    )?;

                    // Create a new Result<T,E> with Err variant for early return
                    let return_result_alloca =
                        self.builder.build_alloca(struct_type, "return_result")?;

                    // Set tag to 1 (Err)
                    let return_tag_ptr = self.builder.build_struct_gep(
                        struct_type,
                        return_result_alloca,
                        0,
                        "return_tag_ptr",
                    )?;
                    self.builder
                        .build_store(return_tag_ptr, self.context.i64_type().const_int(1, false))?;

                    // Store the error value
                    let return_payload_ptr = self.builder.build_struct_gep(
                        struct_type,
                        return_result_alloca,
                        1,
                        "return_payload_ptr",
                    )?;
                    self.builder.build_store(return_payload_ptr, err_value)?;

                    // Load and return the complete Result
                    let return_result = self.builder.build_load(
                        struct_type,
                        return_result_alloca,
                        "return_result",
                    )?;
                    self.builder.build_return(Some(&return_result))?;
                } else {
                    // Function returns a plain type (like i32) - this is an error case
                    // For now, we'll return a default error value (1 for i32, indicating error)
                    // In a proper implementation, this would need better error handling
                    let error_value = self.context.i32_type().const_int(1, false);
                    self.builder.build_return(Some(&error_value))?;
                }

                // Continue with Ok value
                self.builder.position_at_end(continue_bb);
                Ok(ok_value)
            } else {
                // Unit Result (no payload)
                self.builder.build_unconditional_branch(continue_bb)?;

                self.builder.position_at_end(err_bb);
                // For unit Results, handle based on return type
                if returns_result {
                    self.builder.build_return(Some(&struct_val))?;
                } else {
                    // Return error value for plain return type
                    let error_value = self.context.i32_type().const_int(1, false);
                    self.builder.build_return(Some(&error_value))?;
                }

                self.builder.position_at_end(continue_bb);
                Ok(self.context.i64_type().const_int(0, false).into())
            }
        } else if result_value.is_pointer_value() {
            // Result is stored as a pointer to a struct
            let result_ptr = result_value.into_pointer_value();

            // For opaque pointers in LLVM 15+, we need to determine the struct type differently
            // For now, we'll assume it's a Result struct type and try to work with it
            // In a complete implementation, this would be tracked by the type system

            // Create a basic Result struct type for demonstration
            let struct_type = self.context.struct_type(
                &[
                    self.context.i64_type().into(), // tag
                    self.context.i64_type().into(), // payload
                ],
                false,
            );

            // Extract the tag from the first field
            let tag_ptr = self
                .builder
                .build_struct_gep(struct_type, result_ptr, 0, "tag_ptr")?;
            let tag_value = self
                .builder
                .build_load(self.context.i64_type(), tag_ptr, "tag")?;

            // Check if tag == 0 (Ok variant)
            let is_ok = self.builder.build_int_compare(
                inkwell::IntPredicate::EQ,
                tag_value.into_int_value(),
                self.context.i64_type().const_int(0, false),
                "is_ok",
            )?;

            // Branch based on the tag
            self.builder
                .build_conditional_branch(is_ok, ok_bb, err_bb)?;

            // Handle Ok case
            self.builder.position_at_end(ok_bb);
            if struct_type.count_fields() > 1 {
                let payload_ptr =
                    self.builder
                        .build_struct_gep(struct_type, result_ptr, 1, "payload_ptr")?;
                // Load the payload, which is stored as a pointer to the actual value
                let ok_value_ptr = self.builder.build_load(
                    self.context.ptr_type(inkwell::AddressSpace::default()),
                    payload_ptr,
                    "ok_value_ptr",
                )?;

                // Dereference the pointer to get the actual value
                // For now, assume Result<i32, E>
                let ok_value = if ok_value_ptr.is_pointer_value() {
                    let ptr_val = ok_value_ptr.into_pointer_value();
                    self.builder
                        .build_load(self.context.i32_type(), ptr_val, "ok_value_deref")?
                } else {
                    ok_value_ptr
                };
                self.builder.build_unconditional_branch(continue_bb)?;

                // Handle Err case
                self.builder.position_at_end(err_bb);

                if returns_result {
                    // Return the original Result (already contains Err)
                    let err_result =
                        self.builder
                            .build_load(struct_type, result_ptr, "err_result")?;
                    self.builder.build_return(Some(&err_result))?;
                } else {
                    // Function returns a plain type - return error value
                    let error_value = self.context.i32_type().const_int(1, false);
                    self.builder.build_return(Some(&error_value))?;
                }

                // Continue with Ok value
                self.builder.position_at_end(continue_bb);
                Ok(ok_value)
            } else {
                // Unit Result
                self.builder.build_unconditional_branch(continue_bb)?;

                self.builder.position_at_end(err_bb);

                if returns_result {
                    let err_result =
                        self.builder
                            .build_load(struct_type, result_ptr, "err_result")?;
                    self.builder.build_return(Some(&err_result))?;
                } else {
                    // Return error value for plain return type
                    let error_value = self.context.i32_type().const_int(1, false);
                    self.builder.build_return(Some(&error_value))?;
                }

                self.builder.position_at_end(continue_bb);
                Ok(self.context.i64_type().const_int(0, false).into())
            }
        } else {
            // Fallback: if we can't determine the Result structure,
            // treat it as an immediate value and try pattern matching
            return Err(CompileError::TypeError(
                format!(
                    "Unsupported Result type for .raise(): {:?}",
                    result_value.get_type()
                ),
                None,
            ));
        }
    }

    fn compile_option_variant(
        &mut self,
        variant: &str,
        payload: Option<&Expression>,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // For Option type, use a simple tag-based representation
        // Tag 0 = Some, Tag 1 = None (matching the standard enum ordering)
        let tag = if variant == "Some" { 0 } else { 1 };

        // Create a struct to represent the enum variant
        // Use i64 for tag to match what the pattern matching expects
        let tag_type = self.context.i64_type();

        // Compile the payload if present and determine its type
        let (payload_value, payload_type) = if let Some(expr) = payload {
            let val = self.compile_expression(expr)?;

            // Track the type for generic instantiation
            if variant == "Some" {
                // Infer and store the AST type for Option<T>
                let ast_type = self.infer_expression_type(expr);
                if let Ok(t) = ast_type {
                    self.generic_type_context
                        .insert("Option_Some_Type".to_string(), t);
                }
            }

            (val, val.get_type())
        } else {
            // For None variant, use i64 as placeholder
            let placeholder = self.context.i64_type().const_int(0, false).into();
            (placeholder, self.context.i64_type().into())
        };

        let struct_type = self
            .context
            .struct_type(&[tag_type.into(), payload_type], false);

        // Create the struct value
        let struct_val = struct_type.get_undef();
        let struct_val = self.builder.build_insert_value(
            struct_val,
            tag_type.const_int(tag as u64, false),
            0,
            "tag",
        )?;
        let struct_val =
            self.builder
                .build_insert_value(struct_val, payload_value, 1, "payload")?;

        // Convert AggregateValueEnum to BasicValueEnum
        // Since we're creating a struct, we need to handle it properly
        match struct_val {
            inkwell::values::AggregateValueEnum::StructValue(sv) => Ok(sv.into()),
            _ => Err(CompileError::InternalError(
                "Expected struct value".to_string(),
                None,
            )),
        }
    }

    fn compile_collection_loop(
        &mut self,
        collection: &Expression,
        param: &str,
        index_param: Option<&str>,
        body: &Expression,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Check if the collection is a Range
        if let Expression::Range {
            start,
            end,
            inclusive,
        } = collection
        {
            // Convert to compile_range_loop format
            let closure = Expression::Closure {
                params: vec![(param.to_string(), None)],
                body: Box::new(body.clone()),
            };
            return self.compile_range_loop(start, end, *inclusive, &[closure]);
        }

        // Check if collection is an identifier that holds a Range
        if let Expression::Identifier(name) = collection {
            // Look up the variable to see if it's a Range type
            if let Some(var_info) = self.variables.get(name).cloned() {
                // eprintln!("DEBUG: Variable {} has type {:?}", name, var_info.ast_type);
                if let AstType::Range {  .. } = &var_info.ast_type {
                    // We need to extract the range values from the stored struct
                    // Load the range struct
                    let range_type = match self.to_llvm_type(&var_info.ast_type)? {
                        super::Type::Struct(s) => s,
                        _ => {
                            return Err(CompileError::InternalError(
                                "Expected struct type for Range".to_string(),
                                None,
                            ))
                        }
                    };
                    let range_val = self
                        .builder
                        .build_load(range_type, var_info.pointer, name)?;

                    // Extract start, end, and inclusive from the struct
                    let range_struct = range_val.into_struct_value();
                    let start_val = self
                        .builder
                        .build_extract_value(range_struct, 0, "start")?
                        .into_int_value();
                    let end_val = self
                        .builder
                        .build_extract_value(range_struct, 1, "end")?
                        .into_int_value();
                    let inclusive_val = self
                        .builder
                        .build_extract_value(range_struct, 2, "inclusive")?
                        .into_int_value();

                    // Check if inclusive (convert bool to actual value)
                    // Check if the constant is non-zero
                    let is_inclusive =
                        if let Some(const_val) = inclusive_val.get_zero_extended_constant() {
                            const_val != 0
                        } else {
                            false // Default to exclusive if we can't determine
                        };

                    // Create a proper range loop implementation here
                    return self.compile_range_loop_from_values(
                        start_val,
                        end_val,
                        is_inclusive,
                        param,
                        body,
                    );
                }
            }
        }

        // This is a simplified implementation for non-Range collections
        // In a full implementation, we'd need to handle different collection types

        // Compile the collection expression
        let _collection_val = self.compile_expression(collection)?;

        // For now, create a simple counted loop as a placeholder
        let parent_function = self.current_function.ok_or_else(|| {
            CompileError::InternalError("No current function for loop".to_string(), None)
        })?;

        let loop_bb = self
            .context
            .append_basic_block(parent_function, "collection_loop");
        let body_bb = self
            .context
            .append_basic_block(parent_function, "collection_body");
        let after_loop_bb = self
            .context
            .append_basic_block(parent_function, "after_collection_loop");

        // Jump to loop header
        self.builder.build_unconditional_branch(loop_bb)?;
        self.builder.position_at_end(loop_bb);

        // For now, just execute the body once and exit
        // A full implementation would iterate over the collection
        self.builder.build_unconditional_branch(body_bb)?;
        self.builder.position_at_end(body_bb);

        // Push loop onto stack for break/continue
        self.loop_stack.push((loop_bb, after_loop_bb));

        // Create a dummy variable for the loop parameter
        let param_type = self.context.i64_type();
        let param_ptr = self.builder.build_alloca(param_type, param)?;
        self.builder
            .build_store(param_ptr, param_type.const_int(0, false))?;

        self.variables.insert(
            param.to_string(),
            VariableInfo {
                pointer: param_ptr,
                ast_type: AstType::I64,
                is_mutable: false,
                is_initialized: true,
            },
        );

        // Handle index parameter if present
        if let Some(idx_param) = index_param {
            let idx_ptr = self.builder.build_alloca(param_type, idx_param)?;
            self.builder
                .build_store(idx_ptr, param_type.const_int(0, false))?;

            self.variables.insert(
                idx_param.to_string(),
                VariableInfo {
                    pointer: idx_ptr,
                    ast_type: AstType::I64,
                    is_mutable: false,
                    is_initialized: true,
                },
            );
        }

        // Compile loop body
        let _body_val = self.compile_expression(body)?;

        // For now, just exit after one iteration
        self.builder.build_unconditional_branch(after_loop_bb)?;

        // Position after loop
        self.builder.position_at_end(after_loop_bb);

        // Pop loop stack
        self.loop_stack.pop();

        // Return unit value
        Ok(self.context.i32_type().const_int(0, false).into())
    }

    fn compile_range_loop_from_values(
        &mut self,
        start_val: inkwell::values::IntValue<'ctx>,
        end_val: inkwell::values::IntValue<'ctx>,
        inclusive: bool,
        param: &str,
        body: &Expression,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Create loop blocks
        let loop_header = self
            .context
            .append_basic_block(self.current_function.unwrap(), "range_header");
        let loop_body_block = self
            .context
            .append_basic_block(self.current_function.unwrap(), "range_body");
        let after_loop = self
            .context
            .append_basic_block(self.current_function.unwrap(), "after_range");

        // Allocate space for the loop variable
        let loop_var = self.builder.build_alloca(start_val.get_type(), param)?;
        self.builder.build_store(loop_var, start_val)?;

        // Jump to loop header
        self.builder.build_unconditional_branch(loop_header)?;
        self.builder.position_at_end(loop_header);

        // Load current value
        let current = self
            .builder
            .build_load(start_val.get_type(), loop_var, param)?;
        let current_int = current.into_int_value();

        // Check loop condition
        let condition = if inclusive {
            self.builder.build_int_compare(
                inkwell::IntPredicate::SLE,
                current_int,
                end_val,
                "range_check",
            )?
        } else {
            self.builder.build_int_compare(
                inkwell::IntPredicate::SLT,
                current_int,
                end_val,
                "range_check",
            )?
        };

        self.builder
            .build_conditional_branch(condition, loop_body_block, after_loop)?;
        self.builder.position_at_end(loop_body_block);

        // Push loop context for break/continue
        self.loop_stack.push((loop_header, after_loop));

        // Enter a new scope for the loop body
        self.symbols.enter_scope();

        // Store the loop variable in symbols table (AFTER entering scope)
        use crate::codegen::llvm::symbols::Symbol;
        self.symbols.insert(param, Symbol::Variable(loop_var));

        // Also store in the variables map for compatibility
        self.variables.insert(
            param.to_string(),
            super::VariableInfo {
                pointer: loop_var,
                ast_type: crate::ast::AstType::I32,
                is_mutable: false, // Loop variables are immutable
                is_initialized: true,
            },
        );

        // Compile the loop body
        match body {
            Expression::Block(statements) => {
                for stmt in statements {
                    self.compile_statement(stmt)?;
                }
            }
            _ => {
                self.compile_expression(body)?;
            }
        }

        // Exit the scope but DON'T remove from variables map yet
        self.symbols.exit_scope();

        // Increment the loop variable (still needs to access loop_var)
        let current = self
            .builder
            .build_load(start_val.get_type(), loop_var, param)?;
        let one = start_val.get_type().const_int(1, false);
        let next = self
            .builder
            .build_int_add(current.into_int_value(), one, "next")?;
        self.builder.build_store(loop_var, next)?;

        // Jump back to loop header
        self.builder.build_unconditional_branch(loop_header)?;

        // Position after loop
        self.builder.position_at_end(after_loop);

        // Pop loop stack
        self.loop_stack.pop();

        // NOW remove from variables map (after we're done using the loop variable)
        self.variables.remove(param);

        // Return unit value
        Ok(self.context.i32_type().const_int(0, false).into())
    }

    fn register_deferred_expression(&mut self, expr: &Expression) -> Result<(), CompileError> {
        // Add the expression to the defer stack
        // These will be executed in LIFO order at scope exit
        self.defer_stack.push(expr.clone());
        Ok(())
    }
}
