pub mod behaviors;
pub mod inference;
pub mod scope;
pub mod stdlib;
pub mod self_resolution;
pub mod types;
pub mod validation;
pub mod type_resolution;
pub mod declaration_checking;
pub mod function_checking;
pub mod statement_checking;

use crate::ast::{AstType, Declaration, Expression, Function, Program, Statement};
use crate::error::{CompileError, Result};
use crate::stdlib::StdNamespace;
use behaviors::{BehaviorResolver, MethodInfo};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct VariableInfo {
    pub type_: AstType,
    pub is_mutable: bool,
    pub is_initialized: bool,
}

#[allow(dead_code)]
pub struct TypeChecker {
    // Symbol table for tracking variable types and mutability
    scopes: Vec<HashMap<String, VariableInfo>>,
    // Function signatures
    functions: HashMap<String, FunctionSignature>,
    // Struct definitions
    structs: HashMap<String, StructInfo>,
    // Enum definitions
    enums: HashMap<String, EnumInfo>,
    // Behavior/trait resolver
    behavior_resolver: BehaviorResolver,
    // Standard library namespace
    std_namespace: StdNamespace,
    // Module imports (alias -> module_path)
    module_imports: HashMap<String, String>,
    // Current trait implementation type (for resolving Self)
    current_impl_type: Option<String>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct FunctionSignature {
    pub params: Vec<(String, AstType)>,
    pub return_type: AstType,
    pub is_external: bool,
}

#[derive(Clone, Debug)]
pub struct StructInfo {
    pub fields: Vec<(String, AstType)>,
}

#[derive(Clone, Debug)]
pub struct EnumInfo {
    pub variants: Vec<(String, Option<AstType>)>,
}

impl TypeChecker {
    /// Resolve String type from stdlib - returns the struct type definition
    /// String is defined in stdlib/string.zen as:
    /// struct String {
    ///     data: Ptr<u8>
    ///     len: u64
    ///     capacity: u64
    ///     allocator: Allocator
    /// }
    // Use crate::ast::resolve_string_struct_type() instead

    /// Resolve Generic types to Struct types if they're known structs
    /// This handles the case where the parser represents struct types as Generic
    /// Recursively resolves nested Generic types in fields
    /// Uses a visited set to prevent infinite recursion on circular references
    fn resolve_generic_to_struct(&self, ast_type: &AstType) -> AstType {
        type_resolution::resolve_generic_to_struct(self, ast_type)
    }

    /// Get the inferred function signatures
    pub fn get_function_signatures(&self) -> &HashMap<String, FunctionSignature> {
        &self.functions
    }
    
    /// Parse type arguments from a generic type string like "HashMap<i32, i32>"
    fn parse_generic_type_string(type_str: &str) -> (String, Vec<AstType>) {
        if let Some(angle_pos) = type_str.find('<') {
            let base_type = type_str[..angle_pos].to_string();
            let args_str = &type_str[angle_pos + 1..type_str.len() - 1]; // Remove < and >
            
            // Simple parsing - split by comma and trim
            let type_args: Vec<AstType> = args_str
                .split(',')
                .map(|s| {
                    let trimmed = s.trim();
                    match trimmed {
                        "i32" => AstType::I32,
                        "i64" => AstType::I64,
                        "f32" => AstType::F32,
                        "f64" => AstType::F64,
                        "bool" => AstType::Bool,
                        "string" => AstType::StaticString,  // lowercase string maps to StaticString
                        "StaticString" => AstType::StaticString,  // explicit static string type
                        "String" => crate::ast::resolve_string_struct_type(),  // String is a struct from stdlib/string.zen
                        _ => {
                            // Check if it's another generic type
                            if trimmed.contains('<') {
                                let (inner_base, inner_args) = Self::parse_generic_type_string(trimmed);
                                AstType::Generic {
                                    name: inner_base,
                                    type_args: inner_args,
                                }
                            } else {
                                // Unknown type, treat as identifier
                                AstType::Generic {
                                    name: trimmed.to_string(),
                                    type_args: vec![],
                                }
                            }
                        }
                    }
                })
                .collect();
            
            (base_type, type_args)
        } else {
            (type_str.to_string(), vec![])
        }
    }
    
    pub fn new() -> Self {
        let mut enums = HashMap::new();

        // Register Option<T> and Result<T, E> as fallback until stdlib preloading is implemented
        // TODO: These should be loaded from stdlib/core/option.zen and stdlib/core/result.zen
        // For now, keep as fallback to ensure Option/Result work even without explicit imports
        // When stdlib/core/option.zen is loaded, it will override this registration
        enums.insert(
            "Option".to_string(),
            EnumInfo {
                variants: vec![
                    (
                        "Some".to_string(),
                        Some(AstType::Generic {
                            name: "T".to_string(),
                            type_args: vec![],
                        }),
                    ),
                    ("None".to_string(), None),
                ],
            },
        );

        enums.insert(
            "Result".to_string(),
            EnumInfo {
                variants: vec![
                    (
                        "Ok".to_string(),
                        Some(AstType::Generic {
                            name: "T".to_string(),
                            type_args: vec![],
                        }),
                    ),
                    (
                        "Err".to_string(),
                        Some(AstType::Generic {
                            name: "E".to_string(),
                            type_args: vec![],
                        }),
                    ),
                ],
            },
        );

        let mut functions = HashMap::new();

        // Register builtin math functions
        functions.insert(
            "min".to_string(),
            FunctionSignature {
                params: vec![
                    ("a".to_string(), AstType::I32),
                    ("b".to_string(), AstType::I32),
                ],
                return_type: AstType::I32,
                is_external: false,
            },
        );
        functions.insert(
            "max".to_string(),
            FunctionSignature {
                params: vec![
                    ("a".to_string(), AstType::I32),
                    ("b".to_string(), AstType::I32),
                ],
                return_type: AstType::I32,
                is_external: false,
            },
        );
        functions.insert(
            "abs".to_string(),
            FunctionSignature {
                params: vec![("x".to_string(), AstType::I32)],
                return_type: AstType::I32,
                is_external: false,
            },
        );

        Self {
            scopes: vec![HashMap::new()],
            functions,
            structs: HashMap::new(),
            enums,
            behavior_resolver: BehaviorResolver::new(),
            std_namespace: StdNamespace::new(),
            module_imports: HashMap::new(),
            current_impl_type: None,
        }
    }

    pub fn check_program(&mut self, program: &Program) -> Result<()> {
        // First pass: collect all type definitions and function signatures
        for declaration in &program.declarations {
            self.collect_declaration_types(declaration)?;
        }
        
        // Second pass: resolve Generic types to Struct types in struct fields
        // This handles forward references - all structs are now registered
        // We do multiple passes until no more changes occur (to handle nested dependencies)
        let mut changed = true;
        let mut iterations = 0;
        while changed && iterations < 10 {
            changed = false;
            iterations += 1;
            
            let struct_names: Vec<String> = self.structs.keys().cloned().collect();
            for struct_name in struct_names {
                let resolved_fields: Vec<(String, AstType)> = {
                    // Get the current fields (immutable borrow)
                    let struct_info = self.structs.get(&struct_name).unwrap();
                    struct_info
                        .fields
                        .iter()
                        .map(|(name, field_type)| {
                            let resolved = self.resolve_generic_to_struct(field_type);
                            if &resolved != field_type {
                                changed = true;
                            }
                            (name.clone(), resolved)
                        })
                        .collect()
                };
                // Now update the struct info (mutable borrow)
                if let Some(struct_info) = self.structs.get_mut(&struct_name) {
                    struct_info.fields = resolved_fields;
                }
            }
        }

        // Third pass: infer return types for functions with Void return type
        for declaration in &program.declarations {
            if let Declaration::Function(func) = declaration {
                if func.return_type == AstType::Void && !func.body.is_empty() {
                    // Try to infer the actual return type from the body
                    match self.infer_function_return_type(func) {
                        Ok(inferred_type) => {
                            // Update the function signature with the inferred return type
                            if let Some(sig) = self.functions.get_mut(&func.name) {
                                sig.return_type = inferred_type;
                            }
                        },
                        Err(_) => {
                            // Keep it as Void if inference fails
                        }
                    }
                }
            }
        }

        // Fourth pass: type check function bodies
        for declaration in &program.declarations {
            self.check_declaration(declaration)?;
        }

        Ok(())
    }

    fn collect_declaration_types(&mut self, declaration: &Declaration) -> Result<()> {
        declaration_checking::collect_declaration_types(self, declaration)
    }

    fn check_declaration(&mut self, declaration: &Declaration) -> Result<()> {
        declaration_checking::check_declaration(self, declaration)
    }

    fn check_function(&mut self, function: &Function) -> Result<()> {
        function_checking::check_function(self, function)
    }

    fn check_statement(&mut self, statement: &Statement) -> Result<()> {
        statement_checking::check_statement(self, statement)
    }

    /// Infer the type of an expression using the current type checker context
    /// Public method for LSP and other tools that need type inference
    pub fn infer_expression_type(&mut self, expr: &Expression) -> Result<AstType> {
        // eprintln!("DEBUG TypeChecker: infer_expression_type called for expr type: {}",
        //     match expr {
        //         Expression::Integer8(_) => "Integer8",
        //         Expression::Integer16(_) => "Integer16",
        //         Expression::Integer32(_) => "Integer32",
        //         Expression::Integer64(_) => "Integer64",
        //         Expression::Identifier(_) => "Identifier",
        //         Expression::Conditional { .. } => "Conditional",
        //         Expression::PatternMatch { .. } => "PatternMatch",
        //         Expression::QuestionMatch { .. } => "QuestionMatch",
        //         Expression::Some(_) => "Some",
        //         Expression::None => "None",
        //         Expression::String(_) => "String",
        //         Expression::Boolean(_) => "Boolean",
        //         Expression::Unit => "Unit",
        //         _ => "Other"
        //     }
        // );
        match expr {
            Expression::Integer32(_) => Ok(AstType::I32),
            Expression::Integer64(_) => Ok(AstType::I64),
            Expression::Float32(_) => Ok(AstType::F32),
            Expression::Float64(_) => Ok(AstType::F64),
            Expression::Boolean(_) => Ok(AstType::Bool),
            Expression::Unit => Ok(AstType::Void),
            Expression::String(_) => Ok(AstType::StaticString),  // String literals are static strings
            Expression::Identifier(name) => {
                // eprintln!("DEBUG TypeChecker: Looking up identifier '{}'", name);
                // First check if it's a function name
                if let Some(sig) = self.functions.get(name) {
                    // Return function pointer type
                    Ok(AstType::FunctionPointer {
                        param_types: sig.params.iter().map(|(_, t)| t.clone()).collect(),
                        return_type: Box::new(sig.return_type.clone()),
                    })
                } else if name == "Array" {
                    // Array is a built-in type with static methods
                    Ok(AstType::Generic {
                        name: "Array".to_string(),
                        type_args: vec![],
                    })
                } else if name.contains('<') {
                    // This looks like a generic type (e.g., HashMap<String, I32>)
                    // Extract the base type name
                    if let Some(angle_pos) = name.find('<') {
                        let base_type = &name[..angle_pos];
                        
                        // Check if it's a known generic collection type
                        match base_type {
                            "HashMap" | "HashSet" | "DynVec" | "Vec" => {
                                // Parse the type arguments from the string
                                let (_, type_args) = Self::parse_generic_type_string(name);
                                Ok(AstType::Generic {
                                    name: base_type.to_string(),
                                    type_args,
                                })
                            }
                            _ => {
                                // Unknown generic type, try as variable
                                self.get_variable_type(name)
                            }
                        }
                    } else {
                        // Shouldn't happen, but try as variable
                        self.get_variable_type(name)
                    }
                } else {
                    // Otherwise check if it's a variable
                    self.get_variable_type(name)
                }
            }
            Expression::BinaryOp { left, op, right } => {
                inference::infer_binary_op_type(self, left, op, right)
            }
            Expression::FunctionCall { name, args } => {
                // Check if this is a stdlib function call (e.g., io.print)
                if name.contains('.') {
                    let parts: Vec<&str> = name.splitn(2, '.').collect();
                    if parts.len() == 2 {
                        let module = parts[0];
                        let func = parts[1];

                        // Handle stdlib function return types
                        match (module, func) {
                            // Compiler primitives - low-level operations
                            ("compiler", "inline_c") => {
                                // Validate inline_c takes a string literal
                                if args.len() != 1 {
                                    return Err(CompileError::TypeError(
                                        format!("compiler.inline_c() expects exactly 1 argument (string literal), got {}", args.len()),
                                        None,
                                    ));
                                }
                                // Check first arg is a string
                                let arg_type = self.infer_expression_type(&args[0])?;
                                match arg_type {
                                    AstType::StaticString | AstType::StaticLiteral => {},
                                    _ => return Err(CompileError::TypeError(
                                        "compiler.inline_c() requires a string literal argument".to_string(),
                                        None,
                                    )),
                                }
                                return Ok(AstType::Void);
                            }
                            ("compiler", "raw_allocate") => {
                                if args.len() != 1 {
                                    return Err(CompileError::TypeError(
                                        format!("compiler.raw_allocate() expects 1 argument (size: usize), got {}", args.len()),
                                        None,
                                    ));
                                }
                                return Ok(AstType::Ptr(Box::new(AstType::U8)));
                            }
                            ("compiler", "raw_deallocate") => {
                                if args.len() != 2 {
                                    return Err(CompileError::TypeError(
                                        format!("compiler.raw_deallocate() expects 2 arguments (ptr, size), got {}", args.len()),
                                        None,
                                    ));
                                }
                                return Ok(AstType::Void);
                            }
                            ("compiler", "raw_reallocate") => {
                                if args.len() != 3 {
                                    return Err(CompileError::TypeError(
                                        format!("compiler.raw_reallocate() expects 3 arguments (ptr, old_size, new_size), got {}", args.len()),
                                        None,
                                    ));
                                }
                                return Ok(AstType::Ptr(Box::new(AstType::U8)));
                            }
                            ("compiler", "raw_ptr_offset") => {
                                if args.len() != 2 {
                                    return Err(CompileError::TypeError(
                                        format!("compiler.raw_ptr_offset() expects 2 arguments (ptr, offset), got {}", args.len()),
                                        None,
                                    ));
                                }
                                return Ok(AstType::RawPtr(Box::new(AstType::U8)));
                            }
                            ("compiler", "raw_ptr_cast") => {
                                if args.len() != 1 {
                                    return Err(CompileError::TypeError(
                                        format!("compiler.raw_ptr_cast() expects 1 argument (ptr), got {}", args.len()),
                                        None,
                                    ));
                                }
                                return Ok(AstType::RawPtr(Box::new(AstType::U8)));
                            }
                            ("compiler", "call_external") => {
                                if args.len() != 2 {
                                    return Err(CompileError::TypeError(
                                        format!("compiler.call_external() expects 2 arguments (func_ptr, args), got {}", args.len()),
                                        None,
                                    ));
                                }
                                return Ok(AstType::RawPtr(Box::new(AstType::U8)));
                            }
                            ("compiler", "load_library") => {
                                if args.len() != 1 {
                                    return Err(CompileError::TypeError(
                                        format!("compiler.load_library() expects 1 argument (path: string), got {}", args.len()),
                                        None,
                                    ));
                                }
                                return Ok(AstType::RawPtr(Box::new(AstType::U8)));
                            }
                            ("compiler", "get_symbol") => {
                                if args.len() != 2 {
                                    return Err(CompileError::TypeError(
                                        format!("compiler.get_symbol() expects 2 arguments (lib_handle, symbol_name), got {}", args.len()),
                                        None,
                                    ));
                                }
                                return Ok(AstType::RawPtr(Box::new(AstType::U8)));
                            }
                            ("compiler", "unload_library") => {
                                if args.len() != 1 {
                                    return Err(CompileError::TypeError(
                                        format!("compiler.unload_library() expects 1 argument (lib_handle), got {}", args.len()),
                                        None,
                                    ));
                                }
                                return Ok(AstType::Void);
                            }
                            ("compiler", "null_ptr") => {
                                if args.len() != 0 {
                                    return Err(CompileError::TypeError(
                                        format!("compiler.null_ptr() expects 0 arguments, got {}", args.len()),
                                        None,
                                    ));
                                }
                                return Ok(AstType::RawPtr(Box::new(AstType::U8)));
                            }
                            // Standard library functions
                            ("io", "print" | "println" | "print_int" | "print_float") => {
                                return Ok(AstType::Void)
                            }
                            ("io", "read_line") => return Ok(crate::ast::resolve_string_struct_type()),
                            ("math", "abs") => return Ok(AstType::I32),
                            ("math", "sqrt") => return Ok(AstType::F64),
                            ("math", "sin" | "cos" | "tan") => return Ok(AstType::F64),
                            ("math", "floor" | "ceil") => return Ok(AstType::I32),
                            ("math", "pow") => return Ok(AstType::F64),
                            ("math", "min" | "max") => return Ok(AstType::I32),
                            ("string", "len") => return Ok(AstType::I32),
                            ("string", "concat") => return Ok(crate::ast::resolve_string_struct_type()),
                            ("mem", "alloc") => return Ok(AstType::Ptr(Box::new(AstType::U8))),
                            ("mem", "free") => return Ok(AstType::Void),
                            ("fs", "read_file") => {
                                let string_type = crate::ast::resolve_string_struct_type();
                                return Ok(AstType::Generic {
                                    name: "Result".to_string(),
                                    type_args: vec![string_type.clone(), string_type],
                                })
                            }
                            ("fs", "write_file") => {
                                return Ok(AstType::Generic {
                                    name: "Result".to_string(),
                                    type_args: vec![
                                        AstType::Void,
                                        crate::ast::resolve_string_struct_type(),
                                    ],
                                })
                            }
                            ("fs", "exists") => return Ok(AstType::Bool),
                            ("fs", "remove_file") => {
                                return Ok(AstType::Generic {
                                    name: "Result".to_string(),
                                    type_args: vec![
                                        AstType::Void,
                                        crate::ast::resolve_string_struct_type(),
                                    ],
                                })
                            }
                            ("fs", "create_dir") => {
                                return Ok(AstType::Generic {
                                    name: "Result".to_string(),
                                    type_args: vec![
                                        AstType::Void,
                                        crate::ast::resolve_string_struct_type(),
                                    ],
                                })
                            }
                            _ => {}
                        }
                    }
                }

                // Check if this is a generic type constructor like HashMap<K,V>()
                if name.contains('<') && name.contains('>') {
                    // Extract the base type name
                    if let Some(angle_pos) = name.find('<') {
                        let base_type = &name[..angle_pos];
                        match base_type {
                            "HashMap" | "HashSet" | "DynVec" => {
                                // These constructors return their respective generic types
                                // Parse the type arguments to construct the proper return type
                                // For now, return a generic placeholder
                                return Ok(AstType::Generic {
                                    name: base_type.to_string(),
                                    type_args: vec![], // TODO: Parse actual type args from name
                                });
                            }
                            _ => {
                                // Continue with regular function lookup
                            }
                        }
                    }
                }

                // First check if it's a known function
                if let Some(sig) = self.functions.get(name) {
                    Ok(sig.return_type.clone())
                } else {
                    // Check if it's a variable holding a function pointer
                    match self.get_variable_type(name) {
                        Ok(AstType::FunctionPointer { return_type, .. }) => {
                            Ok(*return_type)
                        }
                        Ok(_) => Err(CompileError::TypeError(
                            format!("'{}' is not a function", name),
                            None,
                        )),
                        Err(_) => Err(CompileError::TypeError(
                            format!("Unknown function: {}", name),
                            None,
                        )),
                    }
                }
            }
            Expression::MemberAccess { object, member } => {
                // Check if accessing @std namespace
                if let Expression::Identifier(name) = &**object {
                    if StdNamespace::is_std_reference(name) {
                        // Resolve @std.module access
                        return Ok(AstType::Generic {
                            name: format!("StdModule::{}", member),
                            type_args: vec![],
                        });
                    }
                }
                let object_type = self.infer_expression_type(object)?;
                inference::infer_member_type(&object_type, member, &self.structs, &self.enums)
            }
            Expression::Comptime(inner) => self.infer_expression_type(inner),
            Expression::Range { .. } => Ok(AstType::Range {
                start_type: Box::new(AstType::I32),
                end_type: Box::new(AstType::I32),
                inclusive: false,
            }),
            Expression::StructLiteral { name, .. } => {
                // For struct literals, return the struct type
                // Check if it's a known struct
                if let Some(struct_def) = self.structs.get(name) {
                    Ok(AstType::Struct {
                        name: name.clone(),
                        fields: struct_def.fields.clone(),
                    })
                } else {
                    // It might be a generic struct that will be monomorphized
                    // For now, return a struct type with empty fields
                    Ok(AstType::Struct {
                        name: name.clone(),
                        fields: vec![],
                    })
                }
            }
            Expression::StdReference => {
                // Return a type representing @std
                Ok(AstType::Generic {
                    name: "Std".to_string(),
                    type_args: vec![],
                })
            }
            Expression::ThisReference => {
                // Return a type representing @this
                Ok(AstType::Generic {
                    name: "This".to_string(),
                    type_args: vec![],
                })
            }
            Expression::StringInterpolation { .. } => {
                // String interpolation returns dynamic String (requires allocator)
                Ok(crate::ast::resolve_string_struct_type())
            }
            Expression::Closure { params, return_type, body } => {
                // Infer closure type - create a FunctionPointer type
                let param_types: Vec<AstType> = params
                    .iter()
                    .map(|(_, opt_type)| opt_type.clone().unwrap_or(AstType::I32))
                    .collect();

                // If explicit return type provided, use it
                if let Some(rt) = return_type {
                    return Ok(AstType::FunctionPointer {
                        param_types,
                        return_type: Box::new(rt.clone()),
                    });
                }

                // Otherwise, need to infer return type from body with proper scoping
                // Temporarily add closure parameters to scope for return type inference
                self.enter_scope();
                for (_i, (param_name, opt_type)) in params.iter().enumerate() {
                    let param_type = opt_type.clone().unwrap_or(AstType::I32);
                    let _ = self.declare_variable(param_name, param_type.clone(), false);
                }

                // Infer return type from body
                let inferred_return = match body.as_ref() {
                    Expression::Block(stmts) => {
                        // First, process variable declarations to populate the scope
                        for stmt in stmts {
                            match stmt {
                                crate::ast::Statement::VariableDeclaration { .. } |
                                crate::ast::Statement::VariableAssignment { .. } => {
                                    let _ = self.check_statement(stmt);
                                }
                                _ => {}
                            }
                        }
                        
                        // Look for return statements
                        let mut ret_type = Box::new(AstType::Void);
                        for stmt in stmts {
                            if let crate::ast::Statement::Return(ret_expr) = stmt {
                                if let Ok(rt) = self.infer_expression_type(ret_expr) {
                                    ret_type = Box::new(rt);
                                    break;
                                }
                            }
                        }
                        
                        // If void from return search, check last expression
                        if matches!(*ret_type, AstType::Void) {
                            if let Some(crate::ast::Statement::Expression(last_expr)) = stmts.last() {
                                if let Ok(rt) = self.infer_expression_type(last_expr) {
                                    ret_type = Box::new(rt);
                                }
                            }
                        }
                        
                        ret_type
                    }
                    _ => {
                        // Simple expression body
                        if let Ok(rt) = self.infer_expression_type(body) {
                            Box::new(rt)
                        } else {
                            Box::new(AstType::I32)
                        }
                    }
                };

                // Pop the temporary scope
                self.exit_scope();

                Ok(AstType::FunctionPointer {
                    param_types,
                    return_type: inferred_return,
                })
            }
            Expression::ArrayIndex { array, .. } => {
                // Array indexing returns the element type
                let array_type = self.infer_expression_type(array)?;
                match array_type {
                    AstType::Ptr(elem_type) => Ok(*elem_type),
                    AstType::Array(elem_type) => Ok(*elem_type),
                    _ => Err(CompileError::TypeError(
                        format!("Cannot index type {:?}", array_type),
                        None,
                    )),
                }
            }
            Expression::AddressOf(inner) => {
                let inner_type = self.infer_expression_type(inner)?;
                Ok(AstType::Ptr(Box::new(inner_type)))
            }
            Expression::Dereference(inner) => {
                let inner_type = self.infer_expression_type(inner)?;
                match inner_type {
                    AstType::Ptr(elem_type) => Ok(*elem_type),
                    _ => Err(CompileError::TypeError(
                        format!("Cannot dereference non-pointer type {:?}", inner_type),
                        None,
                    )),
                }
            }
            Expression::PointerOffset { pointer, .. } => {
                // Pointer offset returns the same pointer type
                self.infer_expression_type(pointer)
            }
            Expression::StructField { struct_, field } => {
                // eprintln!("DEBUG: StructField access - struct: {:?}, field: {}", struct_, field);
                let struct_type = self.infer_expression_type(struct_)?;
                // eprintln!("DEBUG: StructField - struct_type: {:?}", struct_type);
                match struct_type {
                    AstType::Ptr(inner) => {
                        // Handle pointer to struct - automatically dereference
                        match *inner {
                            AstType::Struct { name, .. } => inference::infer_member_type(
                                &AstType::Struct {
                                    name,
                                    fields: vec![],
                                },
                                field,
                                &self.structs,
                                &self.enums,
                            ),
                            AstType::Generic { ref name, .. } => {
                                // Handle pointer to generic struct
                                inference::infer_member_type(
                                    &AstType::Generic {
                                        name: name.clone(),
                                        type_args: vec![],
                                    },
                                    field,
                                    &self.structs,
                                    &self.enums,
                                )
                            }
                            _ => Err(CompileError::TypeError(
                                format!(
                                    "Cannot access field '{}' on non-struct pointer type",
                                    field
                                ),
                                None,
                            )),
                        }
                    }
                    AstType::Struct { .. } | AstType::Generic { .. } => {
                        inference::infer_member_type(
                            &struct_type,
                            field,
                            &self.structs,
                            &self.enums,
                        )
                    }
                    _ => Err(CompileError::TypeError(
                        format!("Cannot access field '{}' on type {:?}", field, struct_type),
                        None,
                    )),
                }
            }
            Expression::Integer8(_) => Ok(AstType::I8),
            Expression::Integer16(_) => Ok(AstType::I16),
            Expression::Unsigned8(_) => Ok(AstType::U8),
            Expression::Unsigned16(_) => Ok(AstType::U16),
            Expression::Unsigned32(_) => Ok(AstType::U32),
            Expression::Unsigned64(_) => Ok(AstType::U64),
            Expression::ArrayLiteral(elements) => {
                // Infer type from first element
                if elements.is_empty() {
                    Ok(AstType::Array(Box::new(AstType::Void)))
                } else {
                    let elem_type = self.infer_expression_type(&elements[0])?;
                    Ok(AstType::Array(Box::new(elem_type)))
                }
            }
            Expression::TypeCast { target_type, .. } => Ok(target_type.clone()),
            Expression::QuestionMatch { scrutinee, arms } => {
                // QuestionMatch expression type is determined by the arms
                // All arms should have the same type

                // Infer the type of the scrutinee to properly type pattern bindings
                let scrutinee_type = self.infer_expression_type(scrutinee)?;

                if arms.is_empty() {
                    Ok(AstType::Void)
                } else {
                    let mut result_type = AstType::Void;

                    // Process each arm with its own pattern bindings
                    for (i, arm) in arms.iter().enumerate() {
                        // Enter a new scope for the pattern bindings
                        self.enter_scope();

                        // Extract pattern bindings and add them to the scope
                        // Pass the scrutinee type for proper typing
                        self.add_pattern_bindings_to_scope_with_type(
                            &arm.pattern,
                            &scrutinee_type,
                        )?;

                        // Special handling for blocks with early returns
                        // If the arm body is a block, we need to check if it actually
                        // produces a value or just has side effects before returning
                        let arm_type = if let Expression::Block(stmts) = &arm.body {
                            // Check if the block has any non-return statements before the return
                            let mut block_type = AstType::Void;
                            let has_early_return = false;
                            
                            for (j, stmt) in stmts.iter().enumerate() {
                                match stmt {
                                    Statement::Return(_) => {
                                        // Don't use return statement to determine block type
                                        break;
                                    }
                                    Statement::Expression(expr) => {
                                        // If this is the last statement and there's no early return after it
                                        if j == stmts.len() - 1 && !has_early_return {
                                            block_type = self.infer_expression_type(expr)?;
                                        } else {
                                            // Still type-check intermediate expressions
                                            let _ = self.infer_expression_type(expr)?;
                                        }
                                    }
                                    _ => {
                                        self.check_statement(stmt)?;
                                    }
                                }
                            }
                            block_type
                        } else {
                            self.infer_expression_type(&arm.body)?
                        };

                        // The first non-void arm determines the type, or use first arm if all void
                        if i == 0 || (matches!(result_type, AstType::Void) && !matches!(arm_type, AstType::Void)) {
                            result_type = arm_type;
                        }

                        // Exit the scope to remove the bindings
                        self.exit_scope();
                    }

                    Ok(result_type)
                }
            }
            Expression::PatternMatch { arms, .. } => {
                // Pattern match expression type is determined by the first arm
                // All arms should have the same type
                if arms.is_empty() {
                    Ok(AstType::Void)
                } else {
                    let mut result_type = AstType::Void;

                    // Process each arm with its own pattern bindings
                    for (i, arm) in arms.iter().enumerate() {
                        // Enter a new scope for the pattern bindings
                        self.enter_scope();

                        // Extract pattern bindings and add them to the scope
                        self.add_pattern_bindings_to_scope(&arm.pattern)?;

                        // Infer the type with bindings in scope
                        let arm_type = self.infer_expression_type(&arm.body)?;

                        // The first arm determines the type
                        if i == 0 {
                            result_type = arm_type;
                        }

                        // Exit the scope to remove the bindings
                        self.exit_scope();
                    }

                    Ok(result_type)
                }
            }
            Expression::Block(statements) => {
                // Enter a new scope for the block
                self.enter_scope();

                let mut block_type = AstType::Void;

                // Process all statements in the block
                for (i, stmt) in statements.iter().enumerate() {
                    match stmt {
                        Statement::Expression(expr) => {
                            // The last expression determines the block's type
                            if i == statements.len() - 1 {
                                block_type = self.infer_expression_type(expr)?;
                            } else {
                                // Still type-check intermediate expressions
                                self.infer_expression_type(expr)?;
                            }
                        }
                        _ => {
                            // Process other statements (declarations, assignments, etc.)
                            self.check_statement(stmt)?;
                        }
                    }
                }

                // Exit the block's scope
                self.exit_scope();

                Ok(block_type)
            }
            Expression::Return(expr) => self.infer_expression_type(expr),
            Expression::EnumVariant {
                enum_name, variant, payload
            } => {
                // Infer the type of an enum variant
                // If enum_name is empty, search for an enum with this variant
                let enum_type_name = if enum_name.is_empty() {
                    // Search enum registry for enum containing this variant
                    let mut found_enum = None;
                    for (name, info) in &self.enums {
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
                    found_enum.unwrap_or_else(|| "Option".to_string())
                } else {
                    enum_name.clone()
                };

                // For generic types like Option and Result, infer type args from payload
                if enum_type_name == "Option" {
                    let inner_type = if let Some(p) = payload {
                        self.infer_expression_type(p)?
                    } else {
                        // For None variant, use a generic placeholder that can be unified later
                        AstType::Generic {
                            name: "T".to_string(),
                            type_args: vec![],
                        }
                    };
                    Ok(AstType::Generic {
                        name: "Option".to_string(),
                        type_args: vec![inner_type],
                    })
                } else if enum_type_name == "Result" {
                    // For Result, we need to infer based on the variant
                    if variant == "Ok" {
                        let ok_type = if let Some(p) = payload {
                            self.infer_expression_type(p)?
                        } else {
                            // For Ok with no payload, use a generic placeholder
                            AstType::Generic {
                                name: "T".to_string(),
                                type_args: vec![],
                            }
                        };
                        // We don't know the error type yet, default to String like codegen does
                        Ok(AstType::Generic {
                            name: "Result".to_string(),
                            type_args: vec![ok_type, AstType::StaticString],
                        })
                    } else if variant == "Err" {
                        let err_type = if let Some(p) = payload {
                            self.infer_expression_type(p)?
                        } else {
                            // For Err with no payload, use generic placeholder
                            AstType::Generic {
                                name: "E".to_string(),
                                type_args: vec![],
                            }
                        };
                        // We don't know the ok type yet, default to I32 like codegen does
                        Ok(AstType::Generic {
                            name: "Result".to_string(),
                            type_args: vec![AstType::I32, err_type],
                        })
                    } else {
                        // Unknown variant, default
                        Ok(AstType::Generic {
                            name: "Result".to_string(),
                            type_args: vec![AstType::I32, AstType::StaticString],
                        })
                    }
                } else {
                    // Look up the enum to get its variant info
                    if let Some(enum_info) = self.enums.get(&enum_type_name) {
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
                        // Fallback to generic for unknown enums
                        Ok(AstType::Generic {
                            name: enum_type_name,
                            type_args: vec![],
                        })
                    }
                }
            }
            Expression::StringLength(_) => Ok(AstType::I64),
            Expression::MethodCall {
                object,
                method,
                args: _,
            } => {
                // Special handling for collection .new() static methods
                if let Expression::Identifier(name) = &**object {
                    if method == "new" {
                        if name == "Array" {
                            // Array.new() returns an Array<T> type  
                            return Ok(AstType::Generic {
                                name: "Array".to_string(),
                                type_args: vec![AstType::I32], // Default to i32 array for now
                            });
                        } else if name.contains('<') {
                            // Generic collection constructor like HashMap<i32, i32>.new()
                            let (base_type, type_args) = Self::parse_generic_type_string(name);
                            return Ok(AstType::Generic {
                                name: base_type,
                                type_args,
                            });
                        }
                    }
                }
                
                // Implement UFC (Uniform Function Call)
                // Any function can be called as a method: object.function(args) -> function(object, args)

                // First check if it's a built-in method on the object type
                let object_type = self.infer_expression_type(object)?;

                // Handle method calls on references/pointers by dereferencing first
                // When we have vec_ref.get(0) where vec_ref is &DynVec<String>,
                // we need to dereference to DynVec<String> and then resolve the method
                let dereferenced_type = match &object_type {
                    AstType::Ptr(inner) | AstType::MutPtr(inner) | AstType::RawPtr(inner) => {
                        // For method calls on pointers, dereference to the inner type
                        // This allows vec_ref.get(0) to work when vec_ref is &DynVec<String>
                        Some(inner.as_ref().clone())
                    }
                    _ => None,
                };

                // Use dereferenced type if available, otherwise use original type
                let effective_type = dereferenced_type.as_ref().unwrap_or(&object_type);

                // Special handling for Vec type
                if let AstType::Vec { element_type, .. } = effective_type {
                    match method.as_str() {
                        "get" => return Ok(element_type.as_ref().clone()),  // Returns element type directly
                        "pop" => return Ok(AstType::Generic {       // Returns Option<element_type>
                            name: "Option".to_string(),
                            type_args: vec![element_type.as_ref().clone()],
                        }),
                        "len" | "capacity" => return Ok(AstType::I64),
                        "push" | "set" | "clear" => return Ok(AstType::Void),
                        _ => {}
                    }
                }

                // Special handling for generic collection methods
                if let AstType::Generic { name, type_args } = effective_type {
                    if name == "Array" && !type_args.is_empty() {
                        // Array methods
                        match method.as_str() {
                            "get" => return Ok(type_args[0].clone()),  // Returns element type directly
                            "pop" => return Ok(AstType::Generic {       // Returns Option<element_type>
                                name: "Option".to_string(),
                                type_args: vec![type_args[0].clone()],
                            }),
                            "len" => return Ok(AstType::I64),
                            "push" | "set" => return Ok(AstType::Void),
                            _ => {}
                        }
                    } else if name == "HashMap" && type_args.len() >= 2 {
                        // HashMap methods
                        match method.as_str() {
                            "get" | "remove" => return Ok(AstType::Generic {
                                name: "Option".to_string(),
                                type_args: vec![type_args[1].clone()],
                            }),
                            "contains" => return Ok(AstType::Bool),
                            "len" | "size" => return Ok(AstType::I64),
                            "is_empty" => return Ok(AstType::Bool),
                            "insert" | "clear" => return Ok(AstType::Void),
                            _ => {}
                        }
                    } else if name == "HashSet" && !type_args.is_empty() {
                        // HashSet methods
                        match method.as_str() {
                            "contains" => return Ok(AstType::Bool),
                            "remove" => return Ok(AstType::Bool),
                            "len" | "size" => return Ok(AstType::I64),
                            "is_empty" => return Ok(AstType::Bool),
                            "insert" | "clear" => return Ok(AstType::Void),
                            _ => {}
                        }
                    } else if name == "Vec" && !type_args.is_empty() {
                        // Vec methods - Vec.get returns the element directly, not Option
                        match method.as_str() {
                            "get" => return Ok(type_args[0].clone()),  // Returns element type directly
                            "pop" => return Ok(AstType::Generic {       // Returns Option<element_type>
                                name: "Option".to_string(),
                                type_args: vec![type_args[0].clone()],
                            }),
                            "len" | "capacity" => return Ok(AstType::I64),
                            "push" | "set" | "clear" => return Ok(AstType::Void),
                            _ => {}
                        }
                    } else if name == "DynVec" && !type_args.is_empty() {
                        // DynVec methods
                        match method.as_str() {
                            "get" | "pop" => return Ok(AstType::Generic {
                                name: "Option".to_string(),
                                type_args: vec![type_args[0].clone()],
                            }),
                            "len" => return Ok(AstType::I64),
                            "push" | "set" | "clear" => return Ok(AstType::Void),
                            _ => {}
                        }
                    }
                }
                
                // Special handling for DynVec type (not generic)
                if let AstType::DynVec { element_types, .. } = effective_type {
                    if !element_types.is_empty() {
                        match method.as_str() {
                            "get" | "pop" => return Ok(AstType::Generic {
                                name: "Option".to_string(),
                                type_args: vec![element_types[0].clone()],
                            }),
                            "len" => return Ok(AstType::I64),
                            "push" | "set" | "clear" => return Ok(AstType::Void),
                            _ => {}
                        }
                    }
                }

                // Try to find the function in scope
                // The method call object.method(args) becomes method(object, args)
                // For references, we need to pass the dereferenced type to UFC
                if let Some(func_type) = self.functions.get(method) {
                    // For UFC, the first parameter should match the object type (or dereferenced type)
                    if !func_type.params.is_empty() {
                        // Check if first param matches the effective type (dereferenced if pointer)
                        let (_, first_param_type) = &func_type.params[0];
                        if first_param_type == effective_type || first_param_type == &object_type {
                            // Return the function's return type
                            return Ok(func_type.return_type.clone());
                        }
                    }
                }

                // Special handling for string methods (both StaticString and String struct)
                let is_string_struct = matches!(effective_type, AstType::Struct { name, .. } if name == "String");
                if is_string_struct || *effective_type == AstType::StaticString || *effective_type == AstType::StaticLiteral {
                    // Common string methods with hardcoded return types for now
                    match method.as_str() {
                        "len" => return Ok(AstType::I64),
                        "to_i32" => {
                            return Ok(AstType::Generic {
                                name: "Option".to_string(),
                                type_args: vec![AstType::I32],
                            })
                        }
                        "to_i64" => {
                            return Ok(AstType::Generic {
                                name: "Option".to_string(),
                                type_args: vec![AstType::I64],
                            })
                        }
                        "to_f32" => {
                            return Ok(AstType::Generic {
                                name: "Option".to_string(),
                                type_args: vec![AstType::F32],
                            })
                        }
                        "to_f64" => {
                            return Ok(AstType::Generic {
                                name: "Option".to_string(),
                                type_args: vec![AstType::F64],
                            })
                        }
                        "substr" => {
                            // substr returns same type as input (static stays static, dynamic stays dynamic)
                            return Ok(if is_string_struct { crate::ast::resolve_string_struct_type() } else { AstType::StaticString })
                        }
                        "char_at" => return Ok(AstType::I32),
                        "split" => {
                            // split returns array of same string type as input
                            let string_type = if is_string_struct { crate::ast::resolve_string_struct_type() } else { AstType::StaticString };
                            return Ok(AstType::Generic {
                                name: "Array".to_string(),
                                type_args: vec![string_type],
                            })
                        }
                        "trim" => {
                            // trim returns same type as input
                            return Ok(if is_string_struct { crate::ast::resolve_string_struct_type() } else { AstType::StaticString })
                        }
                        "to_upper" => {
                            // to_upper returns same type as input
                            return Ok(if is_string_struct { crate::ast::resolve_string_struct_type() } else { AstType::StaticString })
                        }
                        "to_lower" => {
                            // to_lower returns same type as input
                            return Ok(if is_string_struct { crate::ast::resolve_string_struct_type() } else { AstType::StaticString })
                        }
                        "contains" => return Ok(AstType::Bool),
                        "starts_with" => return Ok(AstType::Bool),
                        "ends_with" => return Ok(AstType::Bool),
                        "index_of" => return Ok(AstType::I64),
                        _ => {}
                    }
                }

                // Special handling for .raise() method which extracts T from Result<T,E>
                if method == "raise" {
                    // Get the type of the object being raised
                    if let AstType::Generic { name, type_args } = &object_type {
                        if name == "Result" && type_args.len() == 2 {
                            // The raise() method returns the Ok type (T) from Result<T,E>
                            return Ok(type_args[0].clone());
                        }
                    }
                    // If not a Result type, return Void (will error during compilation)
                    return Ok(AstType::Void);
                }
                
                // Special handling for built-in methods like .loop()
                if method == "loop" {
                    // .loop() on ranges and collections returns void
                    return Ok(AstType::Void);
                }
                
                // Special handling for HashMap/HashSet/Vec methods
                if let AstType::Generic { name, type_args } = &object_type {
                    if name == "HashMap" {
                        match method.as_str() {
                            "size" | "len" => return Ok(AstType::I64),
                            "is_empty" => return Ok(AstType::Bool),
                            "clear" => return Ok(AstType::Void),
                            "contains" => return Ok(AstType::Bool),
                            "remove" => {
                                // HashMap.remove() returns Option<V>
                                if type_args.len() >= 2 {
                                    return Ok(AstType::Generic {
                                        name: "Option".to_string(),
                                        type_args: vec![type_args[1].clone()],
                                    });
                                }
                            }
                            "get" => {
                                // HashMap.get() returns Option<V>
                                if type_args.len() >= 2 {
                                    return Ok(AstType::Generic {
                                        name: "Option".to_string(),
                                        type_args: vec![type_args[1].clone()],
                                    });
                                }
                            }
                            "insert" => return Ok(AstType::Void),
                            _ => {}
                        }
                    } else if name == "HashSet" {
                        match method.as_str() {
                            "size" | "len" => return Ok(AstType::I64),
                            "is_empty" => return Ok(AstType::Bool),
                            "clear" => return Ok(AstType::Void),
                            "contains" => return Ok(AstType::Bool),
                            "insert" => return Ok(AstType::Bool),
                            "remove" => return Ok(AstType::Bool),
                            _ => {}
                        }
                    }
                }
                
                // Special handling for Vec<T, N> methods
                if let AstType::Vec { element_type, .. } = &object_type {
                    match method.as_str() {
                        "get" => return Ok(element_type.as_ref().clone()),  // Returns element directly
                        "pop" => return Ok(AstType::Generic {               // Returns Option<element>
                            name: "Option".to_string(),
                            type_args: vec![element_type.as_ref().clone()],
                        }),
                        "len" | "capacity" => return Ok(AstType::I64),
                        "push" | "set" | "clear" => return Ok(AstType::Void),
                        _ => {}
                    }
                }

                // Special handling for pointer methods (only if not already handled above)
                // These are explicit pointer operations, not method calls on the dereferenced type
                if dereferenced_type.is_none() {
                    match &object_type {
                        AstType::Ptr(_) | AstType::MutPtr(_) | AstType::RawPtr(_) => {
                            if method == "val" {
                                // Dereference pointer
                                return match object_type {
                                    AstType::Ptr(inner)
                                    | AstType::MutPtr(inner)
                                    | AstType::RawPtr(inner) => Ok(inner.as_ref().clone()),
                                    _ => unreachable!(),
                                };
                            } else if method == "addr" {
                                // Get address as usize
                                return Ok(AstType::Usize);
                            }
                        }
                        _ => {}
                    }
                }

                // Special handling for Result.raise()
                if method == "raise" {
                    match object_type {
                        AstType::Generic { name, type_args }
                            if name == "Result" && !type_args.is_empty() =>
                        {
                            return Ok(type_args[0].clone());
                        }
                        // Legacy Result type removed - all Results are Generic now
                        _ => {}
                    }
                }

                // If no specific handling found, return void as a fallback for unknown method calls
                // This is because most method calls without explicit handling are side-effect operations
                Ok(AstType::Void)
            }
            Expression::Loop { body: _ } => {
                // Loop expressions return void for now
                Ok(AstType::Void)
            }
            Expression::Raise(expr) => {
                // .raise() unwraps a Result type and returns the Ok variant
                // If it's an Err, it propagates the error
                let result_type = self.infer_expression_type(expr)?;
                match result_type {
                    // Handle generic Result<T, E> type
                    AstType::Generic { name, type_args }
                        if name == "Result" && type_args.len() >= 1 =>
                    {
                        // Return the Ok type (first type argument)
                        Ok(type_args[0].clone())
                    }
                    // Legacy Result type removed - all Results are Generic now
                    // Type error: .raise() can only be used on Result types
                    _ => Err(CompileError::TypeError(
                        format!(
                            ".raise() can only be used on Result<T, E> types, found: {:?}",
                            result_type
                        ),
                        None,
                    )),
                }
            }
            Expression::Break { .. } | Expression::Continue { .. } => {
                // Break and continue don't return a value, they transfer control
                // For type checking purposes, they can be considered to return void
                Ok(AstType::Void)
            }
            Expression::EnumLiteral { variant, payload } => {
                // For enum literals, we need to infer the enum type from context
                // For now, handle known types like Option and Result
                if variant == "Some" {
                    let inner_type = if let Some(p) = payload {
                        self.infer_expression_type(p)?
                    } else {
                        AstType::Void
                    };
                    Ok(AstType::Generic {
                        name: "Option".to_string(),
                        type_args: vec![inner_type],
                    })
                } else if variant == "None" {
                    // None can be any Option type - will need context to determine
                    // eprintln!("DEBUG: Struct field None being converted to Option<T>");
                    Ok(AstType::Generic {
                        name: "Option".to_string(),
                        type_args: vec![AstType::Generic {
                            name: "T".to_string(),
                            type_args: vec![],
                        }], // Use generic T instead of Void
                    })
                } else if variant == "Ok" {
                    let ok_type = if let Some(p) = payload {
                        self.infer_expression_type(p)?
                    } else {
                        AstType::Void
                    };
                    // For Result, we don't know the error type yet
                    Ok(AstType::Generic {
                        name: "Result".to_string(),
                        type_args: vec![ok_type, crate::ast::resolve_string_struct_type()], // Default to String for errors
                    })
                } else if variant == "Err" {
                    let err_type = if let Some(p) = payload {
                        self.infer_expression_type(p)?
                    } else {
                        AstType::Void
                    };
                    // For Result, we don't know the ok type yet
                    Ok(AstType::Generic {
                        name: "Result".to_string(),
                        type_args: vec![
                            AstType::Generic {
                                name: "T".to_string(),
                                type_args: vec![],
                            },
                            err_type
                        ], // Use generic T for unknown ok type
                    })
                } else {
                    // Unknown enum literal - would need more context
                    Ok(AstType::Void)
                }
            }
            Expression::Conditional { scrutinee, arms } => {
                // eprintln!("DEBUG TypeChecker: Processing conditional with {} arms", arms.len());
                // Conditional expression type is determined by the first arm
                // All arms should have the same type (checked during type checking)

                // Infer the type of the scrutinee to properly type pattern bindings
                let scrutinee_type = self.infer_expression_type(scrutinee)?;

                if arms.is_empty() {
                    Ok(AstType::Void)
                } else {
                    let mut result_type = AstType::Void;

                    // Process each arm with its own pattern bindings
                    for (i, arm) in arms.iter().enumerate() {
                        // eprintln!("DEBUG TypeChecker: Processing arm {} pattern: {:?}", i, arm.pattern);

                        // Enter a new scope for the pattern bindings
                        self.enter_scope();
                        // eprintln!("DEBUG TypeChecker: Entered scope for arm {}", i);

                        // Extract pattern bindings and add them to the scope
                        self.add_pattern_bindings_to_scope_with_type(
                            &arm.pattern,
                            &scrutinee_type,
                        )?;
                        // eprintln!("DEBUG TypeChecker: Added pattern bindings for arm {}", i);

                        // Infer the type with bindings in scope
                        // eprintln!("DEBUG TypeChecker: Inferring type for arm {} body", i);
                        let arm_type = self.infer_expression_type(&arm.body)?;
                        // eprintln!("DEBUG TypeChecker: Arm {} type: {:?}", i, arm_type);

                        // The first arm determines the type
                        if i == 0 {
                            result_type = arm_type;
                        }

                        // Exit the scope to remove the bindings
                        self.exit_scope();
                        // eprintln!("DEBUG TypeChecker: Exited scope for arm {}", i);
                    }

                    Ok(result_type)
                }
            }
            // Zen spec pointer operations
            Expression::PointerDereference(expr) => {
                // ptr.val -> T (if ptr is Ptr<T>, MutPtr<T>, or RawPtr<T>)
                let ptr_type = self.infer_expression_type(expr)?;
                match ptr_type {
                    AstType::Ptr(inner) | AstType::MutPtr(inner) | AstType::RawPtr(inner) => {
                        Ok(*inner)
                    }
                    _ => Err(CompileError::TypeError(
                        format!("Cannot dereference non-pointer type: {:?}", ptr_type),
                        None,
                    )),
                }
            }
            Expression::PointerAddress(expr) => {
                // expr.addr -> RawPtr<T> (if expr is of type T)
                let expr_type = self.infer_expression_type(expr)?;
                Ok(AstType::RawPtr(Box::new(expr_type)))
            }
            Expression::CreateReference(expr) => {
                // expr.ref() -> Ptr<T> (if expr is of type T)
                let expr_type = self.infer_expression_type(expr)?;
                Ok(AstType::Ptr(Box::new(expr_type)))
            }
            Expression::CreateMutableReference(expr) => {
                // expr.mut_ref() -> MutPtr<T> (if expr is of type T)
                let expr_type = self.infer_expression_type(expr)?;
                Ok(AstType::MutPtr(Box::new(expr_type)))
            }
            Expression::VecConstructor {
                element_type,
                size,
                initial_values: _,
            } => {
                // Vec<T, size>() -> Vec<T, size>
                Ok(AstType::Vec {
                    element_type: Box::new(element_type.clone()),
                    size: *size,
                })
            }
            Expression::DynVecConstructor {
                element_types,
                allocator: _,
                initial_capacity: _,
            } => {
                // DynVec<T>() or DynVec<T1, T2, ...>() -> DynVec<T, ...>
                Ok(AstType::DynVec {
                    element_types: element_types.clone(),
                    allocator_type: None, // Allocator type inferred from constructor arg
                })
            }
            Expression::ArrayConstructor { element_type } => {
                // Array<T>() -> Generic { name: "Array", type_args: [T] }
                // This matches the expected type format for generic types
                Ok(AstType::Generic {
                    name: "Array".to_string(),
                    type_args: vec![element_type.clone()],
                })
            }
            Expression::Some(inner) => {
                // eprintln!("DEBUG TypeChecker: Processing Some() with inner expr");
                // Check the inner expression to determine the actual type
                let inner_type = self.infer_expression_type(inner)?;
                // eprintln!("DEBUG TypeChecker: Some() inner type: {:?}", inner_type);
                // Option::Some(T) -> Option<T>
                Ok(AstType::Generic {
                    name: "Option".to_string(),
                    type_args: vec![inner_type],
                })
            }
            Expression::None => {
                // Option::None -> Option<Void> as a default
                // The actual type will be inferred from context during type checking
                // Using Void as placeholder to avoid unresolved generic T
                Ok(AstType::Generic {
                    name: "Option".to_string(),
                    type_args: vec![AstType::Void],
                })
            }
            Expression::CollectionLoop { .. } => {
                // collection.loop() returns unit/void
                Ok(AstType::Void)
            }
            Expression::Defer(_) => {
                // @this.defer() returns unit/void
                Ok(AstType::Void)
            }
            Expression::InlineC { code, interpolations } => {
                // Validate inline C code
                // Check that code is not empty
                if code.trim().is_empty() {
                    return Err(CompileError::TypeError(
                        "compiler.inline_c() requires non-empty C code".to_string(),
                        None,
                    ));
                }
                
                // Validate interpolations - each should be a valid expression
                for (var_name, expr) in interpolations {
                    // Check that variable name is valid C identifier
                    if !var_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                        return Err(CompileError::TypeError(
                            format!("Invalid C variable name in inline_c interpolation: '{}'", var_name),
                            None,
                        ));
                    }
                    
                    // Validate the expression type
                    let expr_type = self.infer_expression_type(expr)?;
                    // Inline C can work with primitive types, pointers, etc.
                    // We'll allow most types but warn about complex types
                    match expr_type {
                        AstType::I8 | AstType::I16 | AstType::I32 | AstType::I64 |
                        AstType::U8 | AstType::U16 | AstType::U32 | AstType::U64 | AstType::Usize |
                        AstType::F32 | AstType::F64 |
                        AstType::Bool |
                        AstType::Ptr(_) | AstType::MutPtr(_) | AstType::RawPtr(_) => {
                            // These are fine
                        }
                        _ => {
                            // Warn but allow - user knows what they're doing
                            eprintln!("Warning: Using complex type {:?} in inline_c() - ensure C compatibility", expr_type);
                        }
                    }
                }
                
                // inline_c returns void
                Ok(AstType::Void)
            }
        }
    }

    fn types_compatible(&self, expected: &AstType, actual: &AstType) -> bool {
        validation::types_compatible(expected, actual)
    }

    fn register_stdlib_module(&mut self, alias: &str, module_path: &str) -> Result<()> {
        stdlib::register_stdlib_module(self, alias, module_path)
    }

    fn enter_scope(&mut self) {
        scope::enter_scope(self)
    }

    fn exit_scope(&mut self) {
        scope::exit_scope(self)
    }

    fn declare_variable(&mut self, name: &str, type_: AstType, is_mutable: bool) -> Result<()> {
        scope::declare_variable(self, name, type_, is_mutable)
    }

    fn declare_variable_with_init(
        &mut self,
        name: &str,
        type_: AstType,
        is_mutable: bool,
        is_initialized: bool,
    ) -> Result<()> {
        scope::declare_variable_with_init(self, name, type_, is_mutable, is_initialized)
    }

    fn mark_variable_initialized(&mut self, name: &str) -> Result<()> {
        scope::mark_variable_initialized(self, name)
    }

    /// Infer the return type of a function from its body
    fn infer_function_return_type(&mut self, func: &Function) -> Result<AstType> {
        // Create a temporary scope for the function
        self.enter_scope();
        
        // Add function parameters to scope
        for (param_name, param_type) in &func.args {
            self.declare_variable(param_name, param_type.clone(), false)?;
        }
        
        // Analyze the body to find the return type
        let return_type = if let Some(last_stmt) = func.body.last() {
            match last_stmt {
                Statement::Expression(expr) => {
                    // The last expression is the return value
                    self.infer_expression_type(expr)?
                }
                Statement::Return(expr) => {
                    // Explicit return statement
                    self.infer_expression_type(expr)?
                }
                _ => {
                    // Other statements don't produce a return value
                    AstType::Void
                }
            }
        } else {
            // Empty body returns void
            AstType::Void
        };
        
        self.exit_scope();
        Ok(return_type)
    }
    
    fn get_variable_type(&self, name: &str) -> Result<AstType> {
        scope::get_variable_type(self, name, &self.enums)
    }

    fn get_variable_info(&self, name: &str) -> Result<VariableInfo> {
        scope::get_variable_info(self, name)
    }

    fn add_pattern_bindings_to_scope(&mut self, pattern: &crate::ast::Pattern) -> Result<()> {
        // Default to I32 when no type context is available (legacy behavior)
        self.add_pattern_bindings_to_scope_with_type(pattern, &AstType::I32)
    }

    fn add_pattern_bindings_to_scope_with_type(
        &mut self,
        pattern: &crate::ast::Pattern,
        scrutinee_type: &AstType,
    ) -> Result<()> {
        use crate::ast::Pattern;

        // eprintln!("DEBUG TypeChecker: add_pattern_bindings_to_scope_with_type for pattern: {:?}, type: {:?}", pattern, scrutinee_type);

        match pattern {
            Pattern::Identifier(name) => {
                // Simple identifier pattern binds the name to the type of the matched value
                // Check if the scrutinee is a primitive generic type that should be unwrapped
                let binding_type = if let AstType::Generic { name: type_name, type_args } = scrutinee_type {
                    if type_args.is_empty() {
                        // Check if it's a primitive type name that got wrapped as Generic
                        match type_name.as_str() {
                            "i32" | "I32" => AstType::I32,
                            "i64" | "I64" => AstType::I64,
                            "f32" | "F32" => AstType::F32,
                            "f64" | "F64" => AstType::F64,
                            "bool" | "Bool" => AstType::Bool,
                            "string" => AstType::StaticString,
                            "String" => crate::ast::resolve_string_struct_type(),
                            _ => scrutinee_type.clone()
                        }
                    } else {
                        scrutinee_type.clone()
                    }
                } else {
                    scrutinee_type.clone()
                };

                self.declare_variable(name, binding_type, false)?;
            }
            Pattern::EnumLiteral { variant, payload } => {
                // For enum patterns with payloads, determine the payload type based on the variant
                if let Some(payload_pattern) = payload {
                    let payload_type = if let AstType::Generic {
                        name: enum_name,
                        type_args,
                    } = scrutinee_type
                    {
                        if enum_name == "Result" && type_args.len() >= 2 {
                            // For Result<T,E>, Ok has type T, Err has type E
                            if variant == "Ok" {
                                type_args[0].clone()
                            } else if variant == "Err" {
                                type_args[1].clone()
                            } else {
                                AstType::I32
                            }
                        } else if enum_name == "Option" && !type_args.is_empty() {
                            // For Option<T>, Some has type T
                            if variant == "Some" {
                                type_args[0].clone()
                            } else {
                                AstType::Void
                            }
                        } else {
                            scrutinee_type.clone()
                        }
                    } else {
                        scrutinee_type.clone()
                    };
                    self.add_pattern_bindings_to_scope_with_type(payload_pattern, &payload_type)?;
                }
            }
            Pattern::EnumVariant {
                enum_name: _,
                variant, payload, ..
            } => {
                // For qualified enum patterns with payloads, determine the payload type based on the variant
                if let Some(payload_pattern) = payload {
                    let payload_type = if let AstType::Generic {
                        name: enum_name,
                        type_args,
                    } = scrutinee_type
                    {
                        if enum_name == "Result" && type_args.len() >= 2 {
                            // For Result<T,E>, Ok has type T, Err has type E
                            if variant == "Ok" {
                                type_args[0].clone()
                            } else if variant == "Err" {
                                type_args[1].clone()
                            } else {
                                AstType::I32
                            }
                        } else if enum_name == "Option" && !type_args.is_empty() {
                            // For Option<T>, Some has type T
                            if variant == "Some" {
                                type_args[0].clone()
                            } else {
                                AstType::Void
                            }
                        } else {
                            scrutinee_type.clone()
                        }
                    } else {
                        scrutinee_type.clone()
                    };
                    self.add_pattern_bindings_to_scope_with_type(payload_pattern, &payload_type)?;
                }
            }
            Pattern::Binding { name, pattern } => {
                // Binding pattern: name @ pattern
                // Add the name as a variable with the scrutinee type
                self.declare_variable(name, scrutinee_type.clone(), false)?;
                // And recursively process the pattern
                self.add_pattern_bindings_to_scope_with_type(pattern, scrutinee_type)?;
            }
            Pattern::Or(patterns) => {
                // For or patterns, we need to ensure all alternatives bind the same names
                // For now, just process the first one
                if let Some(first) = patterns.first() {
                    self.add_pattern_bindings_to_scope_with_type(first, scrutinee_type)?;
                }
            }
            Pattern::Struct { fields, .. } => {
                // For struct patterns, add bindings for all fields
                for field in fields {
                    // field is (String, Pattern)
                    // TODO: Should extract field type from struct type
                    self.add_pattern_bindings_to_scope_with_type(&field.1, scrutinee_type)?;
                }
            }
            Pattern::Type { binding, .. } => {
                // Type pattern with optional binding
                if let Some(name) = binding {
                    self.declare_variable(name, scrutinee_type.clone(), false)?;
                }
            }
            // Other patterns don't create bindings
            Pattern::Wildcard
            | Pattern::Literal(_)
            | Pattern::Range { .. }
            | Pattern::Guard { .. } => {}
        }
        Ok(())
    }

    fn variable_exists(&self, name: &str) -> bool {
        scope::variable_exists(self, name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    #[test]
    fn test_basic_type_checking() {
        let input = "main: () void = {
            x = 42
            y : i32 = 100
            z = x + y
        }";

        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();

        let mut type_checker = TypeChecker::new();
        assert!(type_checker.check_program(&program).is_ok());
    }

    #[test]
    fn test_type_mismatch_error() {
        let input = "main: () void = {
            x : i32 = \"hello\"
        }";

        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_program(&program);
        assert!(result.is_err());
        if let Err(CompileError::TypeError(msg, _)) = result {
            assert!(msg.contains("Type mismatch"));
        }
    }
}
