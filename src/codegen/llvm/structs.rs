use super::{symbols, LLVMCompiler, Type};
use crate::ast::{AstType, Expression};
use crate::error::CompileError;
use inkwell::types::BasicType;
use inkwell::{types::StructType, values::BasicValueEnum};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct StructTypeInfo<'ctx> {
    /// The LLVM struct type
    pub llvm_type: StructType<'ctx>,
    /// Mapping from field name to (index, type)
    pub fields: HashMap<String, (usize, AstType)>,
}
// Move any struct registration, lookup, or field access helpers here as needed.

impl<'ctx> LLVMCompiler<'ctx> {
    pub fn compile_struct_literal(
        &mut self,
        name: &str,
        fields: &[(String, Expression)],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        let (llvm_type, fields_with_info) = {
            let struct_info = self.struct_types.get(name).ok_or_else(|| {
                CompileError::TypeError(format!("Undefined struct type: {}", name), None)
            })?;
            let mut fields_with_info = Vec::new();
            for (field_name, field_expr) in fields {
                let (field_index, field_type) =
                    struct_info.fields.get(field_name).ok_or_else(|| {
                        CompileError::TypeError(
                            format!("No field '{}' in struct '{}'", field_name, name),
                            None,
                        )
                    })?;
                fields_with_info.push((
                    field_name.clone(),
                    *field_index,
                    field_type.clone(),
                    field_expr.clone(),
                ));
            }
            fields_with_info.sort_by_key(|&(_, idx, _, _)| idx);
            (struct_info.llvm_type, fields_with_info)
        };
        let alloca = self
            .builder
            .build_alloca(llvm_type, &format!("{}_tmp", name))?;
        for (field_name, field_index, _field_type, field_expr) in fields_with_info {
            let field_val = self.compile_expression(&field_expr)?;
            let field_ptr = self.builder.build_struct_gep(
                llvm_type,
                alloca,
                field_index as u32,
                &format!("{}_ptr", field_name),
            )?;
            self.builder.build_store(field_ptr, field_val)?;
        }
        match self
            .builder
            .build_load(llvm_type, alloca, &format!("{}_val", name))
        {
            Ok(val) => Ok(val),
            Err(e) => Err(CompileError::InternalError(e.to_string(), None)),
        }
    }

    pub fn compile_struct_field(
        &mut self,
        struct_: &Expression,
        field: &str,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Special handling for identifiers
        if let Expression::Identifier(name) = struct_ {
            // First check if this is an enum type (GameEntity.Player syntax)
            if let Some(symbol) = self.symbols.lookup(name) {
                if let symbols::Symbol::EnumType(enum_info) = symbol {
                    // This is an enum variant creation, not a struct field access
                    if enum_info.variant_indices.contains_key(field) {
                        // Create the enum variant
                        return self.compile_enum_variant(name, field, &None);
                    } else {
                        return Err(CompileError::TypeError(
                            format!("Unknown variant '{}' for enum '{}'", field, name),
                            None,
                        ));
                    }
                }
            } else {
            }

            // Not an enum, check if it's a variable
            // First check self.variables (main storage)
            // eprintln!("DEBUG: Looking for variable '{}' in compile_struct_field", name);
            let (alloca, var_type) = if let Some(var_info) = self.variables.get(name) {
                // eprintln!("DEBUG: Found '{}' in variables with type: {:?}", name, var_info.ast_type);
                (var_info.pointer, var_info.ast_type.clone())
            } else if let Some(symbol) = self.symbols.lookup(name) {
                // Then check self.symbols (used in trait methods)
                if let symbols::Symbol::Variable(ptr) = symbol {
                    // For 'self' in trait methods, we need to infer the type
                    // eprintln!("DEBUG: Looking up type for '{}' in symbols", name);
                    let inferred_type = if name == "self" {
                        // Use the current implementing type if available
                        if let Some(impl_type) = &self.current_impl_type {
                            // Look up the actual struct fields
                            if let Some(struct_info) = self.struct_types.get(impl_type) {
                                let mut fields = Vec::new();
                                for (field_name, (_index, field_type)) in &struct_info.fields {
                                    fields.push((field_name.clone(), field_type.clone()));
                                }
                                AstType::Ptr(Box::new(AstType::Struct {
                                    name: impl_type.clone(),
                                    fields,
                                }))
                            } else {
                                AstType::Struct {
                                    name: impl_type.clone(),
                                    fields: vec![],
                                }
                            }
                        } else {
                            // Fallback: try to determine from registered struct types
                            if let Some((struct_name, _)) = self.struct_types.iter().next() {
                                AstType::Struct {
                                    name: struct_name.clone(),
                                    fields: vec![],
                                }
                            } else {
                                return Err(CompileError::TypeError(
                                    format!("Cannot determine type of 'self' in trait method"),
                                    None,
                                ));
                            }
                        }
                    } else {
                        // For other variables in symbols, use a generic type
                        AstType::Void
                    };
                    (*ptr, inferred_type)
                } else {
                    return Err(CompileError::TypeError(
                        format!("'{}' is not a variable", name),
                        None,
                    ));
                }
            } else {
                // Variable not found in either storage
                return Err(CompileError::UndeclaredVariable(name.to_string(), None));
            };

            // Check if this is a stdlib module reference (GPA.init())
            if var_type == AstType::StdModule {
                // This is a module method call like GPA.init()
                // For now, return a dummy value to allow compilation to proceed
                // Real implementation would create the proper type
                match field {
                    "init" => {
                        // Return a dummy struct value representing the allocator
                        // In a real implementation, this would create the actual allocator type
                        return Ok(self.context.i64_type().const_int(1, false).into());
                    }
                    _ => {
                        return Err(CompileError::TypeError(
                            format!("Unknown method '{}' for module '{}'", field, name),
                            None,
                        ));
                    }
                }
            }

            {
                let alloca = alloca; // Shadow to keep same name as before
                let var_type = &var_type; // Take reference to match original code
                                          // Handle pointer to struct (p.x where p is *Point)
                if let AstType::Ptr(inner_type) = var_type {
                    // Check if it's a struct or a generic type that represents a struct
                    let struct_name = match &**inner_type {
                        AstType::Struct { name, .. } => Some(name.clone()),
                        AstType::Generic { name, .. } => {
                            // Check if this generic is actually a registered struct type
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
                        let struct_info = self.struct_types.get(&struct_name).ok_or_else(|| {
                            CompileError::TypeError(
                                format!("Struct type '{}' not found", struct_name),
                                None,
                            )
                        })?;

                        // Find field index
                        let field_index = struct_info
                            .fields
                            .get(field)
                            .map(|(idx, _)| *idx)
                            .ok_or_else(|| {
                                CompileError::TypeError(
                                    format!(
                                        "Field '{}' not found in struct '{}'",
                                        field, struct_name
                                    ),
                                    None,
                                )
                            })?;

                        // Get field type
                        let field_type = struct_info
                            .fields
                            .get(field)
                            .map(|(_, ty)| ty.clone())
                            .unwrap();

                        // Load the pointer value
                        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                        let struct_ptr = self.builder.build_load(
                            ptr_type,
                            alloca,
                            &format!("load_{}_ptr", name),
                        )?;
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
                                &format!("{}_{}_ptr", name, field),
                            )?
                        };

                        // Load the field value
                        let field_llvm_type = self.to_llvm_type(&field_type)?;
                        let basic_type = match field_llvm_type {
                            Type::Basic(ty) => ty,
                            Type::Struct(st) => st.as_basic_type_enum(),
                            _ => {
                                return Err(CompileError::TypeError(
                                    "Field type must be basic type".to_string(),
                                    None,
                                ))
                            }
                        };

                        let value = self.builder.build_load(
                            basic_type,
                            field_ptr,
                            &format!("load_{}_{}", name, field),
                        )?;
                        return Ok(value);
                    }
                }

                // Handle regular struct (non-pointer)
                let struct_name = match var_type {
                    AstType::Struct { name, .. } => Some(name.clone()),
                    AstType::Generic { name, .. } => {
                        // Check if this generic is actually a registered struct type
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
                    let struct_info = self.struct_types.get(&struct_name).ok_or_else(|| {
                        CompileError::TypeError(
                            format!("Struct type '{}' not found", struct_name),
                            None,
                        )
                    })?;

                    // Find field index
                    let field_index = struct_info
                        .fields
                        .get(field)
                        .map(|(idx, _)| *idx)
                        .ok_or_else(|| {
                            CompileError::TypeError(
                                format!("Field '{}' not found in struct '{}'", field, struct_name),
                                None,
                            )
                        })?;

                    // Get field type
                    let field_type = struct_info
                        .fields
                        .get(field)
                        .map(|(_, ty)| ty.clone())
                        .unwrap();

                    // Build GEP to get field pointer
                    let indices = vec![
                        self.context.i32_type().const_zero(),
                        self.context.i32_type().const_int(field_index as u64, false),
                    ];

                    let field_ptr = unsafe {
                        self.builder.build_gep(
                            struct_info.llvm_type,
                            alloca,
                            &indices,
                            &format!("{}.{}", name, field),
                        )?
                    };

                    // Load the field value
                    let field_llvm_type = self.to_llvm_type(&field_type)?;
                    let basic_type = match field_llvm_type {
                        Type::Basic(ty) => ty,
                        Type::Struct(st) => st.as_basic_type_enum(),
                        _ => {
                            return Err(CompileError::TypeError(
                                "Field type must be basic type".to_string(),
                                None,
                            ))
                        }
                    };

                    let value = self.builder.build_load(
                        basic_type,
                        field_ptr,
                        &format!("load_{}", field),
                    )?;
                    return Ok(value);
                }
            } // End of inner scope
        }

        // Handle dereference case - when accessing field of a dereferenced pointer
        if let Expression::Dereference(inner) = struct_ {
            // Compile the inner expression to get the pointer
            let ptr_val = self.compile_expression(inner)?;

            if let BasicValueEnum::PointerValue(ptr) = ptr_val {
                // Load the pointer value to get the actual struct pointer
                let struct_ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                let struct_ptr =
                    self.builder
                        .build_load(struct_ptr_type, ptr, "load_struct_ptr")?;
                let struct_ptr = struct_ptr.into_pointer_value();

                // Now we need to find the struct type from the pointer
                // This is a bit tricky - we need to look up the variable type
                if let Expression::Identifier(ptr_name) = &**inner {
                    if let Some(var_info) = self.variables.get(ptr_name) {
                        let ptr_type = &var_info.ast_type;
                        if let AstType::Ptr(inner_type) = ptr_type {
                            if let AstType::Struct {
                                name: struct_name, ..
                            } = &**inner_type
                            {
                                // Get struct type info
                                let struct_info =
                                    self.struct_types.get(struct_name).ok_or_else(|| {
                                        CompileError::TypeError(
                                            format!("Struct type '{}' not found", struct_name),
                                            None,
                                        )
                                    })?;

                                // Find field index
                                let field_index = struct_info
                                    .fields
                                    .get(field)
                                    .map(|(idx, _)| *idx)
                                    .ok_or_else(|| {
                                        CompileError::TypeError(
                                            format!(
                                                "Field '{}' not found in struct '{}'",
                                                field, struct_name
                                            ),
                                            None,
                                        )
                                    })?;

                                // Get field type
                                let field_type = struct_info
                                    .fields
                                    .get(field)
                                    .map(|(_, ty)| ty.clone())
                                    .unwrap();

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
                                        &format!("{}->{}", ptr_name, field),
                                    )?
                                };

                                // Load the field value
                                let field_llvm_type = self.to_llvm_type(&field_type)?;
                                let basic_type = match field_llvm_type {
                                    Type::Basic(ty) => ty,
                                    Type::Struct(st) => st.as_basic_type_enum(),
                                    _ => {
                                        return Err(CompileError::TypeError(
                                            "Field type must be basic type".to_string(),
                                            None,
                                        ))
                                    }
                                };

                                let value = self.builder.build_load(
                                    basic_type,
                                    field_ptr,
                                    &format!("load_{}", field),
                                )?;
                                return Ok(value);
                            }
                        }
                    }
                }
            }
        }

        // For MemberAccess, don't handle it specially - let the normal flow process it
        // The issue is when we have rect.top_left.x - the outer MemberAccess has
        // object = MemberAccess { object: "rect", member: "top_left" } and member = "x"
        // We need to compile the inner MemberAccess to get the Point value

        /* Comment out for now to prevent infinite recursion
        if let Expression::MemberAccess { object, member } = struct_ {
            // This is nested field access like rect.top_left.x
            // First get the inner struct value (rect.top_left)
            let inner_val = self.compile_struct_field(object, member)?;

            // Now we need to determine what type inner_val is
            // Look up the type of the parent struct and its field
            let parent_struct_name = if let Expression::Identifier(name) = &**object {
                // Get the type of the variable
                if let Some(var_info) = self.variables.get(name) {
                    match &var_info.ast_type {
                        AstType::Struct { name: struct_name, .. } => struct_name.clone(),
                        _ => {
                            return Err(CompileError::TypeError(
                                format!("'{}' is not a struct type", name),
                                None
                            ))
                        }
                    }
                } else {
                    return Err(CompileError::UndeclaredVariable(name.clone(), None))
                }
            } else {
                // For deeper nesting, would need more complex type inference
                return Err(CompileError::TypeError(
                    format!("Complex nested struct access not yet supported"),
                    None
                ))
            };

            // Get the type of the field
            let field_struct_name = if let Some(parent_info) = self.struct_types.get(&parent_struct_name) {
                if let Some((_, field_type)) = parent_info.fields.get(member) {
                    match field_type {
                        AstType::Struct { name, .. } => name.clone(),
                        AstType::Generic { name, .. } if self.struct_types.contains_key(name) => {
                            // Generic type that's actually a struct
                            name.clone()
                        },
                        _ => {
                            return Err(CompileError::TypeError(
                                format!("Field '{}' is not a struct type, it's type is {:?}", member, field_type),
                                None
                            ))
                        }
                    }
                } else {
                    return Err(CompileError::TypeError(
                        format!("Field '{}' not found in struct '{}'", member, parent_struct_name),
                        None
                    ))
                }
            } else {
                return Err(CompileError::TypeError(
                    format!("Struct type '{}' not found", parent_struct_name),
                    None
                ))
            };

            // Now access the field of the nested struct
            let struct_info = self.struct_types.get(&field_struct_name)
                .ok_or_else(|| CompileError::TypeError(
                    format!("Struct type '{}' not found", field_struct_name),
                    None
                ))?;

            let field_index = struct_info.fields.get(field)
                .map(|(idx, _)| *idx)
                .ok_or_else(|| CompileError::TypeError(
                    format!("Field '{}' not found in struct '{}'", field, field_struct_name),
                    None
                ))?;

            let field_type = struct_info.fields.get(field)
                .map(|(_, ty)| ty.clone())
                .unwrap();

            // inner_val is the struct value, we need to access its field
            let struct_ptr = if inner_val.is_pointer_value() {
                inner_val.into_pointer_value()
            } else {
                // Store the struct value in a temporary alloca
                let temp_alloca = self.builder.build_alloca(
                    struct_info.llvm_type,
                    &format!("temp_{}", field_struct_name)
                )?;
                self.builder.build_store(temp_alloca, inner_val)?;
                temp_alloca
            };

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
                    &format!("{}_{}_ptr", field_struct_name, field)
                )?
            };

            // Load the field value
            let field_llvm_type = self.to_llvm_type(&field_type)?;
            let basic_type = match field_llvm_type {
                Type::Basic(ty) => ty,
                Type::Struct(st) => st.as_basic_type_enum(),
                _ => return Err(CompileError::TypeError(
                    "Field type must be basic type".to_string(),
                    None
                )),
            };

            let value = self.builder.build_load(basic_type, field_ptr, &format!("load_{}_{}", field_struct_name, field))?;
            return Ok(value);
        }
        */

        // For any other expression type, we need to:
        // 1. Compile the expression to get a struct value or pointer
        // 2. If it's a value, store it in a temporary alloca
        // 3. Access the field from there

        // First compile the struct expression
        let struct_val = self.compile_expression(struct_)?;

        // Infer the struct type from the value
        let struct_name = self.infer_struct_type_from_value(&struct_val, struct_)?;

        // Get struct type info (clone to avoid borrow checker issues)
        let (llvm_type, field_index, field_type) = {
            let struct_info = self.struct_types.get(&struct_name).ok_or_else(|| {
                CompileError::TypeError(format!("Struct type '{}' not found", struct_name), None)
            })?;

            // Find field index and type
            let field_index = struct_info
                .fields
                .get(field)
                .map(|(idx, _)| *idx)
                .ok_or_else(|| {
                    CompileError::TypeError(
                        format!("Field '{}' not found in struct '{}'", field, struct_name),
                        None,
                    )
                })?;

            let field_type = struct_info
                .fields
                .get(field)
                .map(|(_, ty)| ty.clone())
                .unwrap();

            (struct_info.llvm_type, field_index, field_type)
        };

        // Determine if we have a pointer or a value
        let struct_ptr = if struct_val.is_pointer_value() {
            // It's already a pointer, use it directly
            struct_val.into_pointer_value()
        } else {
            // It's a value, store it in a temporary alloca
            let temp_alloca = self
                .builder
                .build_alloca(llvm_type, &format!("temp_struct_{}", struct_name))?;
            self.builder.build_store(temp_alloca, struct_val)?;
            temp_alloca
        };

        // Build GEP to access the field
        let indices = vec![
            self.context.i32_type().const_zero(),
            self.context.i32_type().const_int(field_index as u64, false),
        ];

        let field_ptr = unsafe {
            self.builder.build_gep(
                llvm_type,
                struct_ptr,
                &indices,
                &format!("field_{}_ptr", field),
            )?
        };

        // Load the field value
        let field_llvm_type = self.to_llvm_type(&field_type)?;
        let basic_type = match field_llvm_type {
            Type::Basic(ty) => ty,
            Type::Struct(st) => st.as_basic_type_enum(),
            _ => {
                return Err(CompileError::TypeError(
                    "Field type must be basic type".to_string(),
                    None,
                ))
            }
        };

        let value = self
            .builder
            .build_load(basic_type, field_ptr, &format!("load_{}", field))?;
        Ok(value)
    }

    // Helper function to handle field access from a compiled value
    fn compile_struct_field_from_value(
        &mut self,
        _struct_val: BasicValueEnum<'ctx>,
        _field: &str,
        original_expr: &Expression,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // This is a placeholder for handling nested struct field access
        // In a complete implementation, we'd need to track the type of struct_val
        // For now, return an error
        Err(CompileError::TypeError(
            format!(
                "Nested struct field access not yet fully implemented for expression: {:?}",
                original_expr
            ),
            None,
        ))
    }

    /// Infer struct type name from a compiled value and the original expression
    pub fn infer_struct_type_from_value(
        &mut self,
        _val: &BasicValueEnum<'ctx>,
        expr: &Expression,
    ) -> Result<String, CompileError> {
        match expr {
            Expression::Identifier(name) => {
                // Look up the variable type
                if let Some(var_info) = self.variables.get(name) {
                    let var_type = &var_info.ast_type;
                    match var_type {
                        AstType::Struct {
                            name: struct_name, ..
                        } => Ok(struct_name.clone()),
                        AstType::Generic {
                            name: type_name, ..
                        } => {
                            // Check if this generic is actually a registered struct type
                            if self.struct_types.contains_key(type_name) {
                                Ok(type_name.clone())
                            } else {
                                Err(CompileError::TypeError(
                                    format!("Type '{}' is not a registered struct type", type_name),
                                    None,
                                ))
                            }
                        }
                        AstType::Ptr(inner) => {
                            // Handle pointer to struct
                            match &**inner {
                                AstType::Struct {
                                    name: struct_name, ..
                                } => Ok(struct_name.clone()),
                                AstType::Generic {
                                    name: type_name, ..
                                } => {
                                    // Check if this generic is actually a registered struct type
                                    if self.struct_types.contains_key(type_name) {
                                        Ok(type_name.clone())
                                    } else {
                                        Err(CompileError::TypeError(
                                            format!("Variable '{}' is not a struct or pointer to struct", name),
                                            None
                                        ))
                                    }
                                }
                                _ => Err(CompileError::TypeError(
                                    format!(
                                        "Variable '{}' is not a struct or pointer to struct",
                                        name
                                    ),
                                    None,
                                )),
                            }
                        }
                        _ => Err(CompileError::TypeError(
                            format!(
                                "Variable '{}' is not a struct type, got {:?}",
                                name, var_type
                            ),
                            None,
                        )),
                    }
                } else {
                    Err(CompileError::TypeError(
                        format!("Unknown variable '{}'", name),
                        None,
                    ))
                }
            }
            Expression::StructLiteral { name, .. } => Ok(name.clone()),
            Expression::FunctionCall { name, .. } => {
                // Look up function return type
                if let Some(func_type) = self.function_types.get(name) {
                    if let AstType::Struct {
                        name: struct_name, ..
                    } = func_type
                    {
                        Ok(struct_name.clone())
                    } else {
                        Err(CompileError::TypeError(
                            format!("Function '{}' does not return a struct", name),
                            None,
                        ))
                    }
                } else {
                    Err(CompileError::TypeError(
                        format!("Unknown function '{}'", name),
                        None,
                    ))
                }
            }
            Expression::MemberAccess { object, member } => {
                // For self.field, we need to infer the type of the field
                // First get the struct type of the object
                let object_struct = self.infer_struct_type_from_value(_val, object)?;

                // Then look up the field type in that struct
                if let Some(struct_info) = self.struct_types.get(&object_struct) {
                    if let Some((_index, field_type)) = struct_info.fields.get(member) {
                        // Check if the field is itself a struct
                        match field_type {
                            AstType::Struct { name, .. } => Ok(name.clone()),
                            AstType::Generic { name, .. } => {
                                // Check if this generic is actually a registered struct type
                                if self.struct_types.contains_key(name) {
                                    Ok(name.clone())
                                } else {
                                    Err(CompileError::TypeError(
                                        format!("Field '{}' is not a struct type", member),
                                        None,
                                    ))
                                }
                            }
                            _ => Err(CompileError::TypeError(
                                format!(
                                    "Field '{}' is not a struct type, got {:?}",
                                    member, field_type
                                ),
                                None,
                            )),
                        }
                    } else {
                        Err(CompileError::TypeError(
                            format!("Field '{}' not found in struct '{}'", member, object_struct),
                            None,
                        ))
                    }
                } else {
                    Err(CompileError::TypeError(
                        format!("Struct type '{}' not found", object_struct),
                        None,
                    ))
                }
            }
            Expression::StructField { .. } => {
                // This shouldn't be called for struct field access
                Err(CompileError::TypeError(
                    format!("Cannot infer struct type from StructField expression"),
                    None,
                ))
            }
            Expression::PointerDereference(ptr_expr) => {
                // For ptr.val where ptr is Ptr<Struct> or MutPtr<Struct> or RawPtr<Struct>
                match ptr_expr.as_ref() {
                    Expression::Identifier(name) => {
                        if let Some(var_info) = self.variables.get(name) {
                            match &var_info.ast_type {
                                AstType::Ptr(inner)
                                | AstType::MutPtr(inner)
                                | AstType::RawPtr(inner) => {
                                    match &**inner {
                                        AstType::Struct { name: struct_name, .. } => Ok(struct_name.clone()),
                                        AstType::Generic { name: type_name, .. } => {
                                            // Check if this generic is actually a registered struct type
                                            if self.struct_types.contains_key(type_name) {
                                                Ok(type_name.clone())
                                            } else {
                                                Err(CompileError::TypeError(
                                                    format!("Pointer does not point to a struct type, got Generic: {}", type_name),
                                                    None
                                                ))
                                            }
                                        }
                                        _ => Err(CompileError::TypeError(
                                            format!("Pointer does not point to a struct type, got: {:?}", &**inner),
                                            None
                                        ))
                                    }
                                }
                                _ => Err(CompileError::TypeError(
                                    format!("Cannot dereference non-pointer type"),
                                    None,
                                )),
                            }
                        } else {
                            Err(CompileError::TypeError(
                                format!("Unknown variable '{}'", name),
                                None,
                            ))
                        }
                    }
                    _ => Err(CompileError::TypeError(
                        format!(
                            "Complex pointer dereference not yet supported in struct field access"
                        ),
                        None,
                    )),
                }
            }
            _ => Err(CompileError::TypeError(
                format!("Cannot infer struct type from expression: {:?}", expr),
                None,
            )),
        }
    }

    pub fn compile_struct_field_assignment(
        &mut self,
        struct_alloca: inkwell::values::PointerValue<'ctx>,
        field_name: &str,
        value: BasicValueEnum<'ctx>,
        struct_name: &str,
    ) -> Result<(), CompileError> {
        // Get the struct type info using the struct name
        let struct_type_info = self
            .struct_types
            .get(struct_name)
            .ok_or_else(|| CompileError::TypeError(format!("Struct '{}' not found", struct_name), None))?;

        let struct_type = struct_type_info.llvm_type;
        let field_index = struct_type_info
            .fields
            .get(field_name)
            .map(|(index, _)| *index)
            .ok_or_else(|| {
                CompileError::TypeError(format!("Field '{}' not found in struct '{}'", field_name, struct_name), None)
            })?;

        // Create GEP to get the field pointer
        let field_ptr = self
            .builder
            .build_struct_gep(struct_type, struct_alloca, field_index as u32, "field_ptr")
            .map_err(|e| CompileError::from(e))?;

        // Store the value to the field
        self.builder
            .build_store(field_ptr, value)
            .map_err(|e| CompileError::from(e))?;
        Ok(())
    }
}
