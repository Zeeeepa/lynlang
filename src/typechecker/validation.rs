use crate::ast::AstType;
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
        (AstType::Struct { name, .. }, AstType::StaticString) if name == "String" => return true,
        (AstType::Struct { name, .. }, AstType::StaticLiteral) if name == "String" => return true, // Internal literal -> dynamic is ok

        // String struct -> StaticString is NOT ok (would lose allocator)
        (AstType::StaticString, AstType::Struct { name, .. }) if name == "String" => return false,
        (AstType::StaticLiteral, AstType::Struct { name, .. }) if name == "String" => return false,
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

/// Check if a type can be implicitly converted to another
#[allow(dead_code)]
pub fn can_implicitly_convert(from: &AstType, to: &AstType) -> bool {
    // Same type needs no conversion
    if std::mem::discriminant(from) == std::mem::discriminant(to) {
        return true;
    }

    // Numeric widening conversions
    if from.is_numeric() && to.is_numeric() {
        if let (Some(from_size), Some(to_size)) = (from.bit_size(), to.bit_size()) {
            // Allow widening
            if from_size <= to_size {
                // Check sign compatibility
                if from.is_unsigned_integer() && to.is_signed_integer() {
                    // Unsigned to signed needs extra bit for sign
                    return from_size < to_size;
                }
                return true;
            }
        }
    }

    // Array to pointer decay
    if let Some(to_elem) = to.ptr_inner() {
        if let AstType::Array(from_elem) = from {
            if types_compatible(from_elem, to_elem) {
                return true;
            }
        }
        if let AstType::FixedArray { element_type: from_elem, .. } = from {
            if types_compatible(from_elem, to_elem) {
                return true;
            }
        }
    }
    false
}

/// Check if a type requires explicit initialization
#[allow(dead_code)]
pub fn requires_initialization(type_: &AstType) -> bool {
    match type_ {
        // References must be initialized
        AstType::Ref(_) => true,
        // Immutable values should be initialized
        AstType::Struct { .. } | AstType::Enum { .. } => false,
        // Primitives can have default values
        _ => false,
    }
}

/// Check if a type can be used in a loop condition
#[allow(dead_code)]
pub fn is_valid_condition_type(type_: &AstType) -> bool {
    matches!(type_, AstType::Bool)
        || type_.is_numeric()
        || matches!(type_, AstType::Generic { name, .. } if well_known().is_option(name))
}

/// Check if a type can be indexed
#[allow(dead_code)]
pub fn can_be_indexed(type_: &AstType) -> Option<AstType> {
    if let Some(inner) = type_.ptr_inner() {
        return Some(inner.clone());
    }
    match type_ {
        AstType::Array(elem_type) => Some((**elem_type).clone()),
        AstType::FixedArray { element_type, .. } => Some((**element_type).clone()),
        AstType::Struct { name, .. } if name == "String" => Some(AstType::U8), // Indexing string gives bytes
        _ => None,
    }
}

/// Check if a type supports the dereference operation
#[allow(dead_code)]
pub fn can_be_dereferenced(type_: &AstType) -> Option<AstType> {
    if let Some(inner) = type_.ptr_inner() {
        return Some(inner.clone());
    }
    match type_ {
        AstType::Ref(inner) => Some((**inner).clone()),
        _ => None,
    }
}

/// Validate that imports are not inside comptime blocks
#[allow(dead_code)]
pub fn validate_import_not_in_comptime(stmt: &crate::ast::Statement) -> Result<(), String> {
    use crate::ast::Statement;

    // Check if this is a ModuleImport statement
    if let Statement::ModuleImport { alias, module_path } = stmt {
        // This function is called from within comptime block checking,
        // so if we reach here with a ModuleImport, it's invalid
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
    if let Statement::ComptimeBlock(nested_stmts) = stmt {
        for nested_stmt in nested_stmts {
            validate_import_not_in_comptime(nested_stmt)?;
        }
    }

    Ok(())
}

/// Check if an expression contains import-related patterns
#[allow(dead_code)]
pub fn contains_import_expression(expr: &crate::ast::Expression) -> bool {
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
