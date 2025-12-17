use crate::ast::AstType;
use std::collections::HashMap;

pub mod environment;
pub mod instantiation;
pub mod monomorphization;

pub use environment::TypeEnvironment;
pub use instantiation::TypeInstantiator;
pub use monomorphization::Monomorphizer;

#[derive(Debug, Clone, PartialEq)]
pub struct GenericInstance {
    pub base_name: String,
    pub type_args: Vec<AstType>,
    pub specialized_type: AstType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeSubstitution {
    pub mappings: HashMap<String, AstType>,
}

impl TypeSubstitution {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn add(&mut self, param: String, concrete: AstType) {
        self.mappings.insert(param, concrete);
    }

    #[allow(dead_code)]
    pub fn get(&self, param: &str) -> Option<&AstType> {
        self.mappings.get(param)
    }

    #[allow(dead_code)]
    pub fn apply(&self, ast_type: &AstType) -> AstType {
        match ast_type {
            AstType::Generic { name, type_args } => {
                if type_args.is_empty() {
                    if let Some(concrete) = self.mappings.get(name) {
                        return concrete.clone();
                    }
                }

                AstType::Generic {
                    name: name.clone(),
                    type_args: type_args.iter().map(|t| self.apply(t)).collect(),
                }
            }
            AstType::Ptr(inner) => AstType::Ptr(Box::new(self.apply(inner))),
            AstType::Array(inner) => AstType::Array(Box::new(self.apply(inner))),
            // Option and Result are now Generic types - handled in Generic match above
            AstType::Ref(inner) => AstType::Ref(Box::new(self.apply(inner))),
            AstType::Function { args, return_type } => AstType::Function {
                args: args.iter().map(|t| self.apply(t)).collect(),
                return_type: Box::new(self.apply(return_type)),
            },
            _ => ast_type.clone(),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TypeConstraint {
    pub param: String,
    pub bounds: Vec<String>,
}

#[allow(dead_code)]
pub fn is_generic_type(ast_type: &AstType) -> bool {
    match ast_type {
        AstType::Generic { type_args, .. } => {
            // Check if any type arguments are generic
            type_args.iter().any(is_generic_type)
        }
        AstType::Ptr(inner) | AstType::Array(inner) | AstType::Ref(inner) => is_generic_type(inner),
        // Option and Result are now Generic types - handled above
        AstType::Function { args, return_type } => {
            args.iter().any(is_generic_type) || is_generic_type(return_type)
        }
        _ => false,
    }
}

#[allow(dead_code)]
pub fn extract_type_parameters(ast_type: &AstType) -> Vec<String> {
    let mut params = Vec::new();
    extract_type_params_recursive(ast_type, &mut params);
    params
}

#[allow(dead_code)]
fn extract_type_params_recursive(ast_type: &AstType, params: &mut Vec<String>) {
    match ast_type {
        AstType::Generic { name, type_args } => {
            if type_args.is_empty() && !params.contains(name) {
                params.push(name.clone());
            }
            for arg in type_args {
                extract_type_params_recursive(arg, params);
            }
        }
        AstType::Ptr(inner) | AstType::Array(inner) | AstType::Ref(inner) => {
            extract_type_params_recursive(inner, params);
        }
        // Option and Result are now Generic types - handled in Generic match above
        AstType::Function { args, return_type } => {
            for arg in args {
                extract_type_params_recursive(arg, params);
            }
            extract_type_params_recursive(return_type, params);
        }
        _ => {}
    }
}
