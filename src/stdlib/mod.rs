use crate::ast::{Expression, AstType};
use std::collections::HashMap;

pub mod core;
pub mod build;
pub mod result;
pub mod io;
pub mod net;
pub mod math;
pub mod string;
pub mod vec;
pub mod fs;

/// The @std namespace provides built-in compiler intrinsics and standard library access
pub struct StdNamespace {
    modules: HashMap<String, StdModule>,
}

pub enum StdModule {
    Core(core::CoreModule),
    Build(build::BuildModule),
    IO(io::IOModule),
    Math(math::MathModule),
    String(string::StringModule),
    Vec(vec::VecModule),
    Fs(fs::FsModule),
}

impl StdNamespace {
    pub fn new() -> Self {
        let mut modules = HashMap::new();
        
        modules.insert("core".to_string(), StdModule::Core(core::CoreModule::new()));
        modules.insert("build".to_string(), StdModule::Build(build::BuildModule::new()));
        modules.insert("io".to_string(), StdModule::IO(io::IOModule::new()));
        modules.insert("math".to_string(), StdModule::Math(math::MathModule::new()));
        modules.insert("string".to_string(), StdModule::String(string::StringModule::new()));
        modules.insert("vec".to_string(), StdModule::Vec(vec::VecModule::new()));
        modules.insert("fs".to_string(), StdModule::Fs(fs::FsModule::new()));
        
        StdNamespace { modules }
    }
    
    pub fn get_module(&self, name: &str) -> Option<&StdModule> {
        self.modules.get(name)
    }
    
    /// Check if an identifier refers to @std namespace
    pub fn is_std_reference(name: &str) -> bool {
        name == "@std"
    }
    
    /// Resolve @std.module access
    pub fn resolve_std_access(module_name: &str) -> Option<Expression> {
        match module_name {
            "core" => Some(Expression::StdModule("core".to_string())),
            "build" => Some(Expression::StdModule("build".to_string())),
            "io" => Some(Expression::StdModule("io".to_string())),
            "math" => Some(Expression::StdModule("math".to_string())),
            "string" => Some(Expression::StdModule("string".to_string())),
            "vec" => Some(Expression::StdModule("vec".to_string())),
            "fs" => Some(Expression::StdModule("fs".to_string())),
            "net" => Some(Expression::StdModule("net".to_string())),
            "result" => Some(Expression::StdModule("result".to_string())),
            "mem" => Some(Expression::StdModule("mem".to_string())),
            "process" => Some(Expression::StdModule("process".to_string())),
            "thread" => Some(Expression::StdModule("thread".to_string())),
            "collections" => Some(Expression::StdModule("collections".to_string())),
            "hashmap" => Some(Expression::StdModule("hashmap".to_string())),
            "set" => Some(Expression::StdModule("set".to_string())),
            "json" => Some(Expression::StdModule("json".to_string())),
            "regex" => Some(Expression::StdModule("regex".to_string())),
            "random" => Some(Expression::StdModule("random".to_string())),
            "datetime" => Some(Expression::StdModule("datetime".to_string())),
            "crypto" => Some(Expression::StdModule("crypto".to_string())),
            "encoding" => Some(Expression::StdModule("encoding".to_string())),
            "http" => Some(Expression::StdModule("http".to_string())),
            "async" => Some(Expression::StdModule("async".to_string())),
            "async_runtime" => Some(Expression::StdModule("async_runtime".to_string())),
            "iterator" => Some(Expression::StdModule("iterator".to_string())),
            "algorithms" => Some(Expression::StdModule("algorithms".to_string())),
            "assert" => Some(Expression::StdModule("assert".to_string())),
            "test_framework" => Some(Expression::StdModule("test_framework".to_string())),
            _ => None,
        }
    }
}

/// Trait for standard library modules
pub trait StdModuleTrait {
    fn name(&self) -> &str;
    fn get_function(&self, name: &str) -> Option<StdFunction>;
    fn get_type(&self, name: &str) -> Option<AstType>;
}

#[derive(Clone)]
pub struct StdFunction {
    pub name: String,
    pub params: Vec<(String, AstType)>,
    pub return_type: AstType,
    pub is_builtin: bool,
}