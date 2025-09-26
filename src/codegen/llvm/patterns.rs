use super::{symbols, LLVMCompiler};
use crate::ast::Pattern;
use crate::error::CompileError;
use inkwell::values::{BasicValueEnum, IntValue};
use inkwell::{AddressSpace, IntPredicate};
use std::collections::HashMap;

impl<'ctx> LLVMCompiler<'ctx> {
    pub fn compile_pattern_test(
        &mut self,
        scrutinee_val: &BasicValueEnum<'ctx>,
        pattern: &Pattern,
    ) -> Result<(IntValue<'ctx>, Vec<(String, BasicValueEnum<'ctx>)>), CompileError> {
        let mut bindings = Vec::new();

        let matches = match pattern {
            Pattern::Literal(lit_expr) => {
                // Check if the literal expression is a range
                if let crate::ast::Expression::Range {
                    start,
                    end,
                    inclusive,
                } = lit_expr
                {
                    // Handle range pattern directly
                    if !scrutinee_val.is_int_value() {
                        return Err(CompileError::TypeMismatch {
                            expected: "integer for range pattern".to_string(),
                            found: format!("{:?}", scrutinee_val.get_type()),
                            span: None,
                        });
                    }

                    let start_val = self.compile_expression(start)?;
                    let end_val = self.compile_expression(end)?;

                    let scrutinee_int = scrutinee_val.into_int_value();

                    // Ensure start and end are int values
                    if !start_val.is_int_value() || !end_val.is_int_value() {
                        return Err(CompileError::TypeMismatch {
                            expected: "integer values for range bounds".to_string(),
                            found: format!(
                                "start: {:?}, end: {:?}",
                                start_val.get_type(),
                                end_val.get_type()
                            ),
                            span: None,
                        });
                    }

                    let start_int = start_val.into_int_value();
                    let end_int = end_val.into_int_value();

                    // Ensure all integers have the same type
                    let scrutinee_type = scrutinee_int.get_type();
                    let start_type = start_int.get_type();
                    let end_type = end_int.get_type();

                    // Cast if needed to match scrutinee type
                    let start_int = if start_type != scrutinee_type {
                        self.builder
                            .build_int_cast(start_int, scrutinee_type, "start_cast")?
                    } else {
                        start_int
                    };

                    let end_int = if end_type != scrutinee_type {
                        self.builder
                            .build_int_cast(end_int, scrutinee_type, "end_cast")?
                    } else {
                        end_int
                    };

                    let ge_start = self.builder.build_int_compare(
                        IntPredicate::SGE,
                        scrutinee_int,
                        start_int,
                        "range_ge",
                    )?;

                    let le_end = if *inclusive {
                        self.builder.build_int_compare(
                            IntPredicate::SLE,
                            scrutinee_int,
                            end_int,
                            "range_le",
                        )?
                    } else {
                        self.builder.build_int_compare(
                            IntPredicate::SLT,
                            scrutinee_int,
                            end_int,
                            "range_lt",
                        )?
                    };

                    self.builder.build_and(ge_start, le_end, "range_match")?
                } else {
                    // Regular literal pattern
                    let pattern_val = self.compile_expression(lit_expr)?;
                    self.values_equal(scrutinee_val, &pattern_val)?
                }
            }

            Pattern::Wildcard => self.context.bool_type().const_int(1, false),

            Pattern::Identifier(name) => {
                bindings.push((name.clone(), *scrutinee_val));
                self.context.bool_type().const_int(1, false)
            }

            Pattern::Range {
                start,
                end,
                inclusive,
            } => {
                let start_val = self.compile_expression(start)?;
                let end_val = self.compile_expression(end)?;

                if !scrutinee_val.is_int_value() {
                    return Err(CompileError::TypeMismatch {
                        expected: "integer for range pattern".to_string(),
                        found: format!("{:?}", scrutinee_val.get_type()),
                        span: None,
                    });
                }

                let scrutinee_int = scrutinee_val.into_int_value();
                let start_int = start_val.into_int_value();
                let end_int = end_val.into_int_value();

                let ge_start = self.builder.build_int_compare(
                    IntPredicate::SGE,
                    scrutinee_int,
                    start_int,
                    "range_ge",
                )?;

                let le_end = if *inclusive {
                    self.builder.build_int_compare(
                        IntPredicate::SLE,
                        scrutinee_int,
                        end_int,
                        "range_le",
                    )?
                } else {
                    self.builder.build_int_compare(
                        IntPredicate::SLT,
                        scrutinee_int,
                        end_int,
                        "range_lt",
                    )?
                };

                self.builder.build_and(ge_start, le_end, "range_match")?
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
                // Get the enum type info
                let enum_info = match self.symbols.lookup(enum_name) {
                    Some(symbols::Symbol::EnumType(info)) => info.clone(),
                    _ => {
                        return Err(CompileError::UndeclaredVariable(
                            format!("Enum '{}' not found", enum_name),
                            None,
                        ));
                    }
                };

                // Get the expected discriminant value for this variant
                let expected_tag =
                    enum_info
                        .variant_indices
                        .get(variant)
                        .copied()
                        .ok_or_else(|| {
                            CompileError::UndeclaredVariable(
                                format!("Unknown variant '{}' for enum '{}'", variant, enum_name),
                                None,
                            )
                        })?;

                // For now, we need to handle enum values as regular values, not pointers
                // Check if scrutinee is a pointer or direct value
                let discriminant = if scrutinee_val.is_pointer_value() {
                    // Extract discriminant from pointer
                    let enum_struct_type = enum_info.llvm_type;
                    let discriminant_gep = self.builder.build_struct_gep(
                        enum_struct_type,
                        scrutinee_val.into_pointer_value(),
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
                    let extracted =
                        self.builder
                            .build_extract_value(struct_val, 0, "discriminant")?;
                    // Cast to i64 if needed for comparison
                    if extracted.is_int_value() {
                        let int_val = extracted.into_int_value();
                        if int_val.get_type().get_bit_width() < 64 {
                            self.builder
                                .build_int_z_extend(
                                    int_val,
                                    self.context.i64_type(),
                                    "discriminant_extended",
                                )?
                                .into()
                        } else {
                            extracted
                        }
                    } else {
                        extracted
                    }
                } else {
                    // Fallback to treating as integer
                    // If it's an integer that's not i64, extend it
                    if scrutinee_val.is_int_value() {
                        let int_val = scrutinee_val.into_int_value();
                        if int_val.get_type().get_bit_width() < 64 {
                            self.builder
                                .build_int_z_extend(
                                    int_val,
                                    self.context.i64_type(),
                                    "discriminant_extended",
                                )?
                                .into()
                        } else {
                            *scrutinee_val
                        }
                    } else {
                        *scrutinee_val
                    }
                };

                // Compare discriminant with expected value
                let expected_tag_val = self.context.i64_type().const_int(expected_tag, false);
                let matches = self.builder.build_int_compare(
                    inkwell::IntPredicate::EQ,
                    discriminant.into_int_value(),
                    expected_tag_val,
                    "enum_variant_match",
                )?;

                // Handle payload pattern if present
                if let Some(payload_pattern) = payload {
                    // Extract the payload value - preserve the actual type!
                    let payload_val = if scrutinee_val.is_pointer_value() {
                        let enum_struct_type = enum_info.llvm_type;
                        // Check if enum has payload field
                        if enum_struct_type.count_fields() > 1 {
                            let payload_gep = self.builder.build_struct_gep(
                                enum_struct_type,
                                scrutinee_val.into_pointer_value(),
                                1,
                                "payload_ptr",
                            )?;

                            // Get the actual payload type from the enum struct
                            let payload_type =
                                enum_struct_type.get_field_type_at_index(1).ok_or_else(|| {
                                    CompileError::InternalError(
                                        "Enum payload field not found".to_string(),
                                        None,
                                    )
                                })?;

                            // Load the payload pointer
                            let loaded_payload =
                                self.builder
                                    .build_load(payload_type, payload_gep, "payload")?;

                            // If the payload field is a pointer type (which it should be for generic enums),
                            // we need to determine what it points to and possibly dereference
                            let dereferenced_payload = if payload_type.is_pointer_type()
                                && loaded_payload.is_pointer_value()
                            {
                                let ptr_val = loaded_payload.into_pointer_value();

                                // Try to determine what this pointer points to
                                // For integer literals stored in enums, we need to load them with the right type
                                // Default integers are i32, so try that first

                                // Check if we have type information for this enum
                                // Try multiple keys for nested generic support
                                let load_type = if enum_name == "Option" && variant == "Some" {
                                    // Try nested keys first, then regular keys
                                    self.generic_type_context.get("Nested_Option_Some_Type").cloned()
                                        .or_else(|| self.generic_type_context.get("Option_Some_Type").cloned())
                                        .or_else(|| self.generic_tracker.get("Option_Some_Type").cloned())
                                } else if enum_name == "Result" {
                                    if variant == "Ok" {
                                        // For deeply nested Results, check multiple keys
                                        self.generic_type_context.get("Nested_Result_Ok_Result_Ok_Type").cloned()
                                            .or_else(|| self.generic_type_context.get("Nested_Result_Ok_Type").cloned())
                                            .or_else(|| self.generic_type_context.get("Result_Ok_Type").cloned())
                                            .or_else(|| self.generic_tracker.get("Result_Ok_Type").cloned())
                                    } else if variant == "Err" {
                                        self.generic_type_context.get("Nested_Result_Err_Type").cloned()
                                            .or_else(|| self.generic_type_context.get("Result_Err_Type").cloned())
                                            .or_else(|| self.generic_tracker.get("Result_Err_Type").cloned())
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                };
                                

                                if let Some(ast_type) = load_type {
                                    use crate::ast::AstType;
                                    
                                    
                                    match &ast_type {
                                        AstType::Generic { name, type_args } if name == "Option" || name == "Result" => {
                                            // For nested generics, the payload is a pointer to the nested enum struct
                                            // We need to track the nested type information for proper extraction later
                                            
                                            // Enhanced tracking for deeply nested generics
                                            // Track both the immediate type and recursively track inner types
                                            if name == "Option" && !type_args.is_empty() {
                                                // For Result<Option<T>, E> or deeper nesting
                                                self.track_generic_type("Nested_Option_Some_Type".to_string(), type_args[0].clone());
                                                self.generic_tracker.track_generic_type(&type_args[0], "Nested_Option_Some");
                                                
                                                // Also track for immediate context (critical for triple+ nesting)
                                                self.track_generic_type("Option_Some_Type".to_string(), type_args[0].clone());
                                                
                                                // Recursively track if the inner type is also generic
                                                if let AstType::Generic { name: inner_name, type_args: inner_args } = &type_args[0] {
                                                    if inner_name == "Result" && inner_args.len() == 2 {
                                                        self.track_generic_type("Nested_Option_Some_Result_Ok_Type".to_string(), inner_args[0].clone());
                                                        self.track_generic_type("Nested_Option_Some_Result_Err_Type".to_string(), inner_args[1].clone());
                                                    }
                                                }
                                            } else if name == "Result" && type_args.len() == 2 {
                                                // For Option<Result<T,E>> or deeper nesting
                                                self.track_generic_type("Nested_Result_Ok_Type".to_string(), type_args[0].clone());
                                                self.track_generic_type("Nested_Result_Err_Type".to_string(), type_args[1].clone());
                                                self.generic_tracker.track_generic_type(&type_args[0], "Nested_Result_Ok");
                                                self.generic_tracker.track_generic_type(&type_args[1], "Nested_Result_Err");
                                                
                                                // Also track for immediate context (critical for triple+ nesting)
                                                self.track_generic_type("Result_Ok_Type".to_string(), type_args[0].clone());
                                                self.track_generic_type("Result_Err_Type".to_string(), type_args[1].clone());
                                                
                                                // Recursively track if the Ok type is also generic
                                                if let AstType::Generic { name: inner_name, type_args: inner_args } = &type_args[0] {
                                                    if inner_name == "Result" && inner_args.len() == 2 {
                                                        // Triple nesting: Result<Result<Result<T,E>,E>,E>
                                                        self.track_generic_type("Nested_Result_Ok_Result_Ok_Type".to_string(), inner_args[0].clone());
                                                        self.track_generic_type("Nested_Result_Ok_Result_Err_Type".to_string(), inner_args[1].clone());
                                                        
                                                        // Check for quadruple nesting
                                                        if let AstType::Generic { type_args: innermost_args, .. } = &inner_args[0] {
                                                            if !innermost_args.is_empty() {
                                                                self.track_generic_type("Nested_Result_Ok_Result_Ok_Result_Type".to_string(), innermost_args[0].clone());
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            
                                            // The payload is a pointer to the nested enum struct
                                            // We need to load it as a struct value for recursive matching
                                            
                                            // Load the nested enum struct
                                            if ptr_val.is_null() {
                                                loaded_payload
                                            } else {
                                                // Determine the enum struct type for the nested generic
                                                let nested_enum_type = self.context.struct_type(&[
                                                    self.context.i64_type().into(),
                                                    self.context.ptr_type(inkwell::AddressSpace::default()).into(),
                                                ], false);
                                                
                                                // CRITICAL FIX: For deeply nested generics, we need to recursively track types
                                                // Track nested type information BEFORE loading
                                                if name == "Result" && type_args.len() == 2 {
                                                    // Check if the inner type is also a Result or Option
                                                    if let AstType::Generic { name: inner_name, type_args: inner_args } = &type_args[0] {
                                                        if inner_name == "Result" && inner_args.len() == 2 {
                                                            // Result<Result<T,E>,E2> - track inner types
                                                            self.track_generic_type("Nested_Result_Ok_Type".to_string(), inner_args[0].clone());
                                                            self.track_generic_type("Nested_Result_Err_Type".to_string(), inner_args[1].clone());
                                                            
                                                            // Track even deeper nesting if present
                                                            if let AstType::Generic { name: innermost_name, type_args: innermost_args } = &inner_args[0] {
                                                                if innermost_name == "Result" && innermost_args.len() == 2 {
                                                                    self.track_generic_type("Nested_Result_Ok_Result_Ok_Type".to_string(), innermost_args[0].clone());
                                                                    self.track_generic_type("Nested_Result_Ok_Result_Err_Type".to_string(), innermost_args[1].clone());
                                                                }
                                                            }
                                                        } else if inner_name == "Option" && inner_args.len() == 1 {
                                                            // Result<Option<T>,E> - track inner type
                                                            self.track_generic_type("Nested_Result_Ok_Option_Type".to_string(), inner_args[0].clone());
                                                        }
                                                    }
                                                    
                                                    self.track_generic_type("Result_Ok_Type".to_string(), type_args[0].clone());
                                                    self.track_generic_type("Result_Err_Type".to_string(), type_args[1].clone());
                                                } else if name == "Option" && !type_args.is_empty() {
                                                    // Check if inner type is Result or Option
                                                    if let AstType::Generic { name: inner_name, type_args: inner_args } = &type_args[0] {
                                                        if inner_name == "Result" && inner_args.len() == 2 {
                                                            // Option<Result<T,E>> - track inner types
                                                            self.track_generic_type("Nested_Option_Some_Result_Ok_Type".to_string(), inner_args[0].clone());
                                                            self.track_generic_type("Nested_Option_Some_Result_Err_Type".to_string(), inner_args[1].clone());
                                                        } else if inner_name == "Option" && inner_args.len() == 1 {
                                                            // Option<Option<T>> - track inner type
                                                            self.track_generic_type("Nested_Option_Some_Option_Type".to_string(), inner_args[0].clone());
                                                        }
                                                    }
                                                    
                                                    self.track_generic_type("Option_Some_Type".to_string(), type_args[0].clone());
                                                }
                                                
                                                // Load the nested enum struct value
                                                match self.builder.build_load(
                                                    nested_enum_type,
                                                    ptr_val,
                                                    &format!("nested_{}_struct", name.to_lowercase()),
                                                ) {
                                                    Ok(loaded_struct) => loaded_struct,
                                                    Err(_e) => loaded_payload
                                                }
                                            }
                                        }
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
                                        AstType::I32 => {
                                            let loaded_i32 = self
                                                .builder
                                                .build_load(
                                                    self.context.i32_type(),
                                                    ptr_val,
                                                    "payload_i32",
                                                )
                                                .unwrap_or(loaded_payload);
                                            loaded_i32
                                        }
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
                                        AstType::F64 => {
                                            let loaded = self
                                                .builder
                                                .build_load(
                                                    self.context.f64_type(),
                                                    ptr_val,
                                                    "payload_f64",
                                                )
                                                .unwrap_or(loaded_payload);
                                            loaded
                                        }
                                        AstType::String => loaded_payload, // Strings are already pointers, don't load
                                        _ => {
                                            // Try default loading for other types - i32 first
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
                                    // No type info, try i32 first since that's the default for integer literals
                                    match self.builder.build_load(
                                        self.context.i32_type(),
                                        ptr_val,
                                        "try_i32",
                                    ) {
                                        Ok(int_val) => int_val,
                                        _ => {
                                            // Try loading as i64
                                            match self.builder.build_load(
                                                self.context.i64_type(),
                                                ptr_val,
                                                "payload_i64",
                                            ) {
                                                Ok(loaded) => loaded,
                                                _ => {
                                                    // Not an integer, keep as pointer (likely a string)
                                                    loaded_payload
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                loaded_payload
                            };
                            dereferenced_payload
                        } else {
                            // No payload field in enum
                            self.context.i64_type().const_int(0, false).into()
                        }
                    } else if scrutinee_val.is_struct_value() {
                        let struct_val = scrutinee_val.into_struct_value();
                        // Check if struct has payload field
                        if struct_val.get_type().count_fields() > 1 {
                            let extracted =
                                self.builder.build_extract_value(struct_val, 1, "payload")?;
                            // The payload in a struct is a pointer
                            // For integers: pointer to the integer value
                            // For strings: the string pointer itself
                            // We need to dereference integer pointers but not string pointers
                            //
                            // CRITICAL FIX: Check for null pointer before dereferencing
                            // This prevents segfault when matching None variants
                            if extracted.is_pointer_value() {
                                let ptr_val = extracted.into_pointer_value();

                                // Check if pointer is null before attempting to load
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
                                let merge_bb =
                                    self.context.append_basic_block(current_fn, "payload_merge");

                                self.builder
                                    .build_conditional_branch(is_null, else_bb, then_bb)?;

                                // Not null path - attempt to load
                                self.builder.position_at_end(then_bb);

                                
                                // Check if we have type information for this enum
                                let load_type = if enum_name == "Option" && variant == "Some" {
                                    let t = self.generic_type_context.get("Option_Some_Type").cloned();
                                    t
                                } else if enum_name == "Result" {
                                    if variant == "Ok" {
                                        let t = self.generic_type_context.get("Result_Ok_Type").cloned();
                                        t
                                    } else if variant == "Err" {
                                        let t = self.generic_type_context.get("Result_Err_Type").cloned();
                                        t
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                };

                                let loaded_value = if let Some(ast_type) = load_type {
                                    use crate::ast::AstType;
                                    match &ast_type {
                                        AstType::Generic { name, type_args } if name == "Option" || name == "Result" => {
                                            // Track nested generic types for further extraction
                                            if name == "Option" && !type_args.is_empty() {
                                                self.track_generic_type("Option_Some_Type".to_string(), type_args[0].clone());
                                                // Track deeper nesting
                                                self.generic_tracker.track_generic_type(&type_args[0], "Option_Some");
                                            } else if name == "Result" && type_args.len() == 2 {
                                                self.track_generic_type("Result_Ok_Type".to_string(), type_args[0].clone());
                                                self.track_generic_type("Result_Err_Type".to_string(), type_args[1].clone());
                                                // Track deeper nesting
                                                if variant == "Ok" {
                                                    self.generic_tracker.track_generic_type(&type_args[0], "Result_Ok");
                                                } else if variant == "Err" {
                                                    self.generic_tracker.track_generic_type(&type_args[1], "Result_Err");
                                                }
                                            }
                                            
                                            // Load the nested enum struct
                                            let nested_enum_type = self.context.struct_type(&[
                                                self.context.i64_type().into(),
                                                self.context.ptr_type(inkwell::AddressSpace::default()).into(),
                                            ], false);
                                            
                                            match self.builder.build_load(
                                                nested_enum_type,
                                                ptr_val,
                                                &format!("nested_{}_struct", name.to_lowercase()),
                                            ) {
                                                Ok(loaded_struct) => {
                                                    loaded_struct
                                                },
                                                Err(_) => extracted
                                            }
                                        }
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
                                        AstType::I32 => {
                                            
                                            let loaded = self
                                                .builder
                                                .build_load(
                                                    self.context.i32_type(),
                                                    ptr_val,
                                                    "payload_i32",
                                                )
                                                .unwrap_or(extracted);
                                            loaded
                                        }
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
                                        AstType::String => extracted, // Strings are already pointers, don't load
                                        _ => {
                                            // Try default loading - i32 first, then i64
                                            match self.builder.build_load(
                                                self.context.i32_type(),
                                                ptr_val,
                                                "try_i32",
                                            ) {
                                                Ok(loaded) => loaded,
                                                _ => match self.builder.build_load(
                                                    self.context.i64_type(),
                                                    ptr_val,
                                                    "try_i64",
                                                ) {
                                                    Ok(loaded) => loaded,
                                                    _ => extracted, // Keep as pointer if load fails
                                                },
                                            }
                                        }
                                    }
                                } else {
                                    // No type info, try i32 first as default integer type
                                    match self.builder.build_load(
                                        self.context.i32_type(),
                                        ptr_val,
                                        "try_load_i32",
                                    ) {
                                        Ok(loaded) => loaded,
                                        _ => match self.builder.build_load(
                                            self.context.i64_type(),
                                            ptr_val,
                                            "try_load_i64",
                                        ) {
                                            Ok(loaded) => loaded,
                                            _ => extracted, // Keep as pointer if load fails
                                        },
                                    }
                                };
                                self.builder.build_unconditional_branch(merge_bb)?;
                                let then_val = loaded_value;

                                // Null path - use default value matching the loaded type
                                self.builder.position_at_end(else_bb);
                                // Use the same type as then_val for type consistency
                                let null_value: BasicValueEnum = if then_val.is_int_value() {
                                    let int_type = then_val.into_int_value().get_type();
                                    int_type.const_int(0, false).into()
                                } else if then_val.is_float_value() {
                                    let float_type = then_val.into_float_value().get_type();
                                    float_type.const_float(0.0).into()
                                } else if then_val.is_struct_value() {
                                    // For structs (nested enums), create a null struct with same type
                                    let struct_type = then_val.into_struct_value().get_type();
                                    struct_type.const_zero().into()
                                } else {
                                    // For pointers, use null pointer
                                    let ptr_type = then_val.into_pointer_value().get_type();
                                    ptr_type.const_null().into()
                                };
                                self.builder.build_unconditional_branch(merge_bb)?;

                                // Merge paths
                                self.builder.position_at_end(merge_bb);
                                let phi = self
                                    .builder
                                    .build_phi(then_val.get_type(), "payload_result")?;
                                phi.add_incoming(&[(&then_val, then_bb), (&null_value, else_bb)]);
                                phi.as_basic_value()
                            } else {
                                extracted
                            }
                        } else {
                            self.context.i64_type().const_int(0, false).into()
                        }
                    } else {
                        // No payload available
                        self.context.i64_type().const_int(0, false).into()
                    };

                    // Recursively match the payload pattern
                    let (payload_match, mut payload_bindings) =
                        self.compile_pattern_test(&payload_val, payload_pattern)?;

                    // Combine the discriminant match with payload match
                    let combined_match =
                        self.builder
                            .build_and(matches, payload_match, "enum_full_match")?;

                    bindings.append(&mut payload_bindings);
                    combined_match
                } else {
                    matches
                }
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
                                                None
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
                                                    AstType::String => loaded_payload, // Strings are already pointers, don't load
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
                                            None
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
                                                AstType::String => extracted, // Strings are already pointers, don't load
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
                                        self.builder.build_unconditional_branch(merge_bb)?;
                                        let then_val = loaded_value;

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
                                        self.builder.build_unconditional_branch(merge_bb)?;

                                        // Merge paths
                                        self.builder.position_at_end(merge_bb);
                                        let phi = self
                                            .builder
                                            .build_phi(then_val.get_type(), "payload_result")?;
                                        phi.add_incoming(&[
                                            (&then_val, then_bb),
                                            (&null_value, else_bb),
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

    fn values_equal(
        &mut self,
        val1: &BasicValueEnum<'ctx>,
        val2: &BasicValueEnum<'ctx>,
    ) -> Result<IntValue<'ctx>, CompileError> {
        if val1.is_int_value() && val2.is_int_value() {
            let int1 = val1.into_int_value();
            let int2 = val2.into_int_value();

            // Ensure both integers have the same bit width for comparison
            let (int1, int2) = if int1.get_type().get_bit_width() != int2.get_type().get_bit_width()
            {
                // Cast to the larger width
                let target_width = std::cmp::max(
                    int1.get_type().get_bit_width(),
                    int2.get_type().get_bit_width(),
                );
                let target_type = self.context.custom_width_int_type(target_width);

                let int1 = if int1.get_type().get_bit_width() < target_width {
                    self.builder.build_int_z_extend(int1, target_type, "ext1")?
                } else {
                    int1
                };

                let int2 = if int2.get_type().get_bit_width() < target_width {
                    self.builder.build_int_z_extend(int2, target_type, "ext2")?
                } else {
                    int2
                };

                (int1, int2)
            } else {
                (int1, int2)
            };

            Ok(self
                .builder
                .build_int_compare(IntPredicate::EQ, int1, int2, "int_eq")?)
        } else if val1.is_float_value() && val2.is_float_value() {
            Ok(self.builder.build_float_compare(
                inkwell::FloatPredicate::OEQ,
                val1.into_float_value(),
                val2.into_float_value(),
                "float_eq",
            )?)
        } else if val1.is_pointer_value() && val2.is_pointer_value() {
            Ok(self.builder.build_int_compare(
                IntPredicate::EQ,
                val1.into_pointer_value(),
                val2.into_pointer_value(),
                "ptr_eq",
            )?)
        } else {
            Err(CompileError::TypeMismatch {
                expected: "matching types for comparison".to_string(),
                found: format!("{:?} vs {:?}", val1.get_type(), val2.get_type()),
                span: None,
            })
        }
    }

    pub fn apply_pattern_bindings(
        &mut self,
        bindings: &[(String, BasicValueEnum<'ctx>)],
    ) -> HashMap<String, super::VariableInfo<'ctx>> {
        let mut saved = HashMap::new();

        for (name, value) in bindings {
            if let Some(existing) = self.variables.get(name) {
                saved.insert(name.clone(), existing.clone());
            }

            // Determine the AST type based on the LLVM type
            let (actual_value, ast_type) = if value.is_int_value() {
                let int_val = value.into_int_value();
                // Map integer types
                let ast_type = match int_val.get_type().get_bit_width() {
                    8 => crate::ast::AstType::I8,
                    16 => crate::ast::AstType::I16,
                    32 => crate::ast::AstType::I32,
                    64 => crate::ast::AstType::I64,
                    _ => crate::ast::AstType::I64,
                };
                (*value, ast_type)
            } else if value.is_float_value() {
                let fv = value.into_float_value();
                let ast_type = if fv.get_type() == self.context.f32_type() {
                    crate::ast::AstType::F32
                } else {
                    crate::ast::AstType::F64
                };
                (*value, ast_type)
            } else if value.is_pointer_value() {
                // For pointer values in pattern bindings, we need to check what they point to
                // The problem is distinguishing between:
                // 1. Pointer to integer (needs dereferencing)
                // 2. String pointer (should NOT be dereferenced)
                //
                // For now, be conservative and assume pointers are strings
                // This means integer payloads won't work unless we dereference them earlier
                (*value, crate::ast::AstType::String)
            } else if value.is_struct_value() {
                // For struct values, check if it's an enum struct (has 2 fields: discriminant + payload)
                let struct_val = value.into_struct_value();
                if struct_val.get_type().count_fields() == 2 {
                    // This is likely an enum struct (Option or Result)
                    // Try to determine which based on the generic type context
                    let ast_type = if let Some(option_type) = self.generic_tracker.get("Option_Some_Type") {
                        // It's an Option
                        crate::ast::AstType::Generic {
                            name: "Option".to_string(),
                            type_args: vec![option_type.clone()],
                        }
                    } else if let Some(ok_type) = self.generic_tracker.get("Result_Ok_Type") {
                        // It's a Result
                        let err_type = self.generic_tracker.get("Result_Err_Type")
                            .cloned()
                            .unwrap_or(crate::ast::AstType::String);
                        crate::ast::AstType::Generic {
                            name: "Result".to_string(),
                            type_args: vec![ok_type.clone(), err_type],
                        }
                    } else {
                        // Fallback - generic Option with unknown type
                        crate::ast::AstType::Generic {
                            name: "Option".to_string(),
                            type_args: vec![crate::ast::AstType::I64],
                        }
                    };
                    (*value, ast_type)
                } else {
                    // Other struct types
                    (*value, crate::ast::AstType::I64)
                }
            } else {
                // Default case
                (*value, crate::ast::AstType::I64)
            };

            let alloca = self
                .builder
                .build_alloca(actual_value.get_type(), name)
                .unwrap();
            self.builder.build_store(alloca, actual_value).unwrap();

            self.variables.insert(
                name.clone(),
                super::VariableInfo {
                    pointer: alloca,
                    ast_type,
                    is_mutable: false, // Pattern bindings are immutable
                    is_initialized: true,
                },
            );
        }

        saved
    }

    pub fn restore_variables(&mut self, saved: HashMap<String, super::VariableInfo<'ctx>>) {
        for (name, value) in saved {
            self.variables.insert(name, value);
        }
    }
}
