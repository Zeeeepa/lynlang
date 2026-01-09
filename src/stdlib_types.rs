use crate::ast::{AstType, Declaration, StructDefinition};
use crate::error::{CompileError, Result};
use crate::lexer::Lexer;
use crate::parser::Parser;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static STDLIB_TYPES: OnceLock<StdlibTypeRegistry> = OnceLock::new();

pub fn stdlib_types() -> &'static StdlibTypeRegistry {
    STDLIB_TYPES.get_or_init(|| StdlibTypeRegistry::load().unwrap_or_else(|e| {
        eprintln!("Warning: Failed to load stdlib types: {}", e);
        StdlibTypeRegistry::empty()
    }))
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used for future signature checking
pub struct MethodSignature {
    pub receiver_type: String,
    pub method_name: String,
    pub params: Vec<(String, AstType)>,
    pub return_type: AstType,
    pub is_static: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used for future signature checking
pub struct FunctionSignature {
    pub name: String,
    pub module: String,
    pub params: Vec<(String, AstType)>,
    pub return_type: AstType,
}

pub struct StdlibTypeRegistry {
    structs: HashMap<String, StructDefinition>,
    struct_types: HashMap<String, AstType>,
    methods: HashMap<String, MethodSignature>,
    functions: HashMap<String, FunctionSignature>,
}

impl StdlibTypeRegistry {
    fn empty() -> Self {
        Self {
            structs: HashMap::new(),
            struct_types: HashMap::new(),
            methods: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    fn load() -> Result<Self> {
        let mut registry = Self::empty();
        let stdlib_root = Self::find_stdlib_root();
        
        if !stdlib_root.exists() {
            return Ok(registry);
        }

        let files_to_parse = [
            "core/option.zen",
            "core/result.zen",
            "core/iterator.zen",
            "string.zen",
            "memory/gpa.zen",
            "memory/allocator.zen",
            "vec.zen",
            "io/io.zen",
            "math/math.zen",
        ];

        for file in &files_to_parse {
            let path = stdlib_root.join(file);
            if path.exists() {
                let _ = registry.parse_file(&path);
            }
        }

        Ok(registry)
    }

    fn find_stdlib_root() -> PathBuf {
        // Check environment variable first
        if let Ok(path) = std::env::var("ZEN_STDLIB_PATH") {
            let p = PathBuf::from(path);
            if p.exists() && p.is_dir() {
                return p;
            }
        }

        // Try relative paths
        let candidates = [
            PathBuf::from("./stdlib"),
            PathBuf::from("../stdlib"),
            PathBuf::from("../../stdlib"),
        ];

        for candidate in candidates {
            if candidate.exists() && candidate.is_dir() {
                return candidate;
            }
        }

        PathBuf::from("./stdlib")
    }

    fn parse_file(&mut self, path: &Path) -> Result<()> {
        let source = std::fs::read_to_string(path).map_err(|e| {
            CompileError::InternalError(format!("Failed to read {}: {}", path.display(), e), None)
        })?;

        let module_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let lexer = Lexer::new(&source);
        let mut parser = Parser::new(lexer);
        
        let program = match parser.parse_program() {
            Ok(p) => p,
            Err(_) => return Ok(()),
        };

        for decl in program.declarations {
            match decl {
                Declaration::Struct(struct_def) => {
                    let name = struct_def.name.clone();
                    let ast_type = self.struct_def_to_ast_type(&struct_def);
                    self.struct_types.insert(name.clone(), ast_type);
                    self.structs.insert(name, struct_def);
                }
                Declaration::Function(func) => {
                    self.register_function(&func, module_name);
                }
                Declaration::TraitImplementation(trait_impl) => {
                    for method in &trait_impl.methods {
                        let sig = MethodSignature {
                            receiver_type: trait_impl.type_name.clone(),
                            method_name: method.name.clone(),
                            params: method.args.clone(),
                            return_type: method.return_type.clone(),
                            is_static: method.args.first().map(|(n, _)| n != "self").unwrap_or(true),
                        };
                        let key = format!("{}::{}", trait_impl.type_name, method.name);
                        self.methods.insert(key, sig);
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn register_function(&mut self, func: &crate::ast::Function, module_name: &str) {
        if let Some((receiver, method)) = func.name.split_once('.') {
            let is_static = func.args.first()
                .map(|(name, _)| name != "self")
                .unwrap_or(true);

            let sig = MethodSignature {
                receiver_type: receiver.to_string(),
                method_name: method.to_string(),
                params: func.args.clone(),
                return_type: func.return_type.clone(),
                is_static,
            };

            let key = format!("{}::{}", receiver, method);
            self.methods.insert(key, sig);
        } else {
            let sig = FunctionSignature {
                name: func.name.clone(),
                module: module_name.to_string(),
                params: func.args.clone(),
                return_type: func.return_type.clone(),
            };
            
            let key = format!("{}::{}", module_name, &func.name);
            self.functions.insert(key, sig);
        }
    }

    fn struct_def_to_ast_type(&self, struct_def: &StructDefinition) -> AstType {
        let fields: Vec<(String, AstType)> = struct_def
            .fields
            .iter()
            .map(|f| (f.name.clone(), f.type_.clone()))
            .collect();

        AstType::Struct {
            name: struct_def.name.clone(),
            fields,
        }
    }

    pub fn get_string_type(&self) -> AstType {
        self.struct_types
            .get("String")
            .cloned()
            .unwrap_or_else(Self::fallback_string_type)
    }

    fn fallback_string_type() -> AstType {
        AstType::Struct {
            name: "String".to_string(),
            fields: vec![
                ("data".to_string(), AstType::ptr(AstType::U8)),
                ("len".to_string(), AstType::Usize),
                ("capacity".to_string(), AstType::Usize),
                (
                    "allocator".to_string(),
                    AstType::Generic {
                        name: "Allocator".to_string(),
                        type_args: vec![],
                    },
                ),
            ],
        }
    }

    pub fn is_string_type(name: &str) -> bool {
        name == "String"
    }

    /// Get a struct definition by name from stdlib
    pub fn get_struct_definition(&self, name: &str) -> Option<&StructDefinition> {
        self.structs.get(name)
    }

    pub fn get_method_signature(&self, receiver: &str, method: &str) -> Option<&MethodSignature> {
        let key = format!("{}::{}", receiver, method);
        self.methods.get(&key)
    }

    pub fn get_method_return_type(&self, receiver: &str, method: &str) -> Option<&AstType> {
        self.get_method_signature(receiver, method)
            .map(|sig| &sig.return_type)
    }

    pub fn get_function_signature(&self, module: &str, func_name: &str) -> Option<&FunctionSignature> {
        let key = format!("{}::{}", module, func_name);
        self.functions.get(&key)
    }

    pub fn get_function_return_type(&self, module: &str, func_name: &str) -> Option<&AstType> {
        self.get_function_signature(module, func_name)
            .map(|sig| &sig.return_type)
    }
    
    #[allow(dead_code)] // Used in tests
    pub fn debug_list_functions(&self) -> Vec<String> {
        self.functions.keys().cloned().collect()
    }

    #[allow(dead_code)] // Used in tests
    pub fn debug_list_methods(&self) -> Vec<String> {
        self.methods.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdlib_gpa_loading() {
        let registry = stdlib_types();
        
        let funcs = registry.debug_list_functions();
        println!("Registered functions: {:?}", funcs);
        
        let ret = registry.get_function_return_type("gpa", "default_gpa");
        println!("gpa::default_gpa return type: {:?}", ret);
        
        assert!(ret.is_some(), "gpa::default_gpa should be registered");
    }

    #[test]
    fn test_string_methods() {
        let registry = stdlib_types();
        
        let methods: Vec<_> = registry.methods.keys().collect();
        println!("Registered methods: {:?}", methods);
        
        let ret = registry.get_method_return_type("String", "len");
        println!("String::len return type: {:?}", ret);
        
        let ret2 = registry.get_method_return_type("String", "push");
        println!("String::push return type: {:?}", ret2);

        assert!(ret.is_some(), "String::len should be registered");
    }

    #[test]
    fn test_range_methods() {
        let registry = stdlib_types();

        let methods = registry.debug_list_methods();
        println!("All registered methods: {:?}", methods);

        let range_methods: Vec<_> = methods.iter().filter(|m| m.starts_with("Range")).collect();
        println!("Range methods: {:?}", range_methods);

        let ret = registry.get_method_return_type("Range", "has_next");
        println!("Range::has_next return type: {:?}", ret);

        let ret2 = registry.get_method_return_type("Range", "count");
        println!("Range::count return type: {:?}", ret2);
    }
}
