use crate::ast::AstType;
use crate::stdlib_metadata::StdFunction;
use std::collections::HashMap;

/// The IO module provides input/output operations
pub struct IOModule {
    functions: HashMap<String, StdFunction>,
}

impl IOModule {
    pub fn new() -> Self {
        let mut functions = HashMap::new();

        // Print functions
        functions.insert(
            "print".to_string(),
            StdFunction {
                name: "print".to_string(),
                params: vec![(
                    "message".to_string(),
                    crate::ast::resolve_string_struct_type(),
                )],
                return_type: AstType::Void,
                is_builtin: true,
            },
        );

        functions.insert(
            "println".to_string(),
            StdFunction {
                name: "println".to_string(),
                params: vec![(
                    "message".to_string(),
                    crate::ast::resolve_string_struct_type(),
                )],
                return_type: AstType::Void,
                is_builtin: true,
            },
        );

        functions.insert(
            "eprint".to_string(),
            StdFunction {
                name: "eprint".to_string(),
                params: vec![(
                    "message".to_string(),
                    crate::ast::resolve_string_struct_type(),
                )],
                return_type: AstType::Void,
                is_builtin: true,
            },
        );

        functions.insert(
            "eprintln".to_string(),
            StdFunction {
                name: "eprintln".to_string(),
                params: vec![(
                    "message".to_string(),
                    crate::ast::resolve_string_struct_type(),
                )],
                return_type: AstType::Void,
                is_builtin: true,
            },
        );

        // Input functions
        functions.insert(
            "read_line".to_string(),
            StdFunction {
                name: "read_line".to_string(),
                params: vec![],
                return_type: AstType::Generic {
                    name: "Result".to_string(),
                    type_args: vec![
                        crate::ast::resolve_string_struct_type(),
                        crate::ast::resolve_string_struct_type(),
                    ],
                },
                is_builtin: true,
            },
        );

        functions.insert(
            "read_input".to_string(),
            StdFunction {
                name: "read_input".to_string(),
                params: vec![(
                    "prompt".to_string(),
                    crate::ast::resolve_string_struct_type(),
                )],
                return_type: AstType::Generic {
                    name: "Result".to_string(),
                    type_args: vec![
                        crate::ast::resolve_string_struct_type(),
                        crate::ast::resolve_string_struct_type(),
                    ],
                },
                is_builtin: true,
            },
        );

        IOModule { functions }
    }
}

impl super::StdModuleTrait for IOModule {
    fn name(&self) -> &str {
        "io"
    }

    fn get_function(&self, name: &str) -> Option<StdFunction> {
        self.functions.get(name).cloned()
    }

    fn get_type(&self, _name: &str) -> Option<AstType> {
        None // IO module doesn't define types
    }
}
