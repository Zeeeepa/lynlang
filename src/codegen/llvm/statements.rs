use super::{LLVMCompiler, Type};
use crate::ast::{AstType, Expression, Statement};
use crate::error::CompileError;
use inkwell::{
    types::{BasicType, BasicTypeEnum},
    values::{BasicValueEnum, BasicValue},
};

impl<'ctx> LLVMCompiler<'ctx> {
    pub fn compile_statement(&mut self, statement: &Statement) -> Result<(), CompileError> {
        match statement {
            Statement::Expression(expr) => {
                self.compile_expression(expr)?;
                Ok(())
            }
            Statement::Return(expr) => {
                let value = self.compile_expression(expr)?;
                
                // Execute all deferred expressions before returning
                self.execute_deferred_expressions()?;
                
                // Debug: Check if we're returning the right type
                if let Some(func) = self.current_function {
                    let expected_ret_type = func.get_type().get_return_type();
                    if let Some(_expected) = expected_ret_type {
                        let _actual_type = value.get_type();
                        // This will help us debug type mismatches
                    }
                }
                self.builder.build_return(Some(&value))?;
                Ok(())
            }
            Statement::VariableDeclaration { name, type_, initializer, is_mutable: _, declaration_type: _ } => {
                // Handle type inference or explicit type
                // We may need to compile the expression for type inference, so save the value
                let mut compiled_value: Option<BasicValueEnum> = None;
                
                let llvm_type = match type_ {
                    Some(type_) => self.to_llvm_type(type_)?,
                    None => {
                        // Type inference - try to infer from initializer
                        if let Some(init_expr) = initializer {
                            let init_value = self.compile_expression(init_expr)?;
                            // Save the compiled value to avoid recompiling
                            compiled_value = Some(init_value);
                            
                            match init_value {
                                BasicValueEnum::IntValue(int_val) => {
                                    let bit_width = int_val.get_type().get_bit_width();
                                    if bit_width == 1 {
                                        // Boolean type
                                        Type::Basic(self.context.bool_type().into())
                                    } else if bit_width <= 32 {
                                        Type::Basic(self.context.i32_type().into())
                                    } else {
                                        Type::Basic(self.context.i64_type().into())
                                    }
                                }
                                BasicValueEnum::FloatValue(_) => {
                                    // For now, assume all floats are f64
                                    Type::Basic(self.context.f64_type().into())
                                }
                                BasicValueEnum::PointerValue(_) => {
                                    // For pointers (including strings), use ptr type
                                    Type::Basic(self.context.ptr_type(inkwell::AddressSpace::default()).into())
                                }
                                BasicValueEnum::StructValue(struct_val) => {
                                    // For structs (including enums), use the struct type directly
                                    // The struct type is already a BasicTypeEnum
                                    Type::Basic(struct_val.get_type().as_basic_type_enum())
                                }
                                _ => Type::Basic(self.context.i64_type().into()), // Default to i64
                            }
                        } else {
                            return Err(CompileError::TypeError("Cannot infer type without initializer".to_string(), None));
                        }
                    }
                };

                // Extract BasicTypeEnum for build_alloca
                let basic_type = match llvm_type {
                    Type::Basic(basic) => basic,
                    Type::Struct(struct_type) => struct_type.as_basic_type_enum(),
                    Type::Function(_) => self.context.ptr_type(inkwell::AddressSpace::default()).as_basic_type_enum(),
                    _ => return Err(CompileError::TypeError("Cannot allocate non-basic or struct type".to_string(), None)),
                };

                let alloca = self.builder.build_alloca(basic_type, name).map_err(|e| CompileError::from(e))?;

                if let Some(init_expr) = initializer {
                    // Use the saved value if we already compiled it for type inference
                    let value = if let Some(saved_value) = compiled_value {
                        saved_value
                    } else {
                        self.compile_expression(init_expr)?
                    };
                    
                    // Handle function pointers specially
                    if let Some(type_) = type_ {
                        if matches!(type_, AstType::Function { .. }) {
                            // For function pointers, we need to get the function and store its pointer
                            if let Expression::Identifier(func_name) = init_expr {
                                if let Some(function) = self.module.get_function(&func_name) {
                                    // Store the function pointer
                                    let func_ptr = function.as_global_value().as_pointer_value();
                                    self.builder.build_store(alloca, func_ptr).map_err(|e| CompileError::from(e))?;
                                    self.variables.insert(name.clone(), (alloca, type_.clone()));
                                    Ok(())
                                } else {
                                    Err(CompileError::UndeclaredFunction(func_name.clone(), None))
                                }
                            } else {
                                Err(CompileError::TypeError("Function pointer initializer must be a function name".to_string(), None))
                            }
                        } else if let AstType::Ptr(_inner) = type_ {
                            // For pointers, if the initializer is AddressOf, use the pointer inside the alloca
                            let ptr_value = match init_expr {
                                Expression::AddressOf(inner_expr) => {
                                    // Compile the inner expression to get the alloca pointer
                                    match **inner_expr {
                                        Expression::Identifier(ref id) => {
                                            let (inner_alloca, _) = self.variables.get(id).ok_or_else(|| CompileError::UndeclaredVariable(id.clone(), None))?;
                                            inner_alloca.as_basic_value_enum()
                                        }
                                        _ => value.clone(),
                                    }
                                }
                                _ => value.clone(),
                            };
                            self.builder.build_store(alloca, ptr_value).map_err(|e| CompileError::from(e))?;
                            self.variables.insert(name.clone(), (alloca, type_.clone()));
                            Ok(())
                        } else {
                            // Regular value assignment
                            let value = match (value, type_) {
                                (BasicValueEnum::IntValue(int_val), AstType::I32) => {
                                    self.builder.build_int_truncate(int_val, self.context.i32_type(), "trunc").map_err(|e| CompileError::from(e))?.into()
                                }
                                (BasicValueEnum::IntValue(int_val), AstType::I64) => {
                                    self.builder.build_int_s_extend(int_val, self.context.i64_type(), "extend").map_err(|e| CompileError::from(e))?.into()
                                }
                                (BasicValueEnum::FloatValue(float_val), AstType::F32) => {
                                    self.builder.build_float_trunc(float_val, self.context.f32_type(), "trunc").map_err(|e| CompileError::from(e))?.into()
                                }
                                (BasicValueEnum::FloatValue(float_val), AstType::F64) => {
                                    self.builder.build_float_ext(float_val, self.context.f64_type(), "extend").map_err(|e| CompileError::from(e))?.into()
                                }
                                (BasicValueEnum::PointerValue(ptr_val), AstType::Struct { .. }) => {
                                    // If the value is a pointer and the type is a struct, load the struct value
                                    let struct_type = match self.to_llvm_type(type_)? {
                                        Type::Struct(st) => st,
                                        _ => return Err(CompileError::TypeError("Expected struct type".to_string(), None)),
                                    };
                                    self.builder.build_load(struct_type, ptr_val, "load_struct_init")?.into()
                                }
                                _ => value,
                            };
                            self.builder.build_store(alloca, value).map_err(|e| CompileError::from(e))?;
                            self.variables.insert(name.clone(), (alloca, type_.clone()));
                            Ok(())
                        }
                    } else {
                        // Type inference case
                        self.builder.build_store(alloca, value).map_err(|e| CompileError::from(e))?;
                        // For inferred types, we need to determine the type from the value and the expression
                        let inferred_type = if let Expression::StructLiteral { name: struct_name, .. } = init_expr {
                            // If initializer is a struct literal, use the struct type
                            // We need to get the field types from the registered struct
                            if let Some(struct_info) = self.struct_types.get(struct_name) {
                                // Reconstruct the AstType::Struct with field information
                                let mut fields = vec![];
                                for (field_name, (_, field_type)) in &struct_info.fields {
                                    fields.push((field_name.clone(), field_type.clone()));
                                }
                                AstType::Struct {
                                    name: struct_name.clone(),
                                    fields,
                                }
                            } else {
                                // Fallback if struct not found - shouldn't happen in practice
                                AstType::Struct {
                                    name: struct_name.clone(),
                                    fields: vec![],
                                }
                            }
                        } else if let Expression::TypeCast { target_type, .. } = init_expr {
                            // For type casts, use the target type
                            target_type.clone()
                        } else if let Expression::FunctionCall { name, .. } = init_expr {
                            // For function calls, get the return type from function_types
                            if let Some(return_type) = self.function_types.get(name) {
                                return_type.clone()
                            } else {
                                // Fallback to type inference from value
                                match value {
                                    BasicValueEnum::IntValue(int_val) => {
                                        if int_val.get_type().get_bit_width() <= 32 {
                                            AstType::I32
                                        } else {
                                            AstType::I64
                                        }
                                    }
                                    BasicValueEnum::FloatValue(_) => AstType::F64,
                                    BasicValueEnum::PointerValue(_) => {
                                        AstType::Ptr(Box::new(AstType::I8))
                                    }
                                    _ => AstType::I64,
                                }
                            }
                        } else if let Expression::MemberAccess { object, member } = init_expr {
                            // Check if this is an enum variant access (e.g., GameEntity.Player)
                            if let Expression::Identifier(enum_name) = &**object {
                                // Check if this identifier is an enum type
                                if let Some(super::symbols::Symbol::EnumType(_)) = self.symbols.lookup(enum_name) {
                                    // This is an enum variant, use the enum type
                                    AstType::EnumType {
                                        name: enum_name.clone(),
                                    }
                                } else {
                                    // Regular member access, infer from value
                                    match value {
                                        BasicValueEnum::StructValue(_) => {
                                            // For struct values (including enums), use a generic enum type
                                            AstType::EnumType {
                                                name: String::new(),
                                            }
                                        }
                                        _ => AstType::I64,
                                    }
                                }
                            } else {
                                match value {
                                    BasicValueEnum::StructValue(_) => {
                                        // For struct values (including enums), use a generic enum type
                                        AstType::EnumType {
                                            name: String::new(),
                                        }
                                    }
                                    _ => AstType::I64,
                                }
                            }
                        } else {
                            match value {
                                BasicValueEnum::IntValue(int_val) => {
                                    if int_val.get_type().get_bit_width() <= 32 {
                                        AstType::I32
                                    } else {
                                        AstType::I64
                                    }
                                }
                                BasicValueEnum::FloatValue(_) => {
                                    // For now, assume all floats are f64
                                    AstType::F64
                                }
                                BasicValueEnum::PointerValue(_) => {
                                    // For pointers, check if the initializer is a string literal
                                    if matches!(init_expr, Expression::String(_)) {
                                        AstType::String
                                    } else {
                                        AstType::Ptr(Box::new(AstType::I8)) // Generic pointer type
                                    }
                                }
                                BasicValueEnum::StructValue(_) => {
                                    // For struct values (including enums), use a generic enum type
                                    AstType::EnumType {
                                        name: String::new(),
                                    }
                                }
                                _ => AstType::I64, // Default
                            }
                        };
                        self.variables.insert(name.clone(), (alloca, inferred_type));
                        Ok(())
                    }
                } else {
                    // No initializer - initialize to zero/default
                    let zero: BasicValueEnum = match llvm_type {
                        Type::Basic(BasicTypeEnum::IntType(int_type)) => {
                            int_type.const_zero().into()
                        }
                        Type::Basic(BasicTypeEnum::FloatType(float_type)) => {
                            float_type.const_zero().into()
                        }
                        Type::Basic(BasicTypeEnum::PointerType(_)) => {
                            self.context.i64_type().const_zero().into()
                        }
                        _ => self.context.i64_type().const_zero().into(),
                    };
                    self.builder.build_store(alloca, zero).map_err(|e| CompileError::from(e))?;
                    
                    if let Some(type_) = type_ {
                        self.variables.insert(name.clone(), (alloca, type_.clone()));
                        Ok(())
                    } else {
                        // For inferred types without initializer, default to i64
                        self.variables.insert(name.clone(), (alloca, AstType::I64));
                        Ok(())
                    }
                }
            }
            Statement::VariableAssignment { name, value } => {
                // Check if this is a field assignment (e.g., "s.x")
                if let Some(dot_pos) = name.find('.') {
                    let struct_name = &name[..dot_pos];
                    let field_name = &name[dot_pos + 1..];
                    // Get the struct variable
                    let (struct_alloca, struct_type) = self.get_variable(struct_name)?;
                    // Compile the value to assign
                    let value = self.compile_expression(value)?;
                    // Handle type conversion if needed
                    let value = match (&value, &struct_type) {
                        (BasicValueEnum::IntValue(int_val), AstType::Struct { .. }) => {
                            if int_val.get_type().get_bit_width() != 64 {
                                self.builder.build_int_s_extend(*int_val, self.context.i64_type(), "sext").unwrap().into()
                            } else {
                                (*int_val).into()
                            }
                        }
                        _ => value.clone(),
                    };
                    // Use struct field assignment
                    self.compile_struct_field_assignment(struct_alloca, field_name, value)?;
                    return Ok(());
                }
                
                // Check if variable exists - if not, this is a new immutable declaration with =
                if !self.variables.contains_key(name) {
                    // This is a new immutable declaration: name = value
                    // Compile the value first to infer its type
                    let init_value = self.compile_expression(value)?;
                    
                    // Infer the type from the value
                    let var_type = match init_value {
                        BasicValueEnum::IntValue(int_val) => {
                            if int_val.get_type().get_bit_width() <= 32 {
                                AstType::I32
                            } else {
                                AstType::I64
                            }
                        }
                        BasicValueEnum::FloatValue(float_val) => {
                            if float_val.get_type() == self.context.f32_type() {
                                AstType::F32
                            } else {
                                AstType::F64
                            }
                        }
                        BasicValueEnum::PointerValue(_) => {
                            // For now, treat as a generic pointer
                            AstType::Ptr(Box::new(AstType::Void))
                        }
                        BasicValueEnum::StructValue(_) => {
                            // For structs and enums, we can directly use the LLVM struct type
                            // Store the value directly without trying to convert to AstType
                            // We'll handle this as a special case below
                            AstType::Void // Placeholder - we'll handle this specially
                        }
                        _ => AstType::Void, // Default fallback
                    };
                    
                    // Create the alloca and store the value
                    // Special handling for struct values (including enums)
                    if let BasicValueEnum::StructValue(struct_val) = init_value {
                        // Directly use the struct type from the value
                        let struct_type = struct_val.get_type();
                        let alloca = self.builder.build_alloca(struct_type, name)?;
                        self.builder.build_store(alloca, struct_val)?;
                        // Check if this is an enum variant assignment
                        let inferred_type = if let Expression::MemberAccess { object, member } = value {
                            if let Expression::Identifier(enum_name) = &**object {
                                // Check if this is an enum type
                                if let Some(super::symbols::Symbol::EnumType(_)) = self.symbols.lookup(enum_name) {
                                    AstType::EnumType {
                                        name: enum_name.clone(),
                                    }
                                } else {
                                    // Generic enum type
                                    AstType::EnumType {
                                        name: String::new(),
                                    }
                                }
                            } else {
                                // Generic enum type
                                AstType::EnumType {
                                    name: String::new(),
                                }
                            }
                        } else {
                            // Generic enum type
                            AstType::EnumType {
                                name: String::new(),
                            }
                        };
                        self.variables.insert(name.clone(), (alloca, inferred_type));
                    } else {
                        let llvm_type = self.to_llvm_type(&var_type)?;
                        let basic_type = match llvm_type {
                            Type::Basic(basic) => basic,
                            Type::Struct(struct_type) => struct_type.as_basic_type_enum(),
                            Type::Function(_) => self.context.ptr_type(inkwell::AddressSpace::default()).as_basic_type_enum(),
                            _ => return Err(CompileError::TypeError("Cannot allocate non-basic or struct type".to_string(), None)),
                        };
                        let alloca = self.builder.build_alloca(basic_type, name)?;
                        self.builder.build_store(alloca, init_value)?;
                        self.variables.insert(name.clone(), (alloca, var_type));
                    }
                    return Ok(());
                }
                
                // Regular variable assignment to existing variable
                let (alloca, var_type) = self.get_variable(name)?;
                let value = self.compile_expression(value)?;
                let value = match (&value, &var_type) {
                    (BasicValueEnum::IntValue(int_val), AstType::I32) => {
                        if int_val.get_type().get_bit_width() != 32 {
                            self.builder.build_int_truncate(*int_val, self.context.i32_type(), "trunc").unwrap().into()
                        } else {
                            (*int_val).into()
                        }
                    }
                    (BasicValueEnum::IntValue(int_val), AstType::I64) => {
                        if int_val.get_type().get_bit_width() != 64 {
                            self.builder.build_int_s_extend(*int_val, self.context.i64_type(), "sext").unwrap().into()
                        } else {
                            (*int_val).into()
                        }
                    }
                    _ => value.clone(),
                };
                match &var_type {
                    AstType::Ptr(inner) if matches!(**inner, AstType::Function { .. }) => {
                        // Store function pointer directly
                        self.builder.build_store(alloca, value)?;
                    }
                    AstType::Struct { .. } => {
                        // For struct types, we can store directly without checking basic type
                        self.builder.build_store(alloca, value)?;
                    }
                    _ => {
                        let llvm_type = self.to_llvm_type(&var_type)?;
                        let _basic_type = self.expect_basic_type(llvm_type)?;
                        self.builder.build_store(alloca, value)?;
                    }
                }
                Ok(())
            }
            Statement::PointerAssignment { pointer, value } => {
                // Special case for array indexing on left side
                if let Expression::ArrayIndex { array, index } = pointer {
                    // Get the address of the array element
                    let element_ptr = self.compile_array_index_address(array, index)?;
                    let val = self.compile_expression(&value)?;
                    self.builder.build_store(element_ptr, val)?;
                    Ok(())
                }
                // Special case for member access on left side (struct field assignment)
                else if let Expression::MemberAccess { object, member } = pointer {
                    // Handle struct field assignment (p.x = value where p might be a pointer)
                    if let Expression::Identifier(name) = &**object {
                        // Get the variable info (clone to avoid borrow issues)
                        let var_info = self.variables.get(name).map(|(a, t)| (*a, t.clone()));
                        
                        if let Some((alloca, var_type)) = var_info {
                            // Handle pointer to struct (p.x = value where p is *Point)
                            if let AstType::Ptr(inner_type) = &var_type {
                                // Check if inner type is a struct or a generic representing a struct
                                let struct_name = match &**inner_type {
                                    AstType::Struct { name, .. } => Some(name.clone()),
                                    AstType::Generic { name, type_args } if type_args.is_empty() => {
                                        // Check if this generic name is a known struct
                                        if self.struct_types.contains_key(name) {
                                            Some(name.clone())
                                        } else {
                                            None
                                        }
                                    }
                                    _ => None,
                                };
                                
                                if let Some(struct_name) = struct_name {
                                    // Get struct type info
                                    let struct_info = self.struct_types.get(&struct_name)
                                        .ok_or_else(|| CompileError::TypeError(
                                            format!("Struct type '{}' not found", struct_name),
                                            None
                                        ))?;
                                    
                                    // Find field index
                                    let field_index = struct_info.fields.get(member)
                                        .map(|(idx, _)| *idx)
                                        .ok_or_else(|| CompileError::TypeError(
                                            format!("Field '{}' not found in struct '{}'", member, struct_name),
                                            None
                                        ))?;
                                    
                                    // Load the pointer value
                                    let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                                    let struct_ptr = self.builder.build_load(ptr_type, alloca, &format!("load_{}_ptr", name))?;
                                    let struct_ptr = struct_ptr.into_pointer_value();
                                    
                                    // Build GEP to get field pointer
                                    let indices = vec![
                                        self.context.i32_type().const_zero(),
                                        self.context.i32_type().const_int(field_index as u64, false),
                                    ];
                                    
                                    let field_ptr = unsafe {
                                        self.builder.build_gep(
                                            struct_info.llvm_type,
                                            struct_ptr,
                                            &indices,
                                            &format!("{}_{}__ptr", name, member)
                                        )?
                                    };
                                    
                                    // Compile and store the value
                                    let val = self.compile_expression(&value)?;
                                    self.builder.build_store(field_ptr, val)?;
                                    return Ok(());
                                }
                            }
                            // Handle regular struct (non-pointer)
                            else if let AstType::Struct { name: _struct_name, .. } = &var_type {
                                // Use existing struct field assignment logic
                                let val = self.compile_expression(&value)?;
                                self.compile_struct_field_assignment(alloca, member, val)?;
                                return Ok(());
                            }
                        }
                    }
                    // If we get here, it's not a simple struct field assignment
                    return Err(CompileError::TypeError(
                        format!("Cannot assign to member access expression: {:?}.{}", object, member),
                        None
                    ));
                } else {
                    let ptr_val = self.compile_expression(&pointer)?;
                    let val = self.compile_expression(&value)?;
                    
                    // For pointer variables, we need to load the address first, then store to that address
                    if ptr_val.is_pointer_value() {
                        let ptr = ptr_val.into_pointer_value();
                        // Load the address stored in the pointer variable
                        let address = self.builder.build_load(self.context.ptr_type(inkwell::AddressSpace::default()), ptr, "deref_ptr")?;
                        // Store the value at that address
                        let address_ptr = address.into_pointer_value();
                        self.builder.build_store(address_ptr, val)?;
                        Ok(())
                    } else {
                        Err(CompileError::TypeMismatch {
                            expected: "pointer".to_string(),
                            found: format!("{:?}", ptr_val.get_type()),
                            span: None,
                        })
                    }
                }
            }
            Statement::Loop { kind, body, label: _ } => {
                use crate::ast::LoopKind;
                
                match kind {
                    LoopKind::Infinite => {
                        // Create blocks for infinite loop
                        let loop_body = self.context.append_basic_block(self.current_function.unwrap(), "loop_body");
                        let after_loop_block = self.context.append_basic_block(self.current_function.unwrap(), "after_loop");
                        
                        // Push loop context for break/continue
                        self.loop_stack.push((loop_body, after_loop_block));
                        
                        // Jump to loop body
                        self.builder.build_unconditional_branch(loop_body).map_err(|e| CompileError::from(e))?;
                        self.builder.position_at_end(loop_body);
                        
                        // Compile body
                        for stmt in body {
                            self.compile_statement(stmt)?;
                        }
                        
                        // Loop back if no terminator
                        let current_block = self.builder.get_insert_block().unwrap();
                        if current_block.get_terminator().is_none() {
                            self.builder.build_unconditional_branch(loop_body).map_err(|e| CompileError::from(e))?;
                        }
                        
                        self.loop_stack.pop();
                        self.builder.position_at_end(after_loop_block);
                        Ok(())
                    }
                    LoopKind::Condition(cond_expr) => {
                        // Create blocks
                        let loop_header = self.context.append_basic_block(self.current_function.unwrap(), "loop_header");
                        let loop_body = self.context.append_basic_block(self.current_function.unwrap(), "loop_body");
                        let after_loop_block = self.context.append_basic_block(self.current_function.unwrap(), "after_loop");
                        
                        self.loop_stack.push((loop_header, after_loop_block));
                        
                        // Jump to header
                        self.builder.build_unconditional_branch(loop_header).map_err(|e| CompileError::from(e))?;
                        self.builder.position_at_end(loop_header);
                        
                        // Evaluate condition
                        let cond_value = self.compile_expression(cond_expr)?;
                        if let BasicValueEnum::IntValue(int_val) = cond_value {
                            if int_val.get_type().get_bit_width() == 1 {
                                self.builder.build_conditional_branch(int_val, loop_body, after_loop_block).map_err(|e| CompileError::from(e))?;
                            } else {
                                let zero = int_val.get_type().const_zero();
                                let condition = self.builder.build_int_compare(
                                    inkwell::IntPredicate::NE,
                                    int_val,
                                    zero,
                                    "loop_condition"
                                ).map_err(|e| CompileError::from(e))?;
                                self.builder.build_conditional_branch(condition, loop_body, after_loop_block).map_err(|e| CompileError::from(e))?;
                            }
                        } else {
                            return Err(CompileError::TypeError("Loop condition must be an integer".to_string(), None));
                        }
                        
                        // Compile body
                        self.builder.position_at_end(loop_body);
                        for stmt in body {
                            self.compile_statement(stmt)?;
                        }
                        
                        // Loop back to header
                        let current_block = self.builder.get_insert_block().unwrap();
                        if current_block.get_terminator().is_none() {
                            self.builder.build_unconditional_branch(loop_header).map_err(|e| CompileError::from(e))?;
                        }
                        
                        self.loop_stack.pop();
                        self.builder.position_at_end(after_loop_block);
                        Ok(())
                    }
                }
            },
            Statement::Break { label: _ } => {
                // Branch to the break target (after_loop block) of the current loop
                if let Some((_, break_target)) = self.loop_stack.last() {
                    self.builder.build_unconditional_branch(*break_target).map_err(|e| CompileError::from(e))?;
                    // Create a new block for any unreachable code after break
                    let unreachable_block = self.context.append_basic_block(self.current_function.unwrap(), "after_break");
                    self.builder.position_at_end(unreachable_block);
                } else {
                    return Err(CompileError::SyntaxError("Break statement outside of loop".to_string(), None));
                }
                Ok(())
            },
            Statement::Continue { label: _ } => {
                // Branch to the continue target (loop_header) of the current loop
                if let Some((continue_target, _)) = self.loop_stack.last() {
                    self.builder.build_unconditional_branch(*continue_target).map_err(|e| CompileError::from(e))?;
                    // Create a new block for any unreachable code after continue
                    let unreachable_block = self.context.append_basic_block(self.current_function.unwrap(), "after_continue");
                    self.builder.position_at_end(unreachable_block);
                } else {
                    return Err(CompileError::SyntaxError("Continue statement outside of loop".to_string(), None));
                }
                Ok(())
            },
            Statement::ComptimeBlock(statements) => {
                // Evaluate comptime blocks during codegen
                for stmt in statements {
                    if let Err(e) = self.comptime_evaluator.execute_statement(stmt) {
                        return Err(CompileError::InternalError(
                            format!("Comptime evaluation error: {}", e),
                            None
                        ));
                    }
                }
                Ok(())
            },
            Statement::ModuleImport { .. } => {
                // Module imports are handled during parsing, not codegen
                Ok(())
            },
            Statement::Defer(_deferred_stmt) => {
                // Defer statements need special handling - they execute at scope exit
                // For now, we'll implement a basic version that doesn't support defer
                // TODO: Implement proper defer semantics with scope tracking
                Ok(())
            }
            Statement::ThisDefer(expr) => {
                // @this.defer() - add the expression to the defer stack
                // The expressions will be executed in LIFO order at function exit
                self.defer_stack.push(expr.clone());
                Ok(())
            }
        }
    }
    
    /// Execute all deferred expressions in LIFO order (last deferred first)
    pub fn execute_deferred_expressions(&mut self) -> Result<(), CompileError> {
        // Execute in reverse order (LIFO)
        while let Some(expr) = self.defer_stack.pop() {
            // Compile and execute the deferred expression
            self.compile_expression(&expr)?;
        }
        Ok(())
    }
} 