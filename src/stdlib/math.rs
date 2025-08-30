use crate::ast::AstType;
use super::{StdModuleTrait, StdFunction};
use std::collections::HashMap;

/// The @std.math module provides mathematical operations
pub struct MathModule {
    functions: HashMap<String, StdFunction>,
    types: HashMap<String, AstType>,
}

impl MathModule {
    pub fn new() -> Self {
        let mut functions = HashMap::new();
        let types = HashMap::new();
        
        // Basic math functions
        functions.insert("abs".to_string(), StdFunction {
            name: "abs".to_string(),
            params: vec![("value".to_string(), AstType::F64)],
            return_type: AstType::F64,
            is_builtin: true,
        });
        
        functions.insert("sqrt".to_string(), StdFunction {
            name: "sqrt".to_string(),
            params: vec![("value".to_string(), AstType::F64)],
            return_type: AstType::F64,
            is_builtin: true,
        });
        
        functions.insert("pow".to_string(), StdFunction {
            name: "pow".to_string(),
            params: vec![
                ("base".to_string(), AstType::F64),
                ("exp".to_string(), AstType::F64),
            ],
            return_type: AstType::F64,
            is_builtin: true,
        });
        
        functions.insert("sin".to_string(), StdFunction {
            name: "sin".to_string(),
            params: vec![("angle".to_string(), AstType::F64)],
            return_type: AstType::F64,
            is_builtin: true,
        });
        
        functions.insert("cos".to_string(), StdFunction {
            name: "cos".to_string(),
            params: vec![("angle".to_string(), AstType::F64)],
            return_type: AstType::F64,
            is_builtin: true,
        });
        
        functions.insert("tan".to_string(), StdFunction {
            name: "tan".to_string(),
            params: vec![("angle".to_string(), AstType::F64)],
            return_type: AstType::F64,
            is_builtin: true,
        });
        
        functions.insert("log".to_string(), StdFunction {
            name: "log".to_string(),
            params: vec![("value".to_string(), AstType::F64)],
            return_type: AstType::F64,
            is_builtin: true,
        });
        
        functions.insert("log10".to_string(), StdFunction {
            name: "log10".to_string(),
            params: vec![("value".to_string(), AstType::F64)],
            return_type: AstType::F64,
            is_builtin: true,
        });
        
        functions.insert("exp".to_string(), StdFunction {
            name: "exp".to_string(),
            params: vec![("value".to_string(), AstType::F64)],
            return_type: AstType::F64,
            is_builtin: true,
        });
        
        functions.insert("floor".to_string(), StdFunction {
            name: "floor".to_string(),
            params: vec![("value".to_string(), AstType::F64)],
            return_type: AstType::F64,
            is_builtin: true,
        });
        
        functions.insert("ceil".to_string(), StdFunction {
            name: "ceil".to_string(),
            params: vec![("value".to_string(), AstType::F64)],
            return_type: AstType::F64,
            is_builtin: true,
        });
        
        functions.insert("round".to_string(), StdFunction {
            name: "round".to_string(),
            params: vec![("value".to_string(), AstType::F64)],
            return_type: AstType::F64,
            is_builtin: true,
        });
        
        functions.insert("min".to_string(), StdFunction {
            name: "min".to_string(),
            params: vec![
                ("a".to_string(), AstType::F64),
                ("b".to_string(), AstType::F64),
            ],
            return_type: AstType::F64,
            is_builtin: true,
        });
        
        functions.insert("max".to_string(), StdFunction {
            name: "max".to_string(),
            params: vec![
                ("a".to_string(), AstType::F64),
                ("b".to_string(), AstType::F64),
            ],
            return_type: AstType::F64,
            is_builtin: true,
        });
        
        MathModule { functions, types }
    }
}

impl StdModuleTrait for MathModule {
    fn name(&self) -> &str {
        "math"
    }
    
    fn get_function(&self, name: &str) -> Option<StdFunction> {
        self.functions.get(name).cloned()
    }
    
    fn get_type(&self, name: &str) -> Option<AstType> {
        self.types.get(name).cloned()
    }
}