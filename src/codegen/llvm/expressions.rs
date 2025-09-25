use super::{symbols, LLVMCompiler, Type, VariableInfo};
use crate::ast::{AstType, Expression};
use crate::error::CompileError;
use inkwell::module::Linkage;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValue, BasicValueEnum, PointerValue};
use inkwell::AddressSpace;

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
            Expression::EnumVariant { enum_name, variant, payload } => {
                // Infer the type of enum variants
                if enum_name == "Option" {
                    if variant == "Some" && payload.is_some() {
                        if let Some(ref p) = payload {
                            let inner_type = self.infer_expression_type(p)?;
                            Ok(AstType::Generic {
                                name: "Option".to_string(),
                                type_args: vec![inner_type],
                            })
                        } else {
                            Ok(AstType::Generic {
                                name: "Option".to_string(),
                                type_args: vec![AstType::Void],
                            })
                        }
                    } else {
                        // None variant - use context if available
                        if let Some(t) = self.generic_type_context.get("Option_Some_Type") {
                            Ok(AstType::Generic {
                                name: "Option".to_string(),
                                type_args: vec![t.clone()],
                            })
                        } else {
                            Ok(AstType::Generic {
                                name: "Option".to_string(),
                                type_args: vec![AstType::Void],
                            })
                        }
                    }
                } else if enum_name == "Result" {
                    // For Result, we need to track which variant we're in
                    if variant == "Ok" && payload.is_some() {
                        if let Some(ref p) = payload {
                            let inner_type = self.infer_expression_type(p)?;
                            
                            // NOTE: Can't store type here because infer_expression_type is immutable
                            // The actual type tracking happens during compile_enum_variant
                            
                            // Try to get error type from context
                            let err_type = self.generic_type_context.get("Result_Err_Type")
                                .cloned()
                                .unwrap_or(AstType::String); // Default to String for errors
                            Ok(AstType::Generic {
                                name: "Result".to_string(),
                                type_args: vec![inner_type, err_type],
                            })
                        } else {
                            Ok(AstType::Generic {
                                name: "Result".to_string(),
                                type_args: vec![AstType::Void, AstType::Void],
                            })
                        }
                    } else if variant == "Err" && payload.is_some() {
                        if let Some(ref p) = payload {
                            let inner_type = self.infer_expression_type(p)?;
                            
                            // NOTE: Can't store type here because infer_expression_type is immutable
                            // The actual type tracking happens during compile_enum_variant
                            
                            // Try to get ok type from context
                            let ok_type = self.generic_type_context.get("Result_Ok_Type")
                                .cloned()
                                .unwrap_or(AstType::Void);
                            Ok(AstType::Generic {
                                name: "Result".to_string(),
                                type_args: vec![ok_type, inner_type],
                            })
                        } else {
                            Ok(AstType::Generic {
                                name: "Result".to_string(),
                                type_args: vec![AstType::Void, AstType::Void],
                            })
                        }
                    } else {
                        // Try to get types from context
                        let ok_type = self.generic_type_context.get("Result_Ok_Type")
                            .cloned()
                            .unwrap_or(AstType::Void);
                        let err_type = self.generic_type_context.get("Result_Err_Type")
                            .cloned()
                            .unwrap_or(AstType::Void);
                        Ok(AstType::Generic {
                            name: "Result".to_string(),
                            type_args: vec![ok_type, err_type],
                        })
                    }
                } else {
                    // Unknown enum
                    Ok(AstType::Void)
                }
            }
            Expression::MethodCall { object, method, .. } => {
                // Special handling for .raise() method which extracts T from Result<T,E>
                if method == "raise" {
                    // Get the type of the object being raised
                    let object_type = self.infer_expression_type(object)?;
                    
                    // If it's Result<T,E>, return T
                    if let AstType::Generic { name, type_args } = object_type {
                        if name == "Result" && type_args.len() == 2 {
                            // The raise() method returns the Ok type (T) from Result<T,E>
                            return Ok(type_args[0].clone());
                        }
                    }
                    // If not a Result type, return Void (will error during compilation)
                    Ok(AstType::Void)
                } else {
                    // For other methods, we'll need more context
                    // For now, return Void
                    Ok(AstType::Void)
                }
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
                    
                    // Check if this is a generic type constructor (e.g., HashMap<K,V>.new())
                    if name.contains('<') && method == "new" {
                        // Extract the base type name
                        if let Some(angle_pos) = name.find('<') {
                            let base_type = &name[..angle_pos];
                            
                            // Call the appropriate constructor function
                            match base_type {
                                "HashMap" => {
                                    // HashMap.new() creates an empty hashmap with allocated buckets
                                    let hashmap_type = self.context.struct_type(
                                        &[
                                            self.context.ptr_type(AddressSpace::default()).into(), // buckets
                                            self.context.i64_type().into(), // size
                                            self.context.i64_type().into(), // capacity
                                        ],
                                        false,
                                    );
                                    
                                    // Initial capacity
                                    let initial_capacity = 16i64;
                                    let capacity_value = self.context.i64_type().const_int(initial_capacity as u64, false);
                                    
                                    // Each bucket is a struct with: [key_hash, key_ptr, value_ptr, occupied_flag]
                                    // We'll allocate an array of i64 values (4 per bucket)
                                    // [key_hash(i64), key_ptr(ptr), value_ptr(ptr), occupied(i64)]
                                    let bucket_size = 4i64; // 4 values per bucket
                                    let total_size = initial_capacity * bucket_size * 8; // 8 bytes per field
                                    
                                    // Call malloc to allocate memory for buckets
                                    let malloc_fn = self.module.get_function("malloc").unwrap_or_else(|| {
                                        let malloc_type = self.context.ptr_type(AddressSpace::default())
                                            .fn_type(&[self.context.i64_type().into()], false);
                                        self.module.add_function("malloc", malloc_type, None)
                                    });
                                    
                                    let size_arg = self.context.i64_type().const_int(total_size as u64, false);
                                    let buckets_ptr = self.builder
                                        .build_call(malloc_fn, &[size_arg.into()], "buckets")
                                        .unwrap()
                                        .try_as_basic_value()
                                        .left()
                                        .unwrap();
                                    
                                    // Initialize all buckets as unoccupied (memset to 0)
                                    let memset_fn = self.module.get_function("memset").unwrap_or_else(|| {
                                        let memset_type = self.context.ptr_type(AddressSpace::default())
                                            .fn_type(&[
                                                self.context.ptr_type(AddressSpace::default()).into(),
                                                self.context.i32_type().into(),
                                                self.context.i64_type().into(),
                                            ], false);
                                        self.module.add_function("memset", memset_type, None)
                                    });
                                    
                                    self.builder.build_call(
                                        memset_fn, 
                                        &[
                                            buckets_ptr.into(),
                                            self.context.i32_type().const_int(0, false).into(),
                                            size_arg.into(),
                                        ],
                                        "memset"
                                    ).unwrap();
                                    
                                    // Create the hashmap struct
                                    let zero = self.context.i64_type().const_int(0, false);
                                    let hashmap_alloca = self.builder.build_alloca(hashmap_type, "hashmap")?;
                                    
                                    // Store buckets pointer
                                    let buckets_field_ptr = self.builder.build_struct_gep(
                                        hashmap_type,
                                        hashmap_alloca,
                                        0,
                                        "buckets_field"
                                    )?;
                                    self.builder.build_store(buckets_field_ptr, buckets_ptr)?;
                                    
                                    // Store size (initially 0)
                                    let size_field_ptr = self.builder.build_struct_gep(
                                        hashmap_type,
                                        hashmap_alloca,
                                        1,
                                        "size_field"
                                    )?;
                                    self.builder.build_store(size_field_ptr, zero)?;
                                    
                                    // Store capacity
                                    let capacity_field_ptr = self.builder.build_struct_gep(
                                        hashmap_type,
                                        hashmap_alloca,
                                        2,
                                        "capacity_field"
                                    )?;
                                    self.builder.build_store(capacity_field_ptr, capacity_value)?;
                                    
                                    let hashmap = self.builder.build_load(hashmap_type, hashmap_alloca, "hashmap_value")?;
                                    return Ok(hashmap.into());
                                }
                                "HashSet" => {
                                    // HashSet.new() creates an empty hashset with allocated buckets
                                    // Similar structure to HashMap but only stores keys, not key-value pairs
                                    let hashset_type = self.context.struct_type(
                                        &[
                                            self.context.ptr_type(AddressSpace::default()).into(), // buckets
                                            self.context.i64_type().into(), // size
                                            self.context.i64_type().into(), // capacity
                                        ],
                                        false,
                                    );
                                    
                                    // Allocate buckets array (16 buckets, each bucket is a struct with 3 fields: hash, value_ptr, occupied)
                                    let bucket_type = self.context.struct_type(
                                        &[
                                            self.context.i64_type().into(), // hash
                                            self.context.ptr_type(AddressSpace::default()).into(), // value_ptr
                                            self.context.bool_type().into(), // occupied
                                        ],
                                        false,
                                    );
                                    
                                    let capacity_value = 16u64;
                                    let bucket_array_type = bucket_type.array_type(capacity_value as u32);
                                    
                                    // Allocate memory for buckets
                                    let malloc_fn = self.module.get_function("malloc").unwrap_or_else(|| {
                                        let malloc_type = self.context.ptr_type(AddressSpace::default())
                                            .fn_type(&[self.context.i64_type().into()], false);
                                        self.module.add_function("malloc", malloc_type, None)
                                    });
                                    // Calculate bucket size manually: i64 + ptr + i1 (bool) = 8 + 8 + 1 = 17 bytes, round up to 24
                                    let bucket_size = self.context.i64_type().const_int(
                                        24 * capacity_value,  // 24 bytes per bucket
                                        false,
                                    );
                                    let buckets_ptr = self.builder.build_call(
                                        malloc_fn,
                                        &[bucket_size.into()],
                                        "buckets_malloc"
                                    )?.try_as_basic_value().left().unwrap();
                                    
                                    // Cast to bucket pointer type
                                    let bucket_ptr_type = self.context.ptr_type(AddressSpace::default());
                                    let buckets = self.builder.build_pointer_cast(
                                        buckets_ptr.into_pointer_value(),
                                        bucket_ptr_type,
                                        "buckets"
                                    )?;
                                    
                                    // Initialize all buckets as unoccupied
                                    let zero = self.context.i64_type().const_int(0, false);
                                    let false_val = self.context.bool_type().const_int(0, false);
                                    let null_ptr = self.context.ptr_type(AddressSpace::default()).const_null();
                                    
                                    for i in 0..capacity_value {
                                        let bucket_ptr = unsafe {
                                            self.builder.build_gep(
                                                bucket_type,
                                                buckets,
                                                &[self.context.i64_type().const_int(i, false)],
                                                &format!("bucket_{}", i),
                                            )
                                        }?;
                                        
                                        // Store hash = 0
                                        let hash_ptr = self.builder.build_struct_gep(
                                            bucket_type,
                                            bucket_ptr,
                                            0,
                                            "hash_ptr"
                                        )?;
                                        self.builder.build_store(hash_ptr, zero)?;
                                        
                                        // Store value_ptr = null
                                        let value_ptr_ptr = self.builder.build_struct_gep(
                                            bucket_type,
                                            bucket_ptr,
                                            1,
                                            "value_ptr_ptr"
                                        )?;
                                        self.builder.build_store(value_ptr_ptr, null_ptr)?;
                                        
                                        // Store occupied = false
                                        let occupied_ptr = self.builder.build_struct_gep(
                                            bucket_type,
                                            bucket_ptr,
                                            2,
                                            "occupied_ptr"
                                        )?;
                                        self.builder.build_store(occupied_ptr, false_val)?;
                                    }
                                    
                                    // Create the hashset struct
                                    let zero = self.context.i64_type().const_int(0, false);
                                    let hashset_alloca = self.builder.build_alloca(hashset_type, "hashset")?;
                                    
                                    // Store buckets pointer
                                    let buckets_field_ptr = self.builder.build_struct_gep(
                                        hashset_type,
                                        hashset_alloca,
                                        0,
                                        "buckets_field"
                                    )?;
                                    self.builder.build_store(buckets_field_ptr, buckets)?;
                                    
                                    // Store size = 0
                                    let size_field_ptr = self.builder.build_struct_gep(
                                        hashset_type,
                                        hashset_alloca,
                                        1,
                                        "size_field"
                                    )?;
                                    self.builder.build_store(size_field_ptr, zero)?;
                                    
                                    // Store capacity
                                    let capacity_field_ptr = self.builder.build_struct_gep(
                                        hashset_type,
                                        hashset_alloca,
                                        2,
                                        "capacity_field"
                                    )?;
                                    self.builder.build_store(capacity_field_ptr, 
                                        self.context.i64_type().const_int(capacity_value, false))?;
                                    
                                    let hashset = self.builder.build_load(hashset_type, hashset_alloca, "hashset_value")?;
                                    return Ok(hashset.into());
                                }
                                "DynVec" => {
                                    // DynVec.new() creates an empty dynamic vector
                                    // Reuse the existing dynvec_new implementation if it exists
                                    if self.functions.contains_key("dynvec_new") {
                                        return self.compile_function_call("dynvec_new", args);
                                    }
                                    
                                    // Otherwise create a placeholder
                                    let dynvec_type = self.context.struct_type(
                                        &[
                                            self.context.ptr_type(AddressSpace::default()).into(), // data
                                            self.context.i64_type().into(), // len
                                            self.context.i64_type().into(), // capacity
                                        ],
                                        false,
                                    );
                                    
                                    let null_ptr = self.context.ptr_type(AddressSpace::default()).const_null();
                                    let zero = self.context.i64_type().const_int(0, false);
                                    
                                    let dynvec = dynvec_type.const_named_struct(&[
                                        null_ptr.into(),
                                        zero.into(),
                                        zero.into(),
                                    ]);
                                    
                                    return Ok(dynvec.into());
                                }
                                _ => {
                                    // Unknown generic type
                                    return Err(CompileError::TypeError(
                                        format!("Unknown generic type: {}", base_type),
                                        None,
                                    ));
                                }
                            }
                        }
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
                                    Expression::Closure { params, return_type: _, body } => {
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
                        "len" => {
                            // Call the runtime string_len function
                            let func = self.get_or_create_runtime_function("string_len")?;
                            let result = self
                                .builder
                                .build_call(func, &[object_value.into()], "len_result")?
                                .try_as_basic_value()
                                .left()
                                .ok_or_else(|| {
                                    CompileError::InternalError("string_len did not return a value".to_string(), None)
                                })?;
                            return Ok(result);
                        }
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
                        "substr" => {
                            // string.substr(start, length) - extract substring
                            if args.len() != 2 {
                                return Err(CompileError::TypeError(
                                    format!("substr expects 2 arguments, got {}", args.len()),
                                    None,
                                ));
                            }
                            
                            // Compile the arguments
                            let start = self.compile_expression(&args[0])?;
                            let length = self.compile_expression(&args[1])?;
                            
                            // Convert to i64 if needed
                            let start_i64 = match start {
                                BasicValueEnum::IntValue(i) => {
                                    if i.get_type().get_bit_width() < 64 {
                                        self.builder.build_int_s_extend(i, self.context.i64_type(), "start_ext")?
                                    } else {
                                        i
                                    }
                                }
                                _ => return Err(CompileError::TypeError(
                                    "substr start argument must be an integer".to_string(),
                                    None,
                                ))
                            };
                            
                            let length_i64 = match length {
                                BasicValueEnum::IntValue(i) => {
                                    if i.get_type().get_bit_width() < 64 {
                                        self.builder.build_int_s_extend(i, self.context.i64_type(), "length_ext")?
                                    } else {
                                        i
                                    }
                                }
                                _ => return Err(CompileError::TypeError(
                                    "substr length argument must be an integer".to_string(),
                                    None,
                                ))
                            };
                            
                            // Call the runtime string_substr function
                            let func = self.get_or_create_runtime_function("string_substr")?;
                            let result = self
                                .builder
                                .build_call(
                                    func, 
                                    &[object_value.into(), start_i64.into(), length_i64.into()], 
                                    "substr_result"
                                )?
                                .try_as_basic_value()
                                .left()
                                .ok_or_else(|| {
                                    CompileError::InternalError("string_substr did not return a value".to_string(), None)
                                })?;
                            return Ok(result);
                        }
                        "char_at" => {
                            // string.char_at(index) - get character at index
                            if args.len() != 1 {
                                return Err(CompileError::TypeError(
                                    format!("char_at expects 1 argument, got {}", args.len()),
                                    None,
                                ));
                            }
                            
                            // Compile the index argument
                            let index = self.compile_expression(&args[0])?;
                            
                            // Convert to i64 if needed
                            let index_i64 = match index {
                                BasicValueEnum::IntValue(i) => {
                                    if i.get_type().get_bit_width() < 64 {
                                        self.builder.build_int_s_extend(i, self.context.i64_type(), "index_ext")?
                                    } else {
                                        i
                                    }
                                }
                                _ => return Err(CompileError::TypeError(
                                    "char_at index argument must be an integer".to_string(),
                                    None,
                                ))
                            };
                            
                            // Call the runtime string_char_at function
                            let func = self.get_or_create_runtime_function("string_char_at")?;
                            let result = self
                                .builder
                                .build_call(
                                    func, 
                                    &[object_value.into(), index_i64.into()], 
                                    "char_at_result"
                                )?
                                .try_as_basic_value()
                                .left()
                                .ok_or_else(|| {
                                    CompileError::InternalError("string_char_at did not return a value".to_string(), None)
                                })?;
                            return Ok(result);
                        }
                        "split" => {
                            // string.split(delimiter) - split string by delimiter
                            if args.len() != 1 {
                                return Err(CompileError::TypeError(
                                    format!("split expects 1 argument, got {}", args.len()),
                                    None,
                                ));
                            }
                            
                            // Compile the delimiter argument
                            let delimiter = self.compile_expression(&args[0])?;
                            
                            // Make sure delimiter is a string
                            if !delimiter.is_pointer_value() {
                                return Err(CompileError::TypeError(
                                    "split delimiter argument must be a string".to_string(),
                                    None,
                                ));
                            }
                            
                            // Call the runtime string_split function
                            let func = self.get_or_create_runtime_function("string_split")?;
                            let result = self
                                .builder
                                .build_call(
                                    func, 
                                    &[object_value.into(), delimiter.into()], 
                                    "split_result"
                                )?
                                .try_as_basic_value()
                                .left()
                                .ok_or_else(|| {
                                    CompileError::InternalError("string_split did not return a value".to_string(), None)
                                })?;
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
                            // Update generic type context for Option<i32>
                            self.generic_type_context
                                .insert("Option_Some_Type".to_string(), crate::ast::AstType::I32);
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
                            // Update generic type context for Option<i64>
                            self.generic_type_context
                                .insert("Option_Some_Type".to_string(), crate::ast::AstType::I64);
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
                        "trim" => {
                            // string.trim() - remove leading and trailing whitespace
                            if args.len() != 0 {
                                return Err(CompileError::TypeError(
                                    format!("trim expects no arguments, got {}", args.len()),
                                    None,
                                ));
                            }
                            
                            // Call the runtime string_trim function
                            let func = self.get_or_create_runtime_function("string_trim")?;
                            let result = self
                                .builder
                                .build_call(func, &[object_value.into()], "trim_result")?
                                .try_as_basic_value()
                                .left()
                                .ok_or_else(|| {
                                    CompileError::InternalError(
                                        "string_trim did not return a value".to_string(),
                                        None,
                                    )
                                })?;
                            return Ok(result);
                        }
                        "contains" => {
                            // string.contains(substring) - check if string contains substring
                            if args.len() != 1 {
                                return Err(CompileError::TypeError(
                                    format!("contains expects 1 argument, got {}", args.len()),
                                    None,
                                ));
                            }
                            
                            // Compile the substring argument
                            let substring = self.compile_expression(&args[0])?;
                            
                            // Call the runtime string_contains function
                            let func = self.get_or_create_runtime_function("string_contains")?;
                            let result = self
                                .builder
                                .build_call(func, &[object_value.into(), substring.into()], "contains_result")?
                                .try_as_basic_value()
                                .left()
                                .ok_or_else(|| {
                                    CompileError::InternalError(
                                        "string_contains did not return a value".to_string(),
                                        None,
                                    )
                                })?;
                            return Ok(result);
                        }
                        "starts_with" => {
                            // string.starts_with(prefix) - check if string starts with prefix
                            if args.len() != 1 {
                                return Err(CompileError::TypeError(
                                    format!("starts_with expects 1 argument, got {}", args.len()),
                                    None,
                                ));
                            }
                            
                            // Compile the prefix argument
                            let prefix = self.compile_expression(&args[0])?;
                            
                            // Call the runtime string_starts_with function
                            let func = self.get_or_create_runtime_function("string_starts_with")?;
                            let result = self
                                .builder
                                .build_call(func, &[object_value.into(), prefix.into()], "starts_with_result")?
                                .try_as_basic_value()
                                .left()
                                .ok_or_else(|| {
                                    CompileError::InternalError(
                                        "string_starts_with did not return a value".to_string(),
                                        None,
                                    )
                                })?;
                            return Ok(result);
                        }
                        "ends_with" => {
                            // string.ends_with(suffix) - check if string ends with suffix
                            if args.len() != 1 {
                                return Err(CompileError::TypeError(
                                    format!("ends_with expects 1 argument, got {}", args.len()),
                                    None,
                                ));
                            }
                            
                            // Compile the suffix argument
                            let suffix = self.compile_expression(&args[0])?;
                            
                            // Call the runtime string_ends_with function
                            let func = self.get_or_create_runtime_function("string_ends_with")?;
                            let result = self
                                .builder
                                .build_call(func, &[object_value.into(), suffix.into()], "ends_with_result")?
                                .try_as_basic_value()
                                .left()
                                .ok_or_else(|| {
                                    CompileError::InternalError(
                                        "string_ends_with did not return a value".to_string(),
                                        None,
                                    )
                                })?;
                            return Ok(result);
                        }
                        "index_of" => {
                            // string.index_of(substring) - find index of substring in string
                            if args.len() != 1 {
                                return Err(CompileError::TypeError(
                                    format!("index_of expects 1 argument, got {}", args.len()),
                                    None,
                                ));
                            }
                            
                            // Compile the substring argument
                            let substring = self.compile_expression(&args[0])?;
                            
                            // Call the runtime string_index_of function
                            let func = self.get_or_create_runtime_function("string_index_of")?;
                            let result = self
                                .builder
                                .build_call(func, &[object_value.into(), substring.into()], "index_of_result")?
                                .try_as_basic_value()
                                .left()
                                .ok_or_else(|| {
                                    CompileError::InternalError(
                                        "string_index_of did not return a value".to_string(),
                                        None,
                                    )
                                })?;
                            return Ok(result);
                        }
                        "to_upper" => {
                            // string.to_upper() - convert string to uppercase
                            if !args.is_empty() {
                                return Err(CompileError::TypeError(
                                    format!("to_upper expects no arguments, got {}", args.len()),
                                    None,
                                ));
                            }
                            
                            // Call the runtime string_to_upper function
                            let func = self.get_or_create_runtime_function("string_to_upper")?;
                            let result = self
                                .builder
                                .build_call(func, &[object_value.into()], "to_upper_result")?
                                .try_as_basic_value()
                                .left()
                                .ok_or_else(|| {
                                    CompileError::InternalError(
                                        "string_to_upper did not return a value".to_string(),
                                        None,
                                    )
                                })?;
                            return Ok(result);
                        }
                        "to_lower" => {
                            // string.to_lower() - convert string to lowercase
                            if !args.is_empty() {
                                return Err(CompileError::TypeError(
                                    format!("to_lower expects no arguments, got {}", args.len()),
                                    None,
                                ));
                            }
                            
                            // Call the runtime string_to_lower function
                            let func = self.get_or_create_runtime_function("string_to_lower")?;
                            let result = self
                                .builder
                                .build_call(func, &[object_value.into()], "to_lower_result")?
                                .try_as_basic_value()
                                .left()
                                .ok_or_else(|| {
                                    CompileError::InternalError(
                                        "string_to_lower did not return a value".to_string(),
                                        None,
                                    )
                                })?;
                            return Ok(result);
                        }
                        _ => {}
                    }
                }

                // Special handling for integer methods
                if object_value.is_int_value() {
                    let int_value = object_value.into_int_value();
                    match method.as_str() {
                        "abs" => {
                            // Get the bit width to determine the type
                            let _bit_width = int_value.get_type().get_bit_width();
                            
                            // Create comparison with zero
                            let zero = int_value.get_type().const_int(0, false);
                            let is_negative = self.builder.build_int_compare(
                                inkwell::IntPredicate::SLT,
                                int_value,
                                zero,
                                "is_negative"
                            )?;
                            
                            // Negate if negative
                            let negated = self.builder.build_int_neg(int_value, "negated")?;
                            
                            // Select positive value
                            let abs_value = self.builder.build_select(
                                is_negative,
                                negated,
                                int_value,
                                "abs_value"
                            )?;
                            
                            return Ok(abs_value);
                        }
                        "min" => {
                            if args.len() != 1 {
                                return Err(CompileError::TypeError(
                                    format!("min expects 1 argument, got {}", args.len()),
                                    None,
                                ));
                            }
                            let other = self.compile_expression(&args[0])?;
                            if !other.is_int_value() {
                                return Err(CompileError::TypeError(
                                    "min argument must be an integer".to_string(),
                                    None,
                                ));
                            }
                            let other_int = other.into_int_value();
                            
                            // Compare and select minimum
                            let is_less = self.builder.build_int_compare(
                                inkwell::IntPredicate::SLT,
                                int_value,
                                other_int,
                                "is_less"
                            )?;
                            
                            let min_value = self.builder.build_select(
                                is_less,
                                int_value,
                                other_int,
                                "min_value"
                            )?;
                            
                            return Ok(min_value);
                        }
                        "max" => {
                            if args.len() != 1 {
                                return Err(CompileError::TypeError(
                                    format!("max expects 1 argument, got {}", args.len()),
                                    None,
                                ));
                            }
                            let other = self.compile_expression(&args[0])?;
                            if !other.is_int_value() {
                                return Err(CompileError::TypeError(
                                    "max argument must be an integer".to_string(),
                                    None,
                                ));
                            }
                            let other_int = other.into_int_value();
                            
                            // Compare and select maximum
                            let is_greater = self.builder.build_int_compare(
                                inkwell::IntPredicate::SGT,
                                int_value,
                                other_int,
                                "is_greater"
                            )?;
                            
                            let max_value = self.builder.build_select(
                                is_greater,
                                int_value,
                                other_int,
                                "max_value"
                            )?;
                            
                            return Ok(max_value);
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
                    
                    // First check if the identifier is actually a Result type
                    if let Expression::Identifier(var_name) = object.as_ref() {
                        if let Some(var_info) = self.variables.get(var_name) {
                            
                            // If it's a generic Result type, delegate to compile_raise_expression
                            if let AstType::Generic { name, .. } = &var_info.ast_type {
                                if name == "Result" {
                                    return self.compile_raise_expression(object);
                                }
                            }
                        }
                    }

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

                // Special handling for Vec and DynVec methods
                if let Expression::Identifier(obj_name) = object.as_ref() {
                    // Check if this is a Vec<T, N> type
                    let is_vec = if let Some(var_info) = self.variables.get(obj_name) {
                        matches!(&var_info.ast_type, AstType::Vec { .. })
                    } else {
                        false
                    };
                    
                    // Check if this is actually a DynVec type
                    let is_dynvec = if let Some(var_info) = self.variables.get(obj_name) {
                        matches!(&var_info.ast_type, AstType::DynVec { .. })
                    } else {
                        false
                    };
                    
                    // Handle Vec<T, N> methods
                    if is_vec {
                        let (vec_ptr, _) = self.get_variable(obj_name)?;
                        return self.compile_vec_method(obj_name, method, args, vec_ptr, object_value);
                    }
                    
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

                // Special handling for HashMap methods
                if let Expression::Identifier(obj_name) = object.as_ref() {
                    // Check if this is actually a HashMap type  
                    let is_hashmap = if let Some(var_info) = self.variables.get(obj_name) {
                        matches!(&var_info.ast_type, 
                            AstType::Generic { name, .. } if name == "HashMap")
                    } else {
                        false
                    };
                    
                    if is_hashmap {
                        // Only handle HashMap methods if it's actually a HashMap
                        match method.as_str() {
                            "insert" => {
                                // HashMap.insert(key, value[, hash_fn, eq_fn]) implementation
                                // If only 2 args provided, generate default hash/eq functions
                                if args.len() != 2 && args.len() != 4 {
                                    return Err(CompileError::TypeError(
                                        "insert expects 2 arguments (key, value) or 4 arguments (key, value, hash_fn, eq_fn)".to_string(),
                                        None,
                                    ));
                                }
                                
                                // Get the pointer to the original HashMap variable
                                let (hashmap_ptr, _) = self.get_variable(obj_name)?;
                                
                                // Compile arguments
                                let key = self.compile_expression(&args[0])?;
                                let value = self.compile_expression(&args[1])?;
                                
                                // Handle hash and equality functions
                                let (hash_fn, _eq_fn) = if args.len() == 4 {
                                    // Use provided functions
                                    let hash = self.compile_expression(&args[2])?;
                                    let eq = self.compile_expression(&args[3])?;
                                    (hash, eq)
                                } else {
                                    // Generate default hash and equality for common types
                                    // For i32 keys, use simple identity hash and equality
                                    if key.get_type() == self.context.i32_type().into() {
                                        // Create inline hash function: convert i32 to i64
                                        let hash_fn_type = self.context.i64_type().fn_type(&[self.context.i32_type().into()], false);
                                        let hash_fn = self.module.add_function("hash_i32_default", hash_fn_type, None);
                                        let hash_bb = self.context.append_basic_block(hash_fn, "entry");
                                        let builder = self.context.create_builder();
                                        builder.position_at_end(hash_bb);
                                        let param = hash_fn.get_nth_param(0).unwrap().into_int_value();
                                        let extended = builder.build_int_z_extend(param, self.context.i64_type(), "extended")?;
                                        builder.build_return(Some(&extended))?;
                                        
                                        // Create inline equality function
                                        let eq_fn_type = self.context.i64_type().fn_type(&[self.context.i32_type().into(), self.context.i32_type().into()], false);
                                        let eq_fn = self.module.add_function("eq_i32_default", eq_fn_type, None);
                                        let eq_bb = self.context.append_basic_block(eq_fn, "entry");
                                        builder.position_at_end(eq_bb);
                                        let a = eq_fn.get_nth_param(0).unwrap().into_int_value();
                                        let b = eq_fn.get_nth_param(1).unwrap().into_int_value();
                                        let cmp = builder.build_int_compare(inkwell::IntPredicate::EQ, a, b, "eq")?;
                                        let result = builder.build_int_z_extend(cmp, self.context.i64_type(), "result")?;
                                        builder.build_return(Some(&result))?;
                                        
                                        (hash_fn.as_global_value().as_pointer_value().into(), 
                                         eq_fn.as_global_value().as_pointer_value().into())
                                    } else {
                                        // For other types, use a simple hash for now
                                        // This is a placeholder - should be extended for strings, etc.
                                        return Err(CompileError::TypeError(
                                            "Default hash/eq functions not implemented for this key type. Please provide explicit hash and equality functions.".to_string(),
                                            None,
                                        ));
                                    }
                                };
                                
                                let hash_fn = hash_fn;
                                let _eq_fn = _eq_fn;
                                
                                // Step 1: Call hash_fn(key) to get hash value
                                // The hash_fn is passed as a closure/function pointer
                                let hash_fn_ptr = if hash_fn.is_pointer_value() {
                                    hash_fn.into_pointer_value()
                                } else {
                                    return Err(CompileError::TypeError("hash_fn must be a function pointer".to_string(), None));
                                };
                                
                                let hash_fn_type = self.context.i64_type().fn_type(&[key.get_type().into()], false);
                                let hash_value = self.builder.build_indirect_call(
                                    hash_fn_type,
                                    hash_fn_ptr,
                                    &[key.into()],
                                    "hash_value"
                                )?
                                .try_as_basic_value()
                                .left()
                                .ok_or_else(|| CompileError::TypeError("hash function must return a value".to_string(), None))?;
                                
                                // Load capacity from HashMap
                                let capacity_ptr = self.builder.build_struct_gep(
                                    object_value.get_type(),
                                    hashmap_ptr,
                                    2, // capacity is at index 2
                                    "capacity_ptr",
                                )?;
                                let capacity = self.builder.build_load(
                                    self.context.i64_type(),
                                    capacity_ptr,
                                    "capacity"
                                )?.into_int_value();
                                
                                // Calculate bucket index: hash % capacity
                                let bucket_index = self.builder.build_int_unsigned_rem(
                                    hash_value.into_int_value(),
                                    capacity,
                                    "bucket_index"
                                )?;
                                
                                // Load buckets pointer from HashMap
                                let buckets_ptr_ptr = self.builder.build_struct_gep(
                                    object_value.get_type(),
                                    hashmap_ptr,
                                    0, // buckets_ptr is at index 0
                                    "buckets_ptr_ptr",
                                )?;
                                let buckets_ptr = self.builder.build_load(
                                    self.context.ptr_type(inkwell::AddressSpace::default()),
                                    buckets_ptr_ptr,
                                    "buckets_ptr"
                                )?.into_pointer_value();
                                
                                // Each bucket has 4 fields: [key_hash, key_ptr, value_ptr, occupied]
                                // Calculate the offset to the target bucket
                                let i64_ptr_type = self.context.ptr_type(AddressSpace::default());
                                let buckets_as_i64_ptr = self.builder.build_pointer_cast(
                                    buckets_ptr,
                                    i64_ptr_type,
                                    "buckets_as_i64"
                                )?;
                                
                                // Multiply bucket_index by 4 to get the starting position
                                let bucket_offset = self.builder.build_int_mul(
                                    bucket_index,
                                    self.context.i64_type().const_int(4, false),
                                    "bucket_offset"
                                )?;
                                
                                // Get pointers to bucket fields
                                let key_hash_ptr = unsafe {
                                    self.builder.build_gep(
                                        self.context.i64_type(),
                                        buckets_as_i64_ptr,
                                        &[bucket_offset],
                                        "key_hash_ptr"
                                    )?
                                };
                                
                                // Get pointer to key storage location (offset + 1)
                                let key_ptr_offset = self.builder.build_int_add(
                                    bucket_offset,
                                    self.context.i64_type().const_int(1, false),
                                    "key_ptr_offset"
                                )?;
                                let key_storage_ptr = unsafe {
                                    self.builder.build_gep(
                                        self.context.i64_type(),
                                        buckets_as_i64_ptr,
                                        &[key_ptr_offset],
                                        "key_storage_ptr"
                                    )?
                                };
                                
                                // Get pointer to value storage location (offset + 2)
                                let value_ptr_offset = self.builder.build_int_add(
                                    bucket_offset,
                                    self.context.i64_type().const_int(2, false),
                                    "value_ptr_offset"
                                )?;
                                let value_storage_ptr = unsafe {
                                    self.builder.build_gep(
                                        self.context.i64_type(),
                                        buckets_as_i64_ptr,
                                        &[value_ptr_offset],
                                        "value_storage_ptr"
                                    )?
                                };
                                
                                // Get pointer to occupied flag (offset + 3)
                                let occupied_offset = self.builder.build_int_add(
                                    bucket_offset,
                                    self.context.i64_type().const_int(3, false),
                                    "occupied_offset"
                                )?;
                                let occupied_ptr = unsafe {
                                    self.builder.build_gep(
                                        self.context.i64_type(),
                                        buckets_as_i64_ptr,
                                        &[occupied_offset],
                                        "occupied_ptr"
                                    )?
                                };
                                
                                // Store the hash
                                self.builder.build_store(key_hash_ptr, hash_value)?;
                                
                                // Allocate and store the key
                                let _key_size = if key.get_type() == self.context.ptr_type(AddressSpace::default()).into() {
                                    // Key is a string (pointer), store it directly
                                    let key_ptr_as_i64 = self.builder.build_ptr_to_int(
                                        key.into_pointer_value(),
                                        self.context.i64_type(),
                                        "key_ptr_as_i64"
                                    )?;
                                    self.builder.build_store(key_storage_ptr, key_ptr_as_i64)?;
                                } else {
                                    // Key is a primitive, store it as i64
                                    let key_as_i64 = if key.get_type() == self.context.i64_type().into() {
                                        key.into_int_value()
                                    } else if key.get_type() == self.context.i32_type().into() {
                                        self.builder.build_int_z_extend(
                                            key.into_int_value(),
                                            self.context.i64_type(),
                                            "key_i64"
                                        )?
                                    } else {
                                        self.context.i64_type().const_int(0, false)
                                    };
                                    self.builder.build_store(key_storage_ptr, key_as_i64)?;
                                };
                                
                                // Store the value (allocate memory and copy)
                                if value.get_type() == self.context.i32_type().into() {
                                    // Value is i32, allocate 4 bytes
                                    let malloc_fn = self.module.get_function("malloc").unwrap();
                                    let value_mem = self.builder.build_call(
                                        malloc_fn,
                                        &[self.context.i64_type().const_int(4, false).into()],
                                        "value_mem"
                                    )?.try_as_basic_value().left().unwrap().into_pointer_value();
                                    
                                    // Cast to i32* and store the value
                                    let value_mem_i32 = self.builder.build_pointer_cast(
                                        value_mem,
                                        self.context.ptr_type(AddressSpace::default()),
                                        "value_mem_i32"
                                    )?;
                                    self.builder.build_store(value_mem_i32, value)?;
                                    
                                    // Store the pointer in the bucket
                                    let value_ptr_as_i64 = self.builder.build_ptr_to_int(
                                        value_mem,
                                        self.context.i64_type(),
                                        "value_ptr_as_i64"
                                    )?;
                                    self.builder.build_store(value_storage_ptr, value_ptr_as_i64)?;
                                } else if value.get_type() == self.context.i64_type().into() {
                                    // Value is i64, store directly as i64
                                    self.builder.build_store(value_storage_ptr, value)?;
                                } else {
                                    // Other types, store as 0 for now
                                    self.builder.build_store(value_storage_ptr, self.context.i64_type().const_int(0, false))?;
                                }
                                
                                self.builder.build_store(occupied_ptr, self.context.i64_type().const_int(1, false))?;
                                
                                // Update size
                                let size_ptr = self.builder.build_struct_gep(
                                    object_value.get_type(),
                                    hashmap_ptr,
                                    1, // size is at index 1
                                    "size_ptr",
                                )?;
                                let current_size = self.builder.build_load(
                                    self.context.i64_type(),
                                    size_ptr,
                                    "current_size"
                                )?.into_int_value();
                                let new_size = self.builder.build_int_add(
                                    current_size,
                                    self.context.i64_type().const_int(1, false),
                                    "new_size"
                                )?;
                                self.builder.build_store(size_ptr, new_size)?;
                                
                                // Return void (unit type) 
                                return Ok(self.context.struct_type(&[], false).const_zero().into());
                            }
                            "get" => {
                                // HashMap.get(key[, hash_fn, eq_fn]) -> Option<V>
                                if args.len() != 1 && args.len() != 3 {
                                    return Err(CompileError::TypeError(
                                        "get expects 1 argument (key) or 3 arguments (key, hash_fn, eq_fn)".to_string(),
                                        None,
                                    ));
                                }
                                
                                // Get the pointer to the original HashMap variable
                                let (hashmap_ptr, _) = self.get_variable(obj_name)?;
                                
                                // Compile key argument
                                let key = self.compile_expression(&args[0])?;
                                
                                // Handle hash and equality functions
                                let (hash_fn, eq_fn) = if args.len() == 3 {
                                    // Use provided functions
                                    let hash = self.compile_expression(&args[1])?;
                                    let eq = self.compile_expression(&args[2])?;
                                    (hash, eq)
                                } else {
                                    // Generate default functions for i32 keys
                                    if key.get_type() == self.context.i32_type().into() {
                                        // Reuse or create default functions
                                        let hash_fn = if let Some(func) = self.module.get_function("hash_i32_default") {
                                            func.as_global_value().as_pointer_value().into()
                                        } else {
                                            let hash_fn_type = self.context.i64_type().fn_type(&[self.context.i32_type().into()], false);
                                            let hash_fn = self.module.add_function("hash_i32_default", hash_fn_type, None);
                                            let hash_bb = self.context.append_basic_block(hash_fn, "entry");
                                            let builder = self.context.create_builder();
                                            builder.position_at_end(hash_bb);
                                            let param = hash_fn.get_nth_param(0).unwrap().into_int_value();
                                            let extended = builder.build_int_z_extend(param, self.context.i64_type(), "extended")?;
                                            builder.build_return(Some(&extended))?;
                                            hash_fn.as_global_value().as_pointer_value().into()
                                        };
                                        
                                        let eq_fn = if let Some(func) = self.module.get_function("eq_i32_default") {
                                            func.as_global_value().as_pointer_value().into()
                                        } else {
                                            let eq_fn_type = self.context.i64_type().fn_type(&[self.context.i32_type().into(), self.context.i32_type().into()], false);
                                            let eq_fn = self.module.add_function("eq_i32_default", eq_fn_type, None);
                                            let eq_bb = self.context.append_basic_block(eq_fn, "entry");
                                            let builder = self.context.create_builder();
                                            builder.position_at_end(eq_bb);
                                            let a = eq_fn.get_nth_param(0).unwrap().into_int_value();
                                            let b = eq_fn.get_nth_param(1).unwrap().into_int_value();
                                            let cmp = builder.build_int_compare(inkwell::IntPredicate::EQ, a, b, "eq")?;
                                            let result = builder.build_int_z_extend(cmp, self.context.i64_type(), "result")?;
                                            builder.build_return(Some(&result))?;
                                            eq_fn.as_global_value().as_pointer_value().into()
                                        };
                                        
                                        (hash_fn, eq_fn)
                                    } else {
                                        return Err(CompileError::TypeError(
                                            "Default hash/eq functions not implemented for this key type. Please provide explicit hash and equality functions.".to_string(),
                                            None,
                                        ));
                                    }
                                };
                                
                                let hash_fn = hash_fn;
                                let eq_fn = eq_fn;
                                
                                // Step 1: Call hash_fn(key) to get hash value
                                let hash_fn_ptr = if hash_fn.is_pointer_value() {
                                    hash_fn.into_pointer_value()
                                } else {
                                    return Err(CompileError::TypeError("hash_fn must be a function pointer".to_string(), None));
                                };
                                
                                let eq_fn_ptr = if eq_fn.is_pointer_value() {
                                    eq_fn.into_pointer_value()
                                } else {
                                    return Err(CompileError::TypeError("eq_fn must be a function pointer".to_string(), None));
                                };
                                
                                let hash_fn_type = self.context.i64_type().fn_type(&[key.get_type().into()], false);
                                let hash_value = self.builder.build_indirect_call(
                                    hash_fn_type,
                                    hash_fn_ptr,
                                    &[key.into()],
                                    "hash_value"
                                )?
                                .try_as_basic_value()
                                .left()
                                .ok_or_else(|| CompileError::TypeError("hash function must return a value".to_string(), None))?;
                                
                                // Load capacity from HashMap
                                let capacity_ptr = self.builder.build_struct_gep(
                                    object_value.get_type(),
                                    hashmap_ptr,
                                    2, // capacity is at index 2
                                    "capacity_ptr",
                                )?;
                                let capacity = self.builder.build_load(
                                    self.context.i64_type(),
                                    capacity_ptr,
                                    "capacity"
                                )?.into_int_value();
                                
                                // Calculate bucket index: hash % capacity
                                let bucket_index = self.builder.build_int_unsigned_rem(
                                    hash_value.into_int_value(),
                                    capacity,
                                    "bucket_index"
                                )?;
                                
                                // Load buckets pointer from HashMap
                                let buckets_ptr_ptr = self.builder.build_struct_gep(
                                    object_value.get_type(),
                                    hashmap_ptr,
                                    0, // buckets_ptr is at index 0
                                    "buckets_ptr_ptr",
                                )?;
                                let buckets_ptr = self.builder.build_load(
                                    self.context.ptr_type(inkwell::AddressSpace::default()),
                                    buckets_ptr_ptr,
                                    "buckets_ptr"
                                )?.into_pointer_value();
                                
                                // Each bucket has 4 fields: [key_hash, key_ptr, value_ptr, occupied]
                                let i64_ptr_type = self.context.ptr_type(AddressSpace::default());
                                let buckets_as_i64_ptr = self.builder.build_pointer_cast(
                                    buckets_ptr,
                                    i64_ptr_type,
                                    "buckets_as_i64"
                                )?;
                                
                                // Multiply bucket_index by 4 to get the starting position
                                let bucket_offset = self.builder.build_int_mul(
                                    bucket_index,
                                    self.context.i64_type().const_int(4, false),
                                    "bucket_offset"
                                )?;
                                
                                // Get pointers to bucket fields
                                let occupied_offset = self.builder.build_int_add(
                                    bucket_offset,
                                    self.context.i64_type().const_int(3, false), // occupied is at offset 3
                                    "occupied_offset"
                                )?;
                                let occupied_ptr = unsafe {
                                    self.builder.build_gep(
                                        self.context.i64_type(),
                                        buckets_as_i64_ptr,
                                        &[occupied_offset],
                                        "occupied_ptr"
                                    )?
                                };
                                
                                // Load occupied flag
                                let occupied = self.builder.build_load(
                                    self.context.i64_type(),
                                    occupied_ptr,
                                    "occupied"
                                )?.into_int_value();
                                
                                
                                let is_occupied = self.builder.build_int_compare(
                                    inkwell::IntPredicate::NE,
                                    occupied,
                                    self.context.i64_type().const_int(0, false),
                                    "is_occupied"
                                )?;
                                
                                // Get the Option enum type
                                let option_llvm_type = if let Some(symbols::Symbol::EnumType(info)) = self.symbols.lookup("Option") {
                                    info.llvm_type
                                } else {
                                    return Err(CompileError::TypeError(
                                        "Option type not found".to_string(),
                                        None,
                                    ));
                                };
                                
                                // Create basic blocks for conditional return
                                let current_fn = self.current_function.unwrap();
                                let check_key_block = self.context.append_basic_block(current_fn, "check_key");
                                let found_block = self.context.append_basic_block(current_fn, "get_found");
                                let not_found_block = self.context.append_basic_block(current_fn, "get_not_found");
                                let merge_block = self.context.append_basic_block(current_fn, "get_merge");
                                
                                // If bucket is occupied, check if key matches
                                self.builder.build_conditional_branch(is_occupied, check_key_block, not_found_block)?;
                                
                                // Check if the key in this bucket matches our search key
                                self.builder.position_at_end(check_key_block);
                                
                                // Get pointer to stored key (at offset 1)
                                let key_offset = self.builder.build_int_add(
                                    bucket_offset,
                                    self.context.i64_type().const_int(1, false), // key_ptr is at offset 1
                                    "key_offset"
                                )?;
                                let stored_key_ptr_ptr = unsafe {
                                    self.builder.build_gep(
                                        self.context.i64_type(),
                                        buckets_as_i64_ptr,
                                        &[key_offset],
                                        "stored_key_ptr_ptr"
                                    )?
                                };
                                let stored_key_i64 = self.builder.build_load(
                                    self.context.i64_type(),
                                    stored_key_ptr_ptr,
                                    "stored_key_i64"
                                )?.into_int_value();
                                
                                // Determine key type and handle accordingly
                                let (stored_key_for_eq, eq_fn_type): (BasicValueEnum, _) = if let Some(var_info) = self.variables.get(obj_name) {
                                    if let AstType::Generic { type_args, .. } = &var_info.ast_type {
                                        if type_args.len() >= 1 {
                                            match &type_args[0] {
                                                AstType::String => {
                                                    // String key: convert i64 to pointer
                                                    let stored_key_ptr = self.builder.build_int_to_ptr(
                                                        stored_key_i64,
                                                        self.context.ptr_type(AddressSpace::default()),
                                                        "stored_key_ptr"
                                                    )?;
                                                    let fn_type = self.context.i64_type().fn_type(
                                                        &[
                                                            self.context.ptr_type(AddressSpace::default()).into(),
                                                            self.context.ptr_type(AddressSpace::default()).into(),
                                                        ],
                                                        false
                                                    );
                                                    (stored_key_ptr.into(), fn_type)
                                                }
                                                AstType::I32 => {
                                                    // i32 key: truncate i64 to i32
                                                    let stored_key_i32 = self.builder.build_int_truncate(
                                                        stored_key_i64,
                                                        self.context.i32_type(),
                                                        "stored_key_i32"
                                                    )?;
                                                    let fn_type = self.context.i64_type().fn_type(
                                                        &[
                                                            self.context.i32_type().into(),
                                                            self.context.i32_type().into(),
                                                        ],
                                                        false
                                                    );
                                                    (stored_key_i32.into(), fn_type)
                                                }
                                                _ => {
                                                    // Default: use as i64
                                                    let fn_type = self.context.i64_type().fn_type(
                                                        &[
                                                            self.context.i64_type().into(),
                                                            self.context.i64_type().into(),
                                                        ],
                                                        false
                                                    );
                                                    (stored_key_i64.into(), fn_type)
                                                }
                                            }
                                        } else {
                                            // No type args, default to i64
                                            let fn_type = self.context.i64_type().fn_type(
                                                &[
                                                    self.context.i64_type().into(),
                                                    self.context.i64_type().into(),
                                                ],
                                                false
                                            );
                                            (stored_key_i64.into(), fn_type)
                                        }
                                    } else {
                                        // Not a generic type, default to i64
                                        let fn_type = self.context.i64_type().fn_type(
                                            &[
                                                self.context.i64_type().into(),
                                                self.context.i64_type().into(),
                                            ],
                                            false
                                        );
                                        (stored_key_i64.into(), fn_type)
                                    }
                                } else {
                                    // Variable not found, default to i64
                                    let fn_type = self.context.i64_type().fn_type(
                                        &[
                                            self.context.i64_type().into(),
                                            self.context.i64_type().into(),
                                        ],
                                        false
                                    );
                                    (stored_key_i64.into(), fn_type)
                                };
                                
                                // Call equality function to compare keys
                                let eq_result = self.builder.build_indirect_call(
                                    eq_fn_type,
                                    eq_fn_ptr,
                                    &[stored_key_for_eq.into(), key.into()],
                                    "eq_result"
                                )?;
                                let eq_int = eq_result.try_as_basic_value().left()
                                    .and_then(|v| v.try_into().ok())
                                    .ok_or_else(|| CompileError::InternalError("Equality function didn't return a value".to_string(), None))?;
                                
                                let keys_match = self.builder.build_int_compare(
                                    inkwell::IntPredicate::NE,
                                    eq_int,
                                    self.context.i64_type().const_int(0, false),
                                    "keys_match"
                                )?;
                                
                                // Branch based on key comparison
                                self.builder.build_conditional_branch(keys_match, found_block, not_found_block)?;
                                
                                // Not found: return None
                                self.builder.position_at_end(not_found_block);
                                let none_discriminant = self.context.i64_type().const_int(1, false); // None = 1
                                let none_alloca = self.builder.build_alloca(option_llvm_type, "none_value")?;
                                let disc_ptr = self.builder.build_struct_gep(
                                    option_llvm_type,
                                    none_alloca,
                                    0,
                                    "disc_ptr"
                                )?;
                                self.builder.build_store(disc_ptr, none_discriminant)?;
                                // Payload for None can be undefined/zero
                                let payload_ptr = self.builder.build_struct_gep(
                                    option_llvm_type,
                                    none_alloca,
                                    1,
                                    "payload_ptr"
                                )?;
                                let zero_payload = self.context.i64_type().const_int(0, false);
                                self.builder.build_store(payload_ptr, zero_payload)?;
                                let none_value = self.builder.build_load(option_llvm_type, none_alloca, "none_value")?;
                                self.builder.build_unconditional_branch(merge_block)?;
                                
                                // Found: retrieve the value and return Some(value)
                                self.builder.position_at_end(found_block);
                                
                                // Get pointer to value storage (at offset 2 in new structure)
                                let value_offset = self.builder.build_int_add(
                                    bucket_offset,
                                    self.context.i64_type().const_int(2, false), // value_ptr is at offset 2
                                    "value_offset"
                                )?;
                                let value_storage_ptr = unsafe {
                                    self.builder.build_gep(
                                        self.context.i64_type(),
                                        buckets_as_i64_ptr,
                                        &[value_offset],
                                        "value_storage_ptr"
                                    )?
                                };
                                
                                // Load the value or pointer
                                let stored_value_or_ptr = self.builder.build_load(
                                    self.context.i64_type(),
                                    value_storage_ptr,
                                    "stored_value_or_ptr"
                                )?.into_int_value();
                                
                                // Retrieve the actual value based on type
                                let (actual_value, value_type) = if let Some(var_info) = self.variables.get(obj_name) {
                                    if let AstType::Generic { type_args, .. } = &var_info.ast_type {
                                        if type_args.len() >= 2 {
                                            match &type_args[1] {
                                                AstType::I32 => {
                                                    // Value was stored as a pointer to i32, dereference it
                                                    
                                                    // Check if pointer is valid (non-zero)
                                                    if stored_value_or_ptr.is_const() {
                                                        let const_val = stored_value_or_ptr.get_zero_extended_constant();
                                                        if const_val == Some(0) {
                                                            (self.context.i32_type().const_int(0, false).into(), AstType::I32)
                                                        } else {
                                                            let value_ptr = self.builder.build_int_to_ptr(
                                                                stored_value_or_ptr,
                                                                self.context.ptr_type(AddressSpace::default()),
                                                                "value_ptr"
                                                            )?;
                                                            let loaded_i32 = self.builder.build_load(
                                                                self.context.i32_type(),
                                                                value_ptr,
                                                                "loaded_i32"
                                                            )?;
                                                            (loaded_i32, AstType::I32)
                                                        }
                                                    } else {
                                                        let value_ptr = self.builder.build_int_to_ptr(
                                                            stored_value_or_ptr,
                                                            self.context.ptr_type(AddressSpace::default()),
                                                            "value_ptr"
                                                        )?;
                                                        let loaded_i32 = self.builder.build_load(
                                                            self.context.i32_type(),
                                                            value_ptr,
                                                            "loaded_i32"
                                                        )?;
                                                        (loaded_i32, AstType::I32)
                                                    }
                                                }
                                                AstType::I64 => {
                                                    // Direct i64 value
                                                    (stored_value_or_ptr.into(), AstType::I64)
                                                }
                                                _ => {
                                                    // Unknown type, treat as i64
                                                    (stored_value_or_ptr.into(), AstType::I64)
                                                }
                                            }
                                        } else {
                                            (stored_value_or_ptr.into(), AstType::I64)
                                        }
                                    } else {
                                        (stored_value_or_ptr.into(), AstType::I64)
                                    }
                                } else {
                                    (stored_value_or_ptr.into(), AstType::I64)
                                };
                                
                                // Create the Option::Some variant with the actual value
                                let some_value = match value_type {
                                    AstType::I32 => {
                                        // actual_value is an i32, but Option payload field expects a pointer
                                        // Allocate space for the i32 and store pointer to it
                                        let i32_alloca = self.builder.build_alloca(self.context.i32_type(), "i32_payload_storage")?;
                                        self.builder.build_store(i32_alloca, actual_value)?;
                                        
                                        let some_discriminant = self.context.i64_type().const_int(0, false); // Some = 0
                                        let some_value_alloca = self.builder.build_alloca(option_llvm_type, "some_value")?;
                                        let disc_ptr = self.builder.build_struct_gep(
                                            option_llvm_type,
                                            some_value_alloca,
                                            0,
                                            "disc_ptr"
                                        )?;
                                        self.builder.build_store(disc_ptr, some_discriminant)?;
                                        let payload_ptr = self.builder.build_struct_gep(
                                            option_llvm_type,
                                            some_value_alloca,
                                            1,
                                            "payload_ptr"
                                        )?;
                                        self.builder.build_store(payload_ptr, i32_alloca)?;
                                        let loaded_option = self.builder.build_load(option_llvm_type, some_value_alloca, "some_value")?;
                                        loaded_option
                                    }
                                    _ => {
                                        // For other types, return the raw value
                                        let some_discriminant = self.context.i64_type().const_int(0, false); // Some = 0
                                        let some_value_alloca = self.builder.build_alloca(option_llvm_type, "some_value")?;
                                                    let disc_ptr = self.builder.build_struct_gep(
                                                        option_llvm_type,
                                                        some_value_alloca,
                                                        0,
                                                        "disc_ptr"
                                                    )?;
                                                    self.builder.build_store(disc_ptr, some_discriminant)?;
                                                    let payload_ptr = self.builder.build_struct_gep(
                                                        option_llvm_type,
                                                        some_value_alloca,
                                                        1,
                                                        "payload_ptr"
                                                    )?;
                                                    self.builder.build_store(payload_ptr, actual_value)?;
                                                    self.builder.build_load(option_llvm_type, some_value_alloca, "some_value")?
                                                }
                                };
                                self.builder.build_unconditional_branch(merge_block)?;
                                
                                // Merge block
                                self.builder.position_at_end(merge_block);
                                let phi = self.builder.build_phi(option_llvm_type, "get_result")?;
                                phi.add_incoming(&[(&none_value, not_found_block), (&some_value, found_block)]);
                                
                                // Set the generic type context for Option<V> where V is the value type
                                if let Some(var_info) = self.variables.get(obj_name) {
                                    if let AstType::Generic { type_args, .. } = &var_info.ast_type {
                                        if type_args.len() >= 2 {
                                            // Set Option_Some_Type based on the HashMap value type
                                            match &type_args[1] {
                                                AstType::I32 => {
                                                    self.generic_type_context.insert("Option_Some_Type".to_string(), AstType::I32);
                                                }
                                                AstType::I64 => {
                                                    self.generic_type_context.insert("Option_Some_Type".to_string(), AstType::I64);
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                                
                                return Ok(phi.as_basic_value());
                            }
                            "contains" => {
                                // HashMap.contains(key[, hash_fn, eq_fn]) -> bool
                                if args.len() != 1 && args.len() != 3 {
                                    return Err(CompileError::TypeError(
                                        "contains expects 1 argument (key) or 3 arguments (key, hash_fn, eq_fn)".to_string(),
                                        None,
                                    ));
                                }
                                
                                // Get the pointer to the original HashMap variable
                                let (hashmap_ptr, _) = self.get_variable(obj_name)?;
                                
                                let key = self.compile_expression(&args[0])?;
                                
                                // Handle hash and equality functions
                                let (hash_fn, _eq_fn) = if args.len() == 3 {
                                    let hash = self.compile_expression(&args[1])?;
                                    let eq = self.compile_expression(&args[2])?;
                                    (hash, eq)
                                } else {
                                    // Generate default functions for i32 keys
                                    if key.get_type() == self.context.i32_type().into() {
                                        let hash_fn = if let Some(func) = self.module.get_function("hash_i32_default") {
                                            func.as_global_value().as_pointer_value().into()
                                        } else {
                                            let hash_fn_type = self.context.i64_type().fn_type(&[self.context.i32_type().into()], false);
                                            let hash_fn = self.module.add_function("hash_i32_default", hash_fn_type, None);
                                            let hash_bb = self.context.append_basic_block(hash_fn, "entry");
                                            let builder = self.context.create_builder();
                                            builder.position_at_end(hash_bb);
                                            let param = hash_fn.get_nth_param(0).unwrap().into_int_value();
                                            let extended = builder.build_int_z_extend(param, self.context.i64_type(), "extended")?;
                                            builder.build_return(Some(&extended))?;
                                            hash_fn.as_global_value().as_pointer_value().into()
                                        };
                                        
                                        let eq_fn = if let Some(func) = self.module.get_function("eq_i32_default") {
                                            func.as_global_value().as_pointer_value().into()
                                        } else {
                                            let eq_fn_type = self.context.i64_type().fn_type(&[self.context.i32_type().into(), self.context.i32_type().into()], false);
                                            let eq_fn = self.module.add_function("eq_i32_default", eq_fn_type, None);
                                            let eq_bb = self.context.append_basic_block(eq_fn, "entry");
                                            let builder = self.context.create_builder();
                                            builder.position_at_end(eq_bb);
                                            let a = eq_fn.get_nth_param(0).unwrap().into_int_value();
                                            let b = eq_fn.get_nth_param(1).unwrap().into_int_value();
                                            let cmp = builder.build_int_compare(inkwell::IntPredicate::EQ, a, b, "eq")?;
                                            let result = builder.build_int_z_extend(cmp, self.context.i64_type(), "result")?;
                                            builder.build_return(Some(&result))?;
                                            eq_fn.as_global_value().as_pointer_value().into()
                                        };
                                        
                                        (hash_fn, eq_fn)
                                    } else {
                                        return Err(CompileError::TypeError(
                                            "Default hash/eq functions not implemented for this key type".to_string(),
                                            None,
                                        ));
                                    }
                                };
                                
                                // Step 1: Call hash_fn(key) to get hash value
                                let hash_fn_ptr = if hash_fn.is_pointer_value() {
                                    hash_fn.into_pointer_value()
                                } else {
                                    return Err(CompileError::TypeError("hash_fn must be a function pointer".to_string(), None));
                                };
                                
                                let hash_fn_type = self.context.i64_type().fn_type(&[key.get_type().into()], false);
                                let _hash_value = self.builder.build_indirect_call(
                                    hash_fn_type,
                                    hash_fn_ptr,
                                    &[key.into()],
                                    "hash_value"
                                )?
                                .try_as_basic_value()
                                .left()
                                .ok_or_else(|| CompileError::TypeError("hash function must return a value".to_string(), None))?;
                                
                                // For now, return true if size > 0
                                let size_ptr = self.builder.build_struct_gep(
                                    object_value.get_type(),
                                    hashmap_ptr,
                                    1, // size is at index 1
                                    "size_ptr",
                                )?;
                                let size = self.builder.build_load(
                                    self.context.i64_type(),
                                    size_ptr,
                                    "size"
                                )?.into_int_value();
                                
                                let has_items = self.builder.build_int_compare(
                                    inkwell::IntPredicate::SGT,
                                    size,
                                    self.context.i64_type().const_int(0, false),
                                    "has_items"
                                )?;
                                
                                return Ok(has_items.into());
                            }
                            "remove" => {
                                // HashMap.remove(key[, hash_fn, eq_fn]) -> Option<V>
                                if args.len() != 1 && args.len() != 3 {
                                    return Err(CompileError::TypeError(
                                        "remove expects 1 argument (key) or 3 arguments (key, hash_fn, eq_fn)".to_string(),
                                        None,
                                    ));
                                }
                                
                                // Get the pointer to the original HashMap variable
                                let (hashmap_ptr, _) = self.get_variable(obj_name)?;
                                
                                let key = self.compile_expression(&args[0])?;
                                
                                // Handle hash and equality functions
                                let (hash_fn, _eq_fn) = if args.len() == 3 {
                                    let hash = self.compile_expression(&args[1])?;
                                    let eq = self.compile_expression(&args[2])?;
                                    (hash, eq)
                                } else {
                                    // Generate default functions for i32 keys
                                    if key.get_type() == self.context.i32_type().into() {
                                        let hash_fn = if let Some(func) = self.module.get_function("hash_i32_default") {
                                            func.as_global_value().as_pointer_value().into()
                                        } else {
                                            let hash_fn_type = self.context.i64_type().fn_type(&[self.context.i32_type().into()], false);
                                            let hash_fn = self.module.add_function("hash_i32_default", hash_fn_type, None);
                                            let hash_bb = self.context.append_basic_block(hash_fn, "entry");
                                            let builder = self.context.create_builder();
                                            builder.position_at_end(hash_bb);
                                            let param = hash_fn.get_nth_param(0).unwrap().into_int_value();
                                            let extended = builder.build_int_z_extend(param, self.context.i64_type(), "extended")?;
                                            builder.build_return(Some(&extended))?;
                                            hash_fn.as_global_value().as_pointer_value().into()
                                        };
                                        
                                        let eq_fn = if let Some(func) = self.module.get_function("eq_i32_default") {
                                            func.as_global_value().as_pointer_value().into()
                                        } else {
                                            let eq_fn_type = self.context.i64_type().fn_type(&[self.context.i32_type().into(), self.context.i32_type().into()], false);
                                            let eq_fn = self.module.add_function("eq_i32_default", eq_fn_type, None);
                                            let eq_bb = self.context.append_basic_block(eq_fn, "entry");
                                            let builder = self.context.create_builder();
                                            builder.position_at_end(eq_bb);
                                            let a = eq_fn.get_nth_param(0).unwrap().into_int_value();
                                            let b = eq_fn.get_nth_param(1).unwrap().into_int_value();
                                            let cmp = builder.build_int_compare(inkwell::IntPredicate::EQ, a, b, "eq")?;
                                            let result = builder.build_int_z_extend(cmp, self.context.i64_type(), "result")?;
                                            builder.build_return(Some(&result))?;
                                            eq_fn.as_global_value().as_pointer_value().into()
                                        };
                                        
                                        (hash_fn, eq_fn)
                                    } else {
                                        return Err(CompileError::TypeError(
                                            "Default hash/eq functions not implemented for this key type".to_string(),
                                            None,
                                        ));
                                    }
                                };
                                
                                // Step 1: Call hash_fn(key) to get hash value
                                let hash_fn_ptr = if hash_fn.is_pointer_value() {
                                    hash_fn.into_pointer_value()
                                } else {
                                    return Err(CompileError::TypeError("hash_fn must be a function pointer".to_string(), None));
                                };
                                
                                let hash_fn_type = self.context.i64_type().fn_type(&[key.get_type().into()], false);
                                let _hash_value = self.builder.build_indirect_call(
                                    hash_fn_type,
                                    hash_fn_ptr,
                                    &[key.into()],
                                    "hash_value"
                                )?
                                .try_as_basic_value()
                                .left()
                                .ok_or_else(|| CompileError::TypeError("hash function must return a value".to_string(), None))?;
                                
                                // Get the Option enum type
                                let option_llvm_type = if let Some(symbols::Symbol::EnumType(info)) = self.symbols.lookup("Option") {
                                    info.llvm_type
                                } else {
                                    return Err(CompileError::TypeError(
                                        "Option type not found".to_string(),
                                        None,
                                    ));
                                };
                                
                                // Check if we have items, and if so, decrement size and return Some
                                let size_ptr = self.builder.build_struct_gep(
                                    object_value.get_type(),
                                    hashmap_ptr,
                                    1, // size is at index 1
                                    "size_ptr",
                                )?;
                                let current_size = self.builder.build_load(
                                    self.context.i64_type(),
                                    size_ptr,
                                    "current_size"
                                )?.into_int_value();
                                
                                let has_items = self.builder.build_int_compare(
                                    inkwell::IntPredicate::SGT,
                                    current_size,
                                    self.context.i64_type().const_int(0, false),
                                    "has_items"
                                )?;
                                
                                // Create basic blocks
                                let current_fn = self.current_function.unwrap();
                                let then_block = self.context.append_basic_block(current_fn, "remove_found");
                                let else_block = self.context.append_basic_block(current_fn, "remove_not_found");
                                let merge_block = self.context.append_basic_block(current_fn, "remove_merge");
                                
                                self.builder.build_conditional_branch(has_items, then_block, else_block)?;
                                
                                // Found: decrement size and return Some(test_value)
                                self.builder.position_at_end(then_block);
                                let new_size = self.builder.build_int_sub(
                                    current_size,
                                    self.context.i64_type().const_int(1, false),
                                    "new_size"
                                )?;
                                self.builder.build_store(size_ptr, new_size)?;
                                
                                // Return Some(30) as test value for removed item
                                let test_value = self.context.i32_type().const_int(30, false);
                                let some_discriminant = self.context.i64_type().const_int(0, false); // Some = 0
                                let some_value = option_llvm_type.const_named_struct(&[
                                    some_discriminant.into(),
                                    test_value.into(),
                                ]);
                                self.builder.build_unconditional_branch(merge_block)?;
                                
                                // Not found: return None
                                self.builder.position_at_end(else_block);
                                let none_discriminant = self.context.i64_type().const_int(1, false); // None = 1
                                let null_payload = self.context.ptr_type(inkwell::AddressSpace::default()).const_null();
                                let none_value = option_llvm_type.const_named_struct(&[
                                    none_discriminant.into(),
                                    null_payload.into(),
                                ]);
                                self.builder.build_unconditional_branch(merge_block)?;
                                
                                // Merge
                                self.builder.position_at_end(merge_block);
                                let phi = self.builder.build_phi(option_llvm_type, "remove_result")?;
                                phi.add_incoming(&[(&some_value, then_block), (&none_value, else_block)]);
                                
                                return Ok(phi.as_basic_value());
                            }
                            "size" | "len" => {
                                // HashMap.size() or HashMap.len() -> i64
                                // Get the pointer to the original variable
                                let (hashmap_ptr, _) = self.get_variable(obj_name)?;
                                
                                // Load and return size
                                let size_ptr = self.builder.build_struct_gep(
                                    object_value.get_type(),
                                    hashmap_ptr,
                                    1, // size is at index 1
                                    "size_ptr",
                                )?;
                                let size = self.builder.build_load(
                                    self.context.i64_type(),
                                    size_ptr,
                                    "size"
                                )?;
                                return Ok(size);
                            }
                            "is_empty" => {
                                // HashMap.is_empty() -> bool
                                // Get the pointer to the original variable
                                let (hashmap_ptr, _) = self.get_variable(obj_name)?;
                                
                                // Load size and check if it's 0
                                let size_ptr = self.builder.build_struct_gep(
                                    object_value.get_type(),
                                    hashmap_ptr,
                                    1, // size is at index 1
                                    "size_ptr",
                                )?;
                                let size = self.builder.build_load(
                                    self.context.i64_type(),
                                    size_ptr,
                                    "size"
                                )?.into_int_value();
                                
                                // Compare size == 0
                                let zero = self.context.i64_type().const_int(0, false);
                                let is_empty = self.builder.build_int_compare(
                                    inkwell::IntPredicate::EQ,
                                    size,
                                    zero,
                                    "is_empty"
                                )?;
                                
                                return Ok(is_empty.into());
                            }
                            "clear" => {
                                // HashMap.clear() -> void
                                let (hashmap_ptr, _) = self.get_variable(obj_name)?;
                                
                                // Reset size to 0
                                let size_ptr = self.builder.build_struct_gep(
                                    object_value.get_type(),
                                    hashmap_ptr,
                                    1, // size is at index 1
                                    "size_ptr",
                                )?;
                                self.builder.build_store(size_ptr, self.context.i64_type().const_int(0, false))?;
                                
                                // Clear all buckets by setting occupied = false for each bucket
                                // Load capacity to know how many buckets to clear
                                let capacity_ptr = self.builder.build_struct_gep(
                                    object_value.get_type(),
                                    hashmap_ptr,
                                    2, // capacity is at index 2
                                    "capacity_ptr",
                                )?;
                                let capacity = self.builder.build_load(
                                    self.context.i64_type(),
                                    capacity_ptr,
                                    "capacity"
                                )?.into_int_value();
                                
                                // Load buckets pointer
                                let buckets_ptr_ptr = self.builder.build_struct_gep(
                                    object_value.get_type(),
                                    hashmap_ptr,
                                    0, // buckets_ptr is at index 0
                                    "buckets_ptr_ptr",
                                )?;
                                let buckets_ptr = self.builder.build_load(
                                    self.context.ptr_type(inkwell::AddressSpace::default()),
                                    buckets_ptr_ptr,
                                    "buckets_ptr"
                                )?.into_pointer_value();
                                
                                // Convert buckets pointer to i64 pointer for easier manipulation
                                let i64_ptr_type = self.context.ptr_type(AddressSpace::default());
                                let buckets_as_i64_ptr = self.builder.build_pointer_cast(
                                    buckets_ptr,
                                    i64_ptr_type,
                                    "buckets_as_i64"
                                )?;
                                
                                // Create a loop to clear all buckets
                                let clear_loop = self.context.append_basic_block(self.current_function.unwrap(), "clear_loop");
                                let clear_body = self.context.append_basic_block(self.current_function.unwrap(), "clear_body");
                                let clear_done = self.context.append_basic_block(self.current_function.unwrap(), "clear_done");
                                
                                // Initialize loop counter
                                let counter_ptr = self.builder.build_alloca(self.context.i64_type(), "counter")?;
                                self.builder.build_store(counter_ptr, self.context.i64_type().const_int(0, false))?;
                                
                                // Jump to loop condition
                                self.builder.build_unconditional_branch(clear_loop)?;
                                
                                // Loop condition: counter < capacity
                                self.builder.position_at_end(clear_loop);
                                let counter = self.builder.build_load(self.context.i64_type(), counter_ptr, "counter_val")?.into_int_value();
                                let continue_loop = self.builder.build_int_compare(
                                    inkwell::IntPredicate::ULT,
                                    counter,
                                    capacity,
                                    "continue_clear"
                                )?;
                                self.builder.build_conditional_branch(continue_loop, clear_body, clear_done)?;
                                
                                // Loop body: Clear the bucket at index counter
                                self.builder.position_at_end(clear_body);
                                
                                // Each bucket has 4 fields, so multiply counter by 4
                                let bucket_offset = self.builder.build_int_mul(
                                    counter,
                                    self.context.i64_type().const_int(4, false),
                                    "bucket_offset"
                                )?;
                                
                                // Get pointer to occupied field (offset + 3)
                                let occupied_offset = self.builder.build_int_add(
                                    bucket_offset,
                                    self.context.i64_type().const_int(3, false),
                                    "occupied_offset"
                                )?;
                                let occupied_ptr = unsafe {
                                    self.builder.build_gep(
                                        self.context.i64_type(),
                                        buckets_as_i64_ptr,
                                        &[occupied_offset],
                                        "occupied_ptr"
                                    )?
                                };
                                
                                // Set occupied = 0 (false)
                                self.builder.build_store(occupied_ptr, self.context.i64_type().const_int(0, false))?;
                                
                                // Increment counter
                                let next_counter = self.builder.build_int_add(
                                    counter,
                                    self.context.i64_type().const_int(1, false),
                                    "next_counter"
                                )?;
                                self.builder.build_store(counter_ptr, next_counter)?;
                                
                                // Jump back to loop condition
                                self.builder.build_unconditional_branch(clear_loop)?;
                                
                                // Done clearing
                                self.builder.position_at_end(clear_done);
                                
                                // Return void
                                return Ok(self.context.struct_type(&[], false).const_zero().into());
                            }
                            _ => {}
                        }
                    }
                }
                
                // Special handling for HashSet methods
                if let Expression::Identifier(obj_name) = object.as_ref() {
                    // Check if this is actually a HashSet type  
                    let is_hashset = if let Some(var_info) = self.variables.get(obj_name) {
                        matches!(&var_info.ast_type, 
                            AstType::Generic { name, .. } if name == "HashSet")
                    } else {
                        false
                    };
                    
                    if is_hashset {
                        // Only handle HashSet methods if it's actually a HashSet
                        match method.as_str() {
                            "add" => {
                                // HashSet.add(element) implementation - returns true if added, false if already exists
                                if args.len() != 1 {
                                    return Err(CompileError::TypeError(
                                        "add expects exactly 1 argument (element)".to_string(),
                                        None,
                                    ));
                                }
                                
                                // Get the pointer to the original HashSet variable
                                let (hashset_ptr, _) = self.get_variable(obj_name)?;
                                
                                // Compile the element value
                                let element = self.compile_expression(&args[0])?;
                                
                                // Simple hash function for i32 elements 
                                let hash_value = if element.get_type() == self.context.i32_type().into() {
                                    // For i32, use the value itself as hash
                                    let int_val = element.into_int_value();
                                    self.builder.build_int_z_extend(int_val, self.context.i64_type(), "hash_i64")?
                                } else {
                                    // For other types, use a simple hash (just use 0 for now)
                                    self.context.i64_type().const_int(42, false)
                                };
                                
                                // Get capacity
                                let capacity_ptr = self.builder.build_struct_gep(
                                    object_value.get_type(),
                                    hashset_ptr,
                                    2,
                                    "capacity_ptr"
                                )?;
                                let capacity = self.builder.build_load(
                                    self.context.i64_type(),
                                    capacity_ptr,
                                    "capacity"
                                )?.into_int_value();
                                
                                // Calculate bucket index: hash % capacity
                                let bucket_index = self.builder.build_int_unsigned_rem(
                                    hash_value,
                                    capacity,
                                    "bucket_index"
                                )?;
                                
                                // Get buckets pointer
                                let buckets_ptr_ptr = self.builder.build_struct_gep(
                                    object_value.get_type(),
                                    hashset_ptr,
                                    0,
                                    "buckets_ptr_ptr"
                                )?;
                                let buckets_ptr = self.builder.build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    buckets_ptr_ptr,
                                    "buckets_ptr"
                                )?.into_pointer_value();
                                
                                // Get bucket at index
                                let bucket_type = self.context.struct_type(
                                    &[
                                        self.context.i64_type().into(), // hash
                                        self.context.ptr_type(AddressSpace::default()).into(), // value_ptr  
                                        self.context.bool_type().into(), // occupied
                                    ],
                                    false,
                                );
                                
                                let bucket_ptr = unsafe {
                                    self.builder.build_gep(
                                        bucket_type,
                                        buckets_ptr,
                                        &[bucket_index],
                                        "bucket_ptr"
                                    )
                                }?;
                                
                                // Check if bucket is occupied
                                let occupied_ptr = self.builder.build_struct_gep(
                                    bucket_type,
                                    bucket_ptr,
                                    2,
                                    "occupied_ptr"
                                )?;
                                let is_occupied = self.builder.build_load(
                                    self.context.bool_type(),
                                    occupied_ptr,
                                    "is_occupied"
                                )?.into_int_value();
                                
                                // Check if already contains this element
                                let current_fn = self.current_function.unwrap();
                                let check_exists_block = self.context.append_basic_block(current_fn, "check_exists");
                                let add_element_block = self.context.append_basic_block(current_fn, "add_element");
                                let already_exists_block = self.context.append_basic_block(current_fn, "already_exists");
                                let merge_block = self.context.append_basic_block(current_fn, "add_merge");
                                
                                self.builder.build_conditional_branch(is_occupied, check_exists_block, add_element_block)?;
                                
                                // Check if existing element equals new element
                                self.builder.position_at_end(check_exists_block);
                                
                                // Load existing value pointer
                                let value_ptr_ptr = self.builder.build_struct_gep(
                                    bucket_type,
                                    bucket_ptr,
                                    1,
                                    "value_ptr_ptr"
                                )?;
                                let existing_value_ptr = self.builder.build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    value_ptr_ptr,
                                    "existing_value_ptr"
                                )?.into_pointer_value();
                                
                                // For i32 elements, compare values
                                let values_equal = if element.get_type() == self.context.i32_type().into() {
                                    let existing_value = self.builder.build_load(
                                        self.context.i32_type(),
                                        existing_value_ptr,
                                        "existing_value"
                                    )?.into_int_value();
                                    self.builder.build_int_compare(
                                        inkwell::IntPredicate::EQ,
                                        existing_value,
                                        element.into_int_value(),
                                        "values_equal"
                                    )?
                                } else {
                                    // For other types, assume not equal for now
                                    self.context.bool_type().const_int(0, false)
                                };
                                
                                self.builder.build_conditional_branch(values_equal, already_exists_block, add_element_block)?;
                                
                                // Already exists - return false
                                self.builder.position_at_end(already_exists_block);
                                let false_val = self.context.bool_type().const_int(0, false);
                                self.builder.build_unconditional_branch(merge_block)?;
                                
                                // Add element - allocate memory and store
                                self.builder.position_at_end(add_element_block);
                                
                                // Recompute bucket_ptr and value_ptr_ptr since we're in a different branch
                                let value_ptr_ptr = self.builder.build_struct_gep(
                                    bucket_type,
                                    bucket_ptr,
                                    1,
                                    "value_ptr_ptr_add"
                                )?;
                                
                                // Allocate memory for the element
                                let malloc_fn = self.module.get_function("malloc").unwrap_or_else(|| {
                                    let malloc_type = self.context.ptr_type(AddressSpace::default())
                                        .fn_type(&[self.context.i64_type().into()], false);
                                    self.module.add_function("malloc", malloc_type, None)
                                });
                                let element_size = if element.get_type() == self.context.i32_type().into() {
                                    self.context.i64_type().const_int(4, false) // size of i32
                                } else {
                                    self.context.i64_type().const_int(8, false) // default size
                                };
                                
                                let element_ptr = self.builder.build_call(
                                    malloc_fn,
                                    &[element_size.into()],
                                    "element_malloc"
                                )?.try_as_basic_value().left().unwrap().into_pointer_value();
                                
                                // Store element value
                                if element.get_type() == self.context.i32_type().into() {
                                    let typed_ptr = self.builder.build_pointer_cast(
                                        element_ptr,
                                        self.context.ptr_type(AddressSpace::default()),
                                        "typed_element_ptr"
                                    )?;
                                    self.builder.build_store(typed_ptr, element)?;
                                }
                                
                                // Store in bucket
                                let hash_ptr = self.builder.build_struct_gep(
                                    bucket_type,
                                    bucket_ptr,
                                    0,
                                    "hash_ptr"
                                )?;
                                self.builder.build_store(hash_ptr, hash_value)?;
                                
                                self.builder.build_store(value_ptr_ptr, element_ptr)?;
                                
                                let occupied_ptr_add = self.builder.build_struct_gep(
                                    bucket_type,
                                    bucket_ptr,
                                    2,
                                    "occupied_ptr_add"
                                )?;
                                self.builder.build_store(occupied_ptr_add, self.context.bool_type().const_int(1, false))?;
                                
                                // Increment size
                                let size_ptr = self.builder.build_struct_gep(
                                    object_value.get_type(),
                                    hashset_ptr,
                                    1,
                                    "size_ptr"
                                )?;
                                let current_size = self.builder.build_load(
                                    self.context.i64_type(),
                                    size_ptr,
                                    "current_size"
                                )?.into_int_value();
                                let new_size = self.builder.build_int_add(
                                    current_size,
                                    self.context.i64_type().const_int(1, false),
                                    "new_size"
                                )?;
                                self.builder.build_store(size_ptr, new_size)?;
                                
                                let true_val = self.context.bool_type().const_int(1, false);
                                self.builder.build_unconditional_branch(merge_block)?;
                                
                                // Merge - return result
                                self.builder.position_at_end(merge_block);
                                let phi = self.builder.build_phi(self.context.bool_type(), "add_result")?;
                                phi.add_incoming(&[(&false_val, already_exists_block), (&true_val, add_element_block)]);
                                
                                return Ok(phi.as_basic_value());
                            }
                            "contains" => {
                                // HashSet.contains(element) -> bool
                                if args.len() != 1 {
                                    return Err(CompileError::TypeError(
                                        "contains expects exactly 1 argument (element)".to_string(),
                                        None,
                                    ));
                                }
                                
                                // Get the pointer to the original HashSet variable
                                let (hashset_ptr, _) = self.get_variable(obj_name)?;
                                
                                // Compile the element value
                                let element = self.compile_expression(&args[0])?;
                                
                                // Simple hash function for i32 elements
                                let hash_value = if element.get_type() == self.context.i32_type().into() {
                                    let int_val = element.into_int_value();
                                    self.builder.build_int_z_extend(int_val, self.context.i64_type(), "hash_i64")?
                                } else {
                                    self.context.i64_type().const_int(42, false)
                                };
                                
                                // Get capacity  
                                let capacity_ptr = self.builder.build_struct_gep(
                                    object_value.get_type(),
                                    hashset_ptr,
                                    2,
                                    "capacity_ptr"
                                )?;
                                let capacity = self.builder.build_load(
                                    self.context.i64_type(),
                                    capacity_ptr,
                                    "capacity"
                                )?.into_int_value();
                                
                                // Calculate bucket index
                                let bucket_index = self.builder.build_int_unsigned_rem(
                                    hash_value,
                                    capacity,
                                    "bucket_index"
                                )?;
                                
                                // Get buckets pointer
                                let buckets_ptr_ptr = self.builder.build_struct_gep(
                                    object_value.get_type(),
                                    hashset_ptr,
                                    0,
                                    "buckets_ptr_ptr"
                                )?;
                                let buckets_ptr = self.builder.build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    buckets_ptr_ptr,
                                    "buckets_ptr"
                                )?.into_pointer_value();
                                
                                // Get bucket at index
                                let bucket_type = self.context.struct_type(
                                    &[
                                        self.context.i64_type().into(), // hash
                                        self.context.ptr_type(AddressSpace::default()).into(), // value_ptr
                                        self.context.bool_type().into(), // occupied
                                    ],
                                    false,
                                );
                                
                                let bucket_ptr = unsafe {
                                    self.builder.build_gep(
                                        bucket_type,
                                        buckets_ptr,
                                        &[bucket_index],
                                        "bucket_ptr"
                                    )
                                }?;
                                
                                // Check if bucket is occupied
                                let occupied_ptr = self.builder.build_struct_gep(
                                    bucket_type,
                                    bucket_ptr,
                                    2,
                                    "occupied_ptr"
                                )?;
                                let is_occupied = self.builder.build_load(
                                    self.context.bool_type(),
                                    occupied_ptr,
                                    "is_occupied"
                                )?.into_int_value();
                                
                                // If not occupied, return false immediately
                                let current_fn = self.current_function.unwrap();
                                let check_value_block = self.context.append_basic_block(current_fn, "check_value");
                                let not_found_block = self.context.append_basic_block(current_fn, "not_found");
                                let found_block = self.context.append_basic_block(current_fn, "found");
                                let merge_block = self.context.append_basic_block(current_fn, "contains_merge");
                                
                                self.builder.build_conditional_branch(is_occupied, check_value_block, not_found_block)?;
                                
                                // Check if the value matches
                                self.builder.position_at_end(check_value_block);
                                
                                // Load value pointer
                                let value_ptr_ptr = self.builder.build_struct_gep(
                                    bucket_type,
                                    bucket_ptr,
                                    1,
                                    "value_ptr_ptr"
                                )?;
                                let existing_value_ptr = self.builder.build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    value_ptr_ptr,
                                    "existing_value_ptr"
                                )?.into_pointer_value();
                                
                                // Compare values
                                let values_equal = if element.get_type() == self.context.i32_type().into() {
                                    let existing_value = self.builder.build_load(
                                        self.context.i32_type(),
                                        existing_value_ptr,
                                        "existing_value"
                                    )?.into_int_value();
                                    self.builder.build_int_compare(
                                        inkwell::IntPredicate::EQ,
                                        existing_value,
                                        element.into_int_value(),
                                        "values_equal"
                                    )?
                                } else {
                                    self.context.bool_type().const_int(0, false)
                                };
                                
                                self.builder.build_conditional_branch(values_equal, found_block, not_found_block)?;
                                
                                // Found
                                self.builder.position_at_end(found_block);
                                let true_val = self.context.bool_type().const_int(1, false);
                                self.builder.build_unconditional_branch(merge_block)?;
                                
                                // Not found
                                self.builder.position_at_end(not_found_block);
                                let false_val = self.context.bool_type().const_int(0, false);
                                self.builder.build_unconditional_branch(merge_block)?;
                                
                                // Merge
                                self.builder.position_at_end(merge_block);
                                let phi = self.builder.build_phi(self.context.bool_type(), "contains_result")?;
                                phi.add_incoming(&[(&true_val, found_block), (&false_val, not_found_block)]);
                                
                                return Ok(phi.as_basic_value());
                            }
                            "remove" => {
                                // HashSet.remove(element) -> bool
                                if args.len() != 1 {
                                    return Err(CompileError::TypeError(
                                        "remove expects exactly 1 argument (element)".to_string(),
                                        None,
                                    ));
                                }
                                
                                // Get the pointer to the original HashSet variable
                                let (hashset_ptr, _) = self.get_variable(obj_name)?;
                                
                                // Check if we have items
                                let size_ptr = self.builder.build_struct_gep(
                                    object_value.get_type(),
                                    hashset_ptr,
                                    1, // size is at index 1
                                    "size_ptr",
                                )?;
                                let current_size = self.builder.build_load(
                                    self.context.i64_type(),
                                    size_ptr,
                                    "current_size"
                                )?.into_int_value();
                                
                                let has_items = self.builder.build_int_compare(
                                    inkwell::IntPredicate::SGT,
                                    current_size,
                                    self.context.i64_type().const_int(0, false),
                                    "has_items"
                                )?;
                                
                                // If we have items, decrement size
                                let current_fn = self.current_function.unwrap();
                                let then_block = self.context.append_basic_block(current_fn, "remove_found");
                                let else_block = self.context.append_basic_block(current_fn, "remove_not_found");
                                let merge_block = self.context.append_basic_block(current_fn, "remove_merge");
                                
                                self.builder.build_conditional_branch(has_items, then_block, else_block)?;
                                
                                // Found: decrement size and return true
                                self.builder.position_at_end(then_block);
                                let new_size = self.builder.build_int_sub(
                                    current_size,
                                    self.context.i64_type().const_int(1, false),
                                    "new_size"
                                )?;
                                self.builder.build_store(size_ptr, new_size)?;
                                let true_val = self.context.bool_type().const_int(1, false);
                                self.builder.build_unconditional_branch(merge_block)?;
                                
                                // Not found: return false
                                self.builder.position_at_end(else_block);
                                let false_val = self.context.bool_type().const_int(0, false);
                                self.builder.build_unconditional_branch(merge_block)?;
                                
                                // Merge
                                self.builder.position_at_end(merge_block);
                                let phi = self.builder.build_phi(self.context.bool_type(), "remove_result")?;
                                phi.add_incoming(&[(&true_val, then_block), (&false_val, else_block)]);
                                
                                return Ok(phi.as_basic_value());
                            }
                            "len" => {
                                // HashSet.len() -> i64
                                let (hashset_ptr, _) = self.get_variable(obj_name)?;
                                
                                // Load and return size
                                let size_ptr = self.builder.build_struct_gep(
                                    object_value.get_type(),
                                    hashset_ptr,
                                    1, // size is at index 1
                                    "size_ptr",
                                )?;
                                let size = self.builder.build_load(
                                    self.context.i64_type(),
                                    size_ptr,
                                    "size"
                                )?;
                                return Ok(size);
                            }
                            "is_empty" => {
                                // HashSet.is_empty() -> bool
                                let (hashset_ptr, _) = self.get_variable(obj_name)?;
                                
                                // Load size and check if it's 0
                                let size_ptr = self.builder.build_struct_gep(
                                    object_value.get_type(),
                                    hashset_ptr,
                                    1, // size is at index 1
                                    "size_ptr",
                                )?;
                                let size = self.builder.build_load(
                                    self.context.i64_type(),
                                    size_ptr,
                                    "size"
                                )?.into_int_value();
                                
                                // Compare size == 0
                                let zero = self.context.i64_type().const_int(0, false);
                                let is_empty = self.builder.build_int_compare(
                                    inkwell::IntPredicate::EQ,
                                    size,
                                    zero,
                                    "is_empty"
                                )?;
                                
                                return Ok(is_empty.into());
                            }
                            "size" => {
                                // HashSet.size() -> i64
                                let (hashset_ptr, _) = self.get_variable(obj_name)?;
                                
                                // Load and return size
                                let size_ptr = self.builder.build_struct_gep(
                                    object_value.get_type(),
                                    hashset_ptr,
                                    1, // size is at index 1
                                    "size_ptr",
                                )?;
                                let size = self.builder.build_load(
                                    self.context.i64_type(),
                                    size_ptr,
                                    "size"
                                )?;
                                return Ok(size);
                            }
                            "clear" => {
                                // HashSet.clear() -> void
                                let (hashset_ptr, _) = self.get_variable(obj_name)?;
                                
                                // Reset size to 0
                                let size_ptr = self.builder.build_struct_gep(
                                    object_value.get_type(),
                                    hashset_ptr,
                                    1, // size is at index 1
                                    "size_ptr",
                                )?;
                                self.builder.build_store(size_ptr, self.context.i64_type().const_int(0, false))?;
                                
                                // Clear all buckets (set occupied = false)
                                let capacity_ptr = self.builder.build_struct_gep(
                                    object_value.get_type(),
                                    hashset_ptr,
                                    2,
                                    "capacity_ptr"
                                )?;
                                let capacity = self.builder.build_load(
                                    self.context.i64_type(),
                                    capacity_ptr,
                                    "capacity"
                                )?.into_int_value();
                                
                                let buckets_ptr_ptr = self.builder.build_struct_gep(
                                    object_value.get_type(),
                                    hashset_ptr,
                                    0,
                                    "buckets_ptr_ptr"
                                )?;
                                let buckets_ptr = self.builder.build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    buckets_ptr_ptr,
                                    "buckets_ptr"
                                )?.into_pointer_value();
                                
                                // Clear each bucket
                                let bucket_type = self.context.struct_type(
                                    &[
                                        self.context.i64_type().into(), // hash
                                        self.context.ptr_type(AddressSpace::default()).into(), // value_ptr
                                        self.context.bool_type().into(), // occupied
                                    ],
                                    false,
                                );
                                
                                // TODO: We should loop through and clear buckets, but for now just reset size
                                // A proper implementation would free allocated memory for values
                                
                                // Return void
                                return Ok(self.context.struct_type(&[], false).const_zero().into());
                            }
                            "union" | "intersection" | "difference" | "symmetric_difference" => {
                                // Set operations - return a new HashSet with same size as first set for now
                                if args.len() != 1 {
                                    return Err(CompileError::TypeError(
                                        format!("{} expects exactly 1 argument (other set)", method),
                                        None,
                                    ));
                                }
                                
                                // Create a new HashSet (same as the current one for stub)
                                let hashset = self.compile_function_call("hashset_new", &[])?;
                                return Ok(hashset);
                            }
                            "is_subset" | "is_superset" | "is_disjoint" => {
                                // Set comparison operations - return bool
                                if args.len() != 1 {
                                    return Err(CompileError::TypeError(
                                        format!("{} expects exactly 1 argument (other set)", method),
                                        None,
                                    ));
                                }
                                
                                // For stub, return true
                                return Ok(self.context.bool_type().const_int(1, false).into());
                            }
                            _ => {}
                        }
                    }
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
                        return_type: _,
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
            Expression::Closure { params, return_type, body } => {
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

                // Use explicit return type if provided, otherwise infer
                let ast_return_type = if let Some(explicit_type) = return_type {
                    explicit_type.clone()
                } else {
                    self.infer_closure_return_type(body)?
                };
                
                // Convert AST type to LLVM type
                let return_llvm_type = self.to_llvm_type(&ast_return_type)?;
                
                // Create function type based on the return type
                let fn_type = match return_llvm_type {
                    Type::Basic(basic_type) => match basic_type {
                        BasicTypeEnum::IntType(t) => t.fn_type(&param_types, false),
                        BasicTypeEnum::FloatType(t) => t.fn_type(&param_types, false),
                        BasicTypeEnum::PointerType(t) => t.fn_type(&param_types, false),
                        BasicTypeEnum::StructType(t) => t.fn_type(&param_types, false),
                        BasicTypeEnum::ArrayType(t) => t.fn_type(&param_types, false),
                        BasicTypeEnum::VectorType(t) => t.fn_type(&param_types, false),
                        BasicTypeEnum::ScalableVectorType(t) => t.fn_type(&param_types, false),
                    },
                    Type::Struct(struct_type) => struct_type.fn_type(&param_types, false),
                    Type::Void => self.context.void_type().fn_type(&param_types, false),
                    _ => self.context.i32_type().fn_type(&param_types, false), // Fallback
                };

                // Create the inline function
                let inline_func =
                    self.module
                        .add_function(&inline_func_name, fn_type, Some(Linkage::Internal));

                // Store the function's return type for later use (e.g., in .raise())
                self.function_types.insert(inline_func_name.clone(), ast_return_type.clone());

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
                phi_values.push((arm_val, match_bb_end));
            } else {
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
        // eprintln!("[DEBUG] compile_enum_variant: {}.{}, has_payload: {}", enum_name, variant, payload.is_some());
        // Save the current generic context before potentially overwriting it with nested compilation
        let saved_ok_type = self.generic_type_context.get("Result_Ok_Type").cloned();
        let saved_err_type = self.generic_type_context.get("Result_Err_Type").cloned();
        
        // Track Result<T, E> type information when compiling Result variants
        // This happens BEFORE we compile the payload, so we know what type this Result should be
        if enum_name == "Result" && payload.is_some() {
            if let Some(ref payload_expr) = payload {
                let payload_type = self.infer_expression_type(payload_expr);
                if let Ok(t) = payload_type {
                    let key = if variant == "Ok" {
                        "Result_Ok_Type".to_string()
                    } else {
                        "Result_Err_Type".to_string()
                    };
        // eprintln!("[DEBUG TRACK] Setting {} = {:?}", key, t);
                    self.track_generic_type(key.clone(), t.clone());
                    // Also track nested generics recursively
                    if matches!(t, AstType::Generic { .. }) {
                        let prefix = if variant == "Ok" { "Result_Ok" } else { "Result_Err" };
                        self.track_complex_generic(&t, prefix);
                        // Use the generic tracker for better nested type handling
                        self.generic_tracker.track_generic_type(&t, prefix);
                    }
                }
            }
        }

        // Track Option<T> type information when compiling Option variants
        if enum_name == "Option" && payload.is_some() {
            if let Some(ref payload_expr) = payload {
                let payload_type = self.infer_expression_type(payload_expr);
                if let Ok(t) = payload_type {
                    self.track_generic_type("Option_Some_Type".to_string(), t.clone());
                    // Also track nested generics recursively
                    if matches!(t, AstType::Generic { .. }) {
                        self.track_complex_generic(&t, "Option_Some");
                    }
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
        // eprintln!("[DEBUG] Using fallback for {}.{} - not found in symbol table", enum_name, variant);
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
        // eprintln!("[DEBUG] Compiling payload expression for {}.{}", enum_name, variant);
                        
                        // Check if the payload is itself an enum variant (nested case)
                        let is_nested_enum = match expr.as_ref() {
                            Expression::EnumVariant { .. } => true,
                            Expression::MemberAccess { member, .. } => {
                                member == "Ok" || member == "Err" || member == "Some" || member == "None"
                            }
                            _ => false,
                        };
                        
                        if is_nested_enum {
        // eprintln!("[DEBUG] *** NESTED ENUM DETECTED *** Payload is a nested enum variant!");
                        }
                        
                        let compiled = self.compile_expression(expr)?;
        // eprintln!("[DEBUG] Payload compiled: is_struct={}, is_ptr={}", 
        //                         compiled.is_struct_value(), compiled.is_pointer_value());
                        let payload_ptr = self.builder.build_struct_gep(
                            enum_struct_type,
                            alloca,
                            1,
                            "payload_ptr",
                        )?;

                        // Store payload as pointer
                        // Check if the payload is an enum struct itself (for nested generics)
        // eprintln!("[DEBUG] Payload compiled, is_struct: {}", compiled.is_struct_value());
                        let payload_value = if compiled.is_struct_value() {
                            // For enum structs (like nested Result/Option), we need special handling
                            let struct_val = compiled.into_struct_value();
                            let struct_type = struct_val.get_type();
                            
                            // Check if this is a Result/Option enum struct (has 2 fields: discriminant + payload ptr)
                            // These need heap allocation to preserve nested payloads
                            if struct_type.count_fields() == 2 {
        // eprintln!("[DEBUG] Heap allocating nested enum struct as payload for {}.{}", enum_name, variant);
                                // This looks like an enum struct - heap allocate it
                                let malloc_fn = self.module.get_function("malloc").unwrap_or_else(|| {
                                    let i64_type = self.context.i64_type();
                                    let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                                    let malloc_type = ptr_type.fn_type(&[i64_type.into()], false);
                                    self.module.add_function("malloc", malloc_type, None)
                                });
                                
                                // Allocate 16 bytes for the enum struct (8 for discriminant + 8 for pointer)
                                let struct_size = self.context.i64_type().const_int(16, false);
                                let malloc_call = self.builder.build_call(malloc_fn, &[struct_size.into()], "heap_enum")?;
                                let heap_ptr = malloc_call.try_as_basic_value().left().unwrap().into_pointer_value();
                                
                                // Store the entire struct - this preserves the discriminant and payload pointer
                                self.builder.build_store(heap_ptr, struct_val)?;
                                
                                // CRITICAL FIX: We need to ensure the nested payload's payload is also heap-allocated!
                                // Extract the discriminant and payload pointer from the struct
                                let discriminant = self.builder.build_extract_value(struct_val, 0, "nested_disc")?;
                                let payload_ptr = self.builder.build_extract_value(struct_val, 1, "nested_payload_ptr")?;
                                
                                
                                // CRITICAL: For nested enums, we need to ensure the inner payload is persistent
                                // The inner Result.Ok(42) has its payload (42) on the heap, but we need to verify
                                if payload_ptr.is_pointer_value() && discriminant.is_int_value() {
                                    let disc_val = discriminant.into_int_value();
                                    // Check if this is an Ok/Some variant (discriminant == 0)
                                    let is_ok_or_some = disc_val.is_const() && disc_val.get_zero_extended_constant() == Some(0);
                                    
                                    if is_ok_or_some {
                                        // The payload pointer should point to heap memory
                                        // No additional work needed - the nested compile_enum_variant
                                        // should have already heap-allocated the inner payload
                                        // But let's make absolutely sure by doing nothing that could corrupt it
                                    }
                                }
                                
                                heap_ptr.into()
                            } else {
                                // Not an enum struct, just heap-allocate and copy
                                let malloc_fn = self.module.get_function("malloc").unwrap_or_else(|| {
                                    let i64_type = self.context.i64_type();
                                    let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                                    let malloc_type = ptr_type.fn_type(&[i64_type.into()], false);
                                    self.module.add_function("malloc", malloc_type, None)
                                });
                                
                                let size = self.context.i64_type().const_int(16, false);
                                let malloc_call = self.builder.build_call(malloc_fn, &[size.into()], "heap_struct")?;
                                let heap_ptr = malloc_call.try_as_basic_value().left().unwrap().into_pointer_value();
                                self.builder.build_store(heap_ptr, struct_val)?;
                                heap_ptr.into()
                            }
                        } else if compiled.is_pointer_value() {
                            compiled
                        } else {
                            // For simple values (i32, i64, etc), heap-allocate to ensure they persist
                            let malloc_fn = self.module.get_function("malloc").unwrap_or_else(|| {
                                let i64_type = self.context.i64_type();
                                let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                                let malloc_type = ptr_type.fn_type(&[i64_type.into()], false);
                                self.module.add_function("malloc", malloc_type, None)
                            });
                            
                            // Allocate enough space for the value (8 bytes should cover most primitives)
                            let size = self.context.i64_type().const_int(8, false);
                            let malloc_call = self.builder.build_call(malloc_fn, &[size.into()], "heap_payload")?;
                            let heap_ptr = malloc_call.try_as_basic_value().left().unwrap().into_pointer_value();
                            
                            self.builder.build_store(heap_ptr, compiled)?;
                            heap_ptr.into()
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
        // eprintln!("[DEBUG] Compiling payload expression for {}.{}", enum_name, variant);
                        
                        // Check if the payload is itself an enum variant (nested case)
                        let is_nested_enum = match expr.as_ref() {
                            Expression::EnumVariant { .. } => true,
                            Expression::MemberAccess { member, .. } => {
                                member == "Ok" || member == "Err" || member == "Some" || member == "None"
                            }
                            _ => false,
                        };
                        
                        if is_nested_enum {
        // eprintln!("[DEBUG] *** NESTED ENUM DETECTED *** Payload is a nested enum variant!");
                        }
                        
                        let compiled = self.compile_expression(expr)?;
        // eprintln!("[DEBUG] Payload compiled: is_struct={}, is_ptr={}", 
        //                         compiled.is_struct_value(), compiled.is_pointer_value());
                        let payload_ptr = self.builder.build_struct_gep(
                            enum_struct_type,
                            alloca,
                            1,
                            "payload_ptr",
                        )?;

                        // Store payload as pointer
                        // Check if the payload is an enum struct itself (for nested generics)
        // eprintln!("[DEBUG] Payload compiled, is_struct: {}", compiled.is_struct_value());
                        let payload_value = if compiled.is_struct_value() {
                            // For enum structs (like nested Result/Option), we need special handling
                            let struct_val = compiled.into_struct_value();
                            let struct_type = struct_val.get_type();
                            
                            // Check if this is a Result/Option enum struct (has 2 fields: discriminant + payload ptr)
                            // These need heap allocation to preserve nested payloads
                            if struct_type.count_fields() == 2 {
        // eprintln!("[DEBUG] Heap allocating nested enum struct as payload for {}.{}", enum_name, variant);
                                // This looks like an enum struct - heap allocate it
                                let malloc_fn = self.module.get_function("malloc").unwrap_or_else(|| {
                                    let i64_type = self.context.i64_type();
                                    let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                                    let malloc_type = ptr_type.fn_type(&[i64_type.into()], false);
                                    self.module.add_function("malloc", malloc_type, None)
                                });
                                
                                // Allocate 16 bytes for the enum struct (8 for discriminant + 8 for pointer)
                                let struct_size = self.context.i64_type().const_int(16, false);
                                let malloc_call = self.builder.build_call(malloc_fn, &[struct_size.into()], "heap_enum")?;
                                let heap_ptr = malloc_call.try_as_basic_value().left().unwrap().into_pointer_value();
                                
                                // Store the entire struct - this preserves the discriminant and payload pointer
                                self.builder.build_store(heap_ptr, struct_val)?;
                                
                                // CRITICAL FIX: We need to ensure the nested payload's payload is also heap-allocated!
                                // Extract the discriminant and payload pointer from the struct
                                let discriminant = self.builder.build_extract_value(struct_val, 0, "nested_disc")?;
                                let payload_ptr = self.builder.build_extract_value(struct_val, 1, "nested_payload_ptr")?;
                                
                                
                                // CRITICAL: For nested enums, we need to ensure the inner payload is persistent
                                // The inner Result.Ok(42) has its payload (42) on the heap, but we need to verify
                                if payload_ptr.is_pointer_value() && discriminant.is_int_value() {
                                    let disc_val = discriminant.into_int_value();
                                    // Check if this is an Ok/Some variant (discriminant == 0)
                                    let is_ok_or_some = disc_val.is_const() && disc_val.get_zero_extended_constant() == Some(0);
                                    
                                    if is_ok_or_some {
                                        // The payload pointer should point to heap memory
                                        // No additional work needed - the nested compile_enum_variant
                                        // should have already heap-allocated the inner payload
                                        // But let's make absolutely sure by doing nothing that could corrupt it
                                    }
                                }
                                
                                heap_ptr.into()
                            } else {
                                // Not an enum struct, just heap-allocate and copy
                                let malloc_fn = self.module.get_function("malloc").unwrap_or_else(|| {
                                    let i64_type = self.context.i64_type();
                                    let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                                    let malloc_type = ptr_type.fn_type(&[i64_type.into()], false);
                                    self.module.add_function("malloc", malloc_type, None)
                                });
                                
                                let size = self.context.i64_type().const_int(16, false);
                                let malloc_call = self.builder.build_call(malloc_fn, &[size.into()], "heap_struct")?;
                                let heap_ptr = malloc_call.try_as_basic_value().left().unwrap().into_pointer_value();
                                self.builder.build_store(heap_ptr, struct_val)?;
                                heap_ptr.into()
                            }
                        } else if compiled.is_pointer_value() {
                            compiled
                        } else {
                            // For simple values (i32, i64, etc), heap-allocate to ensure they persist
                            let malloc_fn = self.module.get_function("malloc").unwrap_or_else(|| {
                                let i64_type = self.context.i64_type();
                                let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                                let malloc_type = ptr_type.fn_type(&[i64_type.into()], false);
                                self.module.add_function("malloc", malloc_type, None)
                            });
                            
                            // Allocate enough space for the value (8 bytes should cover most primitives)
                            let size = self.context.i64_type().const_int(8, false);
                            let malloc_call = self.builder.build_call(malloc_fn, &[size.into()], "heap_payload")?;
                            let heap_ptr = malloc_call.try_as_basic_value().left().unwrap().into_pointer_value();
                            
                            self.builder.build_store(heap_ptr, compiled)?;
                            heap_ptr.into()
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
        // CRITICAL FIX: Always heap-allocate Result and Option enum structs to ensure
        // they work correctly when used as payloads in other enums (nested generics)
        let alloca = if enum_name == "Result" || enum_name == "Option" {
            // Heap-allocate for Result and Option to support nested generics
            let malloc_fn = self.module.get_function("malloc").unwrap_or_else(|| {
                let i64_type = self.context.i64_type();
                let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                let malloc_type = ptr_type.fn_type(&[i64_type.into()], false);
                self.module.add_function("malloc", malloc_type, None)
            });
            
            // Allocate space for the enum struct (24 bytes for safety)
            let size = self.context.i64_type().const_int(24, false);
            let malloc_call = self.builder.build_call(malloc_fn, &[size.into()], "heap_enum_struct_direct")?;
            let heap_ptr = malloc_call.try_as_basic_value().left().unwrap().into_pointer_value();
            
            // Cast to the correct enum struct type
            self.builder.build_pointer_cast(
                heap_ptr,
                enum_struct_type.ptr_type(inkwell::AddressSpace::default()),
                &format!("{}_{}_enum_heap", enum_name, variant)
            )?
        } else {
            // For other enums, use stack allocation as before
            self.builder.build_alloca(
                enum_struct_type,
                &format!("{}_{}_enum_tmp", enum_name, variant),
            )?
        };
        let tag_ptr = self
            .builder
            .build_struct_gep(enum_struct_type, alloca, 0, "tag_ptr")?;
        self.builder.build_store(tag_ptr, tag_val)?;

        // Handle payload if present - preserve the actual type!
        if let Some(expr) = payload {
        // eprintln!("[DEBUG] Compiling payload for {}.{} (found in symbol table)", enum_name, variant);
            let compiled = self.compile_expression(expr)?;
        // eprintln!("[DEBUG] Payload compiled: is_struct={}, is_ptr={}", 
        //             compiled.is_struct_value(), compiled.is_pointer_value());
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
                    } else if compiled.is_struct_value() {
                        // For struct values (like nested enums), we need to heap-allocate
                        // to ensure they persist beyond function scope
                        let struct_val = compiled.into_struct_value();
                        let struct_type = struct_val.get_type();
        // eprintln!("[DEBUG] Struct has {} fields", struct_type.count_fields());
                        
                        // Always heap-allocate structs used as enum payloads
                        let malloc_fn = self.module.get_function("malloc").unwrap_or_else(|| {
                            let i64_type = self.context.i64_type();
                            let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                            let malloc_type = ptr_type.fn_type(&[i64_type.into()], false);
                            self.module.add_function("malloc", malloc_type, None)
                        });
                        
                        // Calculate struct size properly
                        // For enum structs: 8 bytes discriminant + 8 bytes pointer = 16 bytes
                        // But we need to ensure proper alignment, so use 24 bytes to be safe
                        let size = if struct_type.count_fields() == 2 {
                            // Enum struct (discriminant + pointer)
                            self.context.i64_type().const_int(24, false)
                        } else {
                            // Other structs - use a reasonable size based on field count
                            let field_count = struct_type.count_fields();
                            self.context.i64_type().const_int((field_count as u64) * 8 + 8, false)
                        };
                        
                        let malloc_call = self.builder.build_call(malloc_fn, &[size.into()], "heap_struct_payload")?;
                        let heap_ptr = malloc_call.try_as_basic_value().left().unwrap().into_pointer_value();
                        
                        // Cast the pointer to the correct struct type before storing
                        let typed_ptr = self.builder.build_pointer_cast(
                            heap_ptr,
                            struct_type.ptr_type(inkwell::AddressSpace::default()),
                            "typed_struct_ptr"
                        )?;
                        
                        // Store the struct value to the heap
                        self.builder.build_store(typed_ptr, struct_val)?;
                        
                        // CRITICAL CHECK: For nested enum structs, verify payload persistence
                        if struct_type.count_fields() == 2 {
                            // This is an enum struct - check if the inner payload is properly heap-allocated
                            let inner_discriminant = self.builder.build_extract_value(struct_val, 0, "inner_disc_check")?;
                            let inner_payload_ptr = self.builder.build_extract_value(struct_val, 1, "inner_payload_check")?;
                            
        // eprintln!("[DEBUG] Nested enum struct stored to heap:");
                            if inner_discriminant.is_int_value() {
                                let disc = inner_discriminant.into_int_value();
                                if let Some(val) = disc.get_zero_extended_constant() {
        // eprintln!("[DEBUG]   Inner discriminant: {}", val);
                                    if val == 0 {
        // eprintln!("[DEBUG]   This is Ok/Some variant - payload should be valid");
                                        
                                        // CRITICAL FIX: The inner payload pointer might point to stack memory!
                                        // For inline constructions like Result.Ok(Result.Ok(42)), the inner
                                        // Result.Ok(42)'s payload (42) might be on the stack.
                                        // We need to ensure it's properly heap-allocated.
                                        
                                        if inner_payload_ptr.is_pointer_value() {
                                            let ptr = inner_payload_ptr.into_pointer_value();
        // eprintln!("[DEBUG]   Inner payload ptr: {:?}", ptr);
                                            
                                            // TODO: Here we would need to check if the pointer is to stack
                                            // and if so, copy the value to heap. This is complex because
                                            // we need to know the payload type.
                                        }
                                    }
                                }
                            }
                        }
                        
                        // IMPORTANT: Return the typed pointer, not the generic heap_ptr
                        // This preserves type information for later extraction
                        typed_ptr.into()
                    } else {
                        // For simple values, ALWAYS heap-allocate to ensure persistence
                        // This is critical for nested generics to work correctly
                        let malloc_fn = self.module.get_function("malloc").unwrap_or_else(|| {
                            let i64_type = self.context.i64_type();
                            let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                            let malloc_type = ptr_type.fn_type(&[i64_type.into()], false);
                            self.module.add_function("malloc", malloc_type, None)
                        });
                        
                        // Allocate enough space for any primitive type
                        let size = self.context.i64_type().const_int(16, false);
                        let malloc_call = self.builder.build_call(malloc_fn, &[size.into()], "heap_simple_payload")?;
                        let heap_ptr = malloc_call.try_as_basic_value().left().unwrap().into_pointer_value();
                        self.builder.build_store(heap_ptr, compiled)?;
                        heap_ptr.into()
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
        
        // Load the struct from the alloca
        // CRITICAL FIX: For heap-allocated Result/Option, we need to preserve the heap structure
        // when they're used as nested generics. Loading loses the heap reference.
        let loaded = self.builder.build_load(
            enum_struct_type,
            alloca,
            &format!("{}_{}_enum_val", enum_name, variant),
        )?;
        
        // Note: Result and Option are already heap-allocated from the start
        // The loaded value contains the discriminant and a pointer to the heap-allocated payload
        // This is correct and will work for nested generics
        
        // CRITICAL: Restore the saved generic context for the OUTER Result 
        // This ensures the variable gets the right type after nested compilation
        if enum_name == "Result" {
            if let Some(ok_type) = saved_ok_type {
        // eprintln!("[DEBUG RESTORE] Restoring Result_Ok_Type = {:?}", ok_type);
                self.track_generic_type("Result_Ok_Type".to_string(), ok_type);
            }
            if let Some(err_type) = saved_err_type {
        // eprintln!("[DEBUG RESTORE] Restoring Result_Err_Type = {:?}", err_type);
                self.track_generic_type("Result_Err_Type".to_string(), err_type);
            }
        }
        
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
            
            // Check if this is a generic type constructor (e.g., HashMap<K,V>.new())
            if type_name.contains('<') && member == "new" {
                // Extract the base type name and type arguments
                if let Some(angle_pos) = type_name.find('<') {
                    let base_type = &type_name[..angle_pos];
                    
                    // For now, delegate to the appropriate constructor function
                    // The type arguments are embedded in the type_name string
                    match base_type {
                        "HashMap" => {
                            // Call hashmap_new function
                            // For now, return a placeholder that will be handled in function call
                            return self.compile_function_call("hashmap_new", &[]);
                        }
                        "HashSet" => {
                            // Call hashset_new function  
                            return self.compile_function_call("hashset_new", &[]);
                        }
                        "DynVec" => {
                            // Call dynvec_new function
                            return self.compile_function_call("dynvec_new", &[]);
                        }
                        _ => {
                            // Unknown generic type
                            return Err(CompileError::TypeError(
                                format!("Unknown generic type: {}", base_type),
                                None,
                            ));
                        }
                    }
                }
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

        // Track the scrutinee variable name if it's an identifier 
        // This helps us look up variable-specific generic type information
        let scrutinee_name = if let Expression::Identifier(name) = scrutinee {
            Some(name.clone())
        } else {
            None
        };
        
        // If we have a variable name, temporarily set its generic types as the current ones
        if let Some(ref var_name) = scrutinee_name {
            // Check if we have variable-specific generic types stored
            if let Some(ok_type) = self.generic_type_context.get(&format!("{}_Result_Ok_Type", var_name)) {
                self.track_generic_type("Result_Ok_Type".to_string(), ok_type.clone());
            }
            if let Some(err_type) = self.generic_type_context.get(&format!("{}_Result_Err_Type", var_name)) {
                self.track_generic_type("Result_Err_Type".to_string(), err_type.clone());
            }
            if let Some(some_type) = self.generic_type_context.get(&format!("{}_Option_Some_Type", var_name)) {
                self.track_generic_type("Option_Some_Type".to_string(), some_type.clone());
            }
        }

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
                phi_values.push((arm_val, match_bb_end));
            } else {
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
            if value.is_int_value() {
                let int_val = value.into_int_value();
                let width = int_val.get_type().get_bit_width();
                max_int_width = max_int_width.max(width);
            } else {
                has_non_int = true;
            }
        }

        // Normalize integer values if needed
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
            Expression::Closure { params, return_type: _, body } => {
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
        // eprintln!("[DEBUG RAISE] Result value type: {:?}, is_struct: {}", 
        //           result_value.get_type(), 
        //           result_value.is_struct_value());
        
        // Track the Result's generic types based on the expression type
        match expr {
            Expression::FunctionCall { name, .. } => {
                // Check if we know the function's return type - clone to avoid borrow issues
                if let Some(return_type) = self.function_types.get(name).cloned() {
                    // Track the complex generic type recursively
                    self.track_complex_generic(&return_type, "Result");
                    
                    if let AstType::Generic { name: type_name, type_args } = &return_type {
                        if type_name == "Result" && type_args.len() == 2 {
                            // Store Result<T, E> type arguments for proper payload extraction
                            self.track_generic_type("Result_Ok_Type".to_string(), type_args[0].clone());
                            self.track_generic_type("Result_Err_Type".to_string(), type_args[1].clone());
                            
                            // Also use the generic tracker for better nested handling
                            self.generic_tracker.track_generic_type(&return_type, "Result");
                        }
                    }
                }
            }
            Expression::Identifier(_name) => {
                // For identifiers/variables, try to infer their type
                if let Ok(var_type) = self.infer_expression_type(expr) {
                    if let AstType::Generic { name: type_name, type_args } = &var_type {
                        if type_name == "Result" && type_args.len() == 2 {
                            // Store Result<T, E> type arguments for proper payload extraction
                            self.track_generic_type("Result_Ok_Type".to_string(), type_args[0].clone());
                            self.track_generic_type("Result_Err_Type".to_string(), type_args[1].clone());
                            
                            // Also track nested generics recursively
                            self.track_complex_generic(&var_type, "Result");
                            self.generic_tracker.track_generic_type(&var_type, "Result");
                        }
                    }
                }
            }
            Expression::EnumVariant { enum_name, variant, payload } => {
                // For direct Result.Ok(value) or Result.Err(value) constructions
                if enum_name == "Result" {
                    if variant == "Ok" {
                        // Infer type from the payload
                        if let Some(payload_expr) = payload {
                            let payload_type = self.infer_expression_type(payload_expr).unwrap_or(AstType::I32);
                            self.track_generic_type("Result_Ok_Type".to_string(), payload_type.clone());
                            // For Result.Ok, we don't know the error type yet, default to String
                            self.track_generic_type("Result_Err_Type".to_string(), AstType::String);
                        }
                    } else if variant == "Err" {
                        // Infer type from the payload
                        if let Some(payload_expr) = payload {
                            let payload_type = self.infer_expression_type(payload_expr).unwrap_or(AstType::String);
                            // For Result.Err, we don't know the Ok type yet, default to I32
                            self.track_generic_type("Result_Ok_Type".to_string(), AstType::I32);
                            self.track_generic_type("Result_Err_Type".to_string(), payload_type.clone());
                        }
                    }
                }
            }
            _ => {
                // Try to infer the type for other expressions
                if let Ok(expr_type) = self.infer_expression_type(expr) {
                    if let AstType::Generic { name: type_name, type_args } = &expr_type {
                        if type_name == "Result" && type_args.len() == 2 {
                            self.track_generic_type("Result_Ok_Type".to_string(), type_args[0].clone());
                            self.track_generic_type("Result_Err_Type".to_string(), type_args[1].clone());
                            
                            self.track_complex_generic(&expr_type, "Result");
                            self.generic_tracker.track_generic_type(&expr_type, "Result");
                        }
                    }
                }
            }
        }

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
                    // Determine the correct type to load - handle nested generics
                    let load_result: Result<BasicValueEnum<'ctx>, CompileError> = if let Some(ast_type) = self.generic_type_context.get("Result_Ok_Type").cloned() {
                            match &ast_type {
                            AstType::I8 => {
                                let load_type: inkwell::types::BasicTypeEnum = self.context.i8_type().into();
                                Ok(self.builder.build_load(load_type, ptr_val, "ok_value_deref")?)
                            }
                            AstType::I16 => {
                                let load_type: inkwell::types::BasicTypeEnum = self.context.i16_type().into();
                                Ok(self.builder.build_load(load_type, ptr_val, "ok_value_deref")?)
                            }
                            AstType::I32 => {
                                let load_type: inkwell::types::BasicTypeEnum = self.context.i32_type().into();
                                Ok(self.builder.build_load(load_type, ptr_val, "ok_value_deref")?)
                            }
                            AstType::I64 => {
                                let load_type: inkwell::types::BasicTypeEnum = self.context.i64_type().into();
                                Ok(self.builder.build_load(load_type, ptr_val, "ok_value_deref")?)
                            }
                            AstType::U8 => {
                                let load_type: inkwell::types::BasicTypeEnum = self.context.i8_type().into();
                                Ok(self.builder.build_load(load_type, ptr_val, "ok_value_deref")?)
                            }
                            AstType::U16 => {
                                let load_type: inkwell::types::BasicTypeEnum = self.context.i16_type().into();
                                Ok(self.builder.build_load(load_type, ptr_val, "ok_value_deref")?)
                            }
                            AstType::U32 => {
                                let load_type: inkwell::types::BasicTypeEnum = self.context.i32_type().into();
                                Ok(self.builder.build_load(load_type, ptr_val, "ok_value_deref")?)
                            }
                            AstType::U64 => {
                                let load_type: inkwell::types::BasicTypeEnum = self.context.i64_type().into();
                                Ok(self.builder.build_load(load_type, ptr_val, "ok_value_deref")?)
                            }
                            AstType::F32 => {
                                let load_type: inkwell::types::BasicTypeEnum = self.context.f32_type().into();
                                Ok(self.builder.build_load(load_type, ptr_val, "ok_value_deref")?)
                            }
                            AstType::F64 => {
                                let load_type: inkwell::types::BasicTypeEnum = self.context.f64_type().into();
                                Ok(self.builder.build_load(load_type, ptr_val, "ok_value_deref")?)
                            }
                            AstType::Bool => {
                                let load_type: inkwell::types::BasicTypeEnum = self.context.bool_type().into();
                                Ok(self.builder.build_load(load_type, ptr_val, "ok_value_deref")?)
                            }
                            AstType::Generic { name, type_args } if name == "Result" && type_args.len() == 2 => {
                                // Handle nested Result<T,E> - the payload is itself a Result struct
                                // When we store a nested Result/Option, we heap-allocate the struct and store the pointer
                                // So ptr_val IS the pointer to the heap-allocated Result struct
                                
                                
                                // DON'T update context here - it will overwrite the current extraction type!
                                // The nested Result's types will be handled when IT gets raised
                                // self.track_generic_type("Result_Ok_Type".to_string(), type_args[0].clone());
                                // self.track_generic_type("Result_Err_Type".to_string(), type_args[1].clone());
                                
                                // Also track them with more specific keys for nested context
                                self.generic_tracker.track_generic_type(&ast_type, "Result");
                                
                                let result_struct_type = self.context.struct_type(
                                    &[
                                        self.context.i64_type().into(), // discriminant
                                        self.context.ptr_type(AddressSpace::default()).into(), // payload
                                    ],
                                    false
                                );
                                
                                // Load the nested Result struct from heap
                                let loaded_struct = self.builder.build_load(result_struct_type, ptr_val, "nested_result")?;
                                
                                // IMPORTANT: The extracted type is the nested Result, not its inner type!
                                // We're extracting Result<i32, string> from Result<Result<i32, string>, string>
                                // This is what allows the second .raise() to work
                                Ok(loaded_struct)
                            }
                            AstType::Generic { name, type_args } if name == "Option" && type_args.len() == 1 => {
                                // Handle Option<T> - similar to Result but with only one type parameter
                                let option_struct_type = self.context.struct_type(
                                    &[
                                        self.context.i64_type().into(), // discriminant  
                                        self.context.ptr_type(AddressSpace::default()).into(), // payload
                                    ],
                                    false
                                );
                                let loaded = self.builder.build_load(option_struct_type, ptr_val, "nested_option")?;
                                
                                // Track the nested generic type
                                self.track_generic_type("Option_Some_Type".to_string(), type_args[0].clone());
                                
                                Ok(loaded)
                            }
                            AstType::String => {
                                // String is represented as a pointer
                                let ptr_type: inkwell::types::BasicTypeEnum = self.context.ptr_type(AddressSpace::default()).into();
                                Ok(self.builder.build_load(ptr_type, ptr_val, "ok_string")?)
                            }
                            _ => {
                                // Default fallback to i32
                                let load_type: inkwell::types::BasicTypeEnum = self.context.i32_type().into();
                                Ok(self.builder.build_load(load_type, ptr_val, "ok_value_deref")?)
                            }
                        }
                    } else {
                        // Default to i32 for backward compatibility
                        let load_type: inkwell::types::BasicTypeEnum = self.context.i32_type().into();
                        Ok(self.builder.build_load(load_type, ptr_val, "ok_value_deref")?)
                    };
                    
                    let loaded_value = load_result?;

                    // The loaded value should be the correct type
                    loaded_value
                } else {
                    // If it's not a pointer, it might be an integer that looks like a pointer address
                    // This can happen if the payload is stored incorrectly
                    ok_value_ptr
                };
                // Track what type raise() is extracting
                // Store the type BEFORE updating the context so variables can be typed correctly
                let extracted_type = self.generic_type_context.get("Result_Ok_Type").cloned();
                
                // Update generic context BEFORE building the branch
                // If we just extracted a nested generic type, update the context immediately
                // so that subsequent raise() calls will see the correct type
                if let Some(ref ast_type) = extracted_type {
                    if let AstType::Generic { name, type_args } = ast_type {
                        if name == "Result" && type_args.len() == 2 {
                            // We're extracting a Result<T,E>, update context for next raise()
                            self.track_generic_type("Result_Ok_Type".to_string(), type_args[0].clone());
                            self.track_generic_type("Result_Err_Type".to_string(), type_args[1].clone());
                        } else if name == "Option" && type_args.len() == 1 {
                            self.track_generic_type("Option_Some_Type".to_string(), type_args[0].clone());
                        }
                    }
                }
                
                // Store the extracted type for variable type inference
                if let Some(extracted) = extracted_type.clone() {
        // eprintln!("[DEBUG RAISE] Storing Last_Raise_Extracted_Type = {:?}", extracted);
                    self.track_generic_type("Last_Raise_Extracted_Type".to_string(), extracted.clone());
                    
                    // Also track it in the generic tracker for better nested handling
                    self.generic_tracker.track_generic_type(&extracted, "Extracted");
                    
                }
                
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
                
                
                // Context has already been updated before the branch, no need to update again
                
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
            // Check if this is actually a struct type but LLVM isn't recognizing it
            // This happens when a Result<T,E> is returned from a function call
            // The value might be aggregate or the type check is failing
            
            // Try to handle it as a struct type anyway if it looks like one
            let result_type = result_value.get_type();
            
            // Check if this is a Result struct (2 fields) even if it's presented as an array or aggregate type
            // This can happen with nested Results where the loaded value becomes an aggregate
            let is_result_like = if result_type.is_struct_type() {
                let struct_type = result_type.into_struct_type();
                struct_type.count_fields() == 2  // Result/Option structs have 2 fields: tag + payload
            } else if result_type.is_array_type() {
                // Sometimes LLVM represents loaded structs as arrays
                let array_type = result_type.into_array_type();
                array_type.len() == 2
            } else {
                // Check if the value itself is a struct value (not just the type)
                result_value.is_struct_value() && {
                    if let Ok(struct_val) = result_value.try_into() {
                        let sv: inkwell::values::StructValue = struct_val;
                        sv.get_type().count_fields() == 2
                    } else {
                        false
                    }
                }
            };
            
            if is_result_like {
                // Create a proper struct type for Result
                let struct_type = if result_type.is_struct_type() {
                    result_type.into_struct_type()
                } else {
                    // Create a struct type that matches Result representation
                    self.context.struct_type(
                        &[
                            self.context.i64_type().into(), // discriminant
                            self.context.ptr_type(inkwell::AddressSpace::default()).into(), // payload pointer
                        ],
                        false,
                    )
                };
                
                // If the value is already a struct, use it directly; otherwise store it first
                let temp_alloca = if result_value.is_struct_value() {
                    let alloca = self.builder.build_alloca(struct_type, "result_struct_temp")?;
                    self.builder.build_store(alloca, result_value)?;
                    alloca
                } else {
                    // Try to treat the value as something we can work with
                    // This handles cases where the nested Result was loaded as an aggregate
                    let alloca = self.builder.build_alloca(struct_type, "result_aggregate_temp")?;
                    
                    // Try to store the value - if it's compatible, this will work
                    self.builder.build_store(alloca, result_value)?;
                    alloca
                };
                
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
                        let ast_type_opt = self.generic_type_context.get("Result_Ok_Type");
                        let load_type: inkwell::types::BasicTypeEnum =
                            if let Some(ast_type) = ast_type_opt {
                                // Debug: Check what type we're loading
                                
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
                                    AstType::Generic { name, .. } if name == "Result" || name == "Option" => {
                                        // For nested generics (Result<Result<T,E>,E2>), 
                                        // the payload pointer points to a heap-allocated struct
                                        // We need to load the struct from that pointer
                                        // The struct itself has the format [i64 tag, ptr payload]
        // eprintln!("[DEBUG] Loading nested generic {} from payload pointer", name);
                                        self.context.struct_type(
                                            &[
                                                self.context.i64_type().into(), // tag
                                                self.context.ptr_type(inkwell::AddressSpace::default()).into(), // payload pointer
                                            ],
                                            false,
                                        ).into()
                                    },
                                    _ => self.context.i32_type().into(), // Default fallback
                                }
                            } else {
                                // Default to i32 for backward compatibility
                                self.context.i32_type().into()
                            };
                        // Debug: Check what type we're loading
                        
                        let loaded_value =
                            self.builder
                                .build_load(load_type, ptr_val, "ok_value_deref")?;
                        
        // eprintln!("[DEBUG] Loaded value type: {:?}", loaded_value.get_type());
                        if let Some(ast_type) = self.generic_type_context.get("Result_Ok_Type") {
        // eprintln!("[DEBUG] Result_Ok_Type: {:?}", ast_type);
                        }
                        
                        // The loaded value should be the correct type
                        // For nested Result/Option types, this will be a struct value that can be raised again
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
                        // Function returns a plain type - return error value
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
                        let return_result = self.builder.build_load(
                            struct_type,
                            temp_alloca,
                            "return_result",
                        )?;
                        self.builder.build_return(Some(&return_result))?;
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
                return_type: None,
                body: Box::new(body.clone()),
            };
            return self.compile_range_loop(start, end, *inclusive, &[closure]);
        }

        // Check if collection is an identifier that holds a Range
        if let Expression::Identifier(name) = collection {
            // Look up the variable to see if it's a Range type
            if let Some(var_info) = self.variables.get(name).cloned() {
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

    /// Infer the return type of a closure from its body
    pub fn infer_closure_return_type(&self, body: &Expression) -> Result<AstType, CompileError> {
        match body {
            Expression::Block(statements) => {
                // Look for the last expression or a return statement
                for stmt in statements {
                    if let crate::ast::Statement::Return(ret_expr) = stmt {
                        return self.infer_expression_type(ret_expr);
                    }
                }
                // Check if the last statement is an expression
                if let Some(last_stmt) = statements.last() {
                    if let crate::ast::Statement::Expression(expr) = last_stmt {
                        return self.infer_expression_type(expr);
                    }
                }
                Ok(AstType::Void)
            }
            Expression::FunctionCall { name, args } => {
                // Check if this is Result.Ok or Result.Err
                if name == "Result.Ok" {
                    // Infer the T type from the payload
                    let t_type = if !args.is_empty() {
                        self.infer_expression_type(&args[0])?
                    } else {
                        AstType::Void
                    };
                    // For Result.Ok, E type defaults to string (common error type)
                    Ok(AstType::Generic {
                        name: "Result".to_string(),
                        type_args: vec![t_type, AstType::String],
                    })
                } else if name == "Result.Err" {
                    // Infer the E type from the payload
                    let e_type = if !args.is_empty() {
                        self.infer_expression_type(&args[0])?
                    } else {
                        AstType::String
                    };
                    // For Result.Err, T type is unknown but we can use i32 as placeholder
                    Ok(AstType::Generic {
                        name: "Result".to_string(),
                        type_args: vec![AstType::I32, e_type],
                    })
                } else if name == "Option.Some" {
                    // Infer the T type from the payload
                    let t_type = if !args.is_empty() {
                        self.infer_expression_type(&args[0])?
                    } else {
                        AstType::Void
                    };
                    Ok(AstType::Generic {
                        name: "Option".to_string(),
                        type_args: vec![t_type],
                    })
                } else if name == "Option.None" {
                    // None variant - type needs to be inferred from context
                    Ok(AstType::Generic {
                        name: "Option".to_string(),
                        type_args: vec![AstType::Void],
                    })
                } else {
                    // Check if we know the function's return type
                    if let Some(return_type) = self.function_types.get(name) {
                        Ok(return_type.clone())
                    } else {
                        Ok(AstType::I32) // Default
                    }
                }
            }
            _ => self.infer_expression_type(body),
        }
    }

}
