use crate::ast::AstType;
use super::{StdModuleTrait, StdFunction};
use std::collections::HashMap;

/// The @std.fs module provides filesystem operations
pub struct FsModule {
    functions: HashMap<String, StdFunction>,
    types: HashMap<String, AstType>,
}

impl FsModule {
    pub fn new() -> Self {
        let mut functions = HashMap::new();
        let mut types = HashMap::new();
        
        // File metadata type
        types.insert("FileMetadata".to_string(), AstType::Generic {
            name: "FileMetadata".to_string(),
            type_args: vec![],
        });
        
        // Path type
        types.insert("Path".to_string(), AstType::Generic {
            name: "Path".to_string(),
            type_args: vec![],
        });
        
        // File operations
        functions.insert("read_file".to_string(), StdFunction {
            name: "read_file".to_string(),
            params: vec![("path".to_string(), AstType::String)],
            return_type: AstType::Result {
                ok_type: Box::new(AstType::String),
                err_type: Box::new(AstType::String),
            },
            is_builtin: true,
        });
        
        functions.insert("write_file".to_string(), StdFunction {
            name: "write_file".to_string(),
            params: vec![
                ("path".to_string(), AstType::String),
                ("content".to_string(), AstType::String),
            ],
            return_type: AstType::Result {
                ok_type: Box::new(AstType::Void),
                err_type: Box::new(AstType::String),
            },
            is_builtin: true,
        });
        
        functions.insert("append_file".to_string(), StdFunction {
            name: "append_file".to_string(),
            params: vec![
                ("path".to_string(), AstType::String),
                ("content".to_string(), AstType::String),
            ],
            return_type: AstType::Result {
                ok_type: Box::new(AstType::Void),
                err_type: Box::new(AstType::String),
            },
            is_builtin: true,
        });
        
        functions.insert("exists".to_string(), StdFunction {
            name: "exists".to_string(),
            params: vec![("path".to_string(), AstType::String)],
            return_type: AstType::Bool,
            is_builtin: true,
        });
        
        functions.insert("is_file".to_string(), StdFunction {
            name: "is_file".to_string(),
            params: vec![("path".to_string(), AstType::String)],
            return_type: AstType::Bool,
            is_builtin: true,
        });
        
        functions.insert("is_directory".to_string(), StdFunction {
            name: "is_directory".to_string(),
            params: vec![("path".to_string(), AstType::String)],
            return_type: AstType::Bool,
            is_builtin: true,
        });
        
        functions.insert("create_dir".to_string(), StdFunction {
            name: "create_dir".to_string(),
            params: vec![("path".to_string(), AstType::String)],
            return_type: AstType::Result {
                ok_type: Box::new(AstType::Void),
                err_type: Box::new(AstType::String),
            },
            is_builtin: true,
        });
        
        functions.insert("create_dir_all".to_string(), StdFunction {
            name: "create_dir_all".to_string(),
            params: vec![("path".to_string(), AstType::String)],
            return_type: AstType::Result {
                ok_type: Box::new(AstType::Void),
                err_type: Box::new(AstType::String),
            },
            is_builtin: true,
        });
        
        functions.insert("remove_file".to_string(), StdFunction {
            name: "remove_file".to_string(),
            params: vec![("path".to_string(), AstType::String)],
            return_type: AstType::Result {
                ok_type: Box::new(AstType::Void),
                err_type: Box::new(AstType::String),
            },
            is_builtin: true,
        });
        
        functions.insert("remove_dir".to_string(), StdFunction {
            name: "remove_dir".to_string(),
            params: vec![("path".to_string(), AstType::String)],
            return_type: AstType::Result {
                ok_type: Box::new(AstType::Void),
                err_type: Box::new(AstType::String),
            },
            is_builtin: true,
        });
        
        functions.insert("remove_dir_all".to_string(), StdFunction {
            name: "remove_dir_all".to_string(),
            params: vec![("path".to_string(), AstType::String)],
            return_type: AstType::Result {
                ok_type: Box::new(AstType::Void),
                err_type: Box::new(AstType::String),
            },
            is_builtin: true,
        });
        
        functions.insert("copy".to_string(), StdFunction {
            name: "copy".to_string(),
            params: vec![
                ("from".to_string(), AstType::String),
                ("to".to_string(), AstType::String),
            ],
            return_type: AstType::Result {
                ok_type: Box::new(AstType::I64),
                err_type: Box::new(AstType::String),
            },
            is_builtin: true,
        });
        
        functions.insert("rename".to_string(), StdFunction {
            name: "rename".to_string(),
            params: vec![
                ("from".to_string(), AstType::String),
                ("to".to_string(), AstType::String),
            ],
            return_type: AstType::Result {
                ok_type: Box::new(AstType::Void),
                err_type: Box::new(AstType::String),
            },
            is_builtin: true,
        });
        
        functions.insert("metadata".to_string(), StdFunction {
            name: "metadata".to_string(),
            params: vec![("path".to_string(), AstType::String)],
            return_type: AstType::Result {
                ok_type: Box::new(AstType::Generic {
                    name: "FileMetadata".to_string(),
                    type_args: vec![],
                }),
                err_type: Box::new(AstType::String),
            },
            is_builtin: true,
        });
        
        functions.insert("read_dir".to_string(), StdFunction {
            name: "read_dir".to_string(),
            params: vec![("path".to_string(), AstType::String)],
            return_type: AstType::Result {
                ok_type: Box::new(AstType::Generic {
                    name: "Vec".to_string(),
                    type_args: vec![AstType::String],
                }),
                err_type: Box::new(AstType::String),
            },
            is_builtin: true,
        });
        
        FsModule { functions, types }
    }
}

impl StdModuleTrait for FsModule {
    fn name(&self) -> &str {
        "fs"
    }
    
    fn get_function(&self, name: &str) -> Option<StdFunction> {
        self.functions.get(name).cloned()
    }
    
    fn get_type(&self, name: &str) -> Option<AstType> {
        self.types.get(name).cloned()
    }
}