use crate::ast::AstType;
use super::{StdModuleTrait, StdFunction};
use std::collections::HashMap;

/// The @std.core module provides compiler intrinsics and core functionality
pub struct CoreModule {
    functions: HashMap<String, StdFunction>,
    types: HashMap<String, AstType>,
}

impl CoreModule {
    pub fn new() -> Self {
        let mut functions = HashMap::new();
        let mut types = HashMap::new();
        
        // Core intrinsic functions
        functions.insert("size_of".to_string(), StdFunction {
            name: "size_of".to_string(),
            params: vec![("T".to_string(), AstType::Generic { name: "T".to_string(), type_args: vec![] })],
            return_type: AstType::U64,
            is_builtin: true,
        });
        
        // Additional core functions from SDK
        functions.insert("min".to_string(), StdFunction {
            name: "min".to_string(),
            params: vec![
                ("a".to_string(), AstType::I64),
                ("b".to_string(), AstType::I64),
            ],
            return_type: AstType::I64,
            is_builtin: true,
        });
        
        functions.insert("max".to_string(), StdFunction {
            name: "max".to_string(),
            params: vec![
                ("a".to_string(), AstType::I64),
                ("b".to_string(), AstType::I64),
            ],
            return_type: AstType::I64,
            is_builtin: true,
        });
        
        functions.insert("abs".to_string(), StdFunction {
            name: "abs".to_string(),
            params: vec![("n".to_string(), AstType::I64)],
            return_type: AstType::I64,
            is_builtin: true,
        });
        
        functions.insert("clamp".to_string(), StdFunction {
            name: "clamp".to_string(),
            params: vec![
                ("value".to_string(), AstType::I64),
                ("min".to_string(), AstType::I64),
                ("max".to_string(), AstType::I64),
            ],
            return_type: AstType::I64,
            is_builtin: true,
        });
        
        functions.insert("align_of".to_string(), StdFunction {
            name: "align_of".to_string(),
            params: vec![("T".to_string(), AstType::Generic { name: "T".to_string(), type_args: vec![] })],
            return_type: AstType::U64,
            is_builtin: true,
        });
        
        functions.insert("type_name".to_string(), StdFunction {
            name: "type_name".to_string(),
            params: vec![("T".to_string(), AstType::Generic { name: "T".to_string(), type_args: vec![] })],
            return_type: AstType::String,
            is_builtin: true,
        });
        
        functions.insert("panic".to_string(), StdFunction {
            name: "panic".to_string(),
            params: vec![("message".to_string(), AstType::String)],
            return_type: AstType::Void,
            is_builtin: true,
        });
        
        functions.insert("assert".to_string(), StdFunction {
            name: "assert".to_string(),
            params: vec![("condition".to_string(), AstType::Bool)],
            return_type: AstType::Void,
            is_builtin: true,
        });
        
        // Core types
        types.insert("type".to_string(), AstType::Generic { name: "type".to_string(), type_args: vec![] });
        types.insert("Any".to_string(), AstType::Generic { name: "Any".to_string(), type_args: vec![] });
        types.insert("Result".to_string(), AstType::Generic {
            name: "Result".to_string(),
            type_args: vec![
                AstType::Generic { name: "T".to_string(), type_args: vec![] },
                AstType::Generic { name: "E".to_string(), type_args: vec![] },
            ]
        });
        types.insert("Option".to_string(), AstType::Generic {
            name: "Option".to_string(),
            type_args: vec![AstType::Generic { name: "T".to_string(), type_args: vec![] }]
        });
        types.insert("Range".to_string(), AstType::Generic { name: "Range".to_string(), type_args: vec![] });
        
        CoreModule { functions, types }
    }
}

impl StdModuleTrait for CoreModule {
    fn name(&self) -> &str {
        "core"
    }
    
    fn get_function(&self, name: &str) -> Option<StdFunction> {
        self.functions.get(name).cloned()
    }
    
    fn get_type(&self, name: &str) -> Option<AstType> {
        self.types.get(name).cloned()
    }
}