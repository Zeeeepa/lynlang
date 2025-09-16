// Behaviors System for Zen Language
// Implements structural contracts as per Language Spec v1.1.0
// Behaviors are structs containing function pointers

use std::collections::HashMap;
use std::sync::Arc;
use crate::ast::{AstType, Expression};
use crate::error::{CompileError, Result};

/// Represents a behavior definition - a struct of function pointers
#[derive(Clone, Debug)]
pub struct Behavior {
    pub name: String,
    pub type_params: Vec<String>,
    pub methods: HashMap<String, BehaviorMethod>,
}

/// A method in a behavior
#[derive(Clone)]
pub struct BehaviorMethod {
    pub name: String,
    pub signature: FnSignature,
    pub implementation: Option<Arc<dyn Fn(&[Expression]) -> Expression + Send + Sync>>,
}

impl std::fmt::Debug for BehaviorMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BehaviorMethod")
            .field("name", &self.name)
            .field("signature", &self.signature)
            .field("implementation", &self.implementation.as_ref().map(|_| "<function>"))
            .finish()
    }
}

/// Function signature for behavior methods
#[derive(Clone, Debug, PartialEq)]
pub struct FnSignature {
    pub params: Vec<AstType>,
    pub return_type: AstType,
}

/// Behavior instance for a specific type
#[derive(Clone)]
pub struct BehaviorInstance<T> {
    pub behavior_name: String,
    pub type_name: String,
    pub methods: HashMap<String, Arc<dyn Fn(&T) -> Expression + Send + Sync>>,
}

/// Registry for behaviors and their implementations
pub struct BehaviorRegistry {
    /// Registered behaviors
    behaviors: HashMap<String, Behavior>,
    /// Implementations: (TypeName, BehaviorName) -> Implementation
    implementations: HashMap<(String, String), BehaviorImplementation>,
    /// Derived behaviors: TypeName -> Vec<BehaviorName>
    derived: HashMap<String, Vec<String>>,
}

/// Implementation of a behavior for a type
#[derive(Clone)]
pub struct BehaviorImplementation {
    pub type_name: String,
    pub behavior_name: String,
    pub methods: HashMap<String, Arc<dyn Fn(&[Expression]) -> Expression + Send + Sync>>,
}

impl BehaviorRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            behaviors: HashMap::new(),
            implementations: HashMap::new(),
            derived: HashMap::new(),
        };
        
        // Register built-in behaviors
        registry.register_builtin_behaviors();
        registry
    }
    
    /// Register built-in behaviors like Comparable, Hashable, Serializable
    fn register_builtin_behaviors(&mut self) {
        // Comparable<T>
        self.register_behavior(Behavior {
            name: "Comparable".to_string(),
            type_params: vec!["T".to_string()],
            methods: {
                let mut methods = HashMap::new();
                methods.insert("compare".to_string(), BehaviorMethod {
                    name: "compare".to_string(),
                    signature: FnSignature {
                        params: vec![
                            AstType::Struct { name: "T".to_string(), fields: vec![] },
                            AstType::Struct { name: "T".to_string(), fields: vec![] },
                        ],
                        return_type: AstType::I32,
                    },
                    implementation: None,
                });
                methods
            },
        });
        
        // Hashable<T>
        self.register_behavior(Behavior {
            name: "Hashable".to_string(),
            type_params: vec!["T".to_string()],
            methods: {
                let mut methods = HashMap::new();
                methods.insert("hash".to_string(), BehaviorMethod {
                    name: "hash".to_string(),
                    signature: FnSignature {
                        params: vec![AstType::Struct { name: "T".to_string(), fields: vec![] }],
                        return_type: AstType::U64,
                    },
                    implementation: None,
                });
                methods
            },
        });
        
        // Serializable<T>
        self.register_behavior(Behavior {
            name: "Serializable".to_string(),
            type_params: vec!["T".to_string()],
            methods: {
                let mut methods = HashMap::new();
                methods.insert("serialize".to_string(), BehaviorMethod {
                    name: "serialize".to_string(),
                    signature: FnSignature {
                        params: vec![
                            AstType::Struct { name: "T".to_string(), fields: vec![] },
                            AstType::Ptr(Box::new(AstType::Struct { name: "Writer".to_string(), fields: vec![] })),
                        ],
                        return_type: AstType::Struct { name: "Result".to_string(), fields: vec![] },
                    },
                    implementation: None,
                });
                methods.insert("deserialize".to_string(), BehaviorMethod {
                    name: "deserialize".to_string(),
                    signature: FnSignature {
                        params: vec![
                            AstType::Ptr(Box::new(AstType::Struct { name: "Reader".to_string(), fields: vec![] })),
                        ],
                        return_type: AstType::Struct { name: "Result".to_string(), fields: vec![] },
                    },
                    implementation: None,
                });
                methods
            },
        });
    }
    
    /// Register a new behavior
    pub fn register_behavior(&mut self, behavior: Behavior) {
        self.behaviors.insert(behavior.name.clone(), behavior);
    }
    
    /// Provide an implementation of a behavior for a type
    pub fn provide_implementation(
        &mut self,
        type_name: String,
        behavior_name: String,
        methods: HashMap<String, Arc<dyn Fn(&[Expression]) -> Expression + Send + Sync>>,
    ) -> Result<()> {
        // Verify behavior exists
        let behavior = self.behaviors.get(&behavior_name)
            .ok_or_else(|| CompileError::TypeError(
                format!("Unknown behavior: {}", behavior_name),
                None
            ))?;
        
        // Verify all required methods are provided
        for method_name in behavior.methods.keys() {
            if !methods.contains_key(method_name) {
                return Err(CompileError::TypeError(
                    format!("Missing implementation for method '{}' in behavior '{}'", 
                            method_name, behavior_name),
                    None
                ));
            }
        }
        
        let implementation = BehaviorImplementation {
            type_name: type_name.clone(),
            behavior_name: behavior_name.clone(),
            methods,
        };
        
        self.implementations.insert(
            (type_name, behavior_name),
            implementation
        );
        
        Ok(())
    }
    
    /// Get implementation of a behavior for a type
    pub fn get_implementation(&self, type_name: &str, behavior_name: &str) 
        -> Option<&BehaviorImplementation> 
    {
        self.implementations.get(&(type_name.to_string(), behavior_name.to_string()))
    }
    
    /// Check if a type implements a behavior
    pub fn implements(&self, type_name: &str, behavior_name: &str) -> bool {
        self.implementations.contains_key(&(type_name.to_string(), behavior_name.to_string()))
            || self.is_derived(type_name, behavior_name)
    }
    
    /// Check if a behavior is auto-derived for a type
    fn is_derived(&self, type_name: &str, behavior_name: &str) -> bool {
        self.derived.get(type_name)
            .map(|behaviors| behaviors.contains(&behavior_name.to_string()))
            .unwrap_or(false)
    }
    
    /// Auto-derive common behaviors for a type
    pub fn auto_derive(&mut self, type_name: String, behaviors: Vec<String>) -> Result<()> {
        for behavior_name in &behaviors {
            match behavior_name.as_str() {
                "Comparable" => self.derive_comparable(&type_name)?,
                "Hashable" => self.derive_hashable(&type_name)?,
                _ => {
                    return Err(CompileError::TypeError(
                        format!("Cannot auto-derive behavior: {}", behavior_name),
                        None
                    ));
                }
            }
        }
        
        self.derived.entry(type_name)
            .or_insert_with(Vec::new)
            .extend(behaviors);
        
        Ok(())
    }
    
    /// Auto-derive Comparable for a type
    fn derive_comparable(&mut self, type_name: &str) -> Result<()> {
        // Generate comparison function based on type structure
        let compare_fn = Arc::new(move |_args: &[Expression]| -> Expression {
            // Basic lexicographic comparison for structs
            // This would be expanded based on actual type structure
            Expression::Integer32(0)
        });
        
        let mut methods = HashMap::new();
        methods.insert("compare".to_string(), compare_fn as Arc<dyn Fn(&[Expression]) -> Expression + Send + Sync>);
        
        self.provide_implementation(
            type_name.to_string(),
            "Comparable".to_string(),
            methods
        )?;
        
        Ok(())
    }
    
    /// Auto-derive Hashable for a type
    fn derive_hashable(&mut self, type_name: &str) -> Result<()> {
        // Generate hash function based on type structure
        let hash_fn = Arc::new(move |_args: &[Expression]| -> Expression {
            // Basic hash combining for structs
            // This would be expanded based on actual type structure
            Expression::Integer64(0)
        });
        
        let mut methods = HashMap::new();
        methods.insert("hash".to_string(), hash_fn as Arc<dyn Fn(&[Expression]) -> Expression + Send + Sync>);
        
        self.provide_implementation(
            type_name.to_string(),
            "Hashable".to_string(),
            methods
        )?;
        
        Ok(())
    }
}

/// Helper to create behavior instances for generic functions
pub fn create_behavior_instance<T>(
    _behavior_name: &str,
    compare_fn: impl Fn(&T, &T) -> i32 + Send + Sync + 'static,
) -> Arc<dyn Fn(&T, &T) -> i32 + Send + Sync> {
    Arc::new(compare_fn)
}

/// Example usage of behaviors in generic functions
pub fn sort_with_behavior<T>(
    items: &mut [T],
    comparable: Arc<dyn Fn(&T, &T) -> i32 + Send + Sync>,
) {
    items.sort_by(|a, b| {
        match comparable(a, b) {
            x if x < 0 => std::cmp::Ordering::Less,
            x if x > 0 => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_behavior_registration() {
        let mut registry = BehaviorRegistry::new();
        
        // Check built-in behaviors are registered
        assert!(registry.behaviors.contains_key("Comparable"));
        assert!(registry.behaviors.contains_key("Hashable"));
        assert!(registry.behaviors.contains_key("Serializable"));
    }
    
    #[test]
    fn test_behavior_implementation() {
        let mut registry = BehaviorRegistry::new();
        
        // Provide implementation for i32
        let mut methods = HashMap::new();
        methods.insert("compare".to_string(), Arc::new(|args: &[Expression]| {
            Expression::Integer32(0)
        }) as Arc<dyn Fn(&[Expression]) -> Expression + Send + Sync>);
        
        registry.provide_implementation(
            "i32".to_string(),
            "Comparable".to_string(),
            methods
        ).unwrap();
        
        assert!(registry.implements("i32", "Comparable"));
    }
    
    #[test]
    fn test_auto_derive() {
        let mut registry = BehaviorRegistry::new();
        
        registry.auto_derive(
            "Point".to_string(),
            vec!["Comparable".to_string(), "Hashable".to_string()]
        ).unwrap();
        
        assert!(registry.implements("Point", "Comparable"));
        assert!(registry.implements("Point", "Hashable"));
    }
}