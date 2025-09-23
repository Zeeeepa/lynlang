use crate::ast::AstType;
use super::{StdModuleTrait, StdFunction};
use std::collections::HashMap;

/// The @std.string module provides string manipulation functions
pub struct StringModule {
    functions: HashMap<String, StdFunction>,
    types: HashMap<String, AstType>,
}

impl StringModule {
    pub fn new() -> Self {
        let mut functions = HashMap::new();
        let types = HashMap::new();
        
        // String manipulation functions
        functions.insert("len".to_string(), StdFunction {
            name: "len".to_string(),
            params: vec![("str".to_string(), AstType::String)],
            return_type: AstType::I64,
            is_builtin: true,
        });
        
        functions.insert("concat".to_string(), StdFunction {
            name: "concat".to_string(),
            params: vec![
                ("a".to_string(), AstType::String),
                ("b".to_string(), AstType::String),
            ],
            return_type: AstType::String,
            is_builtin: true,
        });
        
        functions.insert("substring".to_string(), StdFunction {
            name: "substring".to_string(),
            params: vec![
                ("str".to_string(), AstType::String),
                ("start".to_string(), AstType::I64),
                ("end".to_string(), AstType::I64),
            ],
            return_type: AstType::String,
            is_builtin: true,
        });
        
        functions.insert("contains".to_string(), StdFunction {
            name: "contains".to_string(),
            params: vec![
                ("str".to_string(), AstType::String),
                ("pattern".to_string(), AstType::String),
            ],
            return_type: AstType::Bool,
            is_builtin: true,
        });
        
        functions.insert("starts_with".to_string(), StdFunction {
            name: "starts_with".to_string(),
            params: vec![
                ("str".to_string(), AstType::String),
                ("prefix".to_string(), AstType::String),
            ],
            return_type: AstType::Bool,
            is_builtin: true,
        });
        
        functions.insert("ends_with".to_string(), StdFunction {
            name: "ends_with".to_string(),
            params: vec![
                ("str".to_string(), AstType::String),
                ("suffix".to_string(), AstType::String),
            ],
            return_type: AstType::Bool,
            is_builtin: true,
        });
        
        functions.insert("replace".to_string(), StdFunction {
            name: "replace".to_string(),
            params: vec![
                ("str".to_string(), AstType::String),
                ("old".to_string(), AstType::String),
                ("new".to_string(), AstType::String),
            ],
            return_type: AstType::String,
            is_builtin: true,
        });
        
        functions.insert("split".to_string(), StdFunction {
            name: "split".to_string(),
            params: vec![
                ("str".to_string(), AstType::String),
                ("delimiter".to_string(), AstType::String),
            ],
            return_type: AstType::Generic {
                name: "Vec".to_string(),
                type_args: vec![AstType::String],
            },
            is_builtin: true,
        });
        
        functions.insert("trim".to_string(), StdFunction {
            name: "trim".to_string(),
            params: vec![("str".to_string(), AstType::String)],
            return_type: AstType::String,
            is_builtin: true,
        });
        
        functions.insert("to_upper".to_string(), StdFunction {
            name: "to_upper".to_string(),
            params: vec![("str".to_string(), AstType::String)],
            return_type: AstType::String,
            is_builtin: true,
        });
        
        functions.insert("to_lower".to_string(), StdFunction {
            name: "to_lower".to_string(),
            params: vec![("str".to_string(), AstType::String)],
            return_type: AstType::String,
            is_builtin: true,
        });
        
        functions.insert("format".to_string(), StdFunction {
            name: "format".to_string(),
            params: vec![
                ("template".to_string(), AstType::String),
                // Variadic args would be handled specially
            ],
            return_type: AstType::String,
            is_builtin: true,
        });
        
        // String conversion functions
        functions.insert("to_i32".to_string(), StdFunction {
            name: "to_i32".to_string(),
            params: vec![("str".to_string(), AstType::String)],
            return_type: AstType::Generic {
                name: "Option".to_string(),
                type_args: vec![AstType::I32],
            },
            is_builtin: true,
        });
        
        functions.insert("to_i64".to_string(), StdFunction {
            name: "to_i64".to_string(),
            params: vec![("str".to_string(), AstType::String)],
            return_type: AstType::Generic {
                name: "Option".to_string(),
                type_args: vec![AstType::I64],
            },
            is_builtin: true,
        });
        
        functions.insert("to_f32".to_string(), StdFunction {
            name: "to_f32".to_string(),
            params: vec![("str".to_string(), AstType::String)],
            return_type: AstType::Generic {
                name: "Option".to_string(),
                type_args: vec![AstType::F32],
            },
            is_builtin: true,
        });
        
        functions.insert("to_f64".to_string(), StdFunction {
            name: "to_f64".to_string(),
            params: vec![("str".to_string(), AstType::String)],
            return_type: AstType::Generic {
                name: "Option".to_string(),
                type_args: vec![AstType::F64],
            },
            is_builtin: true,
        });
        
        StringModule { functions, types }
    }
}

impl StdModuleTrait for StringModule {
    fn name(&self) -> &str {
        "string"
    }
    
    fn get_function(&self, name: &str) -> Option<StdFunction> {
        self.functions.get(name).cloned()
    }
    
    fn get_type(&self, name: &str) -> Option<AstType> {
        self.types.get(name).cloned()
    }
}