use crate::ast::{AstType, Expression, Function, Statement, TraitImplementation};
use crate::error::Result;

/// Transform Self types to concrete types in trait implementations
#[allow(dead_code)]
pub fn transform_trait_impl_self_types(
    trait_impl: &TraitImplementation,
) -> Result<TraitImplementation> {
    let mut transformed = trait_impl.clone();
    let concrete_type = &trait_impl.type_name;

    // Transform each method in the trait implementation
    for method in &mut transformed.methods {
        *method = transform_function_self_types(method, concrete_type)?;
    }

    Ok(transformed)
}

/// Transform Self types in a function to the concrete type
#[allow(dead_code)]
pub fn transform_function_self_types(func: &Function, concrete_type: &str) -> Result<Function> {
    let mut transformed = func.clone();

    // Transform parameter types
    for (_, param_type) in &mut transformed.args {
        *param_type = replace_self_in_ast_type(param_type, concrete_type);
    }

    // Transform return type
    transformed.return_type = replace_self_in_ast_type(&transformed.return_type, concrete_type);

    // Transform types in function body
    transformed.body = transform_statements_self_types(&transformed.body, concrete_type)?;

    Ok(transformed)
}

/// Replace Self in an AST type
pub fn replace_self_in_ast_type(ast_type: &AstType, concrete_type: &str) -> AstType {
    match ast_type {
        AstType::Generic { name, type_args: _ } if name == "Self" => {
            // Keep Self as Generic but tag it with the concrete type name
            // This allows the typechecker and codegen to resolve it properly with full struct info
            AstType::Generic {
                name: format!("Self_{}", concrete_type), // Tagged with concrete type
                type_args: vec![],
            }
        }
        AstType::Ptr(inner) => {
            AstType::Ptr(Box::new(replace_self_in_ast_type(inner, concrete_type)))
        }
        AstType::MutPtr(inner) => {
            AstType::MutPtr(Box::new(replace_self_in_ast_type(inner, concrete_type)))
        }
        AstType::RawPtr(inner) => {
            AstType::RawPtr(Box::new(replace_self_in_ast_type(inner, concrete_type)))
        }
        AstType::Option(inner) => {
            AstType::Option(Box::new(replace_self_in_ast_type(inner, concrete_type)))
        }
        AstType::Result { ok_type, err_type } => AstType::Result {
            ok_type: Box::new(replace_self_in_ast_type(ok_type, concrete_type)),
            err_type: Box::new(replace_self_in_ast_type(err_type, concrete_type)),
        },
        AstType::Array(element) => {
            AstType::Array(Box::new(replace_self_in_ast_type(element, concrete_type)))
        }
        AstType::FixedArray { element_type, size } => AstType::FixedArray {
            element_type: Box::new(replace_self_in_ast_type(element_type, concrete_type)),
            size: *size,
        },
        AstType::Vec { element_type, size } => AstType::Vec {
            element_type: Box::new(replace_self_in_ast_type(element_type, concrete_type)),
            size: *size,
        },
        AstType::DynVec {
            element_types,
            allocator_type,
        } => AstType::DynVec {
            element_types: element_types
                .iter()
                .map(|t| replace_self_in_ast_type(t, concrete_type))
                .collect(),
            allocator_type: allocator_type
                .as_ref()
                .map(|t| Box::new(replace_self_in_ast_type(t, concrete_type))),
        },
        AstType::Function { args, return_type } => AstType::Function {
            args: args
                .iter()
                .map(|t| replace_self_in_ast_type(t, concrete_type))
                .collect(),
            return_type: Box::new(replace_self_in_ast_type(return_type, concrete_type)),
        },
        AstType::FunctionPointer {
            param_types,
            return_type,
        } => AstType::FunctionPointer {
            param_types: param_types
                .iter()
                .map(|t| replace_self_in_ast_type(t, concrete_type))
                .collect(),
            return_type: Box::new(replace_self_in_ast_type(return_type, concrete_type)),
        },
        // For other types, return as-is
        _ => ast_type.clone(),
    }
}

/// Transform Self types in statements
fn transform_statements_self_types(
    stmts: &[Statement],
    concrete_type: &str,
) -> Result<Vec<Statement>> {
    let mut transformed = Vec::new();

    for stmt in stmts {
        transformed.push(transform_statement_self_types(stmt, concrete_type)?);
    }

    Ok(transformed)
}

/// Transform Self types in a single statement
#[allow(dead_code)]
fn transform_statement_self_types(stmt: &Statement, concrete_type: &str) -> Result<Statement> {
    match stmt {
        Statement::VariableDeclaration {
            name,
            type_,
            declaration_type,
            initializer,
            is_mutable,
        } => Ok(Statement::VariableDeclaration {
            name: name.clone(),
            type_: type_
                .as_ref()
                .map(|t| replace_self_in_ast_type(t, concrete_type)),
            declaration_type: declaration_type.clone(),
            initializer: initializer
                .as_ref()
                .map(|e| transform_expression_self_types(e, concrete_type))
                .transpose()?,
            is_mutable: *is_mutable,
        }),
        Statement::Expression(expr) => Ok(Statement::Expression(transform_expression_self_types(
            expr,
            concrete_type,
        )?)),
        Statement::Return(expr) => Ok(Statement::Return(transform_expression_self_types(
            expr,
            concrete_type,
        )?)),
        Statement::Loop { kind, label, body } => {
            Ok(Statement::Loop {
                kind: kind.clone(), // LoopKind doesn't contain types
                label: label.clone(),
                body: transform_statements_self_types(body, concrete_type)?,
            })
        }
        // For other statements, clone as-is for now
        _ => Ok(stmt.clone()),
    }
}

/// Transform Self types in expressions
#[allow(dead_code)]
fn transform_expression_self_types(expr: &Expression, _concrete_type: &str) -> Result<Expression> {
    // For now, most expressions don't contain type annotations that need transformation
    // The main transformation happens in variable declarations and function signatures
    // This is a placeholder for future expansion if needed
    Ok(expr.clone())
}
