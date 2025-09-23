use crate::ast::{AstType, BinaryOperator, Expression};
use crate::error::{CompileError, Result};
use crate::typechecker::{TypeChecker, StructInfo, EnumInfo};
use std::collections::HashMap;

/// Infer the type of a binary operation
pub fn infer_binary_op_type(
    checker: &TypeChecker,
    left: &Expression,
    op: &BinaryOperator,
    right: &Expression,
) -> Result<AstType> {
    let left_type = checker.infer_expression_type(left)?;
    let right_type = checker.infer_expression_type(right)?;

    match op {
        BinaryOperator::Add
        | BinaryOperator::Subtract
        | BinaryOperator::Multiply
        | BinaryOperator::Divide
        | BinaryOperator::Modulo => {
            // Numeric operations
            if left_type.is_numeric() && right_type.is_numeric() {
                // Promote to the larger type
                promote_numeric_types(&left_type, &right_type)
            } else {
                Err(CompileError::TypeError(format!(
                    "Cannot apply {:?} to types {:?} and {:?}",
                    op, left_type, right_type
                ), None))
            }
        }
        BinaryOperator::Equals
        | BinaryOperator::NotEquals
        | BinaryOperator::LessThan
        | BinaryOperator::GreaterThan
        | BinaryOperator::LessThanEquals
        | BinaryOperator::GreaterThanEquals => {
            // Comparison operations return bool
            if types_comparable(&left_type, &right_type) {
                Ok(AstType::Bool)
            } else {
                Err(CompileError::TypeError(format!(
                    "Cannot compare types {:?} and {:?}",
                    left_type, right_type
                ), None))
            }
        }
        BinaryOperator::And | BinaryOperator::Or => {
            // Logical operations
            if matches!(left_type, AstType::Bool) && matches!(right_type, AstType::Bool) {
                Ok(AstType::Bool)
            } else {
                Err(CompileError::TypeError(format!(
                    "Logical operators require boolean operands, got {:?} and {:?}",
                    left_type, right_type
                ), None))
            }
        }
        BinaryOperator::StringConcat => {
            // String concatenation
            if matches!(left_type, AstType::String) && matches!(right_type, AstType::String) {
                Ok(AstType::String)
            } else {
                Err(CompileError::TypeError(format!(
                    "String concatenation requires string operands, got {:?} and {:?}",
                    left_type, right_type
                ), None))
            }
        }
    }
}

/// Infer the type of a member access expression
pub fn infer_member_type(
    object_type: &AstType,
    member: &str,
    structs: &HashMap<String, StructInfo>,
    enums: &HashMap<String, EnumInfo>,
) -> Result<AstType> {
    match object_type {
        AstType::Struct { name, .. } => {
            if let Some(struct_info) = structs.get(name) {
                for (field_name, field_type) in &struct_info.fields {
                    if field_name == member {
                        return Ok(field_type.clone());
                    }
                }
                Err(CompileError::TypeError(format!(
                    "Struct '{}' has no field '{}'",
                    name, member
                ), None))
            } else {
                Err(CompileError::TypeError(format!(
                    "Unknown struct type: {}",
                    name
                ), None))
            }
        }
        // Handle pointer to struct types
        AstType::Ptr(inner) => {
            // Dereference the pointer and check the inner type
            infer_member_type(inner, member, structs, enums)
        }
        // Handle Generic types that represent structs
        AstType::Generic { name, .. } => {
            // Try to look up the struct info by name
            if let Some(struct_info) = structs.get(name) {
                for (field_name, field_type) in &struct_info.fields {
                    if field_name == member {
                        return Ok(field_type.clone());
                    }
                }
                Err(CompileError::TypeError(format!(
                    "Struct '{}' has no field '{}'",
                    name, member
                ), None))
            } else {
                Err(CompileError::TypeError(format!(
                    "Type '{}' is not a struct or is not defined",
                    name
                ), None))
            }
        }
        // Handle enum type constructors
        AstType::EnumType { name } => {
            // Check if the member is a valid variant of this enum
            if let Some(enum_info) = enums.get(name) {
                for (variant_name, _variant_type) in &enum_info.variants {
                    if variant_name == member {
                        // Return the enum type itself - the variant constructor creates an instance of the enum
                        let enum_variants = enum_info.variants.iter().map(|(name, payload)| {
                            crate::ast::EnumVariant {
                                name: name.clone(),
                                payload: payload.clone(),
                            }
                        }).collect();
                        return Ok(AstType::Enum {
                            name: name.clone(),
                            variants: enum_variants,
                        });
                    }
                }
                Err(CompileError::TypeError(format!(
                    "Enum '{}' has no variant '{}'",
                    name, member
                ), None))
            } else {
                Err(CompileError::TypeError(format!(
                    "Unknown enum type: {}",
                    name
                ), None))
            }
        }
        AstType::StdModule => {
            // Handle stdlib module member access (e.g., math.pi)
            // For now, we'll just return F64 for math.pi
            // TODO: Implement a proper registry of stdlib module members
            if member == "pi" {
                Ok(AstType::F64)
            } else {
                Err(CompileError::TypeError(format!(
                    "Unknown stdlib module member: {}",
                    member
                ), None))
            }
        }
        _ => Err(CompileError::TypeError(format!(
            "Cannot access member '{}' on type {:?}",
            member, object_type
        ), None)),
    }
}

/// Promote two numeric types to their common type
fn promote_numeric_types(left: &AstType, right: &AstType) -> Result<AstType> {
    // If either is a float, promote to float
    if left.is_float() || right.is_float() {
        if matches!(left, AstType::F64) || matches!(right, AstType::F64) {
            Ok(AstType::F64)
        } else {
            Ok(AstType::F32)
        }
    }
    // Both are integers
    else if left.is_integer() && right.is_integer() {
        // Promote to the larger size
        let left_size = left.bit_size().unwrap_or(32);
        let right_size = right.bit_size().unwrap_or(32);
        let max_size = left_size.max(right_size);

        // If either is unsigned and the other is signed, need special handling
        if left.is_unsigned_integer() != right.is_unsigned_integer() {
            // For now, promote to signed of the appropriate size
            match max_size {
                8 => Ok(AstType::I8),
                16 => Ok(AstType::I16),
                32 => Ok(AstType::I32),
                64 => Ok(AstType::I64),
                _ => Ok(AstType::I32),
            }
        } else if left.is_unsigned_integer() {
            // Both unsigned
            match max_size {
                8 => Ok(AstType::U8),
                16 => Ok(AstType::U16),
                32 => Ok(AstType::U32),
                64 => Ok(AstType::U64),
                _ => Ok(AstType::U32),
            }
        } else {
            // Both signed
            match max_size {
                8 => Ok(AstType::I8),
                16 => Ok(AstType::I16),
                32 => Ok(AstType::I32),
                64 => Ok(AstType::I64),
                _ => Ok(AstType::I32),
            }
        }
    } else {
        Err(CompileError::TypeError(format!(
            "Cannot promote types {:?} and {:?}",
            left, right
        ), None))
    }
}

/// Check if two types can be compared
fn types_comparable(left: &AstType, right: &AstType) -> bool {
    // Same type is always comparable
    if std::mem::discriminant(left) == std::mem::discriminant(right) {
        return true;
    }

    // Numeric types can be compared
    if left.is_numeric() && right.is_numeric() {
        return true;
    }

    // Strings can be compared
    if matches!(left, AstType::String) && matches!(right, AstType::String) {
        return true;
    }

    // Booleans can be compared
    if matches!(left, AstType::Bool) && matches!(right, AstType::Bool) {
        return true;
    }

    false
}