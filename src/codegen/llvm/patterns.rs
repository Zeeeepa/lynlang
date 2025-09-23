use super::{LLVMCompiler, symbols};
use crate::ast::Pattern;
use crate::error::CompileError;
use inkwell::values::{BasicValueEnum, IntValue};
use inkwell::IntPredicate;
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
                if let crate::ast::Expression::Range { start, end, inclusive } = lit_expr {
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
                            found: format!("start: {:?}, end: {:?}", start_val.get_type(), end_val.get_type()),
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
                        self.builder.build_int_cast(start_int, scrutinee_type, "start_cast")?
                    } else {
                        start_int
                    };
                    
                    let end_int = if end_type != scrutinee_type {
                        self.builder.build_int_cast(end_int, scrutinee_type, "end_cast")?
                    } else {
                        end_int
                    };
                    
                    let ge_start = self.builder.build_int_compare(
                        IntPredicate::SGE,
                        scrutinee_int,
                        start_int,
                        "range_ge"
                    )?;
                    
                    let le_end = if *inclusive {
                        self.builder.build_int_compare(
                            IntPredicate::SLE,
                            scrutinee_int,
                            end_int,
                            "range_le"
                        )?
                    } else {
                        self.builder.build_int_compare(
                            IntPredicate::SLT,
                            scrutinee_int,
                            end_int,
                            "range_lt"
                        )?
                    };
                    
                    self.builder.build_and(ge_start, le_end, "range_match")?
                } else {
                    // Regular literal pattern
                    let pattern_val = self.compile_expression(lit_expr)?;
                    self.values_equal(scrutinee_val, &pattern_val)?
                }
            }
            
            Pattern::Wildcard => {
                self.context.bool_type().const_int(1, false)
            }
            
            Pattern::Identifier(name) => {
                bindings.push((name.clone(), *scrutinee_val));
                self.context.bool_type().const_int(1, false)
            }
            
            Pattern::Range { start, end, inclusive } => {
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
                    "range_ge"
                )?;
                
                let le_end = if *inclusive {
                    self.builder.build_int_compare(
                        IntPredicate::SLE,
                        scrutinee_int,
                        end_int,
                        "range_le"
                    )?
                } else {
                    self.builder.build_int_compare(
                        IntPredicate::SLT,
                        scrutinee_int,
                        end_int,
                        "range_lt"
                    )?
                };
                
                self.builder.build_and(ge_start, le_end, "range_match")?
            }
            
            Pattern::Or(patterns) => {
                let mut result = self.context.bool_type().const_int(0, false);
                
                for sub_pattern in patterns {
                    let (sub_match, sub_bindings) = self.compile_pattern_test(scrutinee_val, sub_pattern)?;
                    if !sub_bindings.is_empty() {
                        return Err(CompileError::UnsupportedFeature(
                            "Bindings in or-patterns not yet supported".to_string(),
                            None
                        ));
                    }
                    result = self.builder.build_or(result, sub_match, "or_match")?;
                }
                
                result
            }
            
            Pattern::Binding { name, pattern } => {
                bindings.push((name.clone(), *scrutinee_val));
                let (sub_match, mut sub_bindings) = self.compile_pattern_test(scrutinee_val, pattern)?;
                bindings.append(&mut sub_bindings);
                sub_match
            }
            
            Pattern::Struct { .. } => {
                return Err(CompileError::UnsupportedFeature(
                    "Struct patterns not yet implemented".to_string(),
                    None
                ));
            }
            
            Pattern::EnumVariant { enum_name, variant, payload } => {
                // Get the enum type info
                let enum_info = match self.symbols.lookup(enum_name) {
                    Some(symbols::Symbol::EnumType(info)) => info.clone(),
                    _ => {
                        return Err(CompileError::UndeclaredVariable(
                            format!("Enum '{}' not found", enum_name),
                            None
                        ));
                    }
                };
                
                // Get the expected discriminant value for this variant
                let expected_tag = enum_info.variant_indices.get(variant)
                    .copied()
                    .ok_or_else(|| CompileError::UndeclaredVariable(
                        format!("Unknown variant '{}' for enum '{}'", variant, enum_name),
                        None
                    ))?;
                
                // For now, we need to handle enum values as regular values, not pointers
                // Check if scrutinee is a pointer or direct value
                let discriminant = if scrutinee_val.is_pointer_value() {
                    // Extract discriminant from pointer
                    let enum_struct_type = enum_info.llvm_type;
                    let discriminant_gep = self.builder.build_struct_gep(
                        enum_struct_type,
                        scrutinee_val.into_pointer_value(),
                        0,
                        "discriminant_ptr"
                    )?;
                    self.builder.build_load(
                        self.context.i64_type(), 
                        discriminant_gep,
                        "discriminant"
                    )?
                } else if scrutinee_val.is_struct_value() {
                    // Extract discriminant from struct value
                    let struct_val = scrutinee_val.into_struct_value();
                    self.builder.build_extract_value(struct_val, 0, "discriminant")?
                } else {
                    // Fallback to treating as integer
                    *scrutinee_val
                };
                
                // Compare discriminant with expected value
                let expected_tag_val = self.context.i64_type().const_int(expected_tag, false);
                let matches = self.builder.build_int_compare(
                    inkwell::IntPredicate::EQ,
                    discriminant.into_int_value(),
                    expected_tag_val,
                    "enum_variant_match"
                )?;
                
                // Handle payload pattern if present
                if let Some(payload_pattern) = payload {
                    // Extract the payload value
                    let payload_val = if scrutinee_val.is_pointer_value() {
                        let enum_struct_type = enum_info.llvm_type;
                        let payload_gep = self.builder.build_struct_gep(
                            enum_struct_type,
                            scrutinee_val.into_pointer_value(),
                            1,
                            "payload_ptr"
                        )?;
                        let raw_payload = self.builder.build_load(
                            self.context.i64_type(),
                            payload_gep,
                            "payload"
                        )?;
                        
                        raw_payload
                    } else if scrutinee_val.is_struct_value() {
                        let struct_val = scrutinee_val.into_struct_value();
                        self.builder.build_extract_value(struct_val, 1, "payload")?
                    } else {
                        // No payload available
                        self.context.i64_type().const_int(0, false).into()
                    };
                    
                    // Recursively match the payload pattern
                    let (payload_match, mut payload_bindings) = self.compile_pattern_test(
                        &payload_val,
                        payload_pattern
                    )?;
                    
                    // Combine the discriminant match with payload match
                    let combined_match = self.builder.build_and(
                        matches,
                        payload_match,
                        "enum_full_match"
                    )?;
                    
                    bindings.append(&mut payload_bindings);
                    combined_match
                } else {
                    matches
                }
            }
            
            Pattern::Type { type_name: _, binding } => {
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
                
                // Try to extract enum info from scrutinee type
                // For now, we'll try to match against any enum that has this variant
                // In a more complete implementation, we'd use type information
                
                
                // Check if scrutinee is an enum value
                if scrutinee_val.is_struct_value() || scrutinee_val.is_pointer_value() {
                    // Extract the discriminant
                    let discriminant = if scrutinee_val.is_pointer_value() {
                        // Assume it's a pointer to an enum struct
                        let discriminant_gep = self.builder.build_struct_gep(
                            self.context.struct_type(&[
                                self.context.i64_type().into(),
                                self.context.i64_type().into(),
                            ], false),
                            scrutinee_val.into_pointer_value(),
                            0,
                            "discriminant_ptr"
                        )?;
                        self.builder.build_load(
                            self.context.i64_type(),
                            discriminant_gep,
                            "discriminant"
                        )?
                    } else if scrutinee_val.is_struct_value() {
                        // Extract discriminant from struct value
                        let struct_val = scrutinee_val.into_struct_value();
                        self.builder.build_extract_value(struct_val, 0, "discriminant")?
                    } else {
                        return Err(CompileError::TypeMismatch {
                            expected: "enum value for enum literal pattern".to_string(),
                            found: format!("{:?}", scrutinee_val.get_type()),
                            span: None,
                        });
                    };
                    
                    // Search through all known enum types to find one with this variant
                    // This is a temporary solution - ideally we'd have type information
                    let mut variant_tag = None;
                    let mut _enum_info = None;
                    
                    for (_name, symbol) in self.symbols.iter() {
                        if let symbols::Symbol::EnumType(info) = symbol {
                            if let Some(tag) = info.variant_indices.get(variant) {
                                variant_tag = Some(*tag);
                                _enum_info = Some(info.clone());
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
                            "enum_literal_match"
                        )?;
                        
                        // Handle payload pattern if present
                        if let Some(payload_pattern) = payload {
                            // Extract the payload value
                            let payload_val = if scrutinee_val.is_pointer_value() {
                                let payload_gep = self.builder.build_struct_gep(
                                    self.context.struct_type(&[
                                        self.context.i64_type().into(),
                                        self.context.i64_type().into(),
                                    ], false),
                                    scrutinee_val.into_pointer_value(),
                                    1,
                                    "payload_ptr"
                                )?;
                                self.builder.build_load(
                                    self.context.i64_type(),
                                    payload_gep,
                                    "payload"
                                )?
                            } else if scrutinee_val.is_struct_value() {
                                let struct_val = scrutinee_val.into_struct_value();
                                self.builder.build_extract_value(struct_val, 1, "payload")?
                            } else {
                                self.context.i64_type().const_int(0, false).into()
                            };
                            
                            // Recursively match the payload pattern
                            let (payload_match, mut payload_bindings) = self.compile_pattern_test(
                                &payload_val,
                                payload_pattern
                            )?;
                            
                            // Combine the discriminant match with payload match
                            let combined_match = self.builder.build_and(
                                matches,
                                payload_match,
                                "enum_literal_full_match"
                            )?;
                            
                            bindings.append(&mut payload_bindings);
                            combined_match
                        } else {
                            matches
                        }
                    } else {
                        return Err(CompileError::UndeclaredVariable(
                            format!("Unknown enum variant '{}'", variant),
                            None
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
                let (pattern_match, mut pattern_bindings) = self.compile_pattern_test(scrutinee_val, pattern)?;
                
                // If pattern matches, evaluate the guard condition
                // For now, we need to set up the bindings before evaluating the condition
                for (name, value) in &pattern_bindings {
                    // Store the value in memory so we can use it
                    let alloca = self.builder.build_alloca(value.get_type(), &name)?;
                    self.builder.build_store(alloca, *value)?;
                    self.symbols.insert(name.clone(), symbols::Symbol::Variable(alloca));
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
                    "guard_match"
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
            let (int1, int2) = if int1.get_type().get_bit_width() != int2.get_type().get_bit_width() {
                // Cast to the larger width
                let target_width = std::cmp::max(int1.get_type().get_bit_width(), int2.get_type().get_bit_width());
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
            
            Ok(self.builder.build_int_compare(
                IntPredicate::EQ,
                int1,
                int2,
                "int_eq"
            )?)
        } else if val1.is_float_value() && val2.is_float_value() {
            Ok(self.builder.build_float_compare(
                inkwell::FloatPredicate::OEQ,
                val1.into_float_value(),
                val2.into_float_value(),
                "float_eq"
            )?)
        } else if val1.is_pointer_value() && val2.is_pointer_value() {
            Ok(self.builder.build_int_compare(
                IntPredicate::EQ,
                val1.into_pointer_value(),
                val2.into_pointer_value(),
                "ptr_eq"
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
        bindings: &[(String, BasicValueEnum<'ctx>)]
    ) -> HashMap<String, super::VariableInfo<'ctx>> {
        let mut saved = HashMap::new();
        
        for (name, value) in bindings {
            if let Some(existing) = self.variables.get(name) {
                saved.insert(name.clone(), existing.clone());
            }
            
            let actual_value = *value;
            
            let alloca = self.builder.build_alloca(actual_value.get_type(), name).unwrap();
            self.builder.build_store(alloca, actual_value).unwrap();
            
            let ast_type = match actual_value {
                BasicValueEnum::IntValue(iv) => {
                    match iv.get_type().get_bit_width() {
                        8 => crate::ast::AstType::I8,
                        16 => crate::ast::AstType::I16,
                        32 => crate::ast::AstType::I32,
                        64 => crate::ast::AstType::I64,
                        _ => crate::ast::AstType::I64,
                    }
                }
                BasicValueEnum::FloatValue(fv) => {
                    if fv.get_type() == self.context.f32_type() {
                        crate::ast::AstType::F32
                    } else {
                        crate::ast::AstType::F64
                    }
                }
                _ => crate::ast::AstType::I64,
            };
            
            self.variables.insert(name.clone(), super::VariableInfo {
                pointer: alloca,
                ast_type,
                is_mutable: false,  // Pattern bindings are immutable
                is_initialized: true,
            });
        }
        
        saved
    }
    
    pub fn restore_variables(
        &mut self,
        saved: HashMap<String, super::VariableInfo<'ctx>>
    ) {
        for (name, value) in saved {
            self.variables.insert(name, value);
        }
    }
}