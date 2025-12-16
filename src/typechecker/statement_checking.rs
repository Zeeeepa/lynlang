//! Statement type checking

use crate::ast::{AstType, LoopKind, Statement};
use crate::error::{CompileError, Result};
use crate::typechecker::TypeChecker;

/// Type check a statement
pub fn check_statement(checker: &mut TypeChecker, statement: &Statement) -> Result<()> {
        // Note: Import validation is handled in check_declaration for ComptimeBlocks

        match statement {
            Statement::VariableDeclaration {
                name,
                type_,
                initializer,
                is_mutable,
                span,
                ..
            } => {
                // Check if this is an assignment to a forward-declared variable
                // This happens when we have: x: i32 (forward decl) then x = 10 (initialization)
                if let Some(init_expr) = initializer {
                    // Check if variable already exists (forward declaration case)
                    if checker.variable_exists(name) {
                        // This might be initialization of a forward-declared variable
                        let var_info = checker.get_variable_info(name)?;

                        // Allow initialization of forward-declared variables with = operator
                        // This works for both immutable (x: i32 then x = 10) and mutable (w:: i32 then w = 20)
                        if !var_info.is_initialized {
                            // This is initialization of a forward-declared variable
                            // The = operator can be used to initialize both immutable and mutable forward declarations
                            let inferred_type = checker.infer_expression_type(init_expr)?;
                            if !checker.types_compatible(&var_info.type_, &inferred_type) {
                                return Err(CompileError::TypeError(
                                    format!(
                                        "Type mismatch: variable '{}' declared as {:?} but initialized with {:?}",
                                        name, var_info.type_, inferred_type
                                    ),
                                    span.clone()
                                ));
                            }
                            checker.mark_variable_initialized(name)?;
                            return Ok(());
                        } else if var_info.is_initialized && var_info.is_mutable {
                            // This is a reassignment to an existing mutable variable
                            // (e.g., w = 25 after w:: i32 and w = 20)
                            let inferred_type = checker.infer_expression_type(init_expr)?;
                            if !checker.types_compatible(&var_info.type_, &inferred_type) {
                                return Err(CompileError::TypeError(
                                    format!(
                                        "Type mismatch: cannot assign {:?} to variable '{}' of type {:?}",
                                        inferred_type, name, var_info.type_
                                    ),
                                    span.clone()
                                ));
                            }
                            // Reassignment is allowed for mutable variables
                            return Ok(());
                        } else {
                            // Immutable variable already initialized - cannot reassign
                            return Err(CompileError::TypeError(
                                format!("Cannot reassign immutable variable '{}'", name),
                                span.clone(),
                            ));
                        }
                    }

                    // New variable declaration with initializer
                    let inferred_type = checker.infer_expression_type(init_expr)?;

                    if let Some(declared_type) = type_ {
                        // Check that the initializer type matches the declared type
                        if !checker.types_compatible(declared_type, &inferred_type) {
                            return Err(CompileError::TypeError(
                                format!(
                                    "Type mismatch: variable '{}' declared as {:?} but initialized with {:?}",
                                    name, declared_type, inferred_type
                                ),
                                span.clone()
                            ));
                        }
                        checker.declare_variable_with_init(
                            name,
                            declared_type.clone(),
                            *is_mutable,
                            true,
                        )?;
                    } else {
                        // Inferred type from initializer
                        checker.declare_variable_with_init(name, inferred_type, *is_mutable, true)?;
                    }
                } else if let Some(declared_type) = type_ {
                    // Forward declaration without initializer
                    checker.declare_variable_with_init(
                        name,
                        declared_type.clone(),
                        *is_mutable,
                        false,
                    )?;
                } else {
                    return Err(CompileError::TypeError(
                        format!(
                            "Cannot infer type for variable '{}' without initializer",
                            name
                        ),
                        span.clone(),
                    ));
                }
            }
            Statement::VariableAssignment { name, value, .. } => {
                // Check if variable exists
                if !checker.variable_exists(name) {
                    // This is a new immutable declaration using = operator
                    let value_type = checker.infer_expression_type(value)?;
                    checker.declare_variable_with_init(name, value_type.clone(), false, true)?;
                // false = immutable
                } else {
                    // This is an assignment to existing variable
                    let var_info = checker.get_variable_info(name)?;

                    // Check if this is the first assignment to a forward-declared variable
                    if !var_info.is_initialized {
                        // This is the initial assignment
                        let value_type = checker.infer_expression_type(value)?;
                        if !checker.types_compatible(&var_info.type_, &value_type) {
                            return Err(CompileError::TypeError(
                                format!(
                                    "Type mismatch in initial assignment to '{}': expected {:?}, got {:?}",
                                    name, var_info.type_, value_type
                                ),
                                None
                            ));
                        }
                        // Mark as initialized
                        checker.mark_variable_initialized(name)?;
                    } else {
                        // This is a reassignment
                        if !var_info.is_mutable {
                            return Err(CompileError::TypeError(
                                format!("Cannot reassign to immutable variable '{}'", name),
                                None,
                            ));
                        }

                        let value_type = checker.infer_expression_type(value)?;

                        if !checker.types_compatible(&var_info.type_, &value_type) {
                            return Err(CompileError::TypeError(
                                format!(
                                    "Type mismatch: cannot assign {:?} to variable '{}' of type {:?}",
                                    value_type, name, var_info.type_
                                ),
                                None
                            ));
                        }
                    }
                }
            }
            Statement::Return(expr) => {
                let _return_type = checker.infer_expression_type(expr)?;
                // TODO: Check against function return type
            }
            Statement::Expression(expr) => {
                checker.infer_expression_type(expr)?;
            }
            Statement::Loop { kind, body, .. } => {
                checker.enter_scope();

                // Handle loop-specific variables
                match kind {
                    LoopKind::Infinite => {
                        // No special handling needed
                    }
                    LoopKind::Condition(expr) => {
                        // Type check the condition
                        let cond_type = checker.infer_expression_type(expr)?;
                        // Condition should be boolean or integer (truthy)
                        if !matches!(cond_type, AstType::Bool | AstType::I32 | AstType::I64) {
                            return Err(CompileError::TypeError(
                                format!(
                                    "Loop condition must be boolean or integer, got {:?}",
                                    cond_type
                                ),
                                None,
                            ));
                        }
                    }
                }

                // Check loop body with the variable in scope
                for stmt in body {
                    checker.check_statement(stmt)?;
                }
                checker.exit_scope();
            }
            Statement::ComptimeBlock(statements) => {
                checker.enter_scope();
                for stmt in statements {
                    checker.check_statement(stmt)?;
                }
                checker.exit_scope();
            }
            Statement::PointerAssignment { pointer, value } => {
                // For array indexing like arr[i] = value
                // The pointer expression should be a pointer type
                let _pointer_type = checker.infer_expression_type(pointer)?;
                let _value_type = checker.infer_expression_type(value)?;
                // TODO: Type check that value is compatible with the pointed-to type
            }
            Statement::DestructuringImport { names, source: _ } => {
                // Handle destructuring imports: { io, math } = @std
                // Register each imported module as a variable with StdModule type
                for name in names {
                    checker.declare_variable_with_init(name, AstType::StdModule, false, true)?;
                }
            }
            _ => {}
        }
        Ok(())
}

