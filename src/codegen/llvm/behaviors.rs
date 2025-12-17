use super::LLVMCompiler;
use crate::ast::{AstType, Expression, TraitImplementation};
use crate::error::CompileError;
use inkwell::types::BasicMetadataTypeEnum;
use inkwell::values::{BasicValueEnum, FunctionValue, PointerValue};
use std::collections::HashMap;

/// Manages behavior/trait implementations and method dispatch in LLVM
#[allow(dead_code)]
pub struct BehaviorCodegen<'ctx> {
    /// Maps (type_name, behavior_name) -> vtable global
    vtables: HashMap<(String, String), PointerValue<'ctx>>,
    /// Maps (type_name, method_name) -> function
    method_impls: HashMap<(String, String), FunctionValue<'ctx>>,
}

impl<'ctx> BehaviorCodegen<'ctx> {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            vtables: HashMap::new(),
            method_impls: HashMap::new(),
        }
    }

    /// Generate a vtable for a behavior implementation
    pub fn generate_vtable(
        &mut self,
        context: &'ctx inkwell::context::Context,
        module: &inkwell::module::Module<'ctx>,
        type_name: &str,
        behavior_name: &str,
        methods: &[(&str, FunctionValue<'ctx>)],
    ) -> Result<PointerValue<'ctx>, CompileError> {
        // Create vtable type: array of function pointers
        let fn_ptr_type = context.ptr_type(inkwell::AddressSpace::default());
        let field_types: Vec<_> = (0..methods.len()).map(|_| fn_ptr_type.into()).collect();
        let vtable_type = context.struct_type(&field_types, false);

        // Create global vtable
        let vtable_name = format!("vtable_{}_{}", type_name, behavior_name);
        let vtable_global = module.add_global(vtable_type, None, &vtable_name);

        // Initialize vtable with method pointers
        let mut method_ptrs = Vec::new();
        for (_, func) in methods {
            let ptr = func.as_global_value().as_pointer_value();
            method_ptrs.push(ptr.const_cast(fn_ptr_type));
        }

        let method_values: Vec<BasicValueEnum> =
            method_ptrs.into_iter().map(|ptr| ptr.into()).collect();
        let vtable_value = vtable_type.const_named_struct(&method_values);
        vtable_global.set_initializer(&vtable_value);

        let vtable_ptr = vtable_global.as_pointer_value();
        self.vtables.insert(
            (type_name.to_string(), behavior_name.to_string()),
            vtable_ptr,
        );

        Ok(vtable_ptr)
    }

    /// Register a method implementation
    pub fn register_method(
        &mut self,
        type_name: &str,
        method_name: &str,
        function: FunctionValue<'ctx>,
    ) {
        self.method_impls
            .insert((type_name.to_string(), method_name.to_string()), function);
    }

    /// Resolve a method call on a type
    pub fn resolve_method(
        &self,
        type_name: &str,
        method_name: &str,
    ) -> Option<FunctionValue<'ctx>> {
        self.method_impls
            .get(&(type_name.to_string(), method_name.to_string()))
            .copied()
    }
}

impl<'ctx> LLVMCompiler<'ctx> {
    /// Compile an impl block (inherent methods)
    pub fn compile_impl_block(
        &mut self,
        impl_block: &crate::ast::ImplBlock,
    ) -> Result<(), CompileError> {
        let type_name = &impl_block.type_name;

        // Set the current implementing type for proper 'self' resolution
        self.current_impl_type = Some(type_name.clone());

        // Process each method in the impl block
        for method in &impl_block.methods {
            // Generate a mangled name for the method: TypeName_methodName
            let mangled_name = format!("{}_{}", type_name, method.name);

            // Create LLVM function for the method
            let llvm_return_type = self.to_llvm_type(&method.return_type)?;

            let mut param_types = Vec::new();
            for (param_name, param_type) in &method.args {
                // Replace 'Self' with the actual type name
                let resolved_type = if param_name == "self" {
                    // For 'self', use the type name directly
                    // Check if it's a pointer type
                    if let crate::ast::AstType::Ptr(_)
                    | crate::ast::AstType::MutPtr(_)
                    | crate::ast::AstType::RawPtr(_) = param_type
                    {
                        param_type.clone() // Keep pointer types as-is
                    } else {
                        // For non-pointer self, create a pointer type
                        crate::ast::AstType::Ptr(Box::new(crate::ast::AstType::Generic {
                            name: type_name.clone(),
                            type_args: impl_block
                                .type_params
                                .iter()
                                .map(|tp| crate::ast::AstType::Generic {
                                    name: tp.name.clone(),
                                    type_args: vec![],
                                })
                                .collect(),
                        }))
                    }
                } else {
                    // Replace Self with actual type
                    if let crate::ast::AstType::Generic { name, .. } = param_type {
                        if name == "Self" {
                            crate::ast::AstType::Generic {
                                name: type_name.clone(),
                                type_args: vec![],
                            }
                        } else {
                            param_type.clone()
                        }
                    } else {
                        param_type.clone()
                    }
                };

                let llvm_param_type = self.to_llvm_type(&resolved_type)?;
                match llvm_param_type {
                    super::Type::Basic(basic_type) => {
                        param_types.push(BasicMetadataTypeEnum::from(basic_type));
                    }
                    super::Type::Struct(_struct_type) => {
                        // Pass struct by pointer
                        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                        param_types.push(BasicMetadataTypeEnum::from(ptr_type));
                    }
                    _ => {
                        return Err(CompileError::UnsupportedFeature(
                            format!("Unsupported parameter type in impl block method"),
                            None,
                        ))
                    }
                }
            }

            let fn_type = if let super::Type::Void = llvm_return_type {
                self.context.void_type().fn_type(&param_types, false)
            } else if let super::Type::Basic(basic_type) = llvm_return_type {
                match basic_type {
                    inkwell::types::BasicTypeEnum::IntType(int_type) => {
                        int_type.fn_type(&param_types, false)
                    }
                    inkwell::types::BasicTypeEnum::FloatType(float_type) => {
                        float_type.fn_type(&param_types, false)
                    }
                    inkwell::types::BasicTypeEnum::PointerType(ptr_type) => {
                        ptr_type.fn_type(&param_types, false)
                    }
                    inkwell::types::BasicTypeEnum::StructType(struct_type) => {
                        struct_type.fn_type(&param_types, false)
                    }
                    _ => {
                        return Err(CompileError::UnsupportedFeature(
                            format!("Unsupported return type in impl block method"),
                            None,
                        ));
                    }
                }
            } else if let super::Type::Struct(struct_type) = llvm_return_type {
                struct_type.fn_type(&param_types, false)
            } else {
                return Err(CompileError::UnsupportedFeature(
                    format!("Unsupported return type in impl block method"),
                    None,
                ));
            };
            let function = self.module.add_function(&mangled_name, fn_type, None);

            // Store method for later resolution via behavior_codegen
            if let Some(ref mut behavior_codegen) = self.behavior_codegen {
                behavior_codegen
                    .method_impls
                    .insert((type_name.clone(), method.name.clone()), function);
            }
        }

        Ok(())
    }

    /// Compile a trait implementation
    pub fn compile_trait_implementation(
        &mut self,
        trait_impl: &TraitImplementation,
    ) -> Result<(), CompileError> {
        let type_name = &trait_impl.type_name;
        let trait_name = &trait_impl.trait_name;

        // Set the current implementing type for proper 'self' resolution
        self.current_impl_type = Some(type_name.clone());

        // Process each method in the trait implementation
        for method in &trait_impl.methods {
            // Generate a mangled name for the method
            let mangled_name = format!("{}_{}_{}", type_name, trait_name, method.name);

            // Create LLVM function for the method
            let llvm_return_type = self.to_llvm_type(&method.return_type)?;

            let mut param_types = Vec::new();
            for (param_name, param_type) in &method.args {
                // eprintln!("DEBUG: Converting param '{}' type {:?} to LLVM", param_name, param_type);

                // For 'self' parameter, use the implementing type
                let actual_type = if param_name == "self" {
                    // If param_type is Generic { name: "Self" or "Self_Type", ... }, replace with the concrete type
                    match param_type {
                        crate::ast::AstType::Generic { name, .. }
                            if name == "Self" || name.starts_with("Self_") =>
                        {
                            // Look up the actual struct fields from struct_types
                            if let Some(struct_info) = self.struct_types.get(type_name) {
                                // Build the proper struct type with fields
                                let mut fields = Vec::new();
                                for (field_name, (_index, field_type)) in &struct_info.fields {
                                    fields.push((field_name.clone(), field_type.clone()));
                                }
                                crate::ast::AstType::Struct {
                                    name: type_name.clone(),
                                    fields,
                                }
                            } else {
                                // Fallback if struct not found - shouldn't happen
                                crate::ast::AstType::Struct {
                                    name: type_name.clone(),
                                    fields: vec![],
                                }
                            }
                        }
                        _ => param_type.clone(),
                    }
                } else {
                    param_type.clone()
                };

                // For struct types (including 'self'), pass by pointer
                let llvm_param_type = if param_name == "self"
                    || matches!(actual_type, crate::ast::AstType::Struct { .. })
                {
                    // Pass structs by pointer
                    let struct_type = self.to_llvm_type(&actual_type)?;
                    if let super::Type::Struct(_st) = struct_type {
                        super::Type::Basic(inkwell::types::BasicTypeEnum::PointerType(
                            self.context.ptr_type(inkwell::AddressSpace::default()),
                        ))
                    } else {
                        struct_type
                    }
                } else {
                    self.to_llvm_type(&actual_type)?
                };

                // eprintln!("DEBUG: LLVM param type: {:?}", llvm_param_type);
                match llvm_param_type {
                    super::Type::Basic(basic_type) => {
                        param_types.push(BasicMetadataTypeEnum::from(basic_type));
                        // eprintln!("DEBUG: Added param type to function signature");
                    }
                    super::Type::Struct(_struct_type) => {
                        // Pass struct by pointer - use context's pointer type
                        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                        param_types.push(BasicMetadataTypeEnum::from(ptr_type));
                        // eprintln!("DEBUG: Added struct pointer type to function signature");
                    }
                    _ => {
                        // eprintln!("DEBUG: Failed to convert type to function parameter");
                    }
                }
            }

            let fn_type = if let super::Type::Void = llvm_return_type {
                self.context.void_type().fn_type(&param_types, false)
            } else if let super::Type::Basic(basic_type) = llvm_return_type {
                match basic_type {
                    inkwell::types::BasicTypeEnum::IntType(int_type) => {
                        int_type.fn_type(&param_types, false)
                    }
                    inkwell::types::BasicTypeEnum::FloatType(float_type) => {
                        float_type.fn_type(&param_types, false)
                    }
                    inkwell::types::BasicTypeEnum::PointerType(ptr_type) => {
                        ptr_type.fn_type(&param_types, false)
                    }
                    inkwell::types::BasicTypeEnum::StructType(struct_type) => {
                        struct_type.fn_type(&param_types, false)
                    }
                    _ => {
                        return Err(CompileError::UnsupportedFeature(
                            format!("Unsupported method return type: {:?}", basic_type),
                            None,
                        ))
                    }
                }
            } else {
                return Err(CompileError::UnsupportedFeature(
                    format!(
                        "Method return type not yet supported: {:?}",
                        llvm_return_type
                    ),
                    None,
                ));
            };

            let function = self.module.add_function(&mangled_name, fn_type, None);

            // Set up the function body
            let entry = self.context.append_basic_block(function, "entry");
            self.builder.position_at_end(entry);

            // Store the current function
            let prev_function = self.current_function;
            self.current_function = Some(function);

            // Add parameters to symbol table and variables map
            self.symbols.enter_scope();
            // eprintln!("DEBUG: Function has {} params, method has {} args", function.count_params(), method.args.len());
            for (i, (param_name, param_type)) in method.args.iter().enumerate() {
                // eprintln!("DEBUG: Processing param {} - '{}'", i, param_name);
                if i < function.count_params() as usize {
                    let param_value = function.get_nth_param(i as u32).unwrap();
                    let alloca = self
                        .builder
                        .build_alloca(param_value.get_type(), param_name)?;
                    self.builder.build_store(alloca, param_value)?;
                    // eprintln!("DEBUG: Inserting parameter '{}' into symbols and variables", param_name);
                    self.symbols
                        .insert(param_name.clone(), super::symbols::Symbol::Variable(alloca));

                    // Also add to variables map for struct field access
                    // For 'self' parameter, it's a pointer to the implementing type
                    let actual_type = if param_name == "self" {
                        // Check if it's a Self type that needs resolution
                        let needs_resolution = match param_type {
                            crate::ast::AstType::Generic { name, .. } => {
                                name == "Self" || name.starts_with("Self_")
                            }
                            _ => false,
                        };

                        if needs_resolution {
                            // Look up the actual struct fields from struct_types
                            let struct_type =
                                if let Some(struct_info) = self.struct_types.get(type_name) {
                                    // Build the proper struct type with fields
                                    let mut fields = Vec::new();
                                    for (field_name, (_index, field_type)) in &struct_info.fields {
                                        fields.push((field_name.clone(), field_type.clone()));
                                    }
                                    crate::ast::AstType::Struct {
                                        name: type_name.clone(),
                                        fields,
                                    }
                                } else {
                                    // Fallback if struct not found - shouldn't happen
                                    crate::ast::AstType::Struct {
                                        name: type_name.clone(),
                                        fields: vec![],
                                    }
                                };
                            crate::ast::AstType::Ptr(Box::new(struct_type))
                        } else {
                            // Not a Self type, use the parameter type as-is
                            param_type.clone()
                        }
                    } else {
                        param_type.clone()
                    };

                    self.variables.insert(
                        param_name.clone(),
                        super::VariableInfo {
                            pointer: alloca,
                            ast_type: actual_type.clone(),
                            is_mutable: false, // Function parameters are immutable by default
                            is_initialized: true,
                        },
                    );
                    // eprintln!("DEBUG: After inserting '{}', variables map has {} entries", param_name, self.variables.len());
                }
            }

            // Compile method body
            // eprintln!("DEBUG: Before compiling method body, variables map has {} entries", self.variables.len());
            for (_k, _) in &self.variables {
                // eprintln!("DEBUG:   - Variable: {}", _k);
            }
            for stmt in &method.body {
                self.compile_statement(stmt)?;
            }

            // Add implicit return if needed
            if matches!(llvm_return_type, super::Type::Void)
                && self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
            {
                self.builder.build_return(None)?;
            }

            // Clean up
            self.symbols.exit_scope();
            self.variables.clear(); // Clear variables from this method before moving to next one
            self.current_function = prev_function;

            // Register the method in our behavior codegen
            if !self.behavior_codegen.is_some() {
                self.behavior_codegen = Some(BehaviorCodegen::new());
            }

            if let Some(ref mut behavior_codegen) = self.behavior_codegen {
                behavior_codegen.register_method(type_name, &method.name, function);
            }
        }

        // Generate vtable for the trait implementation
        let mut methods = Vec::new();

        for method in &trait_impl.methods {
            let mangled_name = format!("{}_{}_{}", type_name, trait_name, method.name);
            if let Some(func) = self.module.get_function(&mangled_name) {
                methods.push((method.name.as_str(), func));
            }
        }

        if let Some(ref mut behavior_codegen) = self.behavior_codegen {
            behavior_codegen.generate_vtable(
                self.context,
                &self.module,
                type_name,
                trait_name,
                &methods,
            )?;
        }

        // Clear the current implementing type
        self.current_impl_type = None;

        Ok(())
    }

    /// Compile a method call (e.g., obj.method(args))
    pub fn compile_method_call(
        &mut self,
        object: &Expression,
        method_name: &str,
        args: &[Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // First check if the object type is Array
        // Try to infer type from the expression
        if let Expression::Identifier(name) = object {
            // Check if this is an Array type - get what we need and drop the borrow
            let array_info = self.variables.get(name).and_then(|var_info| {
                if let AstType::Generic {
                    name: type_name, ..
                } = &var_info.ast_type
                {
                    if type_name == "Array" {
                        Some((var_info.pointer, var_info.ast_type.clone()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            });

            if let Some((array_ptr, _ast_type)) = array_info {
                // Compile the object to get the array value
                // Array struct type: { ptr, length, capacity }
                let array_struct_type = self.context.struct_type(
                    &[
                        self.context
                            .ptr_type(inkwell::AddressSpace::default())
                            .into(),
                        self.context.i64_type().into(),
                        self.context.i64_type().into(),
                    ],
                    false,
                );
                let object_val =
                    self.builder
                        .build_load(array_struct_type, array_ptr, "array_val")?;

                // Handle Array methods
                match method_name {
                    "push" => {
                        if args.len() != 1 {
                            return Err(CompileError::TypeError(
                                format!("Array.push expects 1 argument, got {}", args.len()),
                                None,
                            ));
                        }
                        let value = self.compile_expression(&args[0])?;
                        // Pass the pointer instead of the value for in-place modification
                        return super::functions::arrays::compile_array_push_by_ptr(
                            self, array_ptr, value,
                        );
                    }
                    "get" => {
                        if args.len() != 1 {
                            return Err(CompileError::TypeError(
                                format!("Array.get expects 1 argument, got {}", args.len()),
                                None,
                            ));
                        }
                        let index = self.compile_expression(&args[0])?;
                        let result =
                            super::functions::arrays::compile_array_get(self, object_val, index)?;
                        return Ok(result);
                    }
                    "len" => {
                        if args.len() != 0 {
                            return Err(CompileError::TypeError(
                                format!("Array.len expects no arguments, got {}", args.len()),
                                None,
                            ));
                        }
                        let result = super::functions::arrays::compile_array_len(self, object_val)?;
                        return Ok(result);
                    }
                    "set" => {
                        if args.len() != 2 {
                            return Err(CompileError::TypeError(
                                format!("Array.set expects 2 arguments, got {}", args.len()),
                                None,
                            ));
                        }
                        let index = self.compile_expression(&args[0])?;
                        let value = self.compile_expression(&args[1])?;
                        let result = super::functions::arrays::compile_array_set(
                            self, object_val, index, value,
                        )?;
                        return Ok(result);
                    }
                    "pop" => {
                        if args.len() != 0 {
                            return Err(CompileError::TypeError(
                                format!("Array.pop expects no arguments, got {}", args.len()),
                                None,
                            ));
                        }
                        // Pass the pointer for in-place modification
                        let result =
                            super::functions::arrays::compile_array_pop_by_ptr(self, array_ptr)?;
                        return Ok(result);
                    }
                    _ => {
                        // Fall through to regular method handling
                    }
                }
            }
        }

        // For now, we'll implement static dispatch only
        // Dynamic dispatch would require trait objects

        // Get the type of the object
        // This is simplified - in a real implementation we'd need proper type tracking
        let type_name = self.infer_type_name(object)?;

        // Look up the method
        if let Some(ref behavior_codegen) = self.behavior_codegen {
            if let Some(function) = behavior_codegen.resolve_method(&type_name, method_name) {
                // Compile arguments
                let mut compiled_args = Vec::new();

                // First argument is 'self' - the object
                // We need to pass a pointer to the struct for self parameter
                let self_value = match object {
                    Expression::Identifier(name) => {
                        // If it's an identifier, get the pointer from variables map
                        if let Some(var_info) = self.variables.get(name) {
                            var_info.pointer.into()
                        } else {
                            self.compile_expression(object)?
                        }
                    }
                    _ => {
                        // For other expressions, compile and store to get a pointer
                        let value = self.compile_expression(object)?;
                        let alloca = self.builder.build_alloca(value.get_type(), "self_temp")?;
                        self.builder.build_store(alloca, value)?;
                        alloca.into()
                    }
                };
                compiled_args.push(self_value);

                // Compile remaining arguments
                for arg in args {
                    compiled_args.push(self.compile_expression(arg)?);
                }

                // Make the call
                let args_metadata: Vec<inkwell::values::BasicMetadataValueEnum> = compiled_args
                    .iter()
                    .map(|arg| inkwell::values::BasicMetadataValueEnum::try_from(*arg).unwrap())
                    .collect();

                let call_site = self
                    .builder
                    .build_call(function, &args_metadata, "method_call")?;

                return call_site.try_as_basic_value().left().ok_or_else(|| {
                    CompileError::TypeError(
                        "Method call returned void where value expected".to_string(),
                        None,
                    )
                });
            }
        }

        // Fallback to UFC (Uniform Function Call): object.method(args) -> method(object, args)
        // This handles cases like vec_ref.get(0) where vec_ref is &DynVec<String>
        // Try to find a function with the method name that takes the object as first parameter
        if let Some(_func_signature) = self.function_types.get(method_name) {
            // Check if first parameter matches the object type (or dereferenced type)
            // func_signature is AstType, we need to extract params from it
            // For now, just try calling the function - if it exists, it should work
            // Build the argument list: [object, ...args]
            let mut ufc_args = vec![object.clone()];
            ufc_args.extend_from_slice(args);

            // Try to call the function through UFC
            match super::functions::calls::compile_function_call(self, method_name, &ufc_args) {
                Ok(result) => return Ok(result),
                Err(_) => {
                    // Function not found or wrong signature, continue to error
                }
            }
        }

        Err(CompileError::UndeclaredFunction(
            format!("{}.{}", type_name, method_name),
            None,
        ))
    }

    /// Helper to infer type name from an expression (simplified)
    fn infer_type_name(&self, expr: &Expression) -> Result<String, CompileError> {
        match expr {
            Expression::Identifier(name) => {
                // Look up the variable's type in our type tracking
                if let Some(var_info) = self.variables.get(name) {
                    // Handle pointer/reference types by dereferencing
                    let effective_type = match &var_info.ast_type {
                        crate::ast::AstType::Ptr(inner)
                        | crate::ast::AstType::MutPtr(inner)
                        | crate::ast::AstType::RawPtr(inner) => {
                            // Dereference pointer to get inner type
                            inner.as_ref()
                        }
                        _ => &var_info.ast_type,
                    };

                    match effective_type {
                        crate::ast::AstType::Struct { name, .. } => Ok(name.clone()),
                        crate::ast::AstType::Generic { name, .. } => {
                            // For generic types like DynVec<String>, return the base name
                            Ok(name.clone())
                        }
                        crate::ast::AstType::DynVec { .. } => Ok("DynVec".to_string()),
                        crate::ast::AstType::Vec { .. } => Ok("Vec".to_string()),
                        _ => Ok("UnknownType".to_string()),
                    }
                } else {
                    // Try to infer from expression type
                    match self.infer_expression_type(expr) {
                        Ok(ast_type) => {
                            // Handle pointer types
                            let effective_type = match &ast_type {
                                crate::ast::AstType::Ptr(inner)
                                | crate::ast::AstType::MutPtr(inner)
                                | crate::ast::AstType::RawPtr(inner) => inner.as_ref(),
                                _ => &ast_type,
                            };

                            match effective_type {
                                crate::ast::AstType::Struct { name, .. } => Ok(name.clone()),
                                crate::ast::AstType::Generic { name, .. } => Ok(name.clone()),
                                crate::ast::AstType::DynVec { .. } => Ok("DynVec".to_string()),
                                crate::ast::AstType::Vec { .. } => Ok("Vec".to_string()),
                                _ => Ok("UnknownType".to_string()),
                            }
                        }
                        Err(_) => Ok("UnknownType".to_string()),
                    }
                }
            }
            Expression::StructLiteral { name, .. } => Ok(name.clone()),
            _ => {
                // Try to infer from expression type
                match self.infer_expression_type(expr) {
                    Ok(ast_type) => {
                        // Handle pointer types
                        let effective_type = match &ast_type {
                            crate::ast::AstType::Ptr(inner)
                            | crate::ast::AstType::MutPtr(inner)
                            | crate::ast::AstType::RawPtr(inner) => inner.as_ref(),
                            _ => &ast_type,
                        };

                        match effective_type {
                            crate::ast::AstType::Struct { name, .. } => Ok(name.clone()),
                            crate::ast::AstType::Generic { name, .. } => Ok(name.clone()),
                            crate::ast::AstType::DynVec { .. } => Ok("DynVec".to_string()),
                            crate::ast::AstType::Vec { .. } => Ok("Vec".to_string()),
                            _ => Ok("UnknownType".to_string()),
                        }
                    }
                    Err(_) => Ok("UnknownType".to_string()),
                }
            }
        }
    }
}
