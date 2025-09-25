use crate::ast::{
    AstType, BehaviorDefinition, TraitDefinition, TraitImplementation, TraitRequirement,
};
use crate::error::{CompileError, Result};
use std::collections::HashMap;

/// Tracks behaviors, implementations, and provides trait resolution
#[allow(dead_code)]
pub struct BehaviorResolver {
    /// All defined behaviors/traits
    behaviors: HashMap<String, BehaviorInfo>,
    /// Maps (type_name, behavior_name) -> implementation
    implementations: HashMap<(String, String), ImplInfo>,
    /// Maps type_name -> inherent methods (impl blocks without behavior)
    inherent_methods: HashMap<String, Vec<MethodInfo>>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct BehaviorInfo {
    pub name: String,
    pub type_params: Vec<String>,
    pub methods: Vec<BehaviorMethodInfo>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct BehaviorMethodInfo {
    pub name: String,
    pub param_types: Vec<AstType>,
    pub return_type: AstType,
    pub has_self: bool,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ImplInfo {
    pub type_name: String,
    pub trait_name: String,
    pub type_params: Vec<String>,
    pub methods: HashMap<String, MethodInfo>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct MethodInfo {
    pub name: String,
    pub param_types: Vec<AstType>,
    pub return_type: AstType,
}

impl BehaviorResolver {
    pub fn new() -> Self {
        Self {
            behaviors: HashMap::new(),
            implementations: HashMap::new(),
            inherent_methods: HashMap::new(),
        }
    }

    /// Register a behavior definition
    pub fn register_behavior(&mut self, behavior: &BehaviorDefinition) -> Result<()> {
        if self.behaviors.contains_key(&behavior.name) {
            return Err(CompileError::TypeError(
                format!("Behavior '{}' already defined", behavior.name),
                None,
            ));
        }

        let methods = behavior
            .methods
            .iter()
            .map(|m| {
                let has_self = m.params.first().map(|p| p.name == "self").unwrap_or(false);

                BehaviorMethodInfo {
                    name: m.name.clone(),
                    param_types: m.params.iter().map(|p| p.type_.clone()).collect(),
                    return_type: m.return_type.clone(),
                    has_self,
                }
            })
            .collect();

        let info = BehaviorInfo {
            name: behavior.name.clone(),
            type_params: behavior
                .type_params
                .iter()
                .map(|tp| tp.name.clone())
                .collect(),
            methods,
        };

        self.behaviors.insert(behavior.name.clone(), info);
        Ok(())
    }

    /// Register a trait definition (using same storage as behaviors)
    pub fn register_trait(&mut self, trait_def: &TraitDefinition) -> Result<()> {
        if self.behaviors.contains_key(&trait_def.name) {
            return Err(CompileError::TypeError(
                format!("Trait '{}' already defined", trait_def.name),
                None,
            ));
        }

        let methods = trait_def
            .methods
            .iter()
            .map(|m| {
                let has_self = m.params.first().map(|p| p.name == "self").unwrap_or(false);

                BehaviorMethodInfo {
                    name: m.name.clone(),
                    param_types: m.params.iter().map(|p| p.type_.clone()).collect(),
                    return_type: m.return_type.clone(),
                    has_self,
                }
            })
            .collect();

        let info = BehaviorInfo {
            name: trait_def.name.clone(),
            type_params: trait_def
                .type_params
                .iter()
                .map(|tp| tp.name.clone())
                .collect(),
            methods,
        };

        self.behaviors.insert(trait_def.name.clone(), info);
        Ok(())
    }

    /// Replace Self type with concrete type in AST type
    fn replace_self_type(&self, ast_type: &AstType, concrete_type: &str) -> AstType {
        match ast_type {
            AstType::Generic { name, type_args: _ } if name == "Self" => {
                // Replace Self with the concrete type - use Struct with just name
                AstType::Struct {
                    name: concrete_type.to_string(),
                    fields: vec![], // Empty fields - this is just a type reference
                }
            }
            AstType::Ptr(inner) => {
                AstType::Ptr(Box::new(self.replace_self_type(inner, concrete_type)))
            }
            AstType::MutPtr(inner) => {
                AstType::MutPtr(Box::new(self.replace_self_type(inner, concrete_type)))
            }
            AstType::RawPtr(inner) => {
                AstType::RawPtr(Box::new(self.replace_self_type(inner, concrete_type)))
            }
            AstType::Option(inner) => {
                AstType::Option(Box::new(self.replace_self_type(inner, concrete_type)))
            }
            AstType::Result { ok_type, err_type } => AstType::Result {
                ok_type: Box::new(self.replace_self_type(ok_type, concrete_type)),
                err_type: Box::new(self.replace_self_type(err_type, concrete_type)),
            },
            AstType::Array(element) => {
                AstType::Array(Box::new(self.replace_self_type(element, concrete_type)))
            }
            AstType::FixedArray { element_type, size } => AstType::FixedArray {
                element_type: Box::new(self.replace_self_type(element_type, concrete_type)),
                size: *size,
            },
            AstType::Function { args, return_type } => AstType::Function {
                args: args
                    .iter()
                    .map(|p| self.replace_self_type(p, concrete_type))
                    .collect(),
                return_type: Box::new(self.replace_self_type(return_type, concrete_type)),
            },
            // For other types, return as is
            _ => ast_type.clone(),
        }
    }

    /// Register an implementation block
    pub fn register_trait_implementation(
        &mut self,
        trait_impl: &TraitImplementation,
    ) -> Result<()> {
        // Register a trait implementation for a type
        let key = (trait_impl.type_name.clone(), trait_impl.trait_name.clone());

        if self.implementations.contains_key(&key) {
            return Err(CompileError::TypeError(
                format!(
                    "Trait '{}' already implemented for type '{}'",
                    trait_impl.trait_name, trait_impl.type_name
                ),
                None,
            ));
        }

        // Verify that the trait exists
        if !self.behaviors.contains_key(&trait_impl.trait_name) {
            return Err(CompileError::TypeError(
                format!("Unknown trait: {}", trait_impl.trait_name),
                None,
            ));
        }

        let mut methods = HashMap::new();
        for method in &trait_impl.methods {
            // Replace Self types in parameters and return type with the concrete type
            let param_types: Vec<AstType> = method
                .args
                .iter()
                .map(|(_, t)| self.replace_self_type(t, &trait_impl.type_name))
                .collect();
            let return_type = self.replace_self_type(&method.return_type, &trait_impl.type_name);

            let method_info = MethodInfo {
                name: method.name.clone(),
                param_types,
                return_type,
            };
            methods.insert(method.name.clone(), method_info);
        }

        let impl_info = ImplInfo {
            type_name: trait_impl.type_name.clone(),
            trait_name: trait_impl.trait_name.clone(),
            type_params: trait_impl
                .type_params
                .iter()
                .map(|p| p.name.clone())
                .collect(),
            methods,
        };

        self.implementations.insert(key, impl_info);
        Ok(())
    }

    pub fn register_trait_requirement(&mut self, trait_req: &TraitRequirement) -> Result<()> {
        // Register that a type requires a trait
        // This is mainly for enum variants that must all implement a trait
        // For now, we just verify the trait exists
        if !self.behaviors.contains_key(&trait_req.trait_name) {
            return Err(CompileError::TypeError(
                format!("Unknown trait: {}", trait_req.trait_name),
                None,
            ));
        }
        // TODO: Store requirements and verify them when checking enum variants
        Ok(())
    }

    pub fn verify_trait_implementation(&mut self, trait_impl: &TraitImplementation) -> Result<()> {
        // Verify that the implementation satisfies the trait
        if let Some(behavior) = self.behaviors.get(&trait_impl.trait_name) {
            // Check that all required methods are implemented
            for required_method in &behavior.methods {
                let mut found = false;
                for impl_method in &trait_impl.methods {
                    if impl_method.name == required_method.name {
                        // TODO: Also check that the method signatures match
                        found = true;
                        break;
                    }
                }
                if !found {
                    return Err(CompileError::TypeError(
                        format!(
                            "Missing required method '{}' in implementation of trait '{}' for type '{}'",
                            required_method.name, trait_impl.trait_name, trait_impl.type_name
                        ),
                        None,
                    ));
                }
            }
            Ok(())
        } else {
            Err(CompileError::TypeError(
                format!("Unknown trait: {}", trait_impl.trait_name),
                None,
            ))
        }
    }

    pub fn verify_trait_requirement(&mut self, trait_req: &TraitRequirement) -> Result<()> {
        // Verify that a trait requirement is valid
        if !self.behaviors.contains_key(&trait_req.trait_name) {
            return Err(CompileError::TypeError(
                format!("Unknown trait: {}", trait_req.trait_name),
                None,
            ));
        }
        Ok(())
    }

    /// Check if a type implements a trait
    pub fn type_implements(&self, type_name: &str, trait_name: &str) -> bool {
        self.implementations
            .contains_key(&(type_name.to_string(), trait_name.to_string()))
    }

    /// Get the implementation of a behavior for a type
    pub fn get_impl(&self, type_name: &str, behavior_name: &str) -> Option<&ImplInfo> {
        self.implementations
            .get(&(type_name.to_string(), behavior_name.to_string()))
    }

    /// Resolve a method call on a type
    pub fn resolve_method(&self, type_name: &str, method_name: &str) -> Option<MethodInfo> {
        // First check inherent methods
        if let Some(methods) = self.inherent_methods.get(type_name) {
            if let Some(method) = methods.iter().find(|m| m.name == method_name) {
                return Some(method.clone());
            }
        }

        // Then check behavior implementations
        for ((impl_type, _), impl_info) in &self.implementations {
            if impl_type == type_name {
                if let Some(method) = impl_info.methods.get(method_name) {
                    return Some(method.clone());
                }
            }
        }

        None
    }

    /// Get all behaviors implemented by a type
    pub fn get_implemented_behaviors(&self, type_name: &str) -> Vec<String> {
        self.implementations
            .keys()
            .filter(|(t, _)| t == type_name)
            .map(|(_, b)| b.clone())
            .collect()
    }
}

// TODO: Update tests to use new TraitImplementation and TraitRequirement
/*
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{BehaviorMethod, Function, Parameter};

    #[test]
    fn test_register_behavior() {
        let mut resolver = BehaviorResolver::new();

        let behavior = BehaviorDefinition {
            name: "Display".to_string(),
            type_params: vec![],
            methods: vec![
                BehaviorMethod {
                    name: "display".to_string(),
                    params: vec![
                        Parameter {
                            name: "self".to_string(),
                            type_: AstType::Ptr(Box::new(AstType::Generic {
                                name: "Self".to_string(),
                                type_args: vec![],
                            })),
                            is_mutable: false,
                        }
                    ],
                    return_type: AstType::String,
                }
            ],
        };

        assert!(resolver.register_behavior(&behavior).is_ok());
        assert!(resolver.behaviors.contains_key("Display"));
    }

    #[test]
    fn test_register_impl() {
        let mut resolver = BehaviorResolver::new();

        // First register the behavior
        let behavior = BehaviorDefinition {
            name: "Display".to_string(),
            type_params: vec![],
            methods: vec![],
        };
        resolver.register_behavior(&behavior).unwrap();

        // Then register an implementation
        let impl_block = ImplBlock {
            type_name: "Point".to_string(),
            behavior_name: Some("Display".to_string()),
            type_params: vec![],
            methods: vec![],
        };

        assert!(resolver.register_impl(&impl_block).is_ok());
        assert!(resolver.type_implements("Point", "Display"));
    }

    #[test]
    fn test_method_resolution() {
        let mut resolver = BehaviorResolver::new();

        // Register an inherent impl
        let impl_block = ImplBlock {
            type_name: "Point".to_string(),
            behavior_name: None,
            type_params: vec![],
            methods: vec![
                Function {
                    name: "distance".to_string(),
                    args: vec![("self".to_string(), AstType::Ptr(Box::new(AstType::Struct {
                        name: "Point".to_string(),
                        fields: vec![],
                    })))],
                    return_type: AstType::F64,
                    body: vec![],
                    type_params: vec![],
                }
            ],
        };

        resolver.register_impl(&impl_block).unwrap();

        let method = resolver.resolve_method("Point", "distance");
        assert!(method.is_some());
        assert_eq!(method.unwrap().name, "distance");
    }
}
*/
