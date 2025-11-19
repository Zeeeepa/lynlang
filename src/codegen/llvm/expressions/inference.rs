use super::super::{symbols, LLVMCompiler};
use crate::ast::{AstType, Expression};
use crate::error::CompileError;

pub fn infer_expression_type(compiler: &LLVMCompiler, expr: &Expression) -> Result<AstType, CompileError> {
        match expr {
            Expression::Integer8(_) => Ok(AstType::I8),
            Expression::Integer16(_) => Ok(AstType::I16),
            Expression::Integer32(_) => Ok(AstType::I32),
            Expression::Integer64(_) => Ok(AstType::I64),
            Expression::Unsigned8(_) => Ok(AstType::U8),
            Expression::Unsigned16(_) => Ok(AstType::U16),
            Expression::Unsigned32(_) => Ok(AstType::U32),
            Expression::Unsigned64(_) => Ok(AstType::U64),
            Expression::Float32(_) => Ok(AstType::F32),
            Expression::Float64(_) => Ok(AstType::F64),
            Expression::Boolean(_) => Ok(AstType::Bool),
            Expression::Unit => Ok(AstType::Void),
            Expression::String(_) => Ok(AstType::StaticString),  // String literals are static strings
            Expression::Identifier(name) => {
                // Look up variable type
                if let Some(var_info) = compiler.variables.get(name) {
                    Ok(var_info.ast_type.clone())
                } else {
                    // If not found, it might be a pattern binding - default to I32
                    // This is a heuristic that works for most cases
                    Ok(AstType::I32)
                }
            }
            Expression::Range {
                start: _,
                end: _,
                inclusive,
            } => {
                // Range expressions have Range type
                Ok(AstType::Range {
                    start_type: Box::new(AstType::I32),
                    end_type: Box::new(AstType::I32),
                    inclusive: *inclusive,
                })
            }
            Expression::EnumVariant { enum_name, variant, payload } => {
                // Infer the type of enum variants
                if enum_name == "Option" {
                    if variant == "Some" && payload.is_some() {
                        if let Some(ref p) = payload {
                            let inner_type = infer_expression_type(compiler, p)?;
                            Ok(AstType::Generic {
                                name: "Option".to_string(),
                                type_args: vec![inner_type],
                            })
                        } else {
                            Ok(AstType::Generic {
                                name: "Option".to_string(),
                                type_args: vec![AstType::Generic {
                                    name: "T".to_string(),
                                    type_args: vec![],
                                }],
                            })
                        }
                    } else {
                        // None variant - use context if available
                        if let Some(t) = compiler.generic_type_context.get("Option_Some_Type") {
                            Ok(AstType::Generic {
                                name: "Option".to_string(),
                                type_args: vec![t.clone()],
                            })
                        } else {
                            // Default to Option<T> with generic T, not Void
                            Ok(AstType::Generic {
                                name: "Option".to_string(),
                                type_args: vec![AstType::Generic {
                                    name: "T".to_string(),
                                    type_args: vec![],
                                }],
                            })
                        }
                    }
                } else if enum_name == "Result" {
                    // For Result, we need to track which variant we're in
                    if variant == "Ok" && payload.is_some() {
                        if let Some(ref p) = payload {
                            let inner_type = infer_expression_type(compiler, p)?;
                            
                            // NOTE: Can't store type here because infer_expression_type is immutable
                            // The actual type tracking happens during compile_enum_variant
                            
                            // Try to get error type from context
                            let err_type = compiler.generic_type_context.get("Result_Err_Type")
                                .cloned()
                                .unwrap_or(AstType::StaticString); // Default to StaticString for errors
                            Ok(AstType::Generic {
                                name: "Result".to_string(),
                                type_args: vec![inner_type, err_type],
                            })
                        } else {
                            Ok(AstType::Generic {
                                name: "Result".to_string(),
                                type_args: vec![
                                    AstType::Generic {
                                        name: "T".to_string(),
                                        type_args: vec![],
                                    },
                                    AstType::Generic {
                                        name: "E".to_string(),
                                        type_args: vec![],
                                    },
                                ],
                            })
                        }
                    } else if variant == "Err" && payload.is_some() {
                        if let Some(ref p) = payload {
                            let inner_type = infer_expression_type(compiler, p)?;
                            
                            // NOTE: Can't store type here because infer_expression_type is immutable
                            // The actual type tracking happens during compile_enum_variant
                            
                            // Try to get ok type from context
                            let ok_type = compiler.generic_type_context.get("Result_Ok_Type")
                                .cloned()
                                .unwrap_or(AstType::Void);
                            Ok(AstType::Generic {
                                name: "Result".to_string(),
                                type_args: vec![ok_type, inner_type],
                            })
                        } else {
                            Ok(AstType::Generic {
                                name: "Result".to_string(),
                                type_args: vec![
                                    AstType::Generic {
                                        name: "T".to_string(),
                                        type_args: vec![],
                                    },
                                    AstType::Generic {
                                        name: "E".to_string(),
                                        type_args: vec![],
                                    },
                                ],
                            })
                        }
                    } else {
                        // Try to get types from context
                        let ok_type = compiler.generic_type_context.get("Result_Ok_Type")
                            .cloned()
                            .unwrap_or(AstType::Void);
                        let err_type = compiler.generic_type_context.get("Result_Err_Type")
                            .cloned()
                            .unwrap_or(AstType::Void);
                        Ok(AstType::Generic {
                            name: "Result".to_string(),
                            type_args: vec![ok_type, err_type],
                        })
                    }
                } else {
                    // Custom enum - look it up in symbol table
                    if let Some(symbols::Symbol::EnumType(enum_info)) = compiler.symbols.lookup(enum_name) {
                        // Found the custom enum - create an Enum type for it
                        let variants = enum_info.variants.iter()
                            .map(|v| crate::ast::EnumVariant {
                                name: v.name.clone(),
                                payload: v.payload.clone(),
                            })
                            .collect();
                        Ok(AstType::Enum {
                            name: enum_name.to_string(),
                            variants,
                        })
                    } else {
                        // For unknown custom enums, use EnumType
                        Ok(AstType::EnumType {
                            name: enum_name.to_string(),
                        })
                    }
                }
            }
            Expression::FunctionCall { name, .. } => {
                // Check if this is a generic type constructor like HashMap<K,V>()
                if name.contains('<') && name.contains('>') {
                    // Parse the generic type from the name
                    if let Some(angle_pos) = name.find('<') {
                        let base_type = &name[..angle_pos];
                        let type_params_str = &name[angle_pos+1..name.len()-1];

                        // Parse type parameters - need to handle nested generics
                        let type_args = super::utils::parse_type_args_string(compiler, type_params_str)?;
                        
                        return Ok(AstType::Generic {
                            name: base_type.to_string(),
                            type_args,
                        });
                    }
                }
                
                // Check if we know the function's return type
                if let Some(return_type) = compiler.function_types.get(name) {
                    Ok(return_type.clone())
                } else if let Ok((_alloca, var_type)) = compiler.get_variable(name) {
                    // It's a function pointer variable - get the return type
                    match var_type {
                        AstType::Function { return_type, .. } | 
                        AstType::FunctionPointer { return_type, .. } => {
                            Ok(return_type.as_ref().clone())
                        }
                        _ => {
                            Ok(AstType::I32) // Default for non-function variables
                        }
                    }
                } else {
                    // Default to I32 for unknown functions
                    Ok(AstType::I32)
                }
            }
            Expression::Raise(object) => {
                // .raise() extracts T from Result<T,E>
                let object_type = compiler.infer_expression_type(object)?;
                // If it's Result<T,E>, return T
                if let AstType::Generic { name, type_args } = object_type {
                    if name == "Result" && type_args.len() == 2 {
                        // The raise() method returns the Ok type (T) from Result<T,E>
                        return Ok(type_args[0].clone());
                    }
                }
                // If not a Result type, return Void (will error during compilation)
                Ok(AstType::Void)
            }
            Expression::MethodCall { object, method, .. } => {
                // Special handling for compiler module methods
                if let Expression::Identifier(name) = &**object {
                    if name == "compiler" {
                        // Compiler intrinsic methods
                       match method.as_str() {
                            "raw_allocate" | "raw_reallocate" | "raw_ptr_offset" |
                            "raw_ptr_cast" | "gep" | "gep_struct" | "get_payload" |
                            "null_ptr" | "load_library" | "get_symbol" | "call_external" => {
                                // All these methods return *u8 (pointer to u8)
                                // Use Ptr(U8) which is more standard than RawPtr
                                return Ok(AstType::Ptr(Box::new(AstType::U8)));
                           }
                            "raw_deallocate" | "deallocate" | "inline_c" | "unload_library" | 
                            "set_discriminant" | "set_payload" => {
                                // These methods return void
                                return Ok(AstType::Void);
                           }
                            "discriminant" => {
                                // Returns i64 (the discriminant tag)
                                return Ok(AstType::I64);
                           }
                            _ => {}
                       }
                   }
               }
               
               // Special handling for .raise() method which extracts T from Result<T,E>
               if method == "raise" {
                    // Get the type of the object being raised
                    let object_type = compiler.infer_expression_type(object)?;
                    // If it's Result<T,E>, return T
                    if let AstType::Generic { name, type_args } = &object_type {
                        if name == "Result" && type_args.len() == 2 {
                            // The raise() method returns the Ok type (T) from Result<T,E>
                            return Ok(type_args[0].clone());
                        }
                    }
                    // If not a Result type, return Void (will error during compilation)
                    Ok(AstType::Void)
                } else if method == "new" || method == "init" {
                    // Collection constructors and init methods return the type
                    if let Expression::Identifier(name) = &**object {
                        // Check if this is a generic type constructor (e.g., HashMap<K,V>.new())
                        if name.contains('<') {
                            // Parse the generic type from the name
                            if let Some(angle_pos) = name.find('<') {
                                let base_type = &name[..angle_pos];
                                
                                // Parse type arguments - handle nested generics
                                let args_str = &name[angle_pos + 1..name.len() - 1]; // Remove < and >
                                let type_args = super::utils::parse_type_args_string(compiler, args_str)?;
                                
                                return Ok(AstType::Generic {
                                    name: base_type.to_string(),
                                    type_args,
                                });
                            }
                        }
                        
                        // Non-generic collection constructors
                        match name.as_str() {
                            "Array" => Ok(AstType::Generic { 
                                name: "Array".to_string(), 
                                type_args: vec![AstType::I32]  // Default element type
                            }),
                            "HashMap" => Ok(AstType::Generic { 
                                name: "HashMap".to_string(), 
                                type_args: vec![crate::ast::resolve_string_struct_type(), AstType::I32]  // Default K,V types
                            }),
                            "HashSet" => Ok(AstType::Generic { 
                                name: "HashSet".to_string(), 
                                type_args: vec![AstType::I32]  // Default element type
                            }),
                            "DynVec" => Ok(AstType::Generic { 
                                name: "DynVec".to_string(), 
                                type_args: vec![AstType::I32]  // Default element type
                            }),
                            "Vec" => Ok(AstType::Generic { 
                                name: "Vec".to_string(), 
                                type_args: vec![AstType::I32]  // Default element type
                            }),
                            "GPA" => Ok(AstType::Struct { 
                                name: "GPA".to_string(), 
                                fields: vec![] 
                            }),
                            "AsyncPool" => Ok(AstType::Struct { 
                                name: "AsyncPool".to_string(), 
                                fields: vec![] 
                            }),
                            _ => Ok(AstType::Void)
                        }
                    } else {
                        Ok(AstType::Void)
                    }
                } else {
                    // For other common methods, try to infer return type
                    match method.as_str() {
                        // Numeric methods - return same type as input
                        "abs" | "min" | "max" => {
                            // These methods return the same type as the object
                            compiler.infer_expression_type(object)
                        }
                        "len" | "size" | "length" => Ok(AstType::I64),
                        "is_empty" => Ok(AstType::Bool),
                        "to_i32" => Ok(AstType::Generic { 
                            name: "Option".to_string(), 
                            type_args: vec![AstType::I32]
                        }),
                        "to_i64" => Ok(AstType::Generic { 
                            name: "Option".to_string(), 
                            type_args: vec![AstType::I64]
                        }),
                        "to_f64" => Ok(AstType::Generic { 
                            name: "Option".to_string(), 
                            type_args: vec![AstType::F64]
                        }),
                        "contains" | "starts_with" | "ends_with" => Ok(AstType::Bool),
                        "index_of" => Ok(AstType::I64),
                        "substr" | "trim" | "to_upper" | "to_lower" => Ok(AstType::StaticString),
                        "split" => Ok(AstType::Generic { 
                            name: "Array".to_string(), 
                            type_args: vec![AstType::StaticString]
                        }),
                        "char_at" => Ok(AstType::I32),
                        "get" | "remove" | "insert" | "pop" => {
                            // Try to infer from object type
                            if let Ok(object_type) = compiler.infer_expression_type(object) {
                                if let AstType::Generic { name, type_args } = object_type {
                                    if name == "HashMap" && type_args.len() >= 2 {
                                        // HashMap methods return Option<V>
                                        return Ok(AstType::Generic { 
                                            name: "Option".to_string(), 
                                            type_args: vec![type_args[1].clone()]
                                        });
                                    } else if name == "HashSet" && !type_args.is_empty() {
                                        // HashSet.remove returns bool
                                        if method == "remove" {
                                            return Ok(AstType::Bool);
                                        }
                                    } else if name == "Array" && !type_args.is_empty() {
                                        // Array.get returns element type directly (not wrapped in Option)
                                        if method == "get" {
                                            return Ok(type_args[0].clone());
                                        } else if method == "pop" {
                                            // Array.pop returns Option<element_type>
                                            return Ok(AstType::Generic { 
                                                name: "Option".to_string(), 
                                                type_args: vec![type_args[0].clone()]
                                            });
                                        }
                                    } else if (name == "Vec" || name == "DynVec") && !type_args.is_empty() {
                                        // Vec/DynVec get and pop return Option<element_type>
                                        return Ok(AstType::Generic { 
                                            name: "Option".to_string(), 
                                            type_args: vec![type_args[0].clone()]
                                        });
                                    }
                                }
                            }
                            
                            // Also check for DynVec struct type (not generic)
                            let object_type = compiler.infer_expression_type(object)?;
                            if let AstType::DynVec { element_types, .. } = &object_type {
                                if !element_types.is_empty() && (method == "get" || method == "pop") {
                                    return Ok(AstType::Generic { 
                                        name: "Option".to_string(), 
                                        type_args: vec![element_types[0].clone()]
                                    });
                                }
                            }
                            
                            Ok(AstType::Void)
                        }
                        "push" | "set" | "clear" => {
                            // Array/Vec/DynVec mutation methods return void
                            Ok(AstType::Void)
                        }
                        "add" => {
                            // HashSet.add returns bool (whether the element was actually added)
                            Ok(AstType::Bool)
                        }
                        "union" | "intersection" | "difference" | "symmetric_difference" => {
                            // HashSet operations return a new HashSet
                            if let Ok(object_type) = compiler.infer_expression_type(object) {
                                if let AstType::Generic { name, type_args } = object_type {
                                    if name == "HashSet" {
                                        return Ok(AstType::Generic { 
                                            name: "HashSet".to_string(), 
                                            type_args: type_args
                                        });
                                    }
                                }
                            }
                            Ok(AstType::Void)
                        }
                        "is_subset" | "is_superset" | "is_disjoint" => {
                            // HashSet comparison methods return bool
                            Ok(AstType::Bool)
                        }
                        _ => {
                            // Try to look up the function for UFC
                            if let Some(func_return_type) = compiler.function_types.get(method) {
                                Ok(func_return_type.clone())
                            } else {
                                Ok(AstType::Void)
                            }
                        }
                    }
                }
            }
            Expression::PatternMatch { arms, .. } => {
                // Pattern match takes the type of its first arm's body
                if let Some(first_arm) = arms.first() {
                    compiler.infer_expression_type(&first_arm.body)
                } else {
                    Ok(AstType::Void)
                }
            }
            Expression::QuestionMatch { arms, .. } => {
                // Question match should return the common type of all arms
                // We iterate through arms to find the first non-void type
                for arm in arms {
                    let arm_type = compiler.infer_expression_type(&arm.body)?;
                    // Return the first non-void type we find
                    if !matches!(arm_type, AstType::Void) {
                        // Handle special cases for pattern binding
                        if let Expression::Block(stmts) = &arm.body {
                            if stmts.len() == 1 {
                                if let crate::ast::Statement::Expression(Expression::Identifier(_)) = &stmts[0] {
                                    // This is likely a pattern binding variable, assume I32 for now
                                    return Ok(AstType::I32);
                                }
                            }
                        }
                        // For arrow syntax with binary ops, check for pattern bindings
                        if let Expression::BinaryOp { left, op: _, right: _ } = &arm.body {
                            if matches!(**left, Expression::Identifier(_)) {
                                // Assume the pattern binding will be an integer type
                                return Ok(AstType::I32);
                            }
                        }
                        return Ok(arm_type);
                    }
                }
                // If all arms are void, return void
                Ok(AstType::Void)
            }
            Expression::Conditional { arms, .. } => {
                // Conditional takes the type of its first arm's body
                if let Some(first_arm) = arms.first() {
                    compiler.infer_expression_type(&first_arm.body)
                } else {
                    Ok(AstType::Void)
                }
            }
            Expression::BinaryOp { op, left, right } => {
                // Binary operations return different types based on the operator
                use crate::ast::BinaryOperator;
                match op {
                    BinaryOperator::GreaterThan | BinaryOperator::LessThan | 
                    BinaryOperator::GreaterThanEquals | BinaryOperator::LessThanEquals |
                    BinaryOperator::Equals | BinaryOperator::NotEquals | 
                    BinaryOperator::And | BinaryOperator::Or => Ok(AstType::Bool),
                    BinaryOperator::Add | BinaryOperator::Subtract | 
                    BinaryOperator::Multiply | BinaryOperator::Divide | 
                    BinaryOperator::Modulo => {
                        // Infer type based on operands
                        let left_type = compiler.infer_expression_type(left)?;
                        let right_type = compiler.infer_expression_type(right)?;
                        
                        // If either operand is a float, the result is a float
                        if matches!(left_type, AstType::F32 | AstType::F64) || 
                           matches!(right_type, AstType::F32 | AstType::F64) {
                            Ok(AstType::F64)
                        } else {
                            // Default to I32 for integer operations
                            Ok(AstType::I32)
                        }
                    }
                    _ => Ok(AstType::Void),
                }
            }
            Expression::Block(stmts) => {
                // Block takes the type of its last expression
                if let Some(last_stmt) = stmts.last() {
                    match last_stmt {
                        crate::ast::Statement::Expression(expr) => {
                            // If the last statement is an expression, the block evaluates to its type
                            compiler.infer_expression_type(expr)
                        }
                        crate::ast::Statement::Return(_) => {
                            // If the block ends with a return, it doesn't evaluate to a value
                            // Instead, it's void since control flow leaves the block
                            Ok(AstType::Void)
                        }
                        _ => Ok(AstType::Void)
                    }
                } else {
                    Ok(AstType::Void)
                }
            }
            Expression::Some(value) => {
                // Option::Some(T) -> Option<T>
                let inner_type = compiler.infer_expression_type(value)?;
                Ok(AstType::Generic {
                    name: "Option".to_string(),
                    type_args: vec![inner_type],
                })
            }
            Expression::None => {
                // Option::None -> Option<T> (with generic T)
                // Try to get type from context first
                if let Some(t) = compiler.generic_type_context.get("Option_Some_Type") {
                    // eprintln!("[DEBUG] Expression::None has context type: {:?}", t);
                    Ok(AstType::Generic {
                        name: "Option".to_string(),
                        type_args: vec![t.clone()],
                    })
                } else {
                    // Default to Option<Void> instead of unresolved generic T
                    // This avoids the "Unresolved generic type 'T'" error
                    // eprintln!("[DEBUG] Expression::None defaulting to Option<Void>");
                    Ok(AstType::Generic {
                        name: "Option".to_string(),
                        type_args: vec![AstType::Void],
                    })
                }
            }
            Expression::StructLiteral { name, .. } => {
                // Get the struct type fields from registered types
                if let Some(struct_info) = compiler.struct_types.get(name) {
                    let mut fields = Vec::new();
                    for (field_name, (_index, field_type)) in &struct_info.fields {
                        fields.push((field_name.clone(), field_type.clone()));
                    }
                    Ok(AstType::Struct {
                        name: name.clone(),
                        fields,
                    })
                } else {
                    // Unknown struct type
                    Ok(AstType::Struct {
                        name: name.clone(),
                        fields: vec![],
                    })
                }
            }
            Expression::VecConstructor { element_type, size, .. } => {
                // Vec<T, N> constructor returns Vec type
                Ok(AstType::Vec {
                    element_type: Box::new(element_type.clone()),
                    size: *size,
                })
            }
            Expression::DynVecConstructor { element_types, .. } => {
                // DynVec<T> constructor returns DynVec type
                Ok(AstType::DynVec {
                    element_types: element_types.clone(),
                    allocator_type: None,
                })
            }
            Expression::ArrayConstructor { element_type } => {
                // Array<T> constructor returns Generic Array type
                Ok(AstType::Generic {
                    name: "Array".to_string(),
                    type_args: vec![element_type.clone()],
                })
            }
            Expression::StringInterpolation { .. } => {
                // String interpolation always returns a string
                Ok(AstType::StaticString)
            }
            Expression::TypeCast { target_type, .. } => {
                // Type cast returns the target type
                Ok(target_type.clone())
            }
            Expression::Closure { return_type, body, .. } => {
                // Closure returns its declared return type, or infer from body
                if let Some(ret_type) = return_type {
                    Ok(ret_type.clone())
                } else {
                    // Try to infer from the closure body
                    infer_closure_return_type(compiler, body)
                }
            }
            Expression::Loop { .. } | Expression::CollectionLoop { .. } => {
                // Loops typically return void, unless they have explicit break with value
                // For now, assume void return
                Ok(AstType::Void)
            }
            Expression::Return(expr) => {
                // Return expression returns the type of its inner expression
                compiler.infer_expression_type(expr)
            }
            Expression::Break { value, .. } => {
                // Break can return a value or void
                if let Some(val) = value {
                    compiler.infer_expression_type(val)
                } else {
                    Ok(AstType::Void)
                }
            }
            Expression::Continue { .. } => {
                // Continue always returns void
                Ok(AstType::Void)
            }
            Expression::MemberAccess { object, member } => {
                // Infer the type of the object first
                let object_type = compiler.infer_expression_type(object)?;
                
                // Handle struct field access
                match &object_type {
                    AstType::Struct { name, .. } => {
                        // Look up the struct type info
                        if let Some(struct_info) = compiler.struct_types.get(name) {
                            // Find the field type
                            if let Some((_index, field_type)) = struct_info.fields.get(member) {
                                Ok(field_type.clone())
                            } else {
                                Err(CompileError::TypeError(
                                    format!("Struct '{}' has no field '{}'", name, member),
                                    None,
                                ))
                            }
                        } else {
                            Err(CompileError::TypeError(
                                format!("Unknown struct type: {}", name),
                                None,
                            ))
                        }
                    }
                    // Handle pointer to struct types
                    AstType::Ptr(inner) => {
                        // Recursively infer member type from inner type
                        if let AstType::Struct { name, .. } = &**inner {
                            if let Some(struct_info) = compiler.struct_types.get(name) {
                                if let Some((_index, field_type)) = struct_info.fields.get(member) {
                                    Ok(field_type.clone())
                                } else {
                                    Err(CompileError::TypeError(
                                        format!("Struct '{}' has no field '{}'", name, member),
                                        None,
                                    ))
                                }
                            } else {
                                Err(CompileError::TypeError(
                                    format!("Unknown struct type: {}", name),
                                    None,
                                ))
                            }
                        } else {
                            Ok(AstType::Void)
                        }
                    }
                    // Handle Generic types that might be structs
                    AstType::Generic { name, .. } => {
                        // Check if this generic is actually a registered struct type
                        if let Some(struct_info) = compiler.struct_types.get(name) {
                            if let Some((_index, field_type)) = struct_info.fields.get(member) {
                                Ok(field_type.clone())
                            } else {
                                Err(CompileError::TypeError(
                                    format!("Struct '{}' has no field '{}'", name, member),
                                    None,
                                ))
                            }
                        } else {
                            // Not a struct, might be a module reference (@std.something)
                            Ok(AstType::Void)
                        }
                    }
                    _ => {
                        // For other types, return void (will error during compilation if needed)
                        Ok(AstType::Void)
                    }
                }
            }
            _ => Ok(AstType::Void),
        }
    }

pub fn infer_closure_return_type(compiler: &LLVMCompiler, body: &Expression) -> Result<AstType, CompileError> {
        match body {
            Expression::QuestionMatch { arms, .. } => {
                // For question match expressions, check the return types of all arms
                // and return the first non-void type found (they should all be the same)
                for arm in arms {
                    if let Expression::Block(statements) = &arm.body {
                        for stmt in statements {
                            if let crate::ast::Statement::Return(ret_expr) = stmt {
                                let ret_type = infer_expression_type(compiler, ret_expr)?;
                                if ret_type != AstType::Void {
                                    return Ok(ret_type);
                                }
                            }
                        }
                    }
                }
                Ok(AstType::Void)
            }
            Expression::Block(statements) => {
                // Look for the last expression or a return statement
                for stmt in statements {
                    if let crate::ast::Statement::Return(ret_expr) = stmt {
                        // Recursively infer the return type, especially for closures that just return a Result
                        return infer_expression_type(compiler, ret_expr);
                    }
                }
                // Check if the last statement is an expression
                if let Some(last_stmt) = statements.last() {
                    match last_stmt {
                        crate::ast::Statement::Expression(expr) => {
                            return infer_expression_type(compiler, expr);
                        }
                        crate::ast::Statement::Return(expr) => {
                            return infer_expression_type(compiler, expr);
                        }
                        _ => {}
                    }
                }
                Ok(AstType::Void)
            }
            Expression::FunctionCall { name, args } => {
                // Check if this is Result.Ok or Result.Err
                if name == "Result.Ok" {
                    // Infer the T type from the payload
                    let t_type = if !args.is_empty() {
                        infer_expression_type(compiler, &args[0])?
                    } else {
                        AstType::Void
                    };
                    // For Result.Ok, E type defaults to string (common error type)
                    Ok(AstType::Generic {
                        name: "Result".to_string(),
                        type_args: vec![t_type, AstType::StaticString],
                    })
                } else if name == "Result.Err" {
                    // Infer the E type from the payload
                    let e_type = if !args.is_empty() {
                        infer_expression_type(compiler, &args[0])?
                    } else {
                        crate::ast::resolve_string_struct_type()
                    };
                    // For Result.Err, T type is unknown but we can use i32 as placeholder
                    Ok(AstType::Generic {
                        name: "Result".to_string(),
                        type_args: vec![AstType::I32, e_type],
                    })
                } else if name == "Option.Some" {
                    // Infer the T type from the payload
                    let t_type = if !args.is_empty() {
                        infer_expression_type(compiler, &args[0])?
                    } else {
                        AstType::Void
                    };
                    Ok(AstType::Generic {
                        name: "Option".to_string(),
                        type_args: vec![t_type],
                    })
                } else if name == "Option.None" {
                    // None variant - type needs to be inferred from context
                    // Use Generic T instead of Void as placeholder
                    Ok(AstType::Generic {
                        name: "Option".to_string(),
                        type_args: vec![AstType::Generic {
                            name: "T".to_string(),
                            type_args: vec![],
                        }],
                    })
                } else {
                    // Check if we know the function's return type
                    if let Some(return_type) = compiler.function_types.get(name) {
                        Ok(return_type.clone())
                    } else {
                        Ok(AstType::I32) // Default
                    }
                }
            }
            _ => infer_expression_type(compiler, body),
        }
    }

