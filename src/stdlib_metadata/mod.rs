use crate::ast::{AstType, Expression};
use std::collections::HashMap;

pub mod build;
pub mod compiler;
pub mod core;
pub mod fs;
pub mod io;
pub mod math;
pub mod net;
pub mod result;
pub mod vec;

/// The @std namespace provides built-in compiler intrinsics and standard library access
#[allow(dead_code)]
pub struct StdNamespace {
    modules: HashMap<String, StdModule>,
}

#[allow(dead_code)]
pub enum StdModule {
    Core(core::CoreModule),
    Compiler(compiler::CompilerModule),
    Build(build::BuildModule),
    IO(io::IOModule),
    Math(math::MathModule),
    Vec(vec::VecModule),
    Fs(fs::FsModule),
}

#[allow(dead_code)]
impl StdNamespace {
    pub fn new() -> Self {
        let mut modules = HashMap::new();

        modules.insert("core".to_string(), StdModule::Core(core::CoreModule::new()));
        modules.insert(
            "compiler".to_string(),
            StdModule::Compiler(compiler::CompilerModule::new()),
        );
        modules.insert(
            "build".to_string(),
            StdModule::Build(build::BuildModule::new()),
        );
        modules.insert("io".to_string(), StdModule::IO(io::IOModule::new()));
        modules.insert("math".to_string(), StdModule::Math(math::MathModule::new()));
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
        // For now, just return StdReference for any valid module name
        // The actual module resolution will happen at a different layer
        match module_name {
            "core" | "compiler" | "build" | "io" | "math" | "string" | "vec" | "fs" | "net"
            | "result" | "mem" | "process" | "thread" | "collections" | "hashmap" | "set"
            | "json" | "regex" | "random" | "datetime" | "crypto" | "encoding" | "http"
            | "concurrency" | "concurrent_runtime" | "iterator" | "algorithms" | "assert"
            | "test_framework" => Some(Expression::StdReference),
            _ => None,
        }
    }
}

/// Trait for standard library modules
#[allow(dead_code)]
pub trait StdModuleTrait {
    fn name(&self) -> &str;
    fn get_function(&self, name: &str) -> Option<StdFunction>;
    fn get_type(&self, name: &str) -> Option<AstType>;
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct StdFunction {
    pub name: String,
    pub params: Vec<(String, AstType)>,
    pub return_type: AstType,
    pub is_builtin: bool,
}
