//! Enum pattern compilation - EnumVariant patterns
//! Handles matching enum variants with optional payload patterns

use super::super::{symbols, LLVMCompiler};
use crate::ast::Pattern;
use crate::error::CompileError;
use inkwell::values::{BasicValueEnum, IntValue};
use inkwell::IntPredicate;

/// Compile an enum variant pattern match
pub fn compile_enum_variant_pattern<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    scrutinee_val: &BasicValueEnum<'ctx>,
    enum_name: &str,
    variant: &str,
    payload: &Option<Box<Pattern>>,
    bindings: &mut Vec<(String, BasicValueEnum<'ctx>)>,
) -> Result<IntValue<'ctx>, CompileError> {
    // Get the enum type info
    let enum_info = match compiler.symbols.lookup(enum_name) {
        Some(symbols::Symbol::EnumType(info)) => info.clone(),
        _ => {
            return Err(CompileError::UndeclaredVariable(
                format!("Enum '{}' not found", enum_name),
                None,
            ));
        }
    };

    // Get the expected discriminant value for this variant
    let expected_tag = enum_info
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
        let discriminant_gep = compiler.builder.build_struct_gep(
            enum_struct_type,
            scrutinee_val.into_pointer_value(),
            0,
            "discriminant_ptr",
        )?;
        compiler.builder.build_load(
            compiler.context.i64_type(),
            discriminant_gep,
            "discriminant",
        )?
    } else if scrutinee_val.is_struct_value() {
        // Extract discriminant from struct value
        let struct_val = scrutinee_val.into_struct_value();
        let extracted = compiler
            .builder
            .build_extract_value(struct_val, 0, "discriminant")?;
        // Cast to i64 if needed for comparison
        if extracted.is_int_value() {
            let int_val = extracted.into_int_value();
            if int_val.get_type().get_bit_width() < 64 {
                compiler
                    .builder
                    .build_int_z_extend(
                        int_val,
                        compiler.context.i64_type(),
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
                compiler
                    .builder
                    .build_int_z_extend(
                        int_val,
                        compiler.context.i64_type(),
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
    let expected_tag_val = compiler.context.i64_type().const_int(expected_tag, false);
    let matches = compiler.builder.build_int_compare(
        IntPredicate::EQ,
        discriminant.into_int_value(),
        expected_tag_val,
        "enum_variant_match",
    )?;

    // Handle payload pattern if present
    if let Some(payload_pattern) = payload {
        // Extract the payload value - preserve the actual type!
        let payload_val = extract_enum_payload(
            compiler,
            scrutinee_val,
            &enum_info,
            enum_name,
            variant,
        )?;

        // Recursively match the payload pattern
        let (payload_match, mut payload_bindings) =
            compiler.compile_pattern_test(&payload_val, payload_pattern)?;

        // Combine the discriminant match with payload match
        let combined_match = compiler
            .builder
            .build_and(matches, payload_match, "enum_literal_full_match")
            .map_err(|e| CompileError::InternalError(format!("LLVM builder error: {}", e), None))?;

        bindings.append(&mut payload_bindings);
        Ok(combined_match)
    } else {
        Ok(matches)
    }
}

/// Extract the payload value from an enum variant
fn extract_enum_payload<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    scrutinee_val: &BasicValueEnum<'ctx>,
    enum_info: &symbols::EnumInfo<'ctx>,
    enum_name: &str,
    variant: &str,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // This is a simplified version - the full implementation would handle
    // all the complex generic type tracking and nested enum extraction
    // For now, return a placeholder that will be expanded
    if scrutinee_val.is_pointer_value() {
        let enum_struct_type = enum_info.llvm_type;
        // Check if enum has payload field
        if enum_struct_type.count_fields() > 1 {
            let payload_gep = compiler.builder.build_struct_gep(
                enum_struct_type,
                scrutinee_val.into_pointer_value(),
                1,
                "payload_ptr",
            )?;

            // Get the actual payload type from the enum struct
            let payload_type = enum_struct_type
                .get_field_type_at_index(1)
                .ok_or_else(|| {
                    CompileError::InternalError(
                        "Enum payload field not found".to_string(),
                        None,
                    )
                })?;

            // Load the payload pointer
            let loaded_payload = compiler
                .builder
                .build_load(payload_type, payload_gep, "payload")?;

            // Handle pointer types and nested generics
            // This is a simplified version - full implementation would handle
            // all the complex type tracking seen in the original code
            if payload_type.is_pointer_type() && loaded_payload.is_pointer_value() {
                let ptr_val = loaded_payload.into_pointer_value();
                
                // Try to determine payload type from generic context
                let load_type = get_payload_type_from_context(compiler, enum_name, variant);
                
                if let Some(ast_type) = load_type {
                    load_payload_by_type(compiler, ptr_val, &ast_type, loaded_payload)
                } else {
                    // Default: try i32 first, then i64
                    try_load_integer(compiler, ptr_val, loaded_payload)
                }
            } else {
                Ok(loaded_payload)
            }
        } else {
            Ok(compiler.context.i64_type().const_int(0, false).into())
        }
    } else if scrutinee_val.is_struct_value() {
        let struct_val = scrutinee_val.into_struct_value();
        if struct_val.get_type().count_fields() > 1 {
            let extracted = compiler
                .builder
                .build_extract_value(struct_val, 1, "payload")?;
            
            if extracted.is_pointer_value() {
                let ptr_val = extracted.into_pointer_value();
                let load_type = get_payload_type_from_context(compiler, enum_name, variant);
                
                if let Some(ast_type) = load_type {
                    load_payload_by_type(compiler, ptr_val, &ast_type, extracted)
                } else {
                    try_load_integer(compiler, ptr_val, extracted)
                }
            } else {
                Ok(extracted)
            }
        } else {
            Ok(compiler.context.i64_type().const_int(0, false).into())
        }
    } else {
        Ok(compiler.context.i64_type().const_int(0, false).into())
    }
}

/// Get payload type from generic type context
fn get_payload_type_from_context<'ctx>(
    compiler: &LLVMCompiler<'ctx>,
    enum_name: &str,
    variant: &str,
) -> Option<crate::ast::AstType> {
    if enum_name == "Option" && variant == "Some" {
        compiler
            .generic_type_context
            .get("Nested_Option_Some_Type")
            .cloned()
            .or_else(|| compiler.generic_type_context.get("Option_Some_Type").cloned())
            .or_else(|| compiler.generic_tracker.get("Option_Some_Type").cloned())
    } else if enum_name == "Result" {
        if variant == "Ok" {
            compiler
                .generic_type_context
                .get("Nested_Result_Ok_Result_Ok_Type")
                .cloned()
                .or_else(|| {
                    compiler
                        .generic_type_context
                        .get("Nested_Result_Ok_Type")
                        .cloned()
                })
                .or_else(|| compiler.generic_type_context.get("Result_Ok_Type").cloned())
                .or_else(|| compiler.generic_tracker.get("Result_Ok_Type").cloned())
        } else if variant == "Err" {
            compiler
                .generic_type_context
                .get("Nested_Result_Err_Type")
                .cloned()
                .or_else(|| compiler.generic_type_context.get("Result_Err_Type").cloned())
                .or_else(|| compiler.generic_tracker.get("Result_Err_Type").cloned())
        } else {
            None
        }
    } else {
        None
    }
}

/// Load payload by AST type
fn load_payload_by_type<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    ptr_val: inkwell::values::PointerValue<'ctx>,
    ast_type: &crate::ast::AstType,
    fallback: BasicValueEnum<'ctx>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    use crate::ast::AstType;
    
    match ast_type {
        AstType::I8 => Ok(compiler
            .builder
            .build_load(compiler.context.i8_type(), ptr_val, "payload_i8")
            .unwrap_or(fallback)),
        AstType::I16 => Ok(compiler
            .builder
            .build_load(compiler.context.i16_type(), ptr_val, "payload_i16")
            .unwrap_or(fallback)),
        AstType::I32 => Ok(compiler
            .builder
            .build_load(compiler.context.i32_type(), ptr_val, "payload_i32")
            .unwrap_or(fallback)),
        AstType::I64 => Ok(compiler
            .builder
            .build_load(compiler.context.i64_type(), ptr_val, "payload_i64")
            .unwrap_or(fallback)),
        AstType::F32 => Ok(compiler
            .builder
            .build_load(compiler.context.f32_type(), ptr_val, "payload_f32")
            .unwrap_or(fallback)),
        AstType::F64 => Ok(compiler
            .builder
            .build_load(compiler.context.f64_type(), ptr_val, "payload_f64")
            .unwrap_or(fallback)),
        AstType::Struct { name, .. } if name == "String" => Ok(fallback),
        AstType::StaticString | AstType::StaticLiteral => Ok(fallback),
        _ => try_load_integer(compiler, ptr_val, fallback),
    }
}

/// Try to load as integer (i32 first, then i64)
fn try_load_integer<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    ptr_val: inkwell::values::PointerValue<'ctx>,
    fallback: BasicValueEnum<'ctx>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    match compiler
        .builder
        .build_load(compiler.context.i32_type(), ptr_val, "try_i32")
    {
        Ok(int_val) => Ok(int_val),
        _ => match compiler
            .builder
            .build_load(compiler.context.i64_type(), ptr_val, "try_i64")
        {
            Ok(loaded) => Ok(loaded),
            _ => Ok(fallback),
        },
    }
}
