use super::{StdFunction, StdModuleTrait};
use crate::ast::AstType;
use std::collections::HashMap;

/// The @std.build module provides build system functionality
pub struct BuildModule {
    functions: HashMap<String, StdFunction>,
    types: HashMap<String, AstType>,
}

impl BuildModule {
    pub fn new() -> Self {
        let mut functions = HashMap::new();
        let mut types = HashMap::new();

        // Import function for module loading
        functions.insert(
            "import".to_string(),
            StdFunction {
                name: "import".to_string(),
                params: vec![("module_name".to_string(), AstType::String)],
                return_type: AstType::Generic {
                    name: "Module".to_string(),
                    type_args: vec![],
                },
                is_builtin: true,
            },
        );

        // Default configuration function
        functions.insert(
            "default_config".to_string(),
            StdFunction {
                name: "default_config".to_string(),
                params: vec![],
                return_type: AstType::Generic {
                    name: "BuildConfig".to_string(),
                    type_args: vec![],
                },
                is_builtin: true,
            },
        );

        // Compile file function
        functions.insert(
            "compile_file".to_string(),
            StdFunction {
                name: "compile_file".to_string(),
                params: vec![
                    ("path".to_string(), AstType::String),
                    (
                        "config".to_string(),
                        AstType::Generic {
                            name: "BuildConfig".to_string(),
                            type_args: vec![],
                        },
                    ),
                ],
                return_type: AstType::Generic {
                    name: "BuildResult".to_string(),
                    type_args: vec![],
                },
                is_builtin: true,
            },
        );

        // Build project function
        functions.insert(
            "build_project".to_string(),
            StdFunction {
                name: "build_project".to_string(),
                params: vec![(
                    "config".to_string(),
                    AstType::Generic {
                        name: "BuildConfig".to_string(),
                        type_args: vec![],
                    },
                )],
                return_type: AstType::Generic {
                    name: "BuildResult".to_string(),
                    type_args: vec![],
                },
                is_builtin: true,
            },
        );

        // Get compiler version
        functions.insert(
            "compiler_version".to_string(),
            StdFunction {
                name: "compiler_version".to_string(),
                params: vec![],
                return_type: AstType::String,
                is_builtin: true,
            },
        );

        // Get target triple
        functions.insert(
            "target_triple".to_string(),
            StdFunction {
                name: "target_triple".to_string(),
                params: vec![],
                return_type: AstType::String,
                is_builtin: true,
            },
        );

        // Check if feature is enabled
        functions.insert(
            "has_feature".to_string(),
            StdFunction {
                name: "has_feature".to_string(),
                params: vec![("feature".to_string(), AstType::String)],
                return_type: AstType::Bool,
                is_builtin: true,
            },
        );

        // Build types - these would need proper struct definitions
        types.insert(
            "BuildConfig".to_string(),
            AstType::Generic {
                name: "BuildConfig".to_string(),
                type_args: vec![],
            },
        );

        types.insert(
            "BuildResult".to_string(),
            AstType::Generic {
                name: "BuildResult".to_string(),
                type_args: vec![],
            },
        );

        types.insert(
            "Module".to_string(),
            AstType::Generic {
                name: "Module".to_string(),
                type_args: vec![],
            },
        );

        types.insert(
            "CompilationUnit".to_string(),
            AstType::Generic {
                name: "CompilationUnit".to_string(),
                type_args: vec![],
            },
        );

        BuildModule { functions, types }
    }
}

impl StdModuleTrait for BuildModule {
    fn name(&self) -> &str {
        "build"
    }

    fn get_function(&self, name: &str) -> Option<StdFunction> {
        self.functions.get(name).cloned()
    }

    fn get_type(&self, name: &str) -> Option<AstType> {
        self.types.get(name).cloned()
    }
}
