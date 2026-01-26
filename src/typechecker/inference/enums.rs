//! Enum type inference

use crate::ast::{AstType, Expression};
use crate::error::Result;
use crate::typechecker::{EnumInfo, TypeChecker};
use crate::well_known::well_known;
use std::collections::HashMap;

/// Infer the type of an enum literal (e.g., .Some(x), .None)
pub fn infer_enum_literal_type(
    checker: &mut TypeChecker,
    variant: &str,
    payload: &Option<Box<Expression>>,
) -> Result<AstType> {
    let wk = well_known();

    if wk.is_some(variant) {
        let inner_type = if let Some(p) = payload {
            checker.infer_expression_type(p)?
        } else {
            AstType::Void
        };
        Ok(AstType::Generic {
            name: wk.get_variant_parent_name(variant).unwrap().to_string(),
            type_args: vec![inner_type],
        })
    } else if wk.is_none(variant) {
        Ok(AstType::Generic {
            name: wk.get_variant_parent_name(variant).unwrap().to_string(),
            type_args: vec![AstType::Generic {
                name: "T".to_string(),
                type_args: vec![],
            }],
        })
    } else if wk.is_ok(variant) {
        let ok_type = if let Some(p) = payload {
            checker.infer_expression_type(p)?
        } else {
            AstType::Void
        };
        Ok(AstType::Generic {
            name: wk.get_variant_parent_name(variant).unwrap().to_string(),
            type_args: vec![ok_type, crate::ast::resolve_string_struct_type()],
        })
    } else if wk.is_err(variant) {
        let err_type = if let Some(p) = payload {
            checker.infer_expression_type(p)?
        } else {
            AstType::Void
        };
        Ok(AstType::Generic {
            name: wk.get_variant_parent_name(variant).unwrap().to_string(),
            type_args: vec![
                AstType::Generic {
                    name: "T".to_string(),
                    type_args: vec![],
                },
                err_type,
            ],
        })
    } else {
        Ok(AstType::Void)
    }
}

/// Infer the type of an enum variant (e.g., Option.Some, Result.Ok)
pub fn infer_enum_variant_type(
    enum_name: &str,
    variant: &str,
    enums: &HashMap<String, EnumInfo>,
) -> Result<AstType> {
    let wk = well_known();
    let enum_type_name = if enum_name.is_empty() {
        let mut found_enum = None;
        for (name, info) in enums {
            for (var_name, _) in &info.variants {
                if var_name == variant {
                    found_enum = Some(name.clone());
                    break;
                }
            }
            if found_enum.is_some() {
                break;
            }
        }
        found_enum.unwrap_or_else(|| {
            wk.get_variant_parent_name(variant)
                .unwrap_or(wk.option_name())
                .to_string()
        })
    } else {
        enum_name.to_string()
    };

    if let Some(enum_info) = enums.get(&enum_type_name) {
        let variants = enum_info
            .variants
            .iter()
            .map(|(name, payload)| crate::ast::EnumVariant {
                name: name.clone(),
                payload: payload.clone(),
            })
            .collect();
        Ok(AstType::Enum {
            name: enum_type_name,
            variants,
        })
    } else {
        // If enum_type_name contains generic syntax, parse it as a type
        if enum_type_name.contains('<') {
            match crate::parser::parse_type_from_string(&enum_type_name) {
                Ok(parsed_type) => {
                    // Handle parsed types
                    match &parsed_type {
                        AstType::Generic { name: base_name, type_args } => {
                            // Handle pointer types specially
                            if wk.is_immutable_ptr(base_name) && type_args.len() == 1 {
                                return Ok(AstType::ptr(type_args[0].clone()));
                            } else if wk.is_mutable_ptr(base_name) && type_args.len() == 1 {
                                return Ok(AstType::mut_ptr(type_args[0].clone()));
                            } else if wk.is_raw_ptr(base_name) && type_args.len() == 1 {
                                return Ok(AstType::raw_ptr(type_args[0].clone()));
                            }
                            // Return the parsed generic type
                            return Ok(parsed_type);
                        }
                        // If it parsed to something else, return it
                        _ => return Ok(parsed_type),
                    }
                }
                Err(_) => {
                    // If parsing fails, fall through to generic with empty args
                }
            }
        }

        Ok(AstType::Generic {
            name: enum_type_name,
            type_args: vec![],
        })
    }
}
