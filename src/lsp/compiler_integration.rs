// Compiler Integration Module for Zen LSP
// Provides LSP-friendly APIs for querying types, methods, and symbols from the compiler

use std::collections::HashMap;
use crate::ast::{AstType, Declaration, Program};
use crate::parser::Parser;
use crate::lexer::Lexer;
use crate::typechecker::TypeChecker;
use crate::error::Result;


// ============================================================================
// COMPILER INTEGRATION STRUCT
// ============================================================================

/// Compiler integration for LSP queries
pub struct CompilerIntegration {
    /// Type checker instance for type queries
    type_checker: TypeChecker,
    /// Parsed stdlib programs indexed by path
    stdlib_programs: HashMap<String, Program>,
    /// Function signatures from stdlib
    stdlib_functions: HashMap<String, FunctionSignature>,
}

#[derive(Clone, Debug)]
pub struct FunctionSignature {
    pub name: String,
    pub params: Vec<(String, AstType)>,
    pub return_type: AstType,
    pub receiver_type: Option<String>, // For UFC methods: the receiver type
}

impl CompilerIntegration {
    /// Create a new compiler integration instance
    pub fn new() -> Self {
        Self {
            type_checker: TypeChecker::new(),
            stdlib_programs: HashMap::new(),
            stdlib_functions: HashMap::new(),
        }
    }

    /// Load and index a stdlib file
    pub fn load_stdlib_file(&mut self, path: &str, content: &str) -> Result<()> {
        let lexer = Lexer::new(content);
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program()?;
        
        // Index functions from this stdlib file
        for decl in &program.declarations {
            if let Declaration::Function(func) = decl {
                // Check if this is a UFC method (first param matches receiver type)
                let receiver_type = if !func.args.is_empty() {
                    // Check if first parameter type matches the function name pattern
                    // This is a heuristic - in real stdlib, methods are defined with receiver as first param
                    let first_param_type = format_type(&func.args[0].1);
                    Some(first_param_type)
                } else {
                    None
                };

                let sig = FunctionSignature {
                    name: func.name.clone(),
                    params: func.args.clone(),
                    return_type: func.return_type.clone(),
                    receiver_type: receiver_type.clone(),
                };
                
                // Store with full name and also with receiver type prefix for method lookup
                self.stdlib_functions.insert(func.name.clone(), sig.clone());
                if let Some(recv) = &receiver_type {
                    let method_key = format!("{}::{}", recv, func.name);
                    self.stdlib_functions.insert(method_key, sig);
                }
            }
        }
        
        self.stdlib_programs.insert(path.to_string(), program);
        Ok(())
    }

    /// Get all methods available for a given receiver type
    pub fn get_methods_for_type(&self, receiver_type: &str) -> Vec<&FunctionSignature> {
        let mut methods = Vec::new();
        
        // Look for methods with this receiver type
        for (key, sig) in &self.stdlib_functions {
            if let Some(recv) = &sig.receiver_type {
                if recv == receiver_type || 
                   recv.starts_with(&format!("{}<", receiver_type)) ||
                   key.starts_with(&format!("{}::", receiver_type)) {
                    methods.push(sig);
                }
            }
        }
        
        methods
    }

    /// Get the return type of a method call
    pub fn get_method_return_type(&self, receiver_type: &str, method_name: &str) -> Option<AstType> {
        // Try direct lookup first
        let method_key = format!("{}::{}", receiver_type, method_name);
        if let Some(sig) = self.stdlib_functions.get(&method_key) {
            return Some(sig.return_type.clone());
        }
        
        // Try looking up by function name and check if receiver matches
        if let Some(sig) = self.stdlib_functions.get(method_name) {
            if let Some(recv) = &sig.receiver_type {
                if recv == receiver_type || 
                   recv.starts_with(&format!("{}<", receiver_type)) {
                    return Some(sig.return_type.clone());
                }
            }
        }
        
        None
    }

    /// Parse a type string using the parser
    pub fn parse_type_string(&self, type_str: &str) -> Result<AstType> {
        // Create a minimal parser context for parsing just the type
        let lexer = Lexer::new(type_str);
        let mut parser = Parser::new(lexer);
        parser.parse_type()
    }

    /// Infer the type of an expression using TypeChecker
    pub fn infer_expression_type(&mut self, program: &Program, expr_str: &str) -> Result<AstType> {
        // Parse the expression
        let lexer = Lexer::new(expr_str);
        let mut parser = Parser::new(lexer);
        let _expr = parser.parse_expression()?;
        
        // Use type checker to infer type
        // Note: This requires a full program context, so we need to check the expression
        // in the context of the program
        self.type_checker.check_program(program)?;
        
        // For now, return a basic inference
        // TODO: Implement proper expression type inference
        // Use Void as placeholder - proper inference requires full context
        Ok(AstType::Void)
    }

    /// Get function signature by name
    pub fn get_function_signature(&self, name: &str) -> Option<&FunctionSignature> {
        self.stdlib_functions.get(name)
    }

    /// Get all function signatures
    pub fn get_all_functions(&self) -> &HashMap<String, FunctionSignature> {
        &self.stdlib_functions
    }

    /// Check if a type has a method
    pub fn has_method(&self, receiver_type: &str, method_name: &str) -> bool {
        self.get_method_return_type(receiver_type, method_name).is_some()
    }
}

// Helper function to format AstType as string
fn format_type(ty: &AstType) -> String {
    match ty {
        AstType::I32 => "i32".to_string(),
        AstType::I64 => "i64".to_string(),
        AstType::F32 => "f32".to_string(),
        AstType::F64 => "f64".to_string(),
        AstType::Bool => "bool".to_string(),
        AstType::String => "String".to_string(),
        AstType::StaticString => "StaticString".to_string(),
        AstType::Void => "void".to_string(),
        AstType::Generic { name, type_args } => {
            if type_args.is_empty() {
                name.clone()
            } else {
                let args: Vec<String> = type_args.iter().map(format_type).collect();
                format!("{}<{}>", name, args.join(", "))
            }
        }
        _ => format!("{:?}", ty),
    }
}

