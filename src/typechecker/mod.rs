pub mod types;
pub mod inference;
pub mod validation;
pub mod behaviors;
pub mod self_resolution;

use crate::ast::{Program, Declaration, Statement, Expression, AstType, Function};
use crate::error::{CompileError, Result};
use crate::stdlib::StdNamespace;
use std::collections::HashMap;
use behaviors::BehaviorResolver;

#[derive(Clone, Debug)]
pub struct VariableInfo {
    pub type_: AstType,
    pub is_mutable: bool,
    pub is_initialized: bool,
}

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
    pub fn new() -> Self {
        let mut enums = HashMap::new();
        
        // Register built-in Option<T> type
        enums.insert("Option".to_string(), EnumInfo {
            variants: vec![
                ("Some".to_string(), Some(AstType::Generic { name: "T".to_string(), type_args: vec![] })),
                ("None".to_string(), None),
            ],
        });
        
        // Register built-in Result<T, E> type
        enums.insert("Result".to_string(), EnumInfo {
            variants: vec![
                ("Ok".to_string(), Some(AstType::Generic { name: "T".to_string(), type_args: vec![] })),
                ("Err".to_string(), Some(AstType::Generic { name: "E".to_string(), type_args: vec![] })),
            ],
        });
        
        Self {
            scopes: vec![HashMap::new()],
            functions: HashMap::new(),
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

        // Second pass: type check function bodies
        for declaration in &program.declarations {
            self.check_declaration(declaration)?;
        }

        Ok(())
    }

    fn collect_declaration_types(&mut self, declaration: &Declaration) -> Result<()> {
        match declaration {
            Declaration::Function(func) => {
                let signature = FunctionSignature {
                    params: func.args.clone(),
                    return_type: func.return_type.clone(),
                    is_external: false,
                };
                self.functions.insert(func.name.clone(), signature);
            }
            Declaration::ExternalFunction(ext_func) => {
                // External functions have args as Vec<AstType>, convert to params format
                let params = ext_func.args.iter().enumerate().map(|(i, t)| {
                    (format!("arg{}", i), t.clone())
                }).collect();
                let signature = FunctionSignature {
                    params,
                    return_type: ext_func.return_type.clone(),
                    is_external: true,
                };
                self.functions.insert(ext_func.name.clone(), signature);
            }
            Declaration::Struct(struct_def) => {
                // Convert StructField to (String, AstType)
                let fields: Vec<(String, AstType)> = struct_def.fields.iter().map(|f| {
                    (f.name.clone(), f.type_.clone())
                }).collect();
                let info = StructInfo {
                    fields: fields.clone(),
                };
                eprintln!("DEBUG: Registering struct '{}' with {} fields", struct_def.name, fields.len());
                for (field_name, field_type) in &fields {
                    eprintln!("DEBUG:   Field '{}': {:?}", field_name, field_type);
                }
                self.structs.insert(struct_def.name.clone(), info);
            }
            Declaration::Enum(enum_def) => {
                // Convert EnumVariant to (String, Option<AstType>)
                let variants = enum_def.variants.iter().map(|v| {
                    (v.name.clone(), v.payload.clone())
                }).collect();
                let info = EnumInfo {
                    variants,
                };
                self.enums.insert(enum_def.name.clone(), info);
            }
            Declaration::Behavior(behavior_def) => {
                self.behavior_resolver.register_behavior(behavior_def)?;
            }
            Declaration::Trait(trait_def) => {
                self.behavior_resolver.register_trait(trait_def)?;
            }
            Declaration::TraitImplementation(trait_impl) => {
                self.behavior_resolver.register_trait_implementation(trait_impl)?;
            }
            Declaration::TraitRequirement(trait_req) => {
                self.behavior_resolver.register_trait_requirement(trait_req)?;
            }
            Declaration::Constant { name, value, type_ } => {
                // Type check the constant value
                let inferred_type = self.infer_expression_type(value)?;
                
                // If a type was specified, verify it matches
                if let Some(declared_type) = type_ {
                    if !self.types_compatible(declared_type, &inferred_type) {
                        return Err(CompileError::TypeError(
                            format!(
                                "Type mismatch: constant '{}' declared as {:?} but has value of type {:?}",
                                name, declared_type, inferred_type
                            ),
                            None
                        ));
                    }
                }
                
                // Store the constant as a global variable (constants are immutable)
                self.declare_variable(name, inferred_type, false)?;
            }
            Declaration::ModuleImport { alias, module_path } => {
                // Track module imports
                self.module_imports.insert(alias.clone(), module_path.clone());
                // Register stdlib functions if this is a known stdlib module
                // Handle "@std.math", "std.math", and "math" formats
                let module_name = if module_path.starts_with("@std.") {
                    &module_path[5..]  // Remove "@std." prefix
                } else if module_path.starts_with("std.") {
                    &module_path[4..]  // Remove "std." prefix
                } else {
                    module_path.as_str()
                };
                self.register_stdlib_module(alias, module_name)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn check_declaration(&mut self, declaration: &Declaration) -> Result<()> {
        match declaration {
            Declaration::Function(func) => {
                self.check_function(func)?;
            }
            Declaration::ComptimeBlock(statements) => {
                // Check for imports in comptime blocks
                for stmt in statements {
                    if let Err(msg) = validation::validate_import_not_in_comptime(stmt) {
                        return Err(CompileError::SyntaxError(msg, None));
                    }
                }
                
                self.enter_scope();
                for statement in statements {
                    self.check_statement(statement)?;
                }
                self.exit_scope();
            }
            Declaration::Trait(_) => {
                // Trait definitions don't need additional checking beyond registration
            }
            Declaration::TraitImplementation(trait_impl) => {
                // Verify that the implementation satisfies the trait
                self.behavior_resolver.verify_trait_implementation(trait_impl)?;
                // Type check each method in the implementation
                // We need to set context for resolving Self types
                self.current_impl_type = Some(trait_impl.type_name.clone());
                eprintln!("DEBUG: Checking trait implementation for type: {}", trait_impl.type_name);
                for method in &trait_impl.methods {
                    eprintln!("DEBUG: Checking method: {}", method.name);
                    self.check_function(method)?;
                }
                self.current_impl_type = None;
            }
            Declaration::TraitRequirement(trait_req) => {
                // Verify that the requirement is valid
                self.behavior_resolver.verify_trait_requirement(trait_req)?;
            }
            Declaration::Constant { .. } => {
                // Constants are already type-checked in collect_declaration_types
            }
            Declaration::ModuleImport { alias, module_path: _ } => {
                // Handle module imports like { io, math } = @std which become
                // ModuleImport declarations at the top level
                // Register the imported module as a variable with StdModule type
                self.declare_variable_with_init(alias, AstType::StdModule, false, true)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn check_function(&mut self, function: &Function) -> Result<()> {
        self.enter_scope();

        // Add function parameters to scope
        // TODO: Parse and handle mutable parameters (:: syntax)
        // For now, all parameters are immutable
        for (param_name, param_type) in &function.args {
            // Special handling for 'self' parameter in trait implementations
            let actual_type = if param_name == "self" {
                match param_type {
                    AstType::Generic { name, .. } if name == "Self" || name.starts_with("Self_") => {
                        // Replace Self with the concrete implementing type
                        if let Some(impl_type) = &self.current_impl_type {
                            // Look up the actual struct fields from the registry
                            if let Some(struct_info) = self.structs.get(impl_type) {
                                AstType::Struct {
                                    name: impl_type.clone(),
                                    fields: struct_info.fields.clone(),
                                }
                            } else {
                                // Fallback if struct not found
                                AstType::Struct {
                                    name: impl_type.clone(),
                                    fields: vec![],
                                }
                            }
                        } else {
                            param_type.clone()
                        }
                    }
                    _ => param_type.clone()
                }
            } else {
                param_type.clone()
            };
            
            self.declare_variable(param_name, actual_type, false)?; // false = immutable
        }

        // Check function body
        for statement in &function.body {
            self.check_statement(statement)?;
        }

        self.exit_scope();
        Ok(())
    }

    fn check_statement(&mut self, statement: &Statement) -> Result<()> {
        // Note: Import validation is handled in check_declaration for ComptimeBlocks
        
        match statement {
            Statement::VariableDeclaration {
                name,
                type_,
                initializer,
                is_mutable,
                declaration_type,
                ..
            } => {
                // Check if this is an assignment to a forward-declared variable
                // This happens when we have: x: i32 (forward decl) then x = 10 (initialization)
                if let Some(init_expr) = initializer {
                    // Check if variable already exists (forward declaration case)
                    if self.variable_exists(name) {
                        // This might be initialization of a forward-declared variable
                        let var_info = self.get_variable_info(name)?;
                        
                        // Allow initialization of forward-declared variables with = operator
                        // This works for both immutable (x: i32 then x = 10) and mutable (w:: i32 then w = 20)
                        if !var_info.is_initialized {
                            // This is initialization of a forward-declared variable
                            // The = operator can be used to initialize both immutable and mutable forward declarations
                            let inferred_type = self.infer_expression_type(init_expr)?;
                            if !self.types_compatible(&var_info.type_, &inferred_type) {
                                return Err(CompileError::TypeError(
                                    format!(
                                        "Type mismatch: variable '{}' declared as {:?} but initialized with {:?}",
                                        name, var_info.type_, inferred_type
                                    ),
                                    None
                                ));
                            }
                            self.mark_variable_initialized(name)?;
                            return Ok(());
                        } else if var_info.is_initialized && var_info.is_mutable {
                            // This is a reassignment to an existing mutable variable
                            // (e.g., w = 25 after w:: i32 and w = 20)
                            let inferred_type = self.infer_expression_type(init_expr)?;
                            if !self.types_compatible(&var_info.type_, &inferred_type) {
                                return Err(CompileError::TypeError(
                                    format!(
                                        "Type mismatch: cannot assign {:?} to variable '{}' of type {:?}",
                                        inferred_type, name, var_info.type_
                                    ),
                                    None
                                ));
                            }
                            // Reassignment is allowed for mutable variables
                            return Ok(());
                        } else {
                            // Immutable variable already initialized - cannot reassign
                            return Err(CompileError::TypeError(
                                format!("Cannot reassign immutable variable '{}'", name),
                                None
                            ));
                        }
                    }
                    
                    // New variable declaration with initializer
                    let inferred_type = self.infer_expression_type(init_expr)?;
                    
                    if let Some(declared_type) = type_ {
                        // Check that the initializer type matches the declared type
                        if !self.types_compatible(declared_type, &inferred_type) {
                            return Err(CompileError::TypeError(
                                format!(
                                    "Type mismatch: variable '{}' declared as {:?} but initialized with {:?}",
                                    name, declared_type, inferred_type
                                ),
                                None
                            ));
                        }
                        self.declare_variable_with_init(name, declared_type.clone(), *is_mutable, true)?;
                    } else {
                        // Inferred type from initializer
                        self.declare_variable_with_init(name, inferred_type, *is_mutable, true)?;
                    }
                } else if let Some(declared_type) = type_ {
                    // Forward declaration without initializer
                    self.declare_variable_with_init(name, declared_type.clone(), *is_mutable, false)?;
                } else {
                    return Err(CompileError::TypeError(
                        format!("Cannot infer type for variable '{}' without initializer", name),
                        None
                    ));
                }
            }
            Statement::VariableAssignment { name, value } => {
                eprintln!("DEBUG TypeChecker: VariableAssignment '{}' with value type: {}", 
                    name,
                    match value {
                        Expression::Conditional { .. } => "Conditional",
                        Expression::Identifier(_) => "Identifier", 
                        Expression::Some(_) => "Some",
                        Expression::None => "None",
                        _ => "Other"
                    }
                );
                // Check if variable exists
                if !self.variable_exists(name) {
                    // This is a new immutable declaration using = operator
                    let value_type = self.infer_expression_type(value)?;
                    self.declare_variable_with_init(name, value_type, false, true)?; // false = immutable
                } else {
                    // This is an assignment to existing variable
                    let var_info = self.get_variable_info(name)?;
                    
                    // Check if this is the first assignment to a forward-declared variable
                    if !var_info.is_initialized {
                        // This is the initial assignment
                        let value_type = self.infer_expression_type(value)?;
                        if !self.types_compatible(&var_info.type_, &value_type) {
                            return Err(CompileError::TypeError(
                                format!(
                                    "Type mismatch in initial assignment to '{}': expected {:?}, got {:?}",
                                    name, var_info.type_, value_type
                                ),
                                None
                            ));
                        }
                        // Mark as initialized
                        self.mark_variable_initialized(name)?;
                    } else {
                        // This is a reassignment
                        if !var_info.is_mutable {
                            return Err(CompileError::TypeError(
                                format!("Cannot reassign to immutable variable '{}'", name),
                                None
                            ));
                        }
                        
                        let value_type = self.infer_expression_type(value)?;
                        
                        if !self.types_compatible(&var_info.type_, &value_type) {
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
                let _return_type = self.infer_expression_type(expr)?;
                // TODO: Check against function return type
            }
            Statement::Expression(expr) => {
                eprintln!("DEBUG TypeChecker: check_statement for Expression: {:?}", expr);
                self.infer_expression_type(expr)?;
            }
            Statement::Loop { kind, body, .. } => {
                use crate::ast::LoopKind;
                self.enter_scope();
                
                // Handle loop-specific variables
                match kind {
                    LoopKind::Infinite => {
                        // No special handling needed
                    }
                    LoopKind::Condition(expr) => {
                        // Type check the condition
                        let cond_type = self.infer_expression_type(expr)?;
                        // Condition should be boolean or integer (truthy)
                        if !matches!(cond_type, AstType::Bool | AstType::I32 | AstType::I64) {
                            return Err(CompileError::TypeError(
                                format!("Loop condition must be boolean or integer, got {:?}", cond_type),
                                None
                            ));
                        }
                    }
                }
                
                // Check loop body with the variable in scope
                for stmt in body {
                    self.check_statement(stmt)?;
                }
                self.exit_scope();
            }
            Statement::ComptimeBlock(statements) => {
                self.enter_scope();
                for stmt in statements {
                    self.check_statement(stmt)?;
                }
                self.exit_scope();
            }
            Statement::PointerAssignment { pointer, value } => {
                // For array indexing like arr[i] = value
                // The pointer expression should be a pointer type
                let _pointer_type = self.infer_expression_type(pointer)?;
                let _value_type = self.infer_expression_type(value)?;
                // TODO: Type check that value is compatible with the pointed-to type
            }
            Statement::DestructuringImport { names, source: _ } => {
                // Handle destructuring imports: { io, math } = @std
                // Register each imported module as a variable with StdModule type
                for name in names {
                    self.declare_variable_with_init(name, AstType::StdModule, false, true)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn infer_expression_type(&mut self, expr: &Expression) -> Result<AstType> {
        eprintln!("DEBUG TypeChecker: infer_expression_type called for expr type: {}", 
            match expr {
                Expression::Integer8(_) => "Integer8",
                Expression::Integer16(_) => "Integer16", 
                Expression::Integer32(_) => "Integer32",
                Expression::Integer64(_) => "Integer64",
                Expression::Identifier(_) => "Identifier",
                Expression::Conditional { .. } => "Conditional",
                Expression::PatternMatch { .. } => "PatternMatch",
                Expression::QuestionMatch { .. } => "QuestionMatch",
                Expression::Some(_) => "Some",
                Expression::None => "None",
                Expression::String(_) => "String",
                Expression::Boolean(_) => "Boolean",
                Expression::Unit => "Unit",
                _ => "Other"
            }
        );
        match expr {
            Expression::Integer32(_) => Ok(AstType::I32),
            Expression::Integer64(_) => Ok(AstType::I64),
            Expression::Float32(_) => Ok(AstType::F32),
            Expression::Float64(_) => Ok(AstType::F64),
            Expression::Boolean(_) => Ok(AstType::Bool),
            Expression::Unit => Ok(AstType::Void),
            Expression::String(_) => Ok(AstType::String),
            Expression::Identifier(name) => {
                eprintln!("DEBUG TypeChecker: Looking up identifier '{}'", name);
                // First check if it's a function name
                if let Some(sig) = self.functions.get(name) {
                    // Return function pointer type
                    Ok(AstType::FunctionPointer {
                        param_types: sig.params.iter().map(|(_, t)| t.clone()).collect(),
                        return_type: Box::new(sig.return_type.clone()),
                    })
                } else {
                    // Otherwise check if it's a variable
                    self.get_variable_type(name)
                }
            }
            Expression::BinaryOp { left, op, right } => {
                inference::infer_binary_op_type(self, left, op, right)
            }
            Expression::FunctionCall { name, .. } => {
                // Check if this is a stdlib function call (e.g., io.print)
                if name.contains('.') {
                    let parts: Vec<&str> = name.splitn(2, '.').collect();
                    if parts.len() == 2 {
                        let module = parts[0];
                        let func = parts[1];
                        
                        // Handle stdlib function return types
                        match (module, func) {
                            ("io", "print" | "println" | "print_int" | "print_float") => return Ok(AstType::Void),
                            ("io", "read_line") => return Ok(AstType::String),
                            ("math", "abs") => return Ok(AstType::I32),
                            ("math", "sqrt") => return Ok(AstType::F64),
                            ("math", "sin" | "cos" | "tan") => return Ok(AstType::F64),
                            ("math", "floor" | "ceil") => return Ok(AstType::I32),
                            ("math", "pow") => return Ok(AstType::F64),
                            ("math", "min" | "max") => return Ok(AstType::I32),
                            ("string", "len") => return Ok(AstType::I32),
                            ("string", "concat") => return Ok(AstType::String),
                            ("mem", "alloc") => return Ok(AstType::Ptr(Box::new(AstType::U8))),
                            ("mem", "free") => return Ok(AstType::Void),
                            ("fs", "read_file") => return Ok(AstType::String),
                            ("fs", "write_file") => return Ok(AstType::Bool),
                            _ => {}
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
                        Ok(_) => {
                            Err(CompileError::TypeError(format!("'{}' is not a function", name), None))
                        }
                        Err(_) => {
                            Err(CompileError::TypeError(format!("Unknown function: {}", name), None))
                        }
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
                // String interpolation always returns a string (pointer to char)
                Ok(AstType::Ptr(Box::new(AstType::I8)))
            }
            Expression::ArrayIndex { array, .. } => {
                // Array indexing returns the element type
                let array_type = self.infer_expression_type(array)?;
                match array_type {
                    AstType::Ptr(elem_type) => Ok(*elem_type),
                    AstType::Array(elem_type) => Ok(*elem_type),
                    _ => Err(CompileError::TypeError(
                        format!("Cannot index type {:?}", array_type),
                        None
                    ))
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
                        None
                    ))
                }
            }
            Expression::PointerOffset { pointer, .. } => {
                // Pointer offset returns the same pointer type
                self.infer_expression_type(pointer)
            }
            Expression::StructField { struct_, field } => {
                let struct_type = self.infer_expression_type(struct_)?;
                match struct_type {
                    AstType::Ptr(inner) => {
                        // Handle pointer to struct - automatically dereference
                        match *inner {
                            AstType::Struct { name, .. } => {
                                inference::infer_member_type(&AstType::Struct { name, fields: vec![] }, field, &self.structs, &self.enums)
                            }
                            AstType::Generic { ref name, .. } => {
                                // Handle pointer to generic struct
                                inference::infer_member_type(&AstType::Generic { name: name.clone(), type_args: vec![] }, field, &self.structs, &self.enums)
                            }
                            _ => Err(CompileError::TypeError(
                                format!("Cannot access field '{}' on non-struct pointer type", field),
                                None
                            ))
                        }
                    }
                    AstType::Struct { .. } | AstType::Generic { .. } => {
                        inference::infer_member_type(&struct_type, field, &self.structs, &self.enums)
                    }
                    _ => Err(CompileError::TypeError(
                        format!("Cannot access field '{}' on type {:?}", field, struct_type),
                        None
                    ))
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
            Expression::TypeCast { target_type, .. } => {
                Ok(target_type.clone())
            }
            Expression::QuestionMatch { scrutinee, arms } => {
                // QuestionMatch expression type is determined by the first arm
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
                        self.add_pattern_bindings_to_scope_with_type(&arm.pattern, &scrutinee_type)?;
                        
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
            Expression::Return(expr) => {
                self.infer_expression_type(expr)
            }
            Expression::EnumVariant { enum_name, variant, .. } => {
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
                
                // For now, return a generic type with the enum name
                // In the future, this should handle type parameters properly
                if enum_type_name == "Option" || enum_type_name == "Result" {
                    // These are generic types - for now, use a simple representation
                    Ok(AstType::Generic { 
                        name: enum_type_name,
                        type_args: vec![AstType::I32] // Default to I32 for now
                    })
                } else {
                    // Look up the enum to get its variant info
                    if let Some(enum_info) = self.enums.get(&enum_type_name) {
                        let variants = enum_info.variants.iter()
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
            Expression::StringLength(_) => {
                Ok(AstType::I64)
            }
            Expression::MethodCall { object, method, args: _ } => {
                // Implement UFC (Uniform Function Call)
                // Any function can be called as a method: object.function(args) -> function(object, args)
                
                // First check if it's a built-in method on the object type
                let object_type = self.infer_expression_type(object)?;
                
                // Try to find the function in scope
                // The method call object.method(args) becomes method(object, args)
                if let Some(func_type) = self.functions.get(method) {
                    // For UFC, the first parameter should match the object type
                    if !func_type.params.is_empty() {
                        // Return the function's return type
                        return Ok(func_type.return_type.clone());
                    }
                }
                
                // Special handling for string methods
                if object_type == AstType::String {
                    // Common string methods with hardcoded return types for now
                    match method.as_str() {
                        "len" => return Ok(AstType::I64),
                        "to_i32" => return Ok(AstType::Generic { 
                            name: "Option".to_string(), 
                            type_args: vec![AstType::I32] 
                        }),
                        "to_i64" => return Ok(AstType::Generic { 
                            name: "Option".to_string(), 
                            type_args: vec![AstType::I64] 
                        }),
                        "to_f32" => return Ok(AstType::Generic { 
                            name: "Option".to_string(), 
                            type_args: vec![AstType::F32] 
                        }),
                        "to_f64" => return Ok(AstType::Generic { 
                            name: "Option".to_string(), 
                            type_args: vec![AstType::F64] 
                        }),
                        _ => {}
                    }
                }
                
                // Special handling for built-in methods like .loop()
                if method == "loop" {
                    // .loop() on ranges and collections returns void
                    return Ok(AstType::Void);
                }
                
                // Special handling for pointer methods
                match &object_type {
                    AstType::Ptr(_) | AstType::MutPtr(_) | AstType::RawPtr(_) => {
                        if method == "val" {
                            // Dereference pointer
                            return match object_type {
                                AstType::Ptr(inner) | AstType::MutPtr(inner) | AstType::RawPtr(inner) => {
                                    Ok(inner.as_ref().clone())
                                }
                                _ => unreachable!()
                            };
                        } else if method == "addr" {
                            // Get address as usize
                            return Ok(AstType::Usize);
                        }
                    }
                    _ => {}
                }
                
                // Special handling for Result.raise()
                if method == "raise" {
                    match object_type {
                        AstType::Generic { name, type_args } if name == "Result" && !type_args.is_empty() => {
                            return Ok(type_args[0].clone());
                        }
                        AstType::Result { ok_type, .. } => {
                            return Ok(ok_type.as_ref().clone());
                        }
                        _ => {}
                    }
                }
                
                // If no specific handling found, try to resolve as UFC
                // For now, return the object type as a fallback
                // TODO: Implement full UFC resolution with function lookup
                Ok(object_type)
            }
            Expression::Loop { body: _ } => {
                // Loop expressions return void for now
                Ok(AstType::Void)
            }
            Expression::Closure { params, body } => {
                // Infer closure type - create a FunctionPointer type
                let param_types: Vec<AstType> = params.iter().map(|(_, opt_type)| {
                    opt_type.clone().unwrap_or(AstType::I32)
                }).collect();
                
                // TODO: Properly infer return type from body
                // For now, try to infer from the body if it's a simple return
                let return_type = match body.as_ref() {
                    Expression::Block(stmts) => {
                        // Look for return statements in the block
                        for stmt in stmts {
                            if let crate::ast::Statement::Return(ret_expr) = stmt {
                                if let Ok(ret_type) = self.infer_expression_type(ret_expr) {
                                    return Ok(AstType::FunctionPointer {
                                        param_types,
                                        return_type: Box::new(ret_type),
                                    });
                                }
                            }
                        }
                        // If no explicit return, check last expression
                        if let Some(crate::ast::Statement::Expression(last_expr)) = stmts.last() {
                            if let Ok(ret_type) = self.infer_expression_type(last_expr) {
                                Box::new(ret_type)
                            } else {
                                Box::new(AstType::Void)
                            }
                        } else {
                            Box::new(AstType::Void)
                        }
                    }
                    _ => {
                        // Try to infer the return type from the body expression
                        if let Ok(ret_type) = self.infer_expression_type(body) {
                            Box::new(ret_type)
                        } else {
                            Box::new(AstType::I32) // Default to i32
                        }
                    }
                };
                
                Ok(AstType::FunctionPointer {
                    param_types,
                    return_type,
                })
            }
            Expression::Raise(expr) => {
                // .raise() unwraps a Result type and returns the Ok variant
                // If it's an Err, it propagates the error
                let result_type = self.infer_expression_type(expr)?;
                match result_type {
                    // Handle modern generic Result<T, E> type
                    AstType::Generic { name, type_args } if name == "Result" && type_args.len() >= 1 => {
                        // Return the Ok type (first type argument)
                        Ok(type_args[0].clone())
                    },
                    // Handle legacy Result type (still used in some places)
                    AstType::Result { ok_type, .. } => {
                        Ok(*ok_type)
                    },
                    // Type error: .raise() can only be used on Result types
                    _ => {
                        Err(CompileError::TypeError(
                            format!(".raise() can only be used on Result<T, E> types, found: {:?}", result_type),
                            None
                        ))
                    }
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
                    Ok(AstType::Generic {
                        name: "Option".to_string(),
                        type_args: vec![AstType::Void], // Placeholder, will be refined by context
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
                        type_args: vec![ok_type, AstType::String], // Default to String for errors
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
                        type_args: vec![AstType::Void, err_type], // Unknown ok type
                    })
                } else {
                    // Unknown enum literal - would need more context
                    Ok(AstType::Void)
                }
            }
            Expression::Conditional { scrutinee, arms } => {
                eprintln!("DEBUG TypeChecker: Processing conditional with {} arms", arms.len());
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
                        eprintln!("DEBUG TypeChecker: Processing arm {} pattern: {:?}", i, arm.pattern);
                        
                        // Enter a new scope for the pattern bindings
                        self.enter_scope();
                        eprintln!("DEBUG TypeChecker: Entered scope for arm {}", i);
                        
                        // Extract pattern bindings and add them to the scope
                        self.add_pattern_bindings_to_scope_with_type(&arm.pattern, &scrutinee_type)?;
                        eprintln!("DEBUG TypeChecker: Added pattern bindings for arm {}", i);
                        
                        // Infer the type with bindings in scope
                        eprintln!("DEBUG TypeChecker: Inferring type for arm {} body", i);
                        let arm_type = self.infer_expression_type(&arm.body)?;
                        eprintln!("DEBUG TypeChecker: Arm {} type: {:?}", i, arm_type);
                        
                        // The first arm determines the type
                        if i == 0 {
                            result_type = arm_type;
                        }
                        
                        // Exit the scope to remove the bindings
                        self.exit_scope();
                        eprintln!("DEBUG TypeChecker: Exited scope for arm {}", i);
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
            Expression::VecConstructor { element_type, size, initial_values: _ } => {
                // Vec<T, size>() -> Vec<T, size>
                Ok(AstType::Vec {
                    element_type: Box::new(element_type.clone()),
                    size: *size,
                })
            }
            Expression::DynVecConstructor { element_types, allocator: _, initial_capacity: _ } => {
                // DynVec<T>() or DynVec<T1, T2, ...>() -> DynVec<T, ...>
                Ok(AstType::DynVec {
                    element_types: element_types.clone(),
                    allocator_type: None, // Allocator type inferred from constructor arg
                })
            }
            Expression::Some(inner) => {
                eprintln!("DEBUG TypeChecker: Processing Some() with inner expr");
                // Check the inner expression to determine the actual type
                let inner_type = self.infer_expression_type(inner)?;
                eprintln!("DEBUG TypeChecker: Some() inner type: {:?}", inner_type);
                // Option::Some(T) -> Option<T>
                Ok(AstType::Generic {
                    name: "Option".to_string(),
                    type_args: vec![inner_type],
                })
            }
            Expression::None => {
                // Option::None -> Option<T>
                Ok(AstType::Generic {
                    name: "Option".to_string(),
                    type_args: vec![AstType::Generic { name: "T".to_string(), type_args: vec![] }],
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
        }
    }

    fn types_compatible(&self, expected: &AstType, actual: &AstType) -> bool {
        validation::types_compatible(expected, actual)
    }

    fn register_stdlib_module(&mut self, alias: &str, module_path: &str) -> Result<()> {
        // Register functions from known stdlib modules
        match module_path {
            "math" => {
                // Register math constants as global variables
                // We'll treat them as functions that return constants for now
                
                // Register math functions
                let math_funcs = vec![
                    ("sqrt", vec![AstType::F64], AstType::F64),
                    ("sin", vec![AstType::F64], AstType::F64),
                    ("cos", vec![AstType::F64], AstType::F64),
                    ("tan", vec![AstType::F64], AstType::F64),
                    ("pow", vec![AstType::F64, AstType::F64], AstType::F64),
                    ("exp", vec![AstType::F64], AstType::F64),
                    ("log", vec![AstType::F64], AstType::F64),
                    ("floor", vec![AstType::F64], AstType::F64),
                    ("ceil", vec![AstType::F64], AstType::F64),
                    ("round", vec![AstType::F64], AstType::F64),
                    ("abs", vec![AstType::I64], AstType::I64),  // For now, just i64 version
                    ("min", vec![AstType::F64, AstType::F64], AstType::F64),
                    ("max", vec![AstType::F64, AstType::F64], AstType::F64),
                ];
                
                for (name, args, ret) in math_funcs {
                    let qualified_name = format!("{}.{}", alias, name);
                    let params = args.into_iter().enumerate().map(|(i, t)| {
                        (format!("arg{}", i), t)
                    }).collect();
                    self.functions.insert(qualified_name, FunctionSignature {
                        params,
                        return_type: ret,
                        is_external: true,
                    });
                }
            }
            "io" => {
                // Register io functions - names must match what codegen expects
                let io_funcs = vec![
                    ("print", vec![AstType::String], AstType::Void),
                    ("print_int", vec![AstType::I64], AstType::Void),
                    ("print_float", vec![AstType::F64], AstType::Void),
                    ("println", vec![AstType::String], AstType::Void),
                    ("read_line", vec![], AstType::String),
                    ("read_file", vec![AstType::String], AstType::String),
                    ("write_file", vec![AstType::String, AstType::String], AstType::Void),
                ];
                
                for (name, args, ret) in io_funcs {
                    let qualified_name = format!("{}.{}", alias, name);
                    let params = args.into_iter().enumerate().map(|(i, t)| {
                        (format!("arg{}", i), t)
                    }).collect();
                    self.functions.insert(qualified_name, FunctionSignature {
                        params,
                        return_type: ret,
                        is_external: true,
                    });
                }
            }
            "core" => {
                // Register core functions
                let core_funcs = vec![
                    // sizeof and alignof are compile-time operations, skip for now
                    ("assert", vec![AstType::Bool], AstType::Void),
                    ("panic", vec![AstType::String], AstType::Void),
                ];
                
                for (name, args, ret) in core_funcs {
                    let qualified_name = format!("{}.{}", alias, name);
                    let params = args.into_iter().enumerate().map(|(i, t)| {
                        (format!("arg{}", i), t)
                    }).collect();
                    self.functions.insert(qualified_name, FunctionSignature {
                        params,
                        return_type: ret,
                        is_external: true,
                    });
                }
            }
            _ => {
                // Unknown stdlib module, but not an error
            }
        }
        Ok(())
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare_variable(&mut self, name: &str, type_: AstType, is_mutable: bool) -> Result<()> {
        self.declare_variable_with_init(name, type_, is_mutable, true)
    }

    fn declare_variable_with_init(&mut self, name: &str, type_: AstType, is_mutable: bool, is_initialized: bool) -> Result<()> {
        if let Some(scope) = self.scopes.last_mut() {
            if scope.contains_key(name) {
                return Err(CompileError::TypeError(
                    format!("Variable '{}' already declared in this scope", name),
                    None
                ));
            }
            scope.insert(name.to_string(), VariableInfo {
                type_,
                is_mutable,
                is_initialized,
            });
            Ok(())
        } else {
            Err(CompileError::TypeError("No active scope".to_string(), None))
        }
    }

    fn mark_variable_initialized(&mut self, name: &str) -> Result<()> {
        // Search from innermost to outermost scope
        for scope in self.scopes.iter_mut().rev() {
            if let Some(var_info) = scope.get_mut(name) {
                var_info.is_initialized = true;
                return Ok(());
            }
        }
        Err(CompileError::TypeError(
            format!("Undefined variable: {}", name),
            None
        ))
    }

    fn get_variable_type(&self, name: &str) -> Result<AstType> {
        // Search from innermost to outermost scope
        for scope in self.scopes.iter().rev() {
            if let Some(var_info) = scope.get(name) {
                return Ok(var_info.type_.clone());
            }
        }
        
        // Check if it's an enum type
        if self.enums.contains_key(name) {
            // Return a special type to indicate this is an enum type constructor
            return Ok(AstType::EnumType { name: name.to_string() });
        }
        
        Err(CompileError::TypeError(format!("Undefined variable: {}", name), None))
    }
    
    fn get_variable_info(&self, name: &str) -> Result<VariableInfo> {
        // Search from innermost to outermost scope
        for scope in self.scopes.iter().rev() {
            if let Some(var_info) = scope.get(name) {
                return Ok(var_info.clone());
            }
        }
        Err(CompileError::TypeError(format!("Undefined variable: {}", name), None))
    }
    
    fn add_pattern_bindings_to_scope(&mut self, pattern: &crate::ast::Pattern) -> Result<()> {
        // Default to I32 when no type context is available (legacy behavior)
        self.add_pattern_bindings_to_scope_with_type(pattern, &AstType::I32)
    }
    
    fn add_pattern_bindings_to_scope_with_type(&mut self, pattern: &crate::ast::Pattern, scrutinee_type: &AstType) -> Result<()> {
        use crate::ast::Pattern;
        
        eprintln!("DEBUG TypeChecker: add_pattern_bindings_to_scope_with_type for pattern: {:?}, type: {:?}", pattern, scrutinee_type);
        
        match pattern {
            Pattern::Identifier(name) => {
                // Simple identifier pattern binds the name to the type of the matched value
                // For Option<T>, extract T from the generic type
                let binding_type = if let AstType::Generic { name: enum_name, type_args } = scrutinee_type {
                    if enum_name == "Option" && !type_args.is_empty() {
                        // For Option<T>, the payload has type T
                        type_args[0].clone()
                    } else if enum_name == "Result" && type_args.len() >= 2 {
                        // For Result<T,E>, depends on the variant being matched
                        // This is simplified - ideally we'd know which variant we're in
                        type_args[0].clone()
                    } else {
                        // For other generics, default to I32
                        AstType::I32
                    }
                } else {
                    // For non-generic types, use the scrutinee type directly
                    scrutinee_type.clone()
                };
                
                eprintln!("DEBUG TypeChecker: Adding variable '{}' with type {:?} from identifier pattern", name, binding_type);
                self.declare_variable(name, binding_type, false)?;
            }
            Pattern::EnumLiteral { variant, payload } => {
                // For enum patterns with payloads, determine the payload type based on the variant
                if let Some(payload_pattern) = payload {
                    let payload_type = if let AstType::Generic { name: enum_name, type_args } = scrutinee_type {
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
            Pattern::EnumVariant { variant, payload, .. } => {
                // For qualified enum patterns with payloads, determine the payload type based on the variant
                if let Some(payload_pattern) = payload {
                    let payload_type = if let AstType::Generic { name: enum_name, type_args } = scrutinee_type {
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
            Pattern::Wildcard | Pattern::Literal(_) | Pattern::Range { .. } | Pattern::Guard { .. } => {}
        }
        Ok(())
    }
    
    fn variable_exists(&self, name: &str) -> bool {
        // Search from innermost to outermost scope
        for scope in self.scopes.iter().rev() {
            if scope.contains_key(name) {
                return true;
            }
        }
        false
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