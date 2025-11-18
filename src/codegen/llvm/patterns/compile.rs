use super::super::{symbols, LLVMCompiler};
use super::super::VariableInfo;
use crate::ast::Pattern;
use crate::error::CompileError;
use inkwell::values::{BasicValueEnum, IntValue};
use inkwell::{AddressSpace, IntPredicate};
use std::collections::HashMap;

// Import helpers
use super::helpers;

impl<'ctx> LLVMCompiler<'ctx> {
    pub fn compile_pattern_test(
        &mut self,
        scrutinee_val: &BasicValueEnum<'ctx>,
        pattern: &Pattern,
    ) -> Result<(IntValue<'ctx>, Vec<(String, BasicValueEnum<'ctx>)>), CompileError> {
        self.compile_pattern_test_with_type(scrutinee_val, pattern, None)
    }
    
    pub fn compile_pattern_test_with_type(
        &mut self,
        scrutinee_val: &BasicValueEnum<'ctx>,
        pattern: &Pattern,
        scrutinee_type: Option<&crate::ast::AstType>,
    ) -> Result<(IntValue<'ctx>, Vec<(String, BasicValueEnum<'ctx>)>), CompileError> {
        let mut bindings = Vec::new();

        let matches = match pattern {
            Pattern::Literal(lit_expr) => {
                use super::literal;
                literal::compile_literal_pattern(self, scrutinee_val, lit_expr)?
            }

            Pattern::Wildcard => {
                use super::literal;
                literal::compile_wildcard_pattern(self)
            }

            Pattern::Identifier(name) => {
                bindings.push((name.clone(), *scrutinee_val));
                use super::literal;
                literal::compile_identifier_pattern(self, name)
            }

            Pattern::Range {
                start,
                end,
                inclusive,
            } => {
                use super::literal;
                literal::compile_range_pattern(self, scrutinee_val, start, end, inclusive)?
            }

            Pattern::Or(patterns) => {
                let mut result = self.context.bool_type().const_int(0, false);

                for sub_pattern in patterns {
                    let (sub_match, sub_bindings) =
                        self.compile_pattern_test(scrutinee_val, sub_pattern)?;
                    if !sub_bindings.is_empty() {
                        return Err(CompileError::UnsupportedFeature(
                            "Bindings in or-patterns not yet supported".to_string(),
                            None,
                        ));
                    }
                    result = self.builder.build_or(result, sub_match, "or_match")?;
                }

                result
            }

            Pattern::Binding { name, pattern } => {
                // Bindings just capture the value as-is
                // The conversion from i64 to pointer for strings is handled
                // in the EnumLiteral pattern when extracting payloads
                bindings.push((name.clone(), *scrutinee_val));
                let (sub_match, mut sub_bindings) =
                    self.compile_pattern_test(scrutinee_val, pattern)?;
                bindings.append(&mut sub_bindings);
                sub_match
            }

            Pattern::Struct { .. } => {
                return Err(CompileError::UnsupportedFeature(
                    "Struct patterns not yet implemented".to_string(),
                    None,
                ));
            }

            Pattern::EnumVariant {
                enum_name,
                variant,
                payload,
            } => {
                use super::enum_pattern;
                let match_result = enum_pattern::compile_enum_variant_pattern(
                    self,
                    scrutinee_val,
                    enum_name,
                    variant,
                    payload,
                    &mut bindings,
                )?;
                match_result
            }


            Pattern::Type {
                type_name: _,
                binding,
            } => {
                // Type patterns check if the scrutinee matches a specific type
                // For now, we'll just bind the value if there's a binding
                if let Some(bind_name) = binding {
                    bindings.push((bind_name.clone(), *scrutinee_val));
                }
                // TODO: Implement actual type checking
                // For now, assume it matches
                self.context.bool_type().const_int(1, false)
            }

            Pattern::EnumLiteral { variant, payload } => {
                // EnumLiteral patterns are like .Some(x) or .None
                // We need to infer the enum type from the scrutinee
                
                // Special case: If scrutinee is a raw pointer and pattern is Some/None,
                // treat it as Option-like (null = None, non-null = Some)
                if scrutinee_val.is_pointer_value() && (variant == "Some" || variant == "None") {
                    let ptr_val = scrutinee_val.into_pointer_value();
                    let ptr_type = ptr_val.get_type();
                    let null_ptr = ptr_type.const_null();
                    // Compare pointers as integers (pointers are just addresses)
                    let is_null = self.builder.build_int_compare(
                        inkwell::IntPredicate::EQ,
                        ptr_val,
                        null_ptr,
                        "ptr_is_null",
                    )?;
                    
                    if variant == "None" {
                        // None pattern matches null pointer
                        return Ok((is_null, bindings));
                    } else if variant == "Some" {
                        // Some pattern matches non-null pointer
                        let is_not_null = self.builder.build_int_compare(
                            inkwell::IntPredicate::NE,
                            ptr_val,
                            null_ptr,
                            "ptr_is_not_null",
                        )?;
                        
                        // If there's a payload pattern, bind the dereferenced value
                        if let Some(payload_pattern) = payload {
                            // Try to infer the pointee type from scrutinee type
                            if let Some(scrut_type) = scrutinee_type {
                                if let crate::ast::AstType::Ptr(inner_type) | 
                                     crate::ast::AstType::MutPtr(inner_type) |
                                     crate::ast::AstType::RawPtr(inner_type) = scrut_type {
                                    // We know the pointee type - dereference and bind
                                    let pointee_llvm_type = self.to_llvm_type(inner_type)?;
                                    let basic_type = match pointee_llvm_type {
                                        super::super::Type::Basic(ty) => ty,
                                        super::super::Type::Struct(st) => st.into(),
                                        _ => {
                                            // Fallback: bind pointer as-is
                                            if let Pattern::Identifier(name) = payload_pattern.as_ref() {
                                                bindings.push((name.clone(), ptr_val.into()));
                                                return Ok((is_not_null, bindings));
                                            } else {
                                                return Ok((is_not_null, bindings));
                                            }
                                        }
                                    };
                                    
                                    // Create basic blocks for null check
                                    let current_fn = self.current_function.unwrap();
                                    let then_bb = self.context.append_basic_block(current_fn, "ptr_not_null");
                                    let else_bb = self.context.append_basic_block(current_fn, "ptr_is_null");
                                    let merge_bb = self.context.append_basic_block(current_fn, "ptr_merge");
                                    
                                    self.builder.build_conditional_branch(is_not_null, then_bb, else_bb)?;
                                    
                                    // Not null path - dereference
                                    self.builder.position_at_end(then_bb);
                                    let dereferenced = self.builder.build_load(basic_type, ptr_val, "deref_ptr")?;
                                    
                                    // Recursively match the payload pattern
                                    let (payload_matches, mut payload_bindings) = self.compile_pattern_test_with_type(
                                        &dereferenced,
                                        &payload_pattern,
                                        Some(inner_type),
                                    )?;
                                    
                                    // Both pointer is not null AND payload matches
                                    let both_match = self.builder.build_and(
                                        is_not_null,
                                        payload_matches,
                                        "ptr_and_payload_match",
                                    )?;
                                    
                                    self.builder.build_unconditional_branch(merge_bb)?;
                                    
                                    // Null path - doesn't match Some
                                    self.builder.position_at_end(else_bb);
                                    let false_val = self.context.bool_type().const_int(0, false);
                                    self.builder.build_unconditional_branch(merge_bb)?;
                                    
                                    // Merge
                                    self.builder.position_at_end(merge_bb);
                                    let phi = self.builder.build_phi(
                                        self.context.bool_type(),
                                        "ptr_match_result",
                                    )?;
                                    phi.add_incoming(&[(&both_match, then_bb), (&false_val, else_bb)]);
                                    
                                    // Merge bindings
                                    bindings.extend(payload_bindings);
                                    
                                    return Ok((phi.as_basic_value().into_int_value(), bindings));
                                }
                            }
                            
                            // No type information - bind pointer as-is
                            if let Pattern::Identifier(name) = payload_pattern.as_ref() {
                                bindings.push((name.clone(), ptr_val.into()));
                                return Ok((is_not_null, bindings));
                            } else {
                                return Ok((is_not_null, bindings));
                            }
                        } else {
                            // No payload pattern, just check if not null
                            return Ok((is_not_null, bindings));
                        }
                    }
                }

                // Check if scrutinee is an enum value
                if scrutinee_val.is_struct_value() || scrutinee_val.is_pointer_value() {
                    // Extract the discriminant
                    let discriminant = if scrutinee_val.is_pointer_value() {
                        let ptr_val = scrutinee_val.into_pointer_value();

                        // For now, assume enum struct layout: {i64 tag, T payload}
                        // This is safe because we control how enums are created
                        let struct_type = self.context.struct_type(
                            &[
                                self.context.i64_type().into(),
                                self.context.i64_type().into(),
                            ],
                            false,
                        );

                        let discriminant_gep = self.builder.build_struct_gep(
                            struct_type,
                            ptr_val,
                            0,
                            "discriminant_ptr",
                        )?;
                        self.builder.build_load(
                            self.context.i64_type(),
                            discriminant_gep,
                            "discriminant",
                        )?
                    } else if scrutinee_val.is_struct_value() {
                        // Extract discriminant from struct value
                        let struct_val = scrutinee_val.into_struct_value();
                        self.builder
                            .build_extract_value(struct_val, 0, "discriminant")?
                    } else {
                        return Err(CompileError::TypeMismatch {
                            expected: "enum value for enum literal pattern".to_string(),
                            found: format!("{:?}", scrutinee_val.get_type()),
                            span: None,
                        });
                    };

                    // Search through all known enum types to find one with this variant
                    let mut variant_tag = None;
                    let mut enum_info = None;

                    for (_name, symbol) in self.symbols.iter() {
                        if let symbols::Symbol::EnumType(info) = symbol {
                            if let Some(tag) = info.variant_indices.get(variant) {
                                variant_tag = Some(*tag);
                                enum_info = Some(info.clone());
                                break;
                            }
                        }
                    }

                    if let Some(tag) = variant_tag {
                        // Compare discriminant with expected value
                        let expected_tag_val = self.context.i64_type().const_int(tag, false);
                        let matches = self.builder.build_int_compare(
                            inkwell::IntPredicate::EQ,
                            discriminant.into_int_value(),
                            expected_tag_val,
                            "enum_literal_match",
                        )?;

                        // Handle payload pattern if present
                        if let Some(payload_pattern) = payload {
                            // Extract the payload value - preserve the actual type!
                            let payload_val = if scrutinee_val.is_pointer_value() {
                                let ptr_val = scrutinee_val.into_pointer_value();

                                // Use enum info if available, otherwise use generic struct with pointer payload
                                let struct_type = if let Some(ref info) = enum_info {
                                    info.llvm_type
                                } else {
                                    // Fallback to generic struct with pointer payload
                                    let ptr_type =
                                        self.context.ptr_type(inkwell::AddressSpace::default());
                                    self.context.struct_type(
                                        &[self.context.i64_type().into(), ptr_type.into()],
                                        false,
                                    )
                                };

                                // Check if struct has payload field
                                if struct_type.count_fields() > 1 {
                                    // Build GEP to access the payload field
                                    let payload_ptr = self.builder.build_struct_gep(
                                        struct_type,
                                        ptr_val,
                                        1,
                                        "payload_ptr",
                                    )?;

                                    // Get the actual payload type from the struct
                                    let payload_type = struct_type
                                        .get_field_type_at_index(1)
                                        .ok_or_else(|| {
                                            CompileError::InternalError(
                                                "Enum payload field not found".to_string(),
                                                None,
                                            )
                                        })?;

                                    // Load the payload pointer first
                                    let loaded_payload = self.builder.build_load(
                                        payload_type,
                                        payload_ptr,
                                        "payload",
                                    )?;

                                    // Check if the payload is null (for None variant)
                                    // This prevents segfaults when pattern matching against None
                                    let loaded_payload = if payload_type.is_pointer_type()
                                        && loaded_payload.is_pointer_value()
                                    {
                                        let ptr_val = loaded_payload.into_pointer_value();

                                        // Check if this is a null pointer
                                        if ptr_val.is_const() && ptr_val.is_null() {
                                            // This is a null pointer (None variant) - don't try to dereference
                                            self.context.i64_type().const_int(0, false).into()
                                        } else {
                                            // Non-null pointer, keep as is for further processing
                                            loaded_payload
                                        }
                                    } else {
                                        loaded_payload
                                    };

                                    // If the payload is a pointer type (generic enums), dereference if it's an integer
                                    let dereferenced_payload = if payload_type.is_pointer_type()
                                        && loaded_payload.is_pointer_value()
                                    {
                                        let ptr_val = loaded_payload.into_pointer_value();

                                        // First check if this pointer is null to avoid segfaults
                                        if ptr_val.is_const() && ptr_val.is_null() {
                                            // Don't try to load from null pointer - return dummy value
                                            // This happens when testing Some(x) pattern against None value
                                            self.context.i64_type().const_int(0, false).into()
                                        } else {
                                            // Check the generic type context for Option<T> and Result<T,E> type info
                                            let load_type = if variant == "Some" {
                                                let t = self.generic_type_context
                                                    .get("Option_Some_Type")
                                                    .cloned();
                                                // Track Option_Some_Type from generic context
                                                t
                                            } else if variant == "Ok" {
                                                self.generic_type_context
                                                    .get("Result_Ok_Type")
                                                    .cloned()
                                            } else if variant == "Err" {
                                                self.generic_type_context
                                                    .get("Result_Err_Type")
                                                    .cloned()
                                            } else {
                                                // For custom enums, look up the payload type from enum_info
                                                enum_info.as_ref()
                                                    .and_then(|info| info.variants.iter()
                                                        .find(|v| v.name == *variant)
                                                        .and_then(|v| v.payload.clone()))
                                            };

                                            // Try to load with the tracked type information
                                            if let Some(ast_type) = load_type {
                                                use crate::ast::AstType;
                                                match ast_type {
                                                    AstType::I8 => self
                                                        .builder
                                                        .build_load(
                                                            self.context.i8_type(),
                                                            ptr_val,
                                                            "payload_i8",
                                                        )
                                                        .unwrap_or(loaded_payload),
                                                    AstType::I16 => self
                                                        .builder
                                                        .build_load(
                                                            self.context.i16_type(),
                                                            ptr_val,
                                                            "payload_i16",
                                                        )
                                                        .unwrap_or(loaded_payload),
                                                    AstType::I32 => self
                                                        .builder
                                                        .build_load(
                                                            self.context.i32_type(),
                                                            ptr_val,
                                                            "payload_i32",
                                                        )
                                                        .unwrap_or(loaded_payload),
                                                    AstType::I64 => self
                                                        .builder
                                                        .build_load(
                                                            self.context.i64_type(),
                                                            ptr_val,
                                                            "payload_i64",
                                                        )
                                                        .unwrap_or(loaded_payload),
                                                    AstType::F32 => self
                                                        .builder
                                                        .build_load(
                                                            self.context.f32_type(),
                                                            ptr_val,
                                                            "payload_f32",
                                                        )
                                                        .unwrap_or(loaded_payload),
                                                    AstType::F64 => self
                                                        .builder
                                                        .build_load(
                                                            self.context.f64_type(),
                                                            ptr_val,
                                                            "payload_f64",
                                                        )
                                                        .unwrap_or(loaded_payload),
                                                    AstType::Struct { name, .. } if name == "String" => loaded_payload, // String struct is already a pointer, don't load
                                        AstType::StaticString | AstType::StaticLiteral => loaded_payload, // Static strings are already pointers, don't load
                                                    AstType::Generic { name, type_args } if name == "Result" || name == "Option" => {
                                                        // For nested generics (Result<Result<T,E>,E2>), 
                                                        // the payload is a struct stored directly
                                                        let struct_type = self.context.struct_type(
                                                            &[
                                                                self.context.i64_type().into(), // tag/discriminant
                                                                self.context.ptr_type(inkwell::AddressSpace::default()).into(), // payload pointer
                                                            ],
                                                            false,
                                                        );
                                                        
                                                        // Update the generic type context for the nested type
                                                        // This ensures that when we pattern match on the nested type,
                                                        // we have the correct type information
                                                        if name == "Result" && type_args.len() == 2 {
                                                            // For Result<T,E>, track the nested types
                                                            self.track_generic_type("Result_Ok_Type".to_string(), type_args[0].clone());
                                                            self.track_generic_type("Result_Err_Type".to_string(), type_args[1].clone());
                                                            
                                                            // Also track nested types recursively
                                                            self.track_complex_generic(&type_args[0], "Result_Ok");
                                                            self.track_complex_generic(&type_args[1], "Result_Err");
                                                        } else if name == "Option" && type_args.len() == 1 {
                                                            // For Option<T>, track the nested type
                                                            self.track_generic_type("Option_Some_Type".to_string(), type_args[0].clone());
                                                            self.track_complex_generic(&type_args[0], "Option_Some");
                                                        }
                                                        
                                                        self.builder.build_load(
                                                            struct_type,
                                                            ptr_val,
                                                            "nested_generic",
                                                        ).unwrap_or(loaded_payload)
                                                    },
                                                    _ => {
                                                        // Try default loading for other types
                                                        // Try i32 first since it's the default integer type
                                                        match self.builder.build_load(
                                                            self.context.i32_type(),
                                                            ptr_val,
                                                            "payload_i32",
                                                        ) {
                                                            Ok(loaded) => loaded,
                                                            _ => match self.builder.build_load(
                                                                self.context.i64_type(),
                                                                ptr_val,
                                                                "payload_i64",
                                                            ) {
                                                                Ok(loaded) => loaded,
                                                                _ => loaded_payload,
                                                            },
                                                        }
                                                    }
                                                }
                                            } else {
                                                // No type info, fall back to trying different types
                                                // Try i32 first since it's the default integer type
                                                match self.builder.build_load(
                                                    self.context.i32_type(),
                                                    ptr_val,
                                                    "payload_i32",
                                                ) {
                                                    Ok(loaded) => loaded,
                                                    _ => match self.builder.build_load(
                                                        self.context.i64_type(),
                                                        ptr_val,
                                                        "payload_i64",
                                                    ) {
                                                        Ok(loaded) => loaded,
                                                        _ => loaded_payload,
                                                    },
                                                }
                                            }
                                        }
                                    } else {
                                        loaded_payload
                                    };
                                    dereferenced_payload
                                } else {
                                    // No payload field
                                    self.context.i64_type().const_int(0, false).into()
                                }
                            } else if scrutinee_val.is_struct_value() {
                                let struct_val = scrutinee_val.into_struct_value();
                                // Check if struct has payload field
                                if struct_val.get_type().count_fields() > 1 {
                                    let extracted = self
                                        .builder
                                        .build_extract_value(struct_val, 1, "payload")?;
                                    // The payload might be a pointer that needs dereferencing
                                    if extracted.is_pointer_value() {
                                        let ptr_val = extracted.into_pointer_value();

                                        // CRITICAL FIX: Check for null pointer before dereferencing
                                        // This prevents segfault when matching None variants
                                        let null_ptr = ptr_val.get_type().const_null();
                                        let is_null = self.builder.build_int_compare(
                                            inkwell::IntPredicate::EQ,
                                            ptr_val,
                                            null_ptr,
                                            "is_null_check",
                                        )?;

                                        // Create blocks for null check
                                        let current_fn = self.current_function.unwrap();
                                        let then_bb = self
                                            .context
                                            .append_basic_block(current_fn, "payload_not_null");
                                        let else_bb = self
                                            .context
                                            .append_basic_block(current_fn, "payload_is_null");
                                        let merge_bb = self
                                            .context
                                            .append_basic_block(current_fn, "payload_merge");

                                        self.builder
                                            .build_conditional_branch(is_null, else_bb, then_bb)?;

                                        // Not null path - check the generic type context for Option<T> and Result<T,E> type info
                                        self.builder.position_at_end(then_bb);
                                        // First check the new generic tracker for nested types
                                        let load_type = if variant == "Some" {
                                            self.generic_tracker.get("Option_Some_Type")
                                                .cloned()
                                                .or_else(|| self.generic_type_context.get("Option_Some_Type").cloned())
                                        } else if variant == "Ok" {
                                            self.generic_tracker.get("Result_Ok_Type")
                                                .cloned()
                                                .or_else(|| self.generic_type_context.get("Result_Ok_Type").cloned())
                                        } else if variant == "Err" {
                                            self.generic_tracker.get("Result_Err_Type")
                                                .cloned()
                                                .or_else(|| self.generic_type_context.get("Result_Err_Type").cloned())
                                        } else {
                                            // For custom enums, try to find the payload type from enum info
                                            // We need to search through all enums to find which one has this variant
                                            let mut found_payload_type = None;
                                            for (_enum_name, symbol) in self.symbols.iter() {
                                                if let symbols::Symbol::EnumType(info) = symbol {
                                                    if let Some(var) = info.variants.iter().find(|v| v.name == *variant) {
                                                        found_payload_type = var.payload.clone();
                                                        break;
                                                    }
                                                }
                                            }
                                            found_payload_type
                                        };

                                        // Try to load with the tracked type information
                                        let loaded_value = if let Some(ast_type) = load_type {
                                                            use crate::ast::AstType;
                                            match ast_type {
                                                AstType::I8 => self
                                                    .builder
                                                    .build_load(
                                                        self.context.i8_type(),
                                                        ptr_val,
                                                        "payload_i8",
                                                    )
                                                    .unwrap_or(extracted),
                                                AstType::I16 => self
                                                    .builder
                                                    .build_load(
                                                        self.context.i16_type(),
                                                        ptr_val,
                                                        "payload_i16",
                                                    )
                                                    .unwrap_or(extracted),
                                                AstType::I32 => self
                                                    .builder
                                                    .build_load(
                                                        self.context.i32_type(),
                                                        ptr_val,
                                                        "payload_i32",
                                                    )
                                                    .unwrap_or(extracted),
                                                AstType::I64 => self
                                                    .builder
                                                    .build_load(
                                                        self.context.i64_type(),
                                                        ptr_val,
                                                        "payload_i64",
                                                    )
                                                    .unwrap_or(extracted),
                                                AstType::F32 => self
                                                    .builder
                                                    .build_load(
                                                        self.context.f32_type(),
                                                        ptr_val,
                                                        "payload_f32",
                                                    )
                                                    .unwrap_or(extracted),
                                                AstType::F64 => self
                                                    .builder
                                                    .build_load(
                                                        self.context.f64_type(),
                                                        ptr_val,
                                                        "payload_f64",
                                                    )
                                                    .unwrap_or(extracted),
                                                AstType::Struct { name, .. } if name == "String" => extracted, // String struct is already a pointer, don't load
                                        AstType::StaticString | AstType::StaticLiteral => extracted, // Static strings are already pointers, don't load
                                                AstType::Generic { name, type_args } if name == "Option" || name == "Result" => {
                                                    // For nested generics (Result<Option<T>, E> or Option<Result<T,E>>),
                                                    // the payload is itself an enum struct. Load it as a struct.
                                                    // The struct has format: { i64 discriminant, ptr payload }
                                                    let enum_struct_type = self.context.struct_type(
                                                        &[
                                                            self.context.i64_type().into(),  // Changed from i32 to i64 to match actual enum representation
                                                            self.context.ptr_type(AddressSpace::default()).into(),
                                                        ],
                                                        false,
                                                    );
                                                    
                                                    // Update generic type context for the nested type
                                                    if name == "Result" && type_args.len() == 2 {
                                                        // For Result<T,E>, track the nested types
                                                        // Use different keys to avoid conflicts with outer Result
                                                        self.track_generic_type("Nested_Result_Ok_Type".to_string(), type_args[0].clone());
                                                        self.track_generic_type("Nested_Result_Err_Type".to_string(), type_args[1].clone());
                                                        
                                                        // Also track nested types recursively
                                                        self.track_complex_generic(&type_args[0], "Nested_Result_Ok");
                                                        self.track_complex_generic(&type_args[1], "Nested_Result_Err");
                                                    } else if name == "Option" && type_args.len() == 1 {
                                                        // For Option<T>, track the nested type
                                                        self.track_generic_type("Nested_Option_Some_Type".to_string(), type_args[0].clone());
                                                        self.track_complex_generic(&type_args[0], "Nested_Option_Some");
                                                    }
                                                    
                                                    match self.builder.build_load(
                                                        enum_struct_type,
                                                        ptr_val,
                                                        "nested_enum",
                                                    ) {
                                                        Ok(loaded) => {
                                                            // Update keys for next level of pattern matching  
                                                            // CRITICAL FIX: When we load a nested Result/Option, we need to update
                                                            // the type context for its inner payloads
                                                            if name == "Result" && type_args.len() == 2 {
                                                                // The inner Result's Ok type becomes the new Result_Ok_Type
                                                                self.track_generic_type("Result_Ok_Type".to_string(), type_args[0].clone());
                                                                self.track_generic_type("Result_Err_Type".to_string(), type_args[1].clone());
                                                                
                                                                
                                                                // If the Ok type is i32, make sure we track that properly
                                                            } else if name == "Option" && type_args.len() == 1 {
                                                                self.track_generic_type("Option_Some_Type".to_string(), type_args[0].clone());
                                                            }
                                                            // For nested enums, we want to return the full struct
                                                            // so that subsequent pattern matching can work with it
                                                            loaded
                                                        },
                                                        Err(_) => extracted,
                                                    }
                                                }
                                                AstType::EnumType { ref name } | AstType::Enum { ref name, .. } => {
                                                    // For custom enums inside generics, load as enum struct
                                                    let enum_struct_type = self.context.struct_type(
                                                        &[
                                                            self.context.i64_type().into(),
                                                            self.context.ptr_type(AddressSpace::default()).into(),
                                                        ],
                                                        false,
                                                    );
                                                    
                                                    // Track the custom enum type for nested extraction
                                                    self.track_generic_type(format!("Nested_{}_Type", name), ast_type.clone());
                                                    
                                                    match self.builder.build_load(
                                                        enum_struct_type,
                                                        ptr_val,
                                                        &format!("nested_{}", name),
                                                    ) {
                                                        Ok(loaded) => loaded,
                                                        Err(_) => extracted,
                                                    }
                                                }
                                                _ => {
                                                    // Try default loading
                                                    match self.builder.build_load(
                                                        self.context.i64_type(),
                                                        ptr_val,
                                                        "try_load_int",
                                                    ) {
                                                        Ok(loaded) => loaded,
                                                        _ => extracted,
                                                    }
                                                }
                                            }
                                        } else {
                                            match self.builder.build_load(
                                                self.context.i64_type(),
                                                ptr_val,
                                                "try_load_int",
                                            ) {
                                                Ok(loaded) => {
                                                    // Successfully loaded - use as integer
                                                    loaded
                                                }
                                                _ => {
                                                    // Couldn't load - keep as pointer
                                                    extracted
                                                }
                                            }
                                        };
                                        // Branch to merge only if block isn't already terminated
                                        let then_current_block = self.builder.get_insert_block().unwrap();
                                        if then_current_block.get_terminator().is_none() {
                                            self.builder.build_unconditional_branch(merge_bb)?;
                                        }
                                        let then_val = loaded_value;
                                        let then_bb_end = self.builder.get_insert_block().unwrap();

                                        // Null path - use default value matching the loaded type
                                        self.builder.position_at_end(else_bb);
                                        // Use the same type as then_val for type consistency
                                        let null_value: BasicValueEnum = if then_val.is_int_value()
                                        {
                                            let int_type = then_val.into_int_value().get_type();
                                            int_type.const_int(0, false).into()
                                        } else if then_val.is_float_value() {
                                            let float_type = then_val.into_float_value().get_type();
                                            float_type.const_float(0.0).into()
                                        } else if then_val.is_struct_value() {
                                            // For structs (like nested enums), create a zero-initialized struct
                                            let struct_type = then_val.into_struct_value().get_type();
                                            struct_type.const_zero().into()
                                        } else {
                                            // For pointers, use null pointer
                                            let ptr_type = then_val.into_pointer_value().get_type();
                                            ptr_type.const_null().into()
                                        };
                                        // Branch to merge only if block isn't already terminated
                                        let else_current_block = self.builder.get_insert_block().unwrap();
                                        if else_current_block.get_terminator().is_none() {
                                            self.builder.build_unconditional_branch(merge_bb)?;
                                        }
                                        let else_bb_end = self.builder.get_insert_block().unwrap();

                                        // Merge paths
                                        self.builder.position_at_end(merge_bb);
                                        let phi = self
                                            .builder
                                            .build_phi(then_val.get_type(), "payload_result")?;
                                        phi.add_incoming(&[
                                            (&then_val, then_bb_end),
                                            (&null_value, else_bb_end),
                                        ]);
                                        phi.as_basic_value()
                                    } else {
                                        extracted
                                    }
                                } else {
                                    // No payload field
                                    self.context.i64_type().const_int(0, false).into()
                                }
                            } else {
                                self.context.i64_type().const_int(0, false).into()
                            };

                            // Don't convert here - let apply_pattern_bindings handle type detection

                            // Recursively match the payload pattern
                            let (payload_match, mut payload_bindings) =
                                self.compile_pattern_test(&payload_val, payload_pattern)?;

                            // Combine the discriminant match with payload match
                            let combined_match = self.builder.build_and(
                                matches,
                                payload_match,
                                "enum_literal_full_match",
                            )?;

                            bindings.append(&mut payload_bindings);
                            combined_match
                        } else {
                            matches
                        }
                    } else {
                        return Err(CompileError::UndeclaredVariable(
                            format!("Unknown enum variant '{}'", variant),
                            None,
                        ));
                    }
                } else {
                    return Err(CompileError::TypeMismatch {
                        expected: "enum value for enum literal pattern".to_string(),
                        found: format!("{:?}", scrutinee_val.get_type()),
                        span: None,
                    });
                }
            }

            Pattern::Guard { pattern, condition } => {
                // First check if the pattern matches
                let (pattern_match, mut pattern_bindings) =
                    self.compile_pattern_test(scrutinee_val, pattern)?;

                // If pattern matches, evaluate the guard condition
                // For now, we need to set up the bindings before evaluating the condition
                for (name, value) in &pattern_bindings {
                    // Store the value in memory so we can use it
                    let alloca = self.builder.build_alloca(value.get_type(), &name)?;
                    self.builder.build_store(alloca, *value)?;
                    self.symbols
                        .insert(name.clone(), symbols::Symbol::Variable(alloca));
                }

                // Compile the guard condition
                let condition_val = self.compile_expression(condition)?;

                // The condition should be a boolean
                if !condition_val.is_int_value() {
                    return Err(CompileError::TypeMismatch {
                        expected: "boolean condition in guard".to_string(),
                        found: format!("{:?}", condition_val.get_type()),
                        span: None,
                    });
                }

                // Combine pattern match with guard condition
                let guard_result = self.builder.build_and(
                    pattern_match,
                    condition_val.into_int_value(),
                    "guard_match",
                )?;

                bindings.append(&mut pattern_bindings);
                guard_result
            }
        };

        Ok((matches, bindings))
    }

    // Delegate to helpers module
    fn values_equal(
        &mut self,
        val1: &BasicValueEnum<'ctx>,
        val2: &BasicValueEnum<'ctx>,
    ) -> Result<IntValue<'ctx>, CompileError> {
        helpers::values_equal(self, val1, val2)
    }

    pub fn apply_pattern_bindings(
        &mut self,
        bindings: &[(String, BasicValueEnum<'ctx>)],
    ) -> HashMap<String, VariableInfo<'ctx>> {
        helpers::apply_pattern_bindings(self, bindings)
    }

    pub fn restore_variables(&mut self, saved: HashMap<String, VariableInfo<'ctx>>) {
        helpers::restore_variables(self, saved)
    }
}
