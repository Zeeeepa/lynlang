use crate::ast::AstType;
use std::collections::HashMap;

/// Tracks generic type instantiations at different scopes
/// This allows nested generics like Result<Option<T>, Vec<E>>
#[derive(Debug, Clone)]
pub struct GenericTypeTracker {
    /// Stack of generic contexts, with the top being the current scope
    contexts: Vec<HashMap<String, AstType>>,
}

impl GenericTypeTracker {
    pub fn new() -> Self {
        Self {
            contexts: vec![HashMap::new()],
        }
    }
    
    /// Push a new generic context scope
    pub fn push_scope(&mut self) {
        self.contexts.push(HashMap::new());
    }
    
    /// Pop the current generic context scope
    pub fn pop_scope(&mut self) {
        if self.contexts.len() > 1 {
            self.contexts.pop();
        }
    }
    
    /// Insert a type mapping in the current scope
    pub fn insert(&mut self, key: String, type_: AstType) {
        if let Some(current) = self.contexts.last_mut() {
            current.insert(key, type_);
        }
    }
    
    /// Get a type mapping, searching from current scope upwards
    pub fn get(&self, key: &str) -> Option<&AstType> {
        for context in self.contexts.iter().rev() {
            if let Some(type_) = context.get(key) {
                return Some(type_);
            }
        }
        None
    }
    
    /// Create a specialized key for nested generics
    /// e.g., "Result<Option<T>,E>" -> "Result_0_Option_0_T"
    pub fn create_nested_key(base_type: &str, type_path: &[usize]) -> String {
        let mut key = base_type.to_string();
        for (i, &index) in type_path.iter().enumerate() {
            key.push_str(&format!("_{}", index));
            if i < type_path.len() - 1 {
                key.push('_');
            }
        }
        key
    }
    
    /// Extract and track generic types from a complex type
    pub fn track_generic_type(&mut self, type_: &AstType, prefix: &str) {
        match type_ {
            AstType::Generic { name, type_args } => {
                // Track the main generic type
                self.insert(format!("{}_type", prefix), type_.clone());
                
                // Track the generic name for easy lookup
                self.insert(format!("{}_name", prefix), AstType::EnumType { name: name.clone() });
                
                // Track each type argument recursively
                if name == "Result" && type_args.len() == 2 {
                    self.insert(format!("{}_Ok_Type", prefix), type_args[0].clone());
                    self.insert(format!("{}_Err_Type", prefix), type_args[1].clone());
                    
                    // Recursively track nested generics
                    self.track_generic_type(&type_args[0], &format!("{}_Ok", prefix));
                    self.track_generic_type(&type_args[1], &format!("{}_Err", prefix));
                } else if name == "Option" && type_args.len() == 1 {
                    self.insert(format!("{}_Some_Type", prefix), type_args[0].clone());
                    
                    // Recursively track nested generics
                    self.track_generic_type(&type_args[0], &format!("{}_Some", prefix));
                } else if name == "Array" && type_args.len() == 1 {
                    self.insert(format!("{}_Element_Type", prefix), type_args[0].clone());
                    
                    // Recursively track nested generics
                    self.track_generic_type(&type_args[0], &format!("{}_Element", prefix));
                } else if name == "Vec" && type_args.len() == 1 {
                    self.insert(format!("{}_Element_Type", prefix), type_args[0].clone());
                    
                    // Recursively track nested generics
                    self.track_generic_type(&type_args[0], &format!("{}_Element", prefix));
                } else if name == "HashMap" && type_args.len() == 2 {
                    self.insert(format!("{}_Key_Type", prefix), type_args[0].clone());
                    self.insert(format!("{}_Value_Type", prefix), type_args[1].clone());
                    
                    // Recursively track nested generics
                    self.track_generic_type(&type_args[0], &format!("{}_Key", prefix));
                    self.track_generic_type(&type_args[1], &format!("{}_Value", prefix));
                } else if name == "HashSet" && type_args.len() == 1 {
                    self.insert(format!("{}_Element_Type", prefix), type_args[0].clone());
                    
                    // Recursively track nested generics
                    self.track_generic_type(&type_args[0], &format!("{}_Element", prefix));
                } else {
                    // For other generic types, track type arguments by index
                    for (i, arg) in type_args.iter().enumerate() {
                        self.insert(format!("{}_arg{}_Type", prefix, i), arg.clone());
                        self.track_generic_type(arg, &format!("{}_arg{}", prefix, i));
                    }
                }
            }
            AstType::EnumType { name: _ } | AstType::Enum { name: _, .. } => {
                // Track custom enum types specially
                self.insert(format!("{}_type", prefix), type_.clone());
                self.insert(format!("{}_enum_name", prefix), type_.clone());
            }
            _ => {
                // For non-generic types, just track them directly
                self.insert(format!("{}_type", prefix), type_.clone());
            }
        }
    }
    
    /// Get the current context (for debugging)
    pub fn current_context(&self) -> Option<&HashMap<String, AstType>> {
        self.contexts.last()
    }
    
    /// Merge a temporary context into the current one
    pub fn merge_context(&mut self, other: HashMap<String, AstType>) {
        if let Some(current) = self.contexts.last_mut() {
            for (k, v) in other {
                current.insert(k, v);
            }
        }
    }
    
    /// Instantiate a generic type with concrete type arguments
    /// This is the core of monomorphization - creating concrete types from generics
    pub fn instantiate_generic(
        &mut self, 
        generic_name: &str,
        type_args: &[AstType],
    ) -> String {
        // Create a unique key for this instantiation
        let mut key = generic_name.to_string();
        for arg in type_args {
            key.push('_');
            key.push_str(&self.type_to_string(arg));
        }
        
        // Track this instantiation
        self.insert(format!("{}_instantiated", key), AstType::Generic {
            name: generic_name.to_string(),
            type_args: type_args.to_vec(),
        });
        
        // Track individual type arguments
        for (i, arg) in type_args.iter().enumerate() {
            self.insert(format!("{}_T{}", key, i), arg.clone());
        }
        
        key
    }
    
    /// Convert a type to a string representation for key generation
    fn type_to_string(&self, type_: &AstType) -> String {
        match type_ {
            AstType::I8 => "i8".to_string(),
            AstType::I16 => "i16".to_string(),
            AstType::I32 => "i32".to_string(),
            AstType::I64 => "i64".to_string(),
            AstType::U8 => "u8".to_string(),
            AstType::U16 => "u16".to_string(),
            AstType::U32 => "u32".to_string(),
            AstType::U64 => "u64".to_string(),
            AstType::F32 => "f32".to_string(),
            AstType::F64 => "f64".to_string(),
            AstType::Bool => "bool".to_string(),
            AstType::String => "string".to_string(),
            AstType::Void => "void".to_string(),
            AstType::Generic { name, type_args } => {
                let mut s = name.clone();
                if !type_args.is_empty() {
                    s.push('_');
                    for (i, arg) in type_args.iter().enumerate() {
                        if i > 0 {
                            s.push('_');
                        }
                        s.push_str(&self.type_to_string(arg));
                    }
                }
                s
            }
            _ => "unknown".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_nested_generic_tracking() {
        let mut tracker = GenericTypeTracker::new();
        
        // Create a nested generic type: Result<Option<i32>, String>
        let nested_type = AstType::Generic {
            name: "Result".to_string(),
            type_args: vec![
                AstType::Generic {
                    name: "Option".to_string(),
                    type_args: vec![AstType::I32],
                },
                AstType::String,
            ],
        };
        
        tracker.track_generic_type(&nested_type, "test");
        
        // Check that all nested types are tracked
        assert!(tracker.get("test_Ok_Type").is_some());
        assert!(tracker.get("test_Err_Type").is_some());
        assert!(tracker.get("test_Ok_Some_Type").is_some());
        assert_eq!(tracker.get("test_Ok_Some_Type"), Some(&AstType::I32));
        assert_eq!(tracker.get("test_Err_Type"), Some(&AstType::String));
    }
}