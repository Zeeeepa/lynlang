use crate::ast::AstType;
use crate::stdlib_types::StdlibTypeRegistry;
use crate::well_known::well_known;

/// Check if two types are compatible (for assignment, parameter passing, etc.)
pub fn types_compatible(expected: &AstType, actual: &AstType) -> bool {
    // Exact match is always compatible
    if std::mem::discriminant(expected) == std::mem::discriminant(actual) {
        return true;
    }

    // Check for numeric compatibility with implicit conversions
    if expected.is_numeric() && actual.is_numeric() {
        // Allow widening conversions (smaller to larger)
        if let (Some(expected_size), Some(actual_size)) = (expected.bit_size(), actual.bit_size()) {
            // Allow if actual fits in expected
            if actual_size <= expected_size {
                // Check sign compatibility
                if expected.is_signed_integer() && actual.is_unsigned_integer() {
                    // Unsigned to signed is OK if there's room
                    return actual_size < expected_size;
                }
                return true;
            }
        }
    }

    // Check for string type compatibility
    // StaticLiteral is internal only
    // StaticString can be coerced to String (requires allocator at runtime)
    // But String cannot be coerced back to StaticString
    match (expected, actual) {
        // StaticLiteral is internal - it should be compatible with StaticString
        (AstType::StaticString, AstType::StaticLiteral) => return true,
        (AstType::StaticLiteral, AstType::StaticString) => return true,

        // StaticString -> String struct is ok (will need allocator at runtime)
        (AstType::Struct { name, .. }, AstType::StaticString) if StdlibTypeRegistry::is_string_type(name) => return true,
        (AstType::Struct { name, .. }, AstType::StaticLiteral) if StdlibTypeRegistry::is_string_type(name) => return true, // Internal literal -> dynamic is ok

        // String struct -> StaticString is NOT ok (would lose allocator)
        (AstType::StaticString, AstType::Struct { name, .. }) if StdlibTypeRegistry::is_string_type(name) => return false,
        (AstType::StaticLiteral, AstType::Struct { name, .. }) if StdlibTypeRegistry::is_string_type(name) => return false,
        _ => {}
    }

    // Check for pointer compatibility
    if expected.is_ptr_type() && actual.is_ptr_type() {
        if let (Some(expected_inner), Some(actual_inner)) = (expected.ptr_inner(), actual.ptr_inner()) {
            return types_compatible(expected_inner, actual_inner);
        }
    }
    // Allow array to decay to pointer
    if expected.is_ptr_type() {
        if let Some(expected_inner) = expected.ptr_inner() {
            if let AstType::Array(actual_inner) = actual {
                return types_compatible(expected_inner, actual_inner);
            }
            if let AstType::FixedArray { element_type, .. } = actual {
                return types_compatible(expected_inner, element_type);
            }
        }
    }
    match (expected, actual) {
        // Check struct compatibility
        (
            AstType::Struct {
                name: expected_name,
                ..
            },
            AstType::Struct {
                name: actual_name, ..
            },
        ) => expected_name == actual_name,
        // Check enum compatibility
        (
            AstType::Enum {
                name: expected_name,
                ..
            },
            AstType::Enum {
                name: actual_name, ..
            },
        ) => expected_name == actual_name,
        // Allow Generic type to match Enum type when name matches (for type declarations)
        (
            AstType::Generic {
                name: expected_name,
                type_args,
            },
            AstType::Enum {
                name: actual_name, ..
            },
        ) => expected_name == actual_name && type_args.is_empty(),
        // Allow Enum type to match Generic type when name matches (for type declarations)
        (
            AstType::Enum {
                name: expected_name,
                ..
            },
            AstType::Generic {
                name: actual_name,
                type_args,
            },
        ) => expected_name == actual_name && type_args.is_empty(),
        // Allow struct type to be assigned to enum if the struct is one of the enum's variants
        (
            AstType::Enum { variants, .. },
            AstType::Struct {
                name: struct_name, ..
            },
        ) => variants.iter().any(|v| v.name == *struct_name),
        // Allow struct type to be assigned to generic enum type
        (
            AstType::Generic {
                name: _enum_name,
                type_args,
            },
            AstType::Struct {
                name: _struct_name, ..
            },
        ) if type_args.is_empty() => {
            // This is a workaround - we'd need to look up the actual enum type to verify
            // For now, we'll assume it's valid if the names are plausible
            // TODO: Improve this by looking up the actual enum definition
            true
        }
        // Option and Result are now Generic types - handled in Generic match below
        // Check Option<T> compatibility using generic syntax
        (
            AstType::Generic {
                name: expected_name,
                type_args: expected_args,
            },
            AstType::Generic {
                name: actual_name,
                type_args: actual_args,
            },
        ) if well_known().is_option(expected_name) && well_known().is_option(actual_name) => {
            expected_args.len() == actual_args.len()
                && expected_args
                    .iter()
                    .zip(actual_args.iter())
                    .all(|(e, a)| types_compatible(e, a))
        }
        // Check Result<T,E> compatibility using generic syntax
        (
            AstType::Generic {
                name: expected_name,
                type_args: expected_args,
            },
            AstType::Generic {
                name: actual_name,
                type_args: actual_args,
            },
        ) if well_known().is_result(expected_name) && well_known().is_result(actual_name) => {
            expected_args.len() == actual_args.len()
                && expected_args
                    .iter()
                    .zip(actual_args.iter())
                    .all(|(e, a)| types_compatible(e, a))
        }
        // Check range compatibility
        (
            AstType::Range {
                start_type: expected_start,
                end_type: expected_end,
                ..
            },
            AstType::Range {
                start_type: actual_start,
                end_type: actual_end,
                ..
            },
        ) => {
            types_compatible(expected_start, actual_start)
                && types_compatible(expected_end, actual_end)
        }
        // Function and FunctionPointer compatibility
        (
            AstType::Function {
                args: expected_args,
                return_type: expected_ret,
            },
            AstType::FunctionPointer {
                param_types: actual_params,
                return_type: actual_ret,
            },
        ) => {
            expected_args.len() == actual_params.len()
                && expected_args
                    .iter()
                    .zip(actual_params.iter())
                    .all(|(e, a)| types_compatible(e, a))
                && types_compatible(expected_ret, actual_ret)
        }
        (
            AstType::FunctionPointer {
                param_types: expected_params,
                return_type: expected_ret,
            },
            AstType::Function {
                args: actual_args,
                return_type: actual_ret,
            },
        ) => {
            expected_params.len() == actual_args.len()
                && expected_params
                    .iter()
                    .zip(actual_args.iter())
                    .all(|(e, a)| types_compatible(e, a))
                && types_compatible(expected_ret, actual_ret)
        }
        // Void is only compatible with void
        (AstType::Void, AstType::Void) => true,
        // All other combinations are incompatible
        _ => false,
    }
}

/// Validate that imports are not inside comptime blocks
pub fn validate_import_not_in_comptime(stmt: &crate::ast::Statement) -> Result<(), String> {
    use crate::ast::Statement;

    // Check if this is a ModuleImport statement
    if let Statement::ModuleImport { alias, module_path } = stmt {
        return Err(format!(
            "Import statement '{}' for module '{}' cannot be inside a comptime block. \
            Imports must be at module level.",
            alias, module_path
        ));
    }

    // Also check for variable declarations that look like imports
    if let Statement::VariableDeclaration {
        name, initializer, ..
    } = stmt
    {
        if let Some(expr) = initializer {
            if contains_import_expression(expr) {
                return Err(format!(
                    "Import-like statement '{}' cannot be inside a comptime block. \
                    Imports must be at module level.",
                    name
                ));
            }
        }
    }

    // Check for nested comptime blocks that might contain imports
    if let Statement::ComptimeBlock { statements: nested_stmts, .. } = stmt {
        for nested_stmt in nested_stmts {
            validate_import_not_in_comptime(nested_stmt)?;
        }
    }

    Ok(())
}

/// Check if an expression contains import-related patterns
fn contains_import_expression(expr: &crate::ast::Expression) -> bool {
    match expr {
        crate::ast::Expression::Identifier(id) if id.starts_with("@std") => true,
        crate::ast::Expression::MemberAccess { object, .. } => {
            if let crate::ast::Expression::Identifier(id) = &**object {
                id.starts_with("@std") || id == "build"
            } else {
                contains_import_expression(object)
            }
        }
        crate::ast::Expression::FunctionCall { name, .. } if name.contains("import") => true,
        _ => false,
    }
}

