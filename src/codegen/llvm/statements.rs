use super::{LLVMCompiler, Type};
use crate::ast::{AstType, Expression, Statement};
use crate::error::CompileError;
use inkwell::{
    types::{BasicType, BasicTypeEnum},
    values::{BasicValue, BasicValueEnum},
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
            Statement::VariableDeclaration {
                name,
                type_,
                initializer,
                is_mutable,
                declaration_type,
            } => {
                use crate::ast::VariableDeclarationType;

                // Check if this is an assignment to a forward-declared variable
                // This happens when we have: x: i32 (forward decl) then x = 10 (initialization)
                if let Some(init_expr) = initializer {
                    // Check if variable already exists (forward declaration case)
                    let existing_var = self.variables.get(name).cloned();
                    if let Some(var_info) = existing_var {
                        // Allow initialization of forward-declared variables with = operator
                        // This works for both immutable (x: i32 then x = 10) and mutable (w:: i32 then w = 40)
                        if !var_info.is_initialized
                            && matches!(
                                declaration_type,
                                VariableDeclarationType::InferredImmutable
                            )
                        {
                            // This is initialization of a forward-declared variable
                            let value = self.compile_expression(init_expr)?;
                            let alloca = var_info.pointer;
                            self.builder.build_store(alloca, value)?;

                            // Mark the variable as initialized
                            if let Some(var_info) = self.variables.get_mut(name) {
                                var_info.is_initialized = true;
                            }
                            return Ok(());
                        } else if var_info.is_initialized
                            && var_info.is_mutable
                            && matches!(
                                declaration_type,
                                VariableDeclarationType::InferredImmutable
                            )
                        {
                            // This is a reassignment to an existing mutable variable
                            // (e.g., w = 45 after w:: i32 and w = 40)
                            let value = self.compile_expression(init_expr)?;
                            let alloca = var_info.pointer;
                            self.builder.build_store(alloca, value)?;
                            // No need to update initialized flag - it's already true
                            return Ok(());
                        } else {
                            // Variable already exists and is initialized or wrong declaration type
                            return Err(CompileError::InternalError(
                                format!(
                                    "Type error: Variable '{}' already declared in this scope",
                                    name
                                ),
                                None,
                            ));
                        }
                    }
                }

                // Handle type inference or explicit type
                // We may need to compile the expression for type inference, so save the value
                let mut compiled_value: Option<BasicValueEnum> = None;

                // Keep track of inferred AST type for closures
                let mut inferred_ast_type: Option<AstType> = None;
                
                // Save generic context before compiling to handle raise() correctly
                let saved_ok_type = self.generic_type_context.get("Result_Ok_Type").cloned();

                let llvm_type = match type_ {
                    Some(type_) => {
                        self.to_llvm_type(type_)?
                    }
                    None => {
                        // Type inference - try to infer from initializer
                        if let Some(init_expr) = initializer {
                            // Check if the initializer is a closure BEFORE compiling it
                            if let Expression::Closure { params, return_type, body } = init_expr {
                                // This is a closure - infer its function pointer type
                                let param_types: Vec<AstType> = params
                                    .iter()
                                    .map(|(_, opt_type)| opt_type.clone().unwrap_or(AstType::I32))
                                    .collect();

                                // Use the explicit return type if provided, otherwise infer from body
                                let inferred_return_type = if let Some(rt) = return_type {
                                    rt.clone()
                                } else {
                                    // Try to infer the return type from the closure body
                                    self.infer_closure_return_type(body).unwrap_or(AstType::I32)
                                };

                                // Store the proper function pointer type
                                let func_type = AstType::FunctionPointer {
                                    param_types: param_types.clone(),
                                    return_type: Box::new(inferred_return_type.clone()),
                                };
                                
                                // Track the function's return type for later lookup when it's called
                                // This is crucial for type inference when calling closures
                                self.function_types.insert(name.clone(), inferred_return_type.clone());

                                // Save this for later when we insert the variable
                                inferred_ast_type = Some(func_type);

                                // Compile the closure
                                let init_value = self.compile_expression(init_expr)?;
                                compiled_value = Some(init_value);

                                // For now, use simple function type - the actual closure compilation
                                // will handle the correct types internally
                                Type::Function(
                                    self.context.i32_type().fn_type(
                                        &param_types
                                            .iter()
                                            .map(|_| self.context.i32_type().into())
                                            .collect::<Vec<_>>(),
                                        false,
                                    ),
                                )
                            } else {
                                // Not a closure - try to infer the AST type first
                                // This is crucial for generic types like Result<T,E>
                                if let Ok(ast_type) = self.infer_expression_type(init_expr) {
                                    // Save the inferred AST type for later
                                    inferred_ast_type = Some(ast_type.clone());
                                    
                                    // Track generic types if this is a Result, Option, Array, etc.
                                    if let AstType::Generic { name: type_name, type_args } = &ast_type {
                                        if type_name == "Result" && type_args.len() == 2 {
                                            self.track_generic_type(format!("{}_Result_Ok_Type", name), type_args[0].clone());
                                            self.track_generic_type(format!("{}_Result_Err_Type", name), type_args[1].clone());
                                            // Also track without variable name prefix for pattern matching
                                            self.track_generic_type("Result_Ok_Type".to_string(), type_args[0].clone());
                                            self.track_generic_type("Result_Err_Type".to_string(), type_args[1].clone());
                                            self.generic_tracker.track_generic_type(&ast_type, name);
                                        } else if type_name == "Option" && type_args.len() == 1 {
                                            self.track_generic_type(format!("{}_Option_Some_Type", name), type_args[0].clone());
                                            // Also track without variable name prefix for pattern matching
                                            self.track_generic_type("Option_Some_Type".to_string(), type_args[0].clone());
                                            self.generic_tracker.track_generic_type(&ast_type, name);
                                        } else if type_name == "Array" && type_args.len() == 1 {
                                            self.track_generic_type(format!("{}_Array_Element_Type", name), type_args[0].clone());
                                            self.generic_tracker.track_generic_type(&ast_type, name);
                                        } else if type_name == "HashMap" && type_args.len() == 2 {
                                            self.track_generic_type(format!("{}_HashMap_Key_Type", name), type_args[0].clone());
                                            self.track_generic_type(format!("{}_HashMap_Value_Type", name), type_args[1].clone());
                                            self.generic_tracker.track_generic_type(&ast_type, name);
                                        } else if type_name == "HashSet" && type_args.len() == 1 {
                                            self.track_generic_type(format!("{}_HashSet_Element_Type", name), type_args[0].clone());
                                            self.generic_tracker.track_generic_type(&ast_type, name);
                                        } else if type_name == "DynVec" {
                                            // DynVec can have multiple element types
                                            for (i, element_type) in type_args.iter().enumerate() {
                                                self.track_generic_type(format!("{}_DynVec_Element_{}_Type", name, i), element_type.clone());
                                            }
                                            self.generic_tracker.track_generic_type(&ast_type, name);
                                        }
                                    }
                                    
                                    // Now compile the expression
                                    let init_value = self.compile_expression(init_expr)?;
                                    compiled_value = Some(init_value);
                                    
                                    // Convert the AST type to LLVM type
                                    self.to_llvm_type(&ast_type)?
                                } else {
                                    // Fall back to compiling and inferring from LLVM value
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
                                        BasicValueEnum::FloatValue(_fv) => {
                                            // Store the AST type as F64 to ensure proper loading later
                                            inferred_ast_type = Some(AstType::F64);
                                            // For now, assume all floats are f64
                                            Type::Basic(self.context.f64_type().into())
                                        }
                                        BasicValueEnum::PointerValue(_) => {
                                            // For pointers (including strings), use ptr type
                                            Type::Basic(
                                                self.context
                                                    .ptr_type(inkwell::AddressSpace::default())
                                                    .into(),
                                            )
                                        }
                                        BasicValueEnum::StructValue(struct_val) => {
                                            // For structs (including enums), use the struct type directly
                                            // The struct type is already a BasicTypeEnum
                                            let struct_type = struct_val.get_type();
                                            Type::Basic(struct_type.as_basic_type_enum())
                                        }
                                        _ => Type::Basic(self.context.i64_type().into()), // Default to i64
                                    }
                                }
                            }
                        } else {
                            return Err(CompileError::TypeError(
                                "Cannot infer type without initializer".to_string(),
                                None,
                            ));
                        }
                    }
                };

                // Extract BasicTypeEnum for build_alloca
                let basic_type = match llvm_type {
                    Type::Basic(basic) => basic,
                    Type::Struct(struct_type) => struct_type.as_basic_type_enum(),
                    Type::Function(_) => self
                        .context
                        .ptr_type(inkwell::AddressSpace::default())
                        .as_basic_type_enum(),
                    Type::Pointer(_) => self
                        .context
                        .ptr_type(inkwell::AddressSpace::default())
                        .as_basic_type_enum(),
                    Type::Void => {
                        return Err(CompileError::TypeError(
                            format!("Cannot infer type for variable '{}' - expression has void type", name),
                            None,
                        ))
                    }
                };

                let alloca = self
                    .builder
                    .build_alloca(basic_type, name)
                    .map_err(|e| CompileError::from(e))?;

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
                                    self.builder
                                        .build_store(alloca, func_ptr)
                                        .map_err(|e| CompileError::from(e))?;
                                    self.variables.insert(
                                        name.clone(),
                                        super::VariableInfo {
                                            pointer: alloca,
                                            ast_type: type_.clone(),
                                            is_mutable: *is_mutable,
                                            is_initialized: true,
                                        },
                                    );
                                    Ok(())
                                } else {
                                    Err(CompileError::UndeclaredFunction(func_name.clone(), None))
                                }
                            } else {
                                Err(CompileError::TypeError(
                                    "Function pointer initializer must be a function name"
                                        .to_string(),
                                    None,
                                ))
                            }
                        } else if let AstType::Bool = type_ {
                            // For booleans, store directly as i1
                            self.builder
                                .build_store(alloca, value)
                                .map_err(|e| CompileError::from(e))?;
                            self.variables.insert(
                                name.clone(),
                                super::VariableInfo {
                                    pointer: alloca,
                                    ast_type: type_.clone(),
                                    is_mutable: *is_mutable,
                                    is_initialized: true,
                                },
                            );
                            Ok(())
                        } else if let AstType::Ptr(_inner) = type_ {
                            // For pointers, if the initializer is AddressOf, use the pointer inside the alloca
                            let ptr_value = match init_expr {
                                Expression::AddressOf(inner_expr) => {
                                    // Compile the inner expression to get the alloca pointer
                                    match **inner_expr {
                                        Expression::Identifier(ref id) => {
                                            let var_info =
                                                self.variables.get(id).ok_or_else(|| {
                                                    CompileError::UndeclaredVariable(
                                                        id.clone(),
                                                        None,
                                                    )
                                                })?;
                                            let inner_alloca = var_info.pointer;
                                            inner_alloca.as_basic_value_enum()
                                        }
                                        _ => value.clone(),
                                    }
                                }
                                _ => value.clone(),
                            };
                            self.builder
                                .build_store(alloca, ptr_value)
                                .map_err(|e| CompileError::from(e))?;
                            self.variables.insert(
                                name.clone(),
                                super::VariableInfo {
                                    pointer: alloca,
                                    ast_type: type_.clone(),
                                    is_mutable: *is_mutable,
                                    is_initialized: true,
                                },
                            );
                            Ok(())
                        } else {
                            // Regular value assignment
                            let value = match (value, type_) {
                                (BasicValueEnum::IntValue(int_val), AstType::I32) => self
                                    .builder
                                    .build_int_truncate(int_val, self.context.i32_type(), "trunc")
                                    .map_err(|e| CompileError::from(e))?
                                    .into(),
                                (BasicValueEnum::IntValue(int_val), AstType::I64) => self
                                    .builder
                                    .build_int_s_extend(int_val, self.context.i64_type(), "extend")
                                    .map_err(|e| CompileError::from(e))?
                                    .into(),
                                (BasicValueEnum::FloatValue(float_val), AstType::F32) => self
                                    .builder
                                    .build_float_trunc(float_val, self.context.f32_type(), "trunc")
                                    .map_err(|e| CompileError::from(e))?
                                    .into(),
                                (BasicValueEnum::FloatValue(float_val), AstType::F64) => self
                                    .builder
                                    .build_float_ext(float_val, self.context.f64_type(), "extend")
                                    .map_err(|e| CompileError::from(e))?
                                    .into(),
                                (BasicValueEnum::PointerValue(ptr_val), AstType::Struct { .. }) => {
                                    // If the value is a pointer and the type is a struct, load the struct value
                                    let struct_type = match self.to_llvm_type(type_)? {
                                        Type::Struct(st) => st,
                                        _ => {
                                            return Err(CompileError::TypeError(
                                                "Expected struct type".to_string(),
                                                None,
                                            ))
                                        }
                                    };
                                    self.builder
                                        .build_load(struct_type, ptr_val, "load_struct_init")?
                                        .into()
                                }
                                _ => value,
                            };
                            self.builder
                                .build_store(alloca, value)
                                .map_err(|e| CompileError::from(e))?;
                            
                            // Track generic type parameters for variables with explicit types
                            if let AstType::Generic { name: type_name, type_args } = type_ {
                                if type_name == "Result" && type_args.len() == 2 {
                                    // Track Result<T,E> types with variable-specific keys
                                    self.track_generic_type(format!("{}_Result_Ok_Type", name), type_args[0].clone());
                                    self.track_generic_type(format!("{}_Result_Err_Type", name), type_args[1].clone());
                                    self.generic_tracker.track_generic_type(type_, name);
                                } else if type_name == "Option" && type_args.len() == 1 {
                                    // Track Option<T> types with variable-specific keys
                                    self.track_generic_type(format!("{}_Option_Some_Type", name), type_args[0].clone());
                                    self.generic_tracker.track_generic_type(type_, name);
                                } else if type_name == "Array" && type_args.len() == 1 {
                                    // Track Array<T> types with variable-specific keys
                                    self.track_generic_type(format!("{}_Array_Element_Type", name), type_args[0].clone());
                                    self.generic_tracker.track_generic_type(type_, name);
                                } else if type_name == "HashMap" && type_args.len() == 2 {
                                    self.track_generic_type(format!("{}_HashMap_Key_Type", name), type_args[0].clone());
                                    self.track_generic_type(format!("{}_HashMap_Value_Type", name), type_args[1].clone());
                                    self.generic_tracker.track_generic_type(type_, name);
                                } else if type_name == "HashSet" && type_args.len() == 1 {
                                    self.track_generic_type(format!("{}_HashSet_Element_Type", name), type_args[0].clone());
                                    self.generic_tracker.track_generic_type(type_, name);
                                } else if type_name == "DynVec" {
                                    // DynVec can have multiple element types
                                    for (i, element_type) in type_args.iter().enumerate() {
                                        self.track_generic_type(format!("{}_DynVec_Element_{}_Type", name, i), element_type.clone());
                                    }
                                    self.generic_tracker.track_generic_type(type_, name);
                                }
                            }
                            
                            self.variables.insert(
                                name.clone(),
                                super::VariableInfo {
                                    pointer: alloca,
                                    ast_type: type_.clone(),
                                    is_mutable: *is_mutable,
                                    is_initialized: true,
                                },
                            );
                            Ok(())
                        }
                    } else {
                        // Type inference case
                        self.builder
                            .build_store(alloca, value)
                            .map_err(|e| CompileError::from(e))?;
                        // For inferred types, we need to determine the type from the value and the expression
                        // eprintln!("[DEBUG TYPE] Inferring type for variable {} from expression", name);
                        let inferred_type = if let Some(ast_type) = inferred_ast_type {
                            // We already inferred the type (e.g., for closures)
                            ast_type
                        } else if let Expression::Boolean(_) = init_expr {
                            // Boolean literal - always infer as bool
                            AstType::Bool
                        } else if let Expression::Some(inner_expr) = init_expr {
                            // Option::Some variant - infer Option<T> where T is the type of inner_expr
                            let inner_type = self.infer_expression_type(inner_expr);
                            match inner_type {
                                Ok(t) => AstType::Generic {
                                    name: "Option".to_string(),
                                    type_args: vec![t],
                                },
                                Err(_) => AstType::Generic {
                                    name: "Option".to_string(),
                                    type_args: vec![AstType::I32], // Default to i32 if we can't infer
                                },
                            }
                        } else if let Expression::None = init_expr {
                            // Option::None variant - use Option<T> with generic T
                            AstType::Generic {
                                name: "Option".to_string(),
                                type_args: vec![AstType::Generic {
                                    name: "T".to_string(),
                                    type_args: vec![],
                                }],
                            }
                        } else if let Expression::Range { inclusive, .. } = init_expr {
                            // Range expression - infer Range type
                            AstType::Range {
                                start_type: Box::new(AstType::I32),
                                end_type: Box::new(AstType::I32),
                                inclusive: *inclusive,
                            }
                        } else if let Expression::StructLiteral {
                            name: struct_name, ..
                        } = init_expr
                        {
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
                        } else if let Expression::CreateReference(inner) = init_expr {
                            // For .ref(), the type is Ptr<T> where T is the type of inner
                            // Try to infer the inner type
                            let inner_type = if let Expression::Identifier(var_name) = &**inner {
                                // Look up the type of the variable
                                if let Some(var_info) = self.variables.get(var_name) {
                                    var_info.ast_type.clone()
                                } else {
                                    AstType::I32 // Default fallback
                                }
                            } else if let Expression::Integer32(_) = &**inner {
                                AstType::I32
                            } else if let Expression::Integer64(_) = &**inner {
                                AstType::I64
                            } else {
                                AstType::I32 // Default fallback
                            };
                            AstType::Ptr(Box::new(inner_type))
                        } else if let Expression::CreateMutableReference(inner) = init_expr {
                            // For .mut_ref(), the type is MutPtr<T> where T is the type of inner
                            let inner_type = if let Expression::Identifier(var_name) = &**inner {
                                if let Some(var_info) = self.variables.get(var_name) {
                                    var_info.ast_type.clone()
                                } else {
                                    AstType::I32
                                }
                            } else if let Expression::Integer32(_) = &**inner {
                                AstType::I32
                            } else if let Expression::Integer64(_) = &**inner {
                                AstType::I64
                            } else {
                                AstType::I32
                            };
                            AstType::MutPtr(Box::new(inner_type))
                        } else if let Expression::DynVecConstructor { element_types, .. } =
                            init_expr
                        {
                            // For DynVec constructors, create the proper DynVec type
                            AstType::DynVec {
                                element_types: element_types.clone(),
                                allocator_type: None,
                            }
                        } else if let Expression::VecConstructor {
                            element_type, size, ..
                        } = init_expr
                        {
                            // For Vec constructors, create the proper Vec type
                            AstType::Vec {
                                element_type: Box::new(element_type.clone()),
                                size: *size,
                            }
                        } else if let Expression::TypeCast { target_type, .. } = init_expr {
                            // For type casts, use the target type
                            target_type.clone()
                        } else if let Expression::MethodCall { object, method, .. } = init_expr {
                            // For method calls like HashMap<K,V>.new()
                            if method == "new" {
                                if let Expression::Identifier(type_name) = &**object {
                                    // Check if it's a generic type constructor
                                    if type_name.contains('<') {
                                        // Extract base type and type args
                                        if let Some(angle_pos) = type_name.find('<') {
                                            let base_type = &type_name[..angle_pos];
                                            let remaining = &type_name[angle_pos+1..];
                                            
                                            // Simple parsing of type arguments (assumes K,V format)
                                            match base_type {
                                                "HashMap" => {
                                                    // Parse the type arguments
                                                    // For now, assume string,i32 or similar simple types
                                                    let type_args = if remaining.contains(',') {
                                                        let parts: Vec<&str> = remaining.trim_end_matches('>').split(',').collect();
                                                        let key_str = parts[0].trim();
                                                        let key_type = match key_str {
                                                            "string" | "String" => AstType::String,
                                                            "i32" | "I32" => AstType::I32,
                                                            "i64" | "I64" => AstType::I64,
                                                            "f32" | "F32" => AstType::F32,
                                                            "f64" | "F64" => AstType::F64,
                                                            _ => {
                                                                // Default to String for unknown types
                                                                AstType::String
                                                            }
                                                        };
                                                        let value_type = if let Some(value_str) = parts.get(1).map(|s| s.trim()) {
                                                            match value_str {
                                                                "string" | "String" => AstType::String,
                                                                "i32" | "I32" => AstType::I32,
                                                                "i64" | "I64" => AstType::I64,
                                                                "f32" | "F32" => AstType::F32,
                                                                "f64" | "F64" => AstType::F64,
                                                                _ => {
                                                                    // Default to i32 for unknown types
                                                                    AstType::I32
                                                                }
                                                            }
                                                        } else {
                                                            AstType::I32
                                                        };
                                                        vec![key_type, value_type]
                                                    } else {
                                                        vec![AstType::String, AstType::I32] // Default
                                                    };
                                                    
                                                    AstType::Generic {
                                                        name: "HashMap".to_string(),
                                                        type_args,
                                                    }
                                                }
                                                "HashSet" => {
                                                    let element_type = match remaining.trim_end_matches('>') {
                                                        "string" => AstType::String,
                                                        "i32" => AstType::I32,
                                                        "i64" => AstType::I64,
                                                        _ => AstType::I32, // Default
                                                    };
                                                    AstType::Generic {
                                                        name: "HashSet".to_string(),
                                                        type_args: vec![element_type],
                                                    }
                                                }
                                                "DynVec" => {
                                                    let element_type = match remaining.trim_end_matches('>') {
                                                        "string" => AstType::String,
                                                        "i32" => AstType::I32,
                                                        "i64" => AstType::I64,
                                                        _ => AstType::I32, // Default
                                                    };
                                                    AstType::DynVec {
                                                        element_types: vec![element_type],
                                                        allocator_type: None,
                                                    }
                                                }
                                                _ => {
                                                    // Unknown generic type, fallback
                                                    AstType::I32
                                                }
                                            }
                                        } else {
                                            AstType::I32
                                        }
                                    } else {
                                        AstType::I32
                                    }
                                } else {
                                    AstType::I32
                                }
                            } else if method == "raise" {
                                // For .raise() method, we need to extract the Ok type from Result<T,E>
                                // First, get the type of the object being raised
                                let object_type = self.infer_expression_type(object).unwrap_or(AstType::Void);
                                // eprintln!("[DEBUG] Inferred type for raise object: {:?}", object_type);
                                
                                // If it's Result<T,E>, the raise() returns T
                                if let AstType::Generic { name, type_args } = &object_type {
                                    if name == "Result" && type_args.len() == 2 {
                                        // The raise() method returns the Ok type (T) from Result<T,E>
                                        let extracted_type = type_args[0].clone();
                                        // eprintln!("[DEBUG] Raise extracts type: {:?}", extracted_type);
                                        extracted_type
                                    } else {
                                        // Not a Result type, shouldn't happen but fall back to inferring from value
                                        match value {
                                            BasicValueEnum::IntValue(int_val) => {
                                                let bit_width = int_val.get_type().get_bit_width();
                                                if bit_width == 1 {
                                                    AstType::Bool
                                                } else if bit_width <= 32 {
                                                    AstType::I32
                                                } else {
                                                    AstType::I64
                                                }
                                            }
                                            BasicValueEnum::FloatValue(_) => AstType::F64,
                                            BasicValueEnum::PointerValue(_) => AstType::String,
                                            BasicValueEnum::StructValue(_) => {
                                                // Could be a nested Result or Option, try to infer
                                                AstType::Void
                                            }
                                            _ => AstType::Void
                                        }
                                    }
                                } else {
                                    // Not a generic type, shouldn't happen for raise but fall back
                                    match value {
                                        BasicValueEnum::IntValue(int_val) => {
                                            let bit_width = int_val.get_type().get_bit_width();
                                            if bit_width == 1 {
                                                AstType::Bool
                                            } else if bit_width <= 32 {
                                                AstType::I32
                                            } else {
                                                AstType::I64
                                            }
                                        }
                                        BasicValueEnum::FloatValue(_) => AstType::F64,
                                        BasicValueEnum::PointerValue(_) => AstType::String,
                                        BasicValueEnum::StructValue(_) => AstType::Void,
                                        _ => AstType::Void
                                    }
                                }
                            } else {
                                // For other method calls, try to infer from value
                                match value {
                                    BasicValueEnum::IntValue(int_val) => {
                                        let bit_width = int_val.get_type().get_bit_width();
                                        if bit_width == 1 {
                                            AstType::Bool
                                        } else if bit_width <= 32 {
                                            AstType::I32
                                        } else {
                                            AstType::I64
                                        }
                                    }
                                    BasicValueEnum::FloatValue(_) => AstType::F64,
                                    BasicValueEnum::PointerValue(_) => {
                                        AstType::Ptr(Box::new(AstType::I8))
                                    }
                                    BasicValueEnum::StructValue(struct_val) => {
                                        // For struct values that are enums (Option, Result), check their structure
                                        let struct_type = struct_val.get_type();
                                        if struct_type.count_fields() == 2 {
                                            // This might be an enum struct (tag + payload)
                                            // Check if it's an Option or Result by examining the method name
                                            if method == "get" {
                                                // HashMap.get returns Option<V>
                                                // Try to get the value type from the object type
                                                if let Expression::Identifier(obj_name) = &**object {
                                                    if let Some(var_info) = self.variables.get(obj_name) {
                                                        if let AstType::Generic { name, type_args, .. } = &var_info.ast_type {
                                                            if name == "HashMap" && type_args.len() >= 2 {
                                                                // Return Option<V> where V is the value type
                                                                AstType::Generic {
                                                                    name: "Option".to_string(),
                                                                    type_args: vec![type_args[1].clone()],
                                                                }
                                                            } else {
                                                                // Not a HashMap, default to Option<i32>
                                                                AstType::Generic {
                                                                    name: "Option".to_string(),
                                                                    type_args: vec![AstType::I32],
                                                                }
                                                            }
                                                        } else {
                                                            // No generic type info, default to Option<i32>
                                                            AstType::Generic {
                                                                name: "Option".to_string(),
                                                                type_args: vec![AstType::I32],
                                                            }
                                                        }
                                                    } else {
                                                        // Variable not found, default to Option<i32>
                                                        AstType::Generic {
                                                            name: "Option".to_string(),
                                                            type_args: vec![AstType::I32],
                                                        }
                                                    }
                                                } else {
                                                    // Object is not an identifier, default to Option<i32>
                                                    AstType::Generic {
                                                        name: "Option".to_string(),
                                                        type_args: vec![AstType::I32],
                                                    }
                                                }
                                            } else if method == "raise" {
                                                // .raise() extracts the Ok value from Result<T,E>
                                                // Check if raise() stored what type it extracted
                                                if let Some(extracted_type) = self.generic_type_context.get("Last_Raise_Extracted_Type") {
                                                    // eprintln!("[DEBUG VAR] Variable {} from raise() gets type {:?}", name, extracted_type);
                                                    extracted_type.clone()
                                                } else {
                                                    // eprintln!("[DEBUG VAR] Variable {} from raise() - no extracted type found, defaulting to I32", name);
                                                    // Fallback when no type was stored
                                                    AstType::I32
                                                }
                                            } else {
                                                // For other methods returning structs, try to determine from context
                                                // For now, default to I32
                                                AstType::I32
                                            }
                                        } else {
                                            // Not an enum struct, default to I32
                                            AstType::I32
                                        }
                                    }
                                    _ => AstType::I64,
                                }
                            }
                        } else if let Expression::FunctionCall { name, .. } = init_expr {
                            // Check if this is a generic type constructor like HashMap<K,V>()
                            if name.contains('<') && name.contains('>') {
                                // Parse the generic type from the name
                                if let Some(angle_pos) = name.find('<') {
                                    let base_type = &name[..angle_pos];
                                    let type_params_str = &name[angle_pos+1..name.len()-1];
                                    
                                    // Parse type parameters (handling nested generics)
                                    let type_args: Vec<AstType> = self.parse_comma_separated_types(type_params_str);
                                    
                                    AstType::Generic {
                                        name: base_type.to_string(),
                                        type_args,
                                    }
                                } else {
                                    // Couldn't parse, fallback to regular function call handling
                                    if let Some(return_type) = self.function_types.get(name) {
                                        return_type.clone()
                                    } else {
                                        AstType::I32
                                    }
                                }
                            } else if let Some(return_type) = self.function_types.get(name) {
                                // Regular function call, get the return type from function_types
                                return_type.clone()
                            } else {
                                // Fallback to type inference from value
                                match value {
                                    BasicValueEnum::IntValue(int_val) => {
                                        let bit_width = int_val.get_type().get_bit_width();
                                        if bit_width == 1 {
                                            AstType::Bool
                                        } else if bit_width <= 32 {
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
                        } else if let Expression::EnumVariant {
                            enum_name,
                            variant,
                            payload,
                        } = init_expr
                        {
                            // Direct enum variant: Result.Ok, Option.Some, etc.
                            // Infer type directly from the payload to handle nested generics correctly
                            if enum_name == "Result" {
                                if variant == "Ok" {
                                    // Infer Ok type from the payload
                                    let ok_type = if let Some(p) = payload {
                                        self.infer_expression_type(p).unwrap_or(AstType::I32)
                                    } else {
                                        AstType::Void
                                    };
                                    
                                    // For Result.Ok, we don't know the error type yet, default to String
                                    AstType::Generic {
                                        name: enum_name.clone(),
                                        type_args: vec![ok_type, AstType::String],
                                    }
                                } else if variant == "Err" {
                                    // Infer Err type from the payload
                                    let err_type = if let Some(p) = payload {
                                        self.infer_expression_type(p).unwrap_or(AstType::String)
                                    } else {
                                        AstType::String
                                    };
                                    
                                    // For Result.Err, we don't know the ok type yet, default to I32
                                    AstType::Generic {
                                        name: enum_name.clone(),
                                        type_args: vec![AstType::I32, err_type],
                                    }
                                } else {
                                    // Unknown variant, use tracked types
                                    let ok_type = self.generic_type_context.get("Result_Ok_Type")
                                        .cloned()
                                        .unwrap_or(AstType::I32);
                                    let err_type = self.generic_type_context.get("Result_Err_Type")
                                        .cloned()
                                        .unwrap_or(AstType::String);
                                    
                                    AstType::Generic {
                                        name: enum_name.clone(),
                                        type_args: vec![ok_type, err_type],
                                    }
                                }
                            } else if enum_name == "Option" {
                                let some_type = if variant == "Some" {
                                    // Infer type from payload
                                    if let Some(p) = payload {
                                        self.infer_expression_type(p).unwrap_or(AstType::I32)
                                    } else {
                                        AstType::Void
                                    }
                                } else {
                                    // None variant, check context
                                    self.generic_type_context.get("Option_Some_Type")
                                        .cloned()
                                        .unwrap_or(AstType::Void)
                                };
                                
                                AstType::Generic {
                                    name: enum_name.clone(),
                                    type_args: vec![some_type],
                                }
                            } else {
                                // Other enums without generics
                                AstType::Generic {
                                    name: enum_name.clone(),
                                    type_args: vec![],
                                }
                            }
                        } else if let Expression::Raise(inner_expr) = init_expr {
                            // .raise() extracts the Ok value from Result<T, E>
                            // Check if the inner expression is a function call to determine the type
                            if let Expression::FunctionCall { name, .. } = &**inner_expr {
                                // Check if we know the function's return type
                                if let Some(return_type) = self.function_types.get(name) {
                                    // If the function returns Result<T, E>, extract T
                                    match return_type {
                                        AstType::Generic { name, type_args }
                                            if name == "Result" && !type_args.is_empty() =>
                                        {
                                            // Return the first type argument (T from Result<T, E>)
                                            type_args[0].clone()
                                        }
                                        _ => {
                                            // Default to i32 if we can't determine
                                            AstType::I32
                                        }
                                    }
                                } else {
                                    // Function type unknown, default to i32
                                    AstType::I32
                                }
                            } else if let Expression::MethodCall { object, method, .. } = &**inner_expr {
                                // Handle method calls like variable.raise()
                                if method == "raise" {
                                    // This is a nested raise - the object should be a Result
                                    if let Expression::Identifier(var_name) = &**object {
                                        // Look up the variable's type
                                        if let Some(var_info) = self.variables.get(var_name) {
                                            // If it's Result<T, E>, extract T
                                            match &var_info.ast_type {
                                                AstType::Generic { name, type_args }
                                                    if name == "Result" && !type_args.is_empty() =>
                                                {
                                                    type_args[0].clone()
                                                }
                                                _ => {
                                                    // Check if we stored the extracted type during raise
                                                    if let Some(extracted_type) = self.generic_type_context.get("Last_Raise_Extracted_Type") {
                                                        extracted_type.clone()
                                                    } else {
                                                        AstType::I32
                                                    }
                                                }
                                            }
                                        } else {
                                            // Check if we stored the extracted type during raise
                                            if let Some(extracted_type) = self.generic_type_context.get("Last_Raise_Extracted_Type") {
                                                extracted_type.clone()
                                            } else {
                                                AstType::I32
                                            }
                                        }
                                    } else {
                                        // Check if we stored the extracted type during raise
                                        if let Some(extracted_type) = self.generic_type_context.get("Last_Raise_Extracted_Type") {
                                            extracted_type.clone()
                                        } else {
                                            AstType::I32
                                        }
                                    }
                                } else {
                                    // Some other method call
                                    // Check if we stored the extracted type during raise
                                    if let Some(extracted_type) = self.generic_type_context.get("Last_Raise_Extracted_Type") {
                                        extracted_type.clone()
                                    } else {
                                        AstType::I32
                                    }
                                }
                            } else {
                                // Check if we stored the extracted type during raise
                                if let Some(extracted_type) = self.generic_type_context.get("Last_Raise_Extracted_Type") {
                                    extracted_type.clone()
                                } else {
                                    // For now, assume Result<i32, E> since that's the common case
                                    // This will be properly fixed when we have full generic type instantiation
                                    AstType::I32
                                }
                            }
                        } else if let Expression::MethodCall {
                            object, method, ..
                        } = init_expr
                        {
                            // Handle other method calls
                            // Check if this is Array.new() 
                            if let Expression::Identifier(name) = &**object {
                                if name == "Array" && method == "new" {
                                    // This is Array<T>.new() - return Array<i32> for now
                                    // TODO: Properly infer T from type arguments
                                    return Ok(());  // The type was already set by explicit type annotation
                                }
                            }
                            
                            // Check the method name first for known return types
                            match method.as_str() {
                                "raise" => {
                                    // For .raise(), get the type of the object and extract T from Result<T,E>
                                    // eprintln!("[DEBUG] Processing raise in MethodCall case");
                                    let object_type = self.infer_expression_type(object).unwrap_or(AstType::Void);
                                    // eprintln!("[DEBUG] Object type for raise: {:?}", object_type);
                                    
                                    match object_type {
                                        AstType::Generic { name, type_args } if name == "Result" && type_args.len() >= 1 => {
                                            let extracted = type_args[0].clone();
                                            // eprintln!("[DEBUG] Extracted type from Result: {:?}", extracted);
                                            extracted
                                        }
                                        _ => {
                                            // eprintln!("[DEBUG] Not a Result type (was {:?}), defaulting to I32", object_type);
                                            AstType::I32
                                        }
                                    }
                                }
                                "pop" => AstType::Generic {
                                    name: "Option".to_string(),
                                    type_args: vec![AstType::I32],  // Array.pop() returns Option<i32> for now
                                },
                                "to_f64" => AstType::Generic {
                                    name: "Option".to_string(),
                                    type_args: vec![AstType::F64],
                                },
                                "to_f32" => AstType::Generic {
                                    name: "Option".to_string(),
                                    type_args: vec![AstType::F32],
                                },
                                "to_i32" => AstType::Generic {
                                    name: "Option".to_string(),
                                    type_args: vec![AstType::I32],
                                },
                                "to_i64" => AstType::Generic {
                                    name: "Option".to_string(),
                                    type_args: vec![AstType::I64],
                                },
                                _ => {
                                    // For other methods, try to infer from the returned value
                                    match value {
                                        BasicValueEnum::IntValue(int_val) => {
                                            let bit_width = int_val.get_type().get_bit_width();
                                            if bit_width == 1 {
                                                AstType::Bool
                                            } else if bit_width <= 32 {
                                                AstType::I32
                                            } else {
                                                AstType::I64
                                            }
                                        }
                                        BasicValueEnum::FloatValue(_) => AstType::F64,
                                        BasicValueEnum::PointerValue(_) => {
                                            AstType::Ptr(Box::new(AstType::I8))
                                        }
                                        BasicValueEnum::StructValue(_) => {
                                            // Could be DynVec or other struct type
                                            AstType::Generic {
                                                name: String::new(),
                                                type_args: vec![],
                                            }
                                        }
                                        _ => AstType::I64,
                                    }
                                }
                            }
                        } else if let Expression::MemberAccess { object, member } = init_expr {
                            // Check if this is an enum variant access (e.g., GameEntity.Player)
                            if let Expression::Identifier(enum_name) = &**object {
                                // Check if this identifier is an enum type
                                if let Some(super::symbols::Symbol::EnumType(_)) =
                                    self.symbols.lookup(enum_name)
                                {
                                    // This is an enum variant, use the Generic type which is how enums are represented in AST
                                    AstType::Generic {
                                        name: enum_name.clone(),
                                        type_args: vec![],
                                    }
                                } else {
                                    // Regular member access - it's a struct field
                                    // Try to infer the type from the parent struct
                                    if let Some(var_info) = self.variables.get(enum_name) {
                                        // Get the type of the parent struct
                                        match &var_info.ast_type {
                                            AstType::Struct {
                                                name: struct_name, ..
                                            } => {
                                                // Look up the field type in the struct
                                                if let Some(struct_info) =
                                                    self.struct_types.get(struct_name)
                                                {
                                                    if let Some((_, field_type)) =
                                                        struct_info.fields.get(member)
                                                    {
                                                        // Return the actual field type
                                                        field_type.clone()
                                                    } else {
                                                        // Field not found, fallback to value inference
                                                        match value {
                                                            BasicValueEnum::StructValue(_) => {
                                                                // Try to infer struct type from context
                                                                // For now, use a placeholder struct type
                                                                AstType::Struct {
                                                                    name: "unknown".to_string(),
                                                                    fields: vec![],
                                                                }
                                                            }
                                                            _ => AstType::I64,
                                                        }
                                                    }
                                                } else {
                                                    // Struct not found, fallback to value inference
                                                    match value {
                                                        BasicValueEnum::StructValue(_) => {
                                                            // Try to infer struct type from context
                                                            // For now, use a placeholder struct type
                                                            AstType::Struct {
                                                                name: "unknown".to_string(),
                                                                fields: vec![],
                                                            }
                                                        }
                                                        _ => AstType::I64,
                                                    }
                                                }
                                            }
                                            _ => {
                                                // Not a struct, fallback to value inference
                                                match value {
                                                    BasicValueEnum::StructValue(_) => {
                                                        // Could be DynVec or other struct type
                                                        // Check if it matches DynVec structure (3 or 4 fields)
                                                        AstType::DynVec {
                                                            element_types: vec![AstType::I32],
                                                            allocator_type: None,
                                                        }
                                                    }
                                                    _ => AstType::I64,
                                                }
                                            }
                                        }
                                    } else {
                                        // Variable not found, fallback to value inference
                                        match value {
                                            BasicValueEnum::StructValue(_) => {
                                                // Could be DynVec or other struct type
                                                // Check if it matches DynVec structure (3 or 4 fields)
                                                AstType::DynVec {
                                                    element_types: vec![AstType::I32],
                                                    allocator_type: None,
                                                }
                                            }
                                            _ => AstType::I64,
                                        }
                                    }
                                }
                            } else {
                                // Complex member access (e.g., nested)
                                // For now fallback to value inference
                                match value {
                                    BasicValueEnum::StructValue(_) => {
                                        // Could be DynVec or other struct type
                                        // Check if it matches DynVec structure (3 or 4 fields)
                                        AstType::DynVec {
                                            element_types: vec![AstType::I32],
                                            allocator_type: None,
                                        }
                                    }
                                    _ => AstType::I64,
                                }
                            }
                        } else {
                            match value {
                                BasicValueEnum::IntValue(int_val) => {
                                    let bit_width = int_val.get_type().get_bit_width();
                                    if bit_width == 1 {
                                        AstType::Bool
                                    } else if bit_width <= 32 {
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
                                        AstType::StaticLiteral  // String literals are compile-time
                                    } else {
                                        AstType::Ptr(Box::new(AstType::I8)) // Generic pointer type
                                    }
                                }
                                BasicValueEnum::StructValue(_) => {
                                    // For struct values (including enums), try to infer from expression
                                    if let Expression::Some(inner_expr) = init_expr {
                                        // Option::Some variant - infer Option<T> where T is the type of inner_expr
                                        let inner_type = self.infer_expression_type(inner_expr);
                                        match inner_type {
                                            Ok(t) => AstType::Generic {
                                                name: "Option".to_string(),
                                                type_args: vec![t],
                                            },
                                            Err(_) => AstType::Generic {
                                                name: "Option".to_string(),
                                                type_args: vec![AstType::I32], // Default to i32 if we can't infer
                                            },
                                        }
                                    } else if let Expression::None = init_expr {
                                        // Option::None variant - use Option<T> with generic T
                                        AstType::Generic {
                                            name: "Option".to_string(),
                                            type_args: vec![AstType::Generic {
                                                name: "T".to_string(),
                                                type_args: vec![],
                                            }],
                                        }
                                    } else {
                                        // For other struct values, use an empty generic type as fallback
                                        // This should be improved to handle other enum types properly
                                        AstType::Generic {
                                            name: String::new(),
                                            type_args: vec![],
                                        }
                                    }
                                }
                                _ => AstType::I64, // Default
                            }
                        };
                        
                        // If this is a closure with a Result return type, track it in function_types
                        // so that .raise() can work correctly when calling the closure
                        if let AstType::FunctionPointer { return_type, .. } = &inferred_type {
                            if let AstType::Generic { name: type_name, .. } = &**return_type {
                                if type_name == "Result" {
                                    // Track this closure's return type for later use
                                    self.function_types.insert(name.clone(), (**return_type).clone());
                                }
                            }
                        }
                        
                        // eprintln!("[DEBUG VAR STORED] Variable {} stored with type {:?}", name, inferred_type);
                        
                        // Track generic type parameters for this variable
                        if let AstType::Generic { name: type_name, type_args } = &inferred_type {
                            if type_name == "Result" && type_args.len() == 2 {
                                // Track Result<T,E> types with variable-specific keys
                                self.track_generic_type(format!("{}_Result_Ok_Type", name), type_args[0].clone());
                                self.track_generic_type(format!("{}_Result_Err_Type", name), type_args[1].clone());
                            } else if type_name == "Option" && type_args.len() == 1 {
                                // Track Option<T> types with variable-specific keys
                                self.track_generic_type(format!("{}_Option_Some_Type", name), type_args[0].clone());
                            } else if type_name == "HashMap" && type_args.len() == 2 {
                                // Track HashMap<K,V> types with variable-specific keys
                                self.track_generic_type(format!("{}_HashMap_Key_Type", name), type_args[0].clone());
                                self.track_generic_type(format!("{}_HashMap_Value_Type", name), type_args[1].clone());
                            }
                        }
                        self.variables.insert(
                            name.clone(),
                            super::VariableInfo {
                                pointer: alloca,
                                ast_type: inferred_type.clone(),
                                is_mutable: *is_mutable,
                                is_initialized: true,
                            },
                        );
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
                    self.builder
                        .build_store(alloca, zero)
                        .map_err(|e| CompileError::from(e))?;

                    if let Some(type_) = type_ {
                        self.variables.insert(
                            name.clone(),
                            super::VariableInfo {
                                pointer: alloca,
                                ast_type: type_.clone(),
                                is_mutable: *is_mutable,
                                is_initialized: false, // Forward declaration without initializer
                            },
                        );
                        Ok(())
                    } else {
                        // For inferred types without initializer, default to i64
                        self.variables.insert(
                            name.clone(),
                            super::VariableInfo {
                                pointer: alloca,
                                ast_type: AstType::I64,
                                is_mutable: *is_mutable,
                                is_initialized: false, // Forward declaration without initializer
                            },
                        );
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
                                self.builder
                                    .build_int_s_extend(*int_val, self.context.i64_type(), "sext")
                                    .unwrap()
                                    .into()
                            } else {
                                (*int_val).into()
                            }
                        }
                        _ => value.clone(),
                    };
                    // Use struct field assignment
                    // Extract struct name from the AstType
                    let type_struct_name = match &struct_type {
                        AstType::Struct { name, .. } => name.clone(),
                        _ => {
                            return Err(CompileError::TypeError(
                                format!("Expected struct type for field assignment, got {:?}", struct_type),
                                None,
                            ));
                        }
                    };
                    self.compile_struct_field_assignment(struct_alloca, field_name, value, &type_struct_name)?;
                    return Ok(());
                }

                // Check if variable exists - if not, this is a new immutable declaration with =
                if !self.variables.contains_key(name) {
                    // eprintln!("DEBUG: Creating new immutable variable '{}'", name);
                    // This is a new immutable declaration: name = value

                    // First check if this is a closure to get proper type info BEFORE compiling
                    let (init_value, var_type) = match value {
                        Expression::Closure { params, return_type, body } => {
                            // Build the function type from the closure parameters
                            let param_types: Vec<AstType> = params
                                .iter()
                                .map(|(_, opt_type)| opt_type.clone().unwrap_or(AstType::I32))
                                .collect();

                            // Use the explicit return type if provided, otherwise infer
                            let ret_type = if let Some(explicit_type) = return_type {
                                Box::new(explicit_type.clone())
                            } else {
                                // Use the more sophisticated type inference that handles Result<T,E> and other generics
                                let inferred_type = self.infer_closure_return_type(body)?;
                                Box::new(inferred_type)
                            };

                            let func_type = AstType::FunctionPointer {
                                param_types,
                                return_type: ret_type.clone(),
                            };
                            
                            // Also track the function's return type for later use when called
                            self.function_types.insert(name.clone(), *ret_type.clone());

                            // Now compile the closure
                            let closure_value = self.compile_expression(value)?;

                            (closure_value, func_type)
                        }
                        _ => {
                            // Compile the value first to infer its type
                            let init_value = self.compile_expression(value)?;

                            // Infer the type from the value
                            let var_type = match init_value {
                                BasicValueEnum::IntValue(int_val) => {
                                    let bit_width = int_val.get_type().get_bit_width();
                                    if bit_width == 1 {
                                        AstType::Bool
                                    } else if bit_width <= 32 {
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

                            (init_value, var_type)
                        }
                    };

                    // Create the alloca and store the value
                    // Special handling for struct values (including enums)
                    // eprintln!("DEBUG: init_value type: {:?}", std::mem::discriminant(&init_value));
                    if let BasicValueEnum::StructValue(struct_val) = init_value {
                        // eprintln!("DEBUG: Handling struct value");
                        // Directly use the struct type from the value
                        let struct_type = struct_val.get_type();
                        let alloca = self.builder.build_alloca(struct_type, name)?;
                        self.builder.build_store(alloca, struct_val)?;
                        // Check what type of value this is
                        let inferred_type = match value {
                            Expression::StructLiteral {
                                name: struct_name, ..
                            } => {
                                // This is a struct literal
                                AstType::Struct {
                                    name: struct_name.clone(),
                                    fields: vec![], // Will be populated from struct_types if needed
                                }
                            }
                            Expression::MemberAccess { object, member: _ } => {
                                if let Expression::Identifier(enum_name) = &**object {
                                    // Check if this is an enum type
                                    if let Some(super::symbols::Symbol::EnumType(_)) =
                                        self.symbols.lookup(enum_name)
                                    {
                                        AstType::Generic {
                                            name: enum_name.clone(),
                                            type_args: vec![],
                                        }
                                    } else {
                                        // Generic enum type
                                        AstType::Generic {
                                            name: String::new(),
                                            type_args: vec![],
                                        }
                                    }
                                } else {
                                    // Generic enum type
                                    AstType::Generic {
                                        name: String::new(),
                                        type_args: vec![],
                                    }
                                }
                            }
                            Expression::Range { inclusive, .. } => {
                                // Range expression
                                AstType::Range {
                                    start_type: Box::new(AstType::I32),
                                    end_type: Box::new(AstType::I32),
                                    inclusive: *inclusive,
                                }
                            }
                            Expression::MethodCall { object: _, method, .. } => {
                                // Check method name to determine return type
                                // These string conversion methods return Option<T>
                                match method.as_str() {
                                    "to_f64" => AstType::Generic {
                                        name: "Option".to_string(),
                                        type_args: vec![AstType::F64],
                                    },
                                    "to_f32" => AstType::Generic {
                                        name: "Option".to_string(),
                                        type_args: vec![AstType::F32],
                                    },
                                    "to_i32" => AstType::Generic {
                                        name: "Option".to_string(),
                                        type_args: vec![AstType::I32],
                                    },
                                    "to_i64" => AstType::Generic {
                                        name: "Option".to_string(),
                                        type_args: vec![AstType::I64],
                                    },
                                    _ => AstType::Generic {
                                        name: String::new(),
                                        type_args: vec![],
                                    }
                                }
                            }
                            _ => {
                                // Default to enum type for other cases
                                AstType::Generic {
                                    name: String::new(),
                                    type_args: vec![],
                                }
                            }
                        };
                        self.variables.insert(
                            name.clone(),
                            super::VariableInfo {
                                pointer: alloca,
                                ast_type: inferred_type,
                                is_mutable: false, // Assignment with = creates immutable variables
                                is_initialized: true,
                            },
                        );
                    } else {
                        let llvm_type = self.to_llvm_type(&var_type)?;
                        let basic_type = match llvm_type {
                            Type::Basic(basic) => basic,
                            Type::Struct(struct_type) => struct_type.as_basic_type_enum(),
                            Type::Function(_) => self
                                .context
                                .ptr_type(inkwell::AddressSpace::default())
                                .as_basic_type_enum(),
                            _ => {
                                return Err(CompileError::TypeError(
                                    "Cannot allocate non-basic or struct type".to_string(),
                                    None,
                                ))
                            }
                        };
                        let alloca = self.builder.build_alloca(basic_type, name)?;
                        self.builder.build_store(alloca, init_value)?;
                        self.variables.insert(
                            name.clone(),
                            super::VariableInfo {
                                pointer: alloca,
                                ast_type: var_type,
                                is_mutable: false, // Assignment with = creates immutable variables
                                is_initialized: true,
                            },
                        );
                    }
                    return Ok(());
                }

                // Regular variable assignment to existing variable
                // First check if the variable is mutable or if this is the first assignment to a forward-declared variable
                if let Some(var_info) = self.variables.get(name) {
                    // Allow assignment if:
                    // 1. Variable is mutable, OR
                    // 2. Variable is immutable but not yet initialized (forward declaration)
                    if !var_info.is_mutable && var_info.is_initialized {
                        return Err(CompileError::TypeError(
                            format!("Cannot assign to immutable variable '{}'. Use '::=' to declare mutable variables.", name), 
                            None
                        ));
                    }
                }

                let (alloca, var_type) = self.get_variable(name)?;
                let value = self.compile_expression(value)?;
                let value = match (&value, &var_type) {
                    (BasicValueEnum::IntValue(int_val), AstType::I32) => {
                        if int_val.get_type().get_bit_width() != 32 {
                            self.builder
                                .build_int_truncate(*int_val, self.context.i32_type(), "trunc")
                                .unwrap()
                                .into()
                        } else {
                            (*int_val).into()
                        }
                    }
                    (BasicValueEnum::IntValue(int_val), AstType::I64) => {
                        if int_val.get_type().get_bit_width() != 64 {
                            self.builder
                                .build_int_s_extend(*int_val, self.context.i64_type(), "sext")
                                .unwrap()
                                .into()
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

                // Mark the variable as initialized after first assignment
                if let Some(var_info) = self.variables.get_mut(name) {
                    var_info.is_initialized = true;
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
                        let var_info = self.variables.get(name).cloned();

                        if let Some(var_info) = var_info {
                            let alloca = var_info.pointer;
                            let var_type = var_info.ast_type;
                            // Handle pointer to struct (p.x = value where p is *Point)
                            if let AstType::Ptr(inner_type) = &var_type {
                                // Check if inner type is a struct or a generic representing a struct
                                let struct_name = match &**inner_type {
                                    AstType::Struct { name, .. } => Some(name.clone()),
                                    AstType::Generic { name, type_args }
                                        if type_args.is_empty() =>
                                    {
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
                                    let struct_info =
                                        self.struct_types.get(&struct_name).ok_or_else(|| {
                                            CompileError::TypeError(
                                                format!("Struct type '{}' not found", struct_name),
                                                None,
                                            )
                                        })?;

                                    // Find field index
                                    let field_index = struct_info
                                        .fields
                                        .get(member)
                                        .map(|(idx, _)| *idx)
                                        .ok_or_else(|| {
                                            CompileError::TypeError(
                                                format!(
                                                    "Field '{}' not found in struct '{}'",
                                                    member, struct_name
                                                ),
                                                None,
                                            )
                                        })?;

                                    // Load the pointer value
                                    let ptr_type =
                                        self.context.ptr_type(inkwell::AddressSpace::default());
                                    let struct_ptr = self.builder.build_load(
                                        ptr_type,
                                        alloca,
                                        &format!("load_{}_ptr", name),
                                    )?;
                                    let struct_ptr = struct_ptr.into_pointer_value();

                                    // Build GEP to get field pointer
                                    let indices = vec![
                                        self.context.i32_type().const_zero(),
                                        self.context
                                            .i32_type()
                                            .const_int(field_index as u64, false),
                                    ];

                                    let field_ptr = unsafe {
                                        self.builder.build_gep(
                                            struct_info.llvm_type,
                                            struct_ptr,
                                            &indices,
                                            &format!("{}_{}__ptr", name, member),
                                        )?
                                    };

                                    // Compile and store the value
                                    let val = self.compile_expression(&value)?;
                                    self.builder.build_store(field_ptr, val)?;
                                    return Ok(());
                                }
                            }
                            // Handle regular struct (non-pointer)
                            else if let AstType::Struct {
                                name: _struct_name, ..
                            } = &var_type
                            {
                                // Use existing struct field assignment logic
                                let val = self.compile_expression(&value)?;
                                self.compile_struct_field_assignment(alloca, member, val, _struct_name)?;
                                return Ok(());
                            }
                        }
                    }
                    // If we get here, it's not a simple struct field assignment
                    return Err(CompileError::TypeError(
                        format!(
                            "Cannot assign to member access expression: {:?}.{}",
                            object, member
                        ),
                        None,
                    ));
                } else {
                    let ptr_val = self.compile_expression(&pointer)?;
                    let val = self.compile_expression(&value)?;

                    // For pointer variables, we need to load the address first, then store to that address
                    if ptr_val.is_pointer_value() {
                        let ptr = ptr_val.into_pointer_value();
                        // Load the address stored in the pointer variable
                        let address = self.builder.build_load(
                            self.context.ptr_type(inkwell::AddressSpace::default()),
                            ptr,
                            "deref_ptr",
                        )?;
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
            Statement::Loop {
                kind,
                body,
                label: _,
            } => {
                use crate::ast::LoopKind;

                match kind {
                    LoopKind::Infinite => {
                        // Create blocks for infinite loop
                        let loop_body = self
                            .context
                            .append_basic_block(self.current_function.unwrap(), "loop_body");
                        let after_loop_block = self
                            .context
                            .append_basic_block(self.current_function.unwrap(), "after_loop");

                        // Push loop context for break/continue
                        self.loop_stack.push((loop_body, after_loop_block));

                        // Jump to loop body
                        self.builder
                            .build_unconditional_branch(loop_body)
                            .map_err(|e| CompileError::from(e))?;
                        self.builder.position_at_end(loop_body);

                        // Compile body
                        for stmt in body {
                            self.compile_statement(stmt)?;
                        }

                        // Loop back if no terminator
                        let current_block = self.builder.get_insert_block().unwrap();
                        if current_block.get_terminator().is_none() {
                            self.builder
                                .build_unconditional_branch(loop_body)
                                .map_err(|e| CompileError::from(e))?;
                        }

                        self.loop_stack.pop();
                        self.builder.position_at_end(after_loop_block);
                        Ok(())
                    }
                    LoopKind::Condition(cond_expr) => {
                        // Create blocks
                        let loop_header = self
                            .context
                            .append_basic_block(self.current_function.unwrap(), "loop_header");
                        let loop_body = self
                            .context
                            .append_basic_block(self.current_function.unwrap(), "loop_body");
                        let after_loop_block = self
                            .context
                            .append_basic_block(self.current_function.unwrap(), "after_loop");

                        self.loop_stack.push((loop_header, after_loop_block));

                        // Jump to header
                        self.builder
                            .build_unconditional_branch(loop_header)
                            .map_err(|e| CompileError::from(e))?;
                        self.builder.position_at_end(loop_header);

                        // Evaluate condition
                        let cond_value = self.compile_expression(cond_expr)?;
                        if let BasicValueEnum::IntValue(int_val) = cond_value {
                            if int_val.get_type().get_bit_width() == 1 {
                                self.builder
                                    .build_conditional_branch(int_val, loop_body, after_loop_block)
                                    .map_err(|e| CompileError::from(e))?;
                            } else {
                                let zero = int_val.get_type().const_zero();
                                let condition = self
                                    .builder
                                    .build_int_compare(
                                        inkwell::IntPredicate::NE,
                                        int_val,
                                        zero,
                                        "loop_condition",
                                    )
                                    .map_err(|e| CompileError::from(e))?;
                                self.builder
                                    .build_conditional_branch(
                                        condition,
                                        loop_body,
                                        after_loop_block,
                                    )
                                    .map_err(|e| CompileError::from(e))?;
                            }
                        } else {
                            return Err(CompileError::TypeError(
                                "Loop condition must be an integer".to_string(),
                                None,
                            ));
                        }

                        // Compile body
                        self.builder.position_at_end(loop_body);
                        for stmt in body {
                            self.compile_statement(stmt)?;
                        }

                        // Loop back to header
                        let current_block = self.builder.get_insert_block().unwrap();
                        if current_block.get_terminator().is_none() {
                            self.builder
                                .build_unconditional_branch(loop_header)
                                .map_err(|e| CompileError::from(e))?;
                        }

                        self.loop_stack.pop();
                        self.builder.position_at_end(after_loop_block);
                        Ok(())
                    }
                }
            }
            Statement::Break { label: _ } => {
                // Branch to the break target (after_loop block) of the current loop
                if let Some((_, break_target)) = self.loop_stack.last() {
                    self.builder
                        .build_unconditional_branch(*break_target)
                        .map_err(|e| CompileError::from(e))?;
                    // Create a new block for any unreachable code after break
                    let unreachable_block = self
                        .context
                        .append_basic_block(self.current_function.unwrap(), "after_break");
                    self.builder.position_at_end(unreachable_block);
                } else {
                    return Err(CompileError::SyntaxError(
                        "Break statement outside of loop".to_string(),
                        None,
                    ));
                }
                Ok(())
            }
            Statement::Continue { label: _ } => {
                // Branch to the continue target (loop_header) of the current loop
                if let Some((continue_target, _)) = self.loop_stack.last() {
                    self.builder
                        .build_unconditional_branch(*continue_target)
                        .map_err(|e| CompileError::from(e))?;
                    // Create a new block for any unreachable code after continue
                    let unreachable_block = self
                        .context
                        .append_basic_block(self.current_function.unwrap(), "after_continue");
                    self.builder.position_at_end(unreachable_block);
                } else {
                    return Err(CompileError::SyntaxError(
                        "Continue statement outside of loop".to_string(),
                        None,
                    ));
                }
                Ok(())
            }
            Statement::ComptimeBlock(statements) => {
                // Evaluate comptime blocks during codegen
                for stmt in statements {
                    if let Err(e) = self.comptime_evaluator.execute_statement(stmt) {
                        return Err(CompileError::InternalError(
                            format!("Comptime evaluation error: {}", e),
                            None,
                        ));
                    }
                }
                Ok(())
            }
            Statement::ModuleImport { .. } => {
                // Module imports are handled during parsing, not codegen
                Ok(())
            }
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
            Statement::DestructuringImport { names, source } => {
                // Handle destructuring imports: { io, math } = @std
                // These imports make stdlib modules available in the current scope

                // Compile the source expression (e.g., @std)
                let _source_val = self.compile_expression(source)?;

                // For each name, create a module reference
                for name in names {
                    // Check if this is a stdlib function or module
                    match name.as_str() {
                        // Stdlib modules
                        "io" | "math" | "core" | "GPA" | "AsyncPool" | "Allocator" => {
                            // Create a marker variable to indicate this is a module reference
                            let alloca = self
                                .builder
                                .build_alloca(self.context.i64_type(), name)
                                .map_err(|e| CompileError::from(e))?;

                            let module_marker = match name.as_str() {
                                "io" => 1,
                                "math" => 2,
                                "core" => 3,
                                "GPA" => 4,
                                "AsyncPool" => 5,
                                "Allocator" => 6,
                                _ => 0,
                            };

                            self.builder
                                .build_store(
                                    alloca,
                                    self.context.i64_type().const_int(module_marker, false),
                                )
                                .map_err(|e| CompileError::from(e))?;

                            // Register the variable as a module reference
                            self.variables.insert(
                                name.clone(),
                                super::VariableInfo {
                                    pointer: alloca,
                                    ast_type: AstType::StdModule, // Mark as stdlib module reference
                                    is_mutable: false,            // Module references are immutable
                                    is_initialized: true,
                                },
                            );
                        }
                        // Math functions available directly from @std
                        "min" | "max" | "abs" => {
                            // Register as a function pointer type
                            let func_type = match name.as_str() {
                                "min" | "max" => AstType::FunctionPointer {
                                    param_types: vec![AstType::I32, AstType::I32],
                                    return_type: Box::new(AstType::I32),
                                },
                                "abs" => AstType::FunctionPointer {
                                    param_types: vec![AstType::I32],
                                    return_type: Box::new(AstType::I32),
                                },
                                _ => unreachable!(),
                            };

                            // Create a dummy alloca for the function reference
                            let alloca = self
                                .builder
                                .build_alloca(self.context.i64_type(), name)
                                .map_err(|e| CompileError::from(e))?;

                            // Store a special marker for builtin functions
                            self.builder
                                .build_store(
                                    alloca,
                                    self.context.i64_type().const_int(1000, false), // Special marker for builtins
                                )
                                .map_err(|e| CompileError::from(e))?;

                            self.variables.insert(
                                name.clone(),
                                super::VariableInfo {
                                    pointer: alloca,
                                    ast_type: func_type,
                                    is_mutable: false,
                                    is_initialized: true,
                                },
                            );
                        }
                        // Option type constructors
                        "Option" | "Some" | "None" => {
                            // For Option, Some, None, we don't need to create variables
                            // They're available as built-in enum types and constructors
                            // The expressions referring to them will be handled at compile time
                            
                            // Register Option as a type reference
                            if name == "Option" {
                                let alloca = self
                                    .builder
                                    .build_alloca(self.context.i64_type(), name)
                                    .map_err(|e| CompileError::from(e))?;

                                self.builder
                                    .build_store(
                                        alloca,
                                        self.context.i64_type().const_int(100, false), // Special marker for Option type
                                    )
                                    .map_err(|e| CompileError::from(e))?;

                                self.variables.insert(
                                    name.clone(),
                                    super::VariableInfo {
                                        pointer: alloca,
                                        ast_type: AstType::EnumType { name: "Option".to_string() },
                                        is_mutable: false,
                                        is_initialized: true,
                                    },
                                );
                            }
                            // Some and None don't need variables - they're used directly as constructors
                        }
                        // Result type constructors
                        "Result" | "Ok" | "Err" => {
                            // Similar to Option - Result is a type, Ok and Err are constructors
                            if name == "Result" {
                                let alloca = self
                                    .builder
                                    .build_alloca(self.context.i64_type(), name)
                                    .map_err(|e| CompileError::from(e))?;

                                self.builder
                                    .build_store(
                                        alloca,
                                        self.context.i64_type().const_int(101, false), // Special marker for Result type
                                    )
                                    .map_err(|e| CompileError::from(e))?;

                                self.variables.insert(
                                    name.clone(),
                                    super::VariableInfo {
                                        pointer: alloca,
                                        ast_type: AstType::EnumType { name: "Result".to_string() },
                                        is_mutable: false,
                                        is_initialized: true,
                                    },
                                );
                            }
                            // Ok and Err don't need variables - they're used directly as constructors
                        }
                        // Other stdlib entities
                        _ => {
                            // For now, create a generic marker
                            let alloca = self
                                .builder
                                .build_alloca(self.context.i64_type(), name)
                                .map_err(|e| CompileError::from(e))?;

                            self.builder
                                .build_store(
                                    alloca,
                                    self.context.i64_type().const_int(0, false),
                                )
                                .map_err(|e| CompileError::from(e))?;

                            self.variables.insert(
                                name.clone(),
                                super::VariableInfo {
                                    pointer: alloca,
                                    ast_type: AstType::StdModule,
                                    is_mutable: false,
                                    is_initialized: true,
                                },
                            );
                        }
                    }
                }

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
