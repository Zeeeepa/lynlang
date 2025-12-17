//! Helper functions for pattern compilation
//! Value comparison, binding application, variable restoration

use super::super::{LLVMCompiler, VariableInfo};
use crate::error::CompileError;
use inkwell::values::{BasicValueEnum, IntValue};
use inkwell::{AddressSpace, IntPredicate};
use std::collections::HashMap;

/// Compare two values for equality
pub fn values_equal<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    val1: &BasicValueEnum<'ctx>,
    val2: &BasicValueEnum<'ctx>,
) -> Result<IntValue<'ctx>, CompileError> {
    if val1.is_int_value() && val2.is_int_value() {
        let int1 = val1.into_int_value();
        let int2 = val2.into_int_value();

        // Ensure both integers have the same bit width for comparison
        let (int1, int2) = if int1.get_type().get_bit_width() != int2.get_type().get_bit_width() {
            // Cast to the larger width
            let target_width = std::cmp::max(
                int1.get_type().get_bit_width(),
                int2.get_type().get_bit_width(),
            );
            let target_type = compiler.context.custom_width_int_type(target_width);

            let int1 = if int1.get_type().get_bit_width() < target_width {
                compiler
                    .builder
                    .build_int_z_extend(int1, target_type, "ext1")?
            } else {
                int1
            };

            let int2 = if int2.get_type().get_bit_width() < target_width {
                compiler
                    .builder
                    .build_int_z_extend(int2, target_type, "ext2")?
            } else {
                int2
            };

            (int1, int2)
        } else {
            (int1, int2)
        };

        Ok(compiler
            .builder
            .build_int_compare(IntPredicate::EQ, int1, int2, "int_eq")?)
    } else if val1.is_float_value() && val2.is_float_value() {
        Ok(compiler.builder.build_float_compare(
            inkwell::FloatPredicate::OEQ,
            val1.into_float_value(),
            val2.into_float_value(),
            "float_eq",
        )?)
    } else if val1.is_pointer_value() && val2.is_pointer_value() {
        Ok(compiler.builder.build_int_compare(
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

/// Apply pattern bindings to variables
pub fn apply_pattern_bindings<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    bindings: &[(String, BasicValueEnum<'ctx>)],
) -> HashMap<String, VariableInfo<'ctx>> {
    let mut saved = HashMap::new();

    for (name, value) in bindings {
        if let Some(existing) = compiler.variables.get(name) {
            saved.insert(name.clone(), existing.clone());
        }

        // Determine the AST type based on the LLVM type
        let (actual_value, ast_type) = if value.is_int_value() {
            let int_val = value.into_int_value();

            // Check if this is actually a pointer stored as i64 (common for Option payloads)
            // When we have type information from Option_Some_Type, use it
            if int_val.get_type().get_bit_width() == 64 {
                if let Some(option_type) = compiler.generic_tracker.get("Option_Some_Type") {
                    // Check if it's a DynVec type
                    if let crate::ast::AstType::DynVec { .. } = option_type {
                        // This i64 is actually a pointer to a DynVec struct
                        // Convert it to a pointer and load the struct
                        let ptr_val = compiler
                            .builder
                            .build_int_to_ptr(
                                int_val,
                                compiler.context.ptr_type(AddressSpace::default()),
                                "dynvec_ptr_from_i64",
                            )
                            .unwrap();

                        // DynVec struct type
                        let dynvec_struct_type = compiler.context.struct_type(
                            &[
                                compiler.context.ptr_type(AddressSpace::default()).into(),
                                compiler.context.i64_type().into(),
                                compiler.context.i64_type().into(),
                                compiler.context.ptr_type(AddressSpace::default()).into(),
                            ],
                            false,
                        );

                        let loaded_struct = compiler
                            .builder
                            .build_load(dynvec_struct_type, ptr_val, "loaded_dynvec")
                            .unwrap();

                        (loaded_struct, option_type.clone())
                    } else {
                        // Regular i64 or other type stored as i64
                        (*value, option_type.clone())
                    }
                } else if let Some(option_type) =
                    compiler.generic_type_context.get("Option_Some_Type")
                {
                    // Check if it's a DynVec type
                    if let crate::ast::AstType::DynVec { .. } = option_type {
                        // This i64 is actually a pointer to a DynVec struct
                        // Convert it to a pointer and load the struct
                        let ptr_val = compiler
                            .builder
                            .build_int_to_ptr(
                                int_val,
                                compiler.context.ptr_type(AddressSpace::default()),
                                "dynvec_ptr_from_i64",
                            )
                            .unwrap();

                        // DynVec struct type
                        let dynvec_struct_type = compiler.context.struct_type(
                            &[
                                compiler.context.ptr_type(AddressSpace::default()).into(),
                                compiler.context.i64_type().into(),
                                compiler.context.i64_type().into(),
                                compiler.context.ptr_type(AddressSpace::default()).into(),
                            ],
                            false,
                        );

                        let loaded_struct = compiler
                            .builder
                            .build_load(dynvec_struct_type, ptr_val, "loaded_dynvec")
                            .unwrap();

                        (loaded_struct, option_type.clone())
                    } else {
                        // Regular i64 or other type stored as i64
                        (*value, option_type.clone())
                    }
                } else {
                    // No type information, treat as regular i64
                    (*value, crate::ast::AstType::I64)
                }
            } else {
                // Map integer types
                let ast_type = match int_val.get_type().get_bit_width() {
                    8 => crate::ast::AstType::I8,
                    16 => crate::ast::AstType::I16,
                    32 => crate::ast::AstType::I32,
                    64 => crate::ast::AstType::I64,
                    _ => crate::ast::AstType::I64,
                };
                (*value, ast_type)
            }
        } else if value.is_float_value() {
            let fv = value.into_float_value();
            let ast_type = if fv.get_type() == compiler.context.f32_type() {
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
            // 3. DynVec or other collection types (pointer to struct)
            //
            // Check if we have type information from the Option_Some_Type tracker
            let ast_type = if let Some(option_type) =
                compiler.generic_tracker.get("Option_Some_Type")
            {
                // Use the tracked type from Option extraction
                option_type.clone()
            } else {
                // Also check the generic_type_context HashMap
                if let Some(option_type) = compiler.generic_type_context.get("Option_Some_Type") {
                    option_type.clone()
                } else {
                    // For pointers, we need to detect if it's a DynVec based on what we stored
                    // But for now, just check our trackers
                    crate::ast::AstType::StaticString
                }
            };
            (*value, ast_type)
        } else if value.is_struct_value() {
            // For struct values, check if it's an enum struct (has 2 fields: discriminant + payload)
            let struct_val = value.into_struct_value();
            if struct_val.get_type().count_fields() == 2 {
                // This is likely an enum struct (Option or Result)
                // We need to be careful: if we extracted a payload from Option.Some where the
                // payload itself is an enum (like Result), then the struct we're looking at
                // IS that inner enum, not the outer Option
                let ast_type =
                    if let Some(option_type) = compiler.generic_tracker.get("Option_Some_Type") {
                        // Check if Option_Some_Type is itself a generic enum
                        if let crate::ast::AstType::Generic { name, .. } = option_type {
                            if name == "Result" || name == "Option" {
                                // The struct we're binding IS the inner enum type
                                option_type.clone()
                            } else {
                                // Regular Option with non-enum payload
                                crate::ast::AstType::Generic {
                                    name: "Option".to_string(),
                                    type_args: vec![option_type.clone()],
                                }
                            }
                        } else {
                            // Regular Option with primitive payload
                            crate::ast::AstType::Generic {
                                name: "Option".to_string(),
                                type_args: vec![option_type.clone()],
                            }
                        }
                    } else if let Some(ok_type) = compiler.generic_tracker.get("Result_Ok_Type") {
                        // We have Result type information - the struct IS a Result
                        let err_type = compiler
                            .generic_tracker
                            .get("Result_Err_Type")
                            .cloned()
                            .unwrap_or(crate::ast::AstType::StaticString);
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

        let alloca = compiler
            .builder
            .build_alloca(actual_value.get_type(), name)
            .unwrap();
        compiler.builder.build_store(alloca, actual_value).unwrap();

        compiler.variables.insert(
            name.clone(),
            VariableInfo {
                pointer: alloca,
                ast_type,
                is_mutable: false, // Pattern bindings are immutable
                is_initialized: true,
            },
        );
    }

    saved
}

/// Restore saved variables
pub fn restore_variables<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    saved: HashMap<String, VariableInfo<'ctx>>,
) {
    for (name, value) in saved {
        compiler.variables.insert(name, value);
    }
}
