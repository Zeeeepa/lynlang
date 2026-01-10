//! TypeContext - Shared type information between compiler phases
//!
//! This module provides a bridge between the typechecker and codegen,
//! eliminating the need for duplicate type inference code.
//!
//! Pipeline: Parser → Typechecker → TypeContext → Codegen
//!
//! The typechecker populates TypeContext with:
//! - Function signatures (params, return types)
//! - Struct definitions (fields and their types)
//! - Enum definitions (variants and payloads)
//! - Method signatures from impl blocks
//!
//! Codegen consumes TypeContext instead of re-inferring types.

use crate::ast::AstType;
use std::collections::HashMap;

/// Shared type context that flows from typechecker to codegen
#[derive(Debug, Clone, Default)]
pub struct TypeContext {
    /// Function signatures: name -> (params, return_type)
    pub functions: HashMap<String, FunctionType>,

    /// Struct definitions: name -> fields
    pub structs: HashMap<String, Vec<(String, AstType)>>,

    /// Enum definitions: name -> variants
    pub enums: HashMap<String, Vec<(String, Option<AstType>)>>,

    /// Method signatures: "TypeName.method" -> return_type
    pub methods: HashMap<String, AstType>,
}

#[derive(Debug, Clone)]
pub struct FunctionType {
    pub params: Vec<(String, AstType)>,
    pub return_type: AstType,
    pub is_external: bool,
}

impl TypeContext {
    pub fn new() -> Self {
        Self::default()
    }

    // ========================================================================
    // Registration methods (called by typechecker)
    // ========================================================================

    pub fn register_function(&mut self, name: String, params: Vec<(String, AstType)>, return_type: AstType, is_external: bool) {
        self.functions.insert(name, FunctionType { params, return_type, is_external });
    }

    pub fn register_struct(&mut self, name: String, fields: Vec<(String, AstType)>) {
        self.structs.insert(name, fields);
    }

    pub fn register_enum(&mut self, name: String, variants: Vec<(String, Option<AstType>)>) {
        self.enums.insert(name, variants);
    }

    pub fn register_method(&mut self, type_name: &str, method_name: &str, return_type: AstType) {
        let key = format!("{}.{}", type_name, method_name);
        self.methods.insert(key, return_type);
    }

    // ========================================================================
    // Query methods (called by codegen)
    // ========================================================================

    pub fn get_function_return_type(&self, name: &str) -> Option<AstType> {
        self.functions.get(name).map(|f| f.return_type.clone())
    }

    pub fn get_struct_fields(&self, name: &str) -> Option<&Vec<(String, AstType)>> {
        self.structs.get(name)
    }

    pub fn get_struct_field_type(&self, struct_name: &str, field_name: &str) -> Option<AstType> {
        self.structs.get(struct_name)
            .and_then(|fields| fields.iter().find(|(n, _)| n == field_name))
            .map(|(_, t)| t.clone())
    }

    pub fn get_enum_variants(&self, name: &str) -> Option<&Vec<(String, Option<AstType>)>> {
        self.enums.get(name)
    }

    pub fn get_method_return_type(&self, type_name: &str, method_name: &str) -> Option<AstType> {
        let key = format!("{}.{}", type_name, method_name);
        self.methods.get(&key).cloned()
    }

    pub fn has_struct(&self, name: &str) -> bool {
        self.structs.contains_key(name)
    }

    pub fn has_function(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }
}
