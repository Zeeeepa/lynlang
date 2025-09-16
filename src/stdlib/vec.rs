use crate::ast::AstType;
use super::{StdModuleTrait, StdFunction};
use std::collections::HashMap;

/// The @std.vec module provides vector/dynamic array operations
pub struct VecModule {
    functions: HashMap<String, StdFunction>,
    types: HashMap<String, AstType>,
}

impl VecModule {
    pub fn new() -> Self {
        let mut functions = HashMap::new();
        let mut types = HashMap::new();
        
        // Vec type
        types.insert("Vec".to_string(), AstType::Generic {
            name: "Vec".to_string(),
            type_args: vec![],
        });
        
        // Vec creation and manipulation
        functions.insert("new".to_string(), StdFunction {
            name: "new".to_string(),
            params: vec![],
            return_type: AstType::Generic {
                name: "Vec".to_string(),
                type_args: vec![AstType::Generic {
                    name: "T".to_string(),
                    type_args: vec![],
                }],
            },
            is_builtin: true,
        });
        
        functions.insert("with_capacity".to_string(), StdFunction {
            name: "with_capacity".to_string(),
            params: vec![("capacity".to_string(), AstType::I64)],
            return_type: AstType::Generic {
                name: "Vec".to_string(),
                type_args: vec![AstType::Generic {
                    name: "T".to_string(),
                    type_args: vec![],
                }],
            },
            is_builtin: true,
        });
        
        functions.insert("push".to_string(), StdFunction {
            name: "push".to_string(),
            params: vec![
                ("vec".to_string(), AstType::Generic {
                    name: "Vec".to_string(),
                    type_args: vec![AstType::Generic {
                        name: "T".to_string(),
                        type_args: vec![],
                    }],
                }),
                ("value".to_string(), AstType::Generic {
                    name: "T".to_string(),
                    type_args: vec![],
                }),
            ],
            return_type: AstType::Void,
            is_builtin: true,
        });
        
        functions.insert("pop".to_string(), StdFunction {
            name: "pop".to_string(),
            params: vec![
                ("vec".to_string(), AstType::Generic {
                    name: "Vec".to_string(),
                    type_args: vec![AstType::Generic {
                        name: "T".to_string(),
                        type_args: vec![],
                    }],
                }),
            ],
            return_type: AstType::Generic { 
                name: "Option".to_string(), 
                type_args: vec![
                    AstType::Generic {
                        name: "T".to_string(),
                        type_args: vec![],
                    }
                ] 
            },
            is_builtin: true,
        });
        
        functions.insert("len".to_string(), StdFunction {
            name: "len".to_string(),
            params: vec![
                ("vec".to_string(), AstType::Generic {
                    name: "Vec".to_string(),
                    type_args: vec![AstType::Generic {
                        name: "T".to_string(),
                        type_args: vec![],
                    }],
                }),
            ],
            return_type: AstType::I64,
            is_builtin: true,
        });
        
        functions.insert("is_empty".to_string(), StdFunction {
            name: "is_empty".to_string(),
            params: vec![
                ("vec".to_string(), AstType::Generic {
                    name: "Vec".to_string(),
                    type_args: vec![AstType::Generic {
                        name: "T".to_string(),
                        type_args: vec![],
                    }],
                }),
            ],
            return_type: AstType::Bool,
            is_builtin: true,
        });
        
        functions.insert("clear".to_string(), StdFunction {
            name: "clear".to_string(),
            params: vec![
                ("vec".to_string(), AstType::Generic {
                    name: "Vec".to_string(),
                    type_args: vec![AstType::Generic {
                        name: "T".to_string(),
                        type_args: vec![],
                    }],
                }),
            ],
            return_type: AstType::Void,
            is_builtin: true,
        });
        
        functions.insert("get".to_string(), StdFunction {
            name: "get".to_string(),
            params: vec![
                ("vec".to_string(), AstType::Generic {
                    name: "Vec".to_string(),
                    type_args: vec![AstType::Generic {
                        name: "T".to_string(),
                        type_args: vec![],
                    }],
                }),
                ("index".to_string(), AstType::I64),
            ],
            return_type: AstType::Generic { 
                name: "Option".to_string(), 
                type_args: vec![
                    AstType::Generic {
                        name: "T".to_string(),
                        type_args: vec![],
                    }
                ] 
            },
            is_builtin: true,
        });
        
        functions.insert("insert".to_string(), StdFunction {
            name: "insert".to_string(),
            params: vec![
                ("vec".to_string(), AstType::Generic {
                    name: "Vec".to_string(),
                    type_args: vec![AstType::Generic {
                        name: "T".to_string(),
                        type_args: vec![],
                    }],
                }),
                ("index".to_string(), AstType::I64),
                ("value".to_string(), AstType::Generic {
                    name: "T".to_string(),
                    type_args: vec![],
                }),
            ],
            return_type: AstType::Void,
            is_builtin: true,
        });
        
        functions.insert("remove".to_string(), StdFunction {
            name: "remove".to_string(),
            params: vec![
                ("vec".to_string(), AstType::Generic {
                    name: "Vec".to_string(),
                    type_args: vec![AstType::Generic {
                        name: "T".to_string(),
                        type_args: vec![],
                    }],
                }),
                ("index".to_string(), AstType::I64),
            ],
            return_type: AstType::Generic {
                name: "T".to_string(),
                type_args: vec![],
            },
            is_builtin: true,
        });
        
        VecModule { functions, types }
    }
}

impl StdModuleTrait for VecModule {
    fn name(&self) -> &str {
        "vec"
    }
    
    fn get_function(&self, name: &str) -> Option<StdFunction> {
        self.functions.get(name).cloned()
    }
    
    fn get_type(&self, name: &str) -> Option<AstType> {
        self.types.get(name).cloned()
    }
}