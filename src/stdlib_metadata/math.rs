use super::{StdFunction, StdModuleTrait};
use crate::ast::AstType;
use crate::register_stdlib_fn;
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

        // Register all math functions using the macro
        register_stdlib_fn!(functions,
            // Basic operations
            abs(value: F64) -> F64,
            sqrt(value: F64) -> F64,
            pow(base: F64, exp: F64) -> F64,

            // Trigonometric
            sin(angle: F64) -> F64,
            cos(angle: F64) -> F64,
            tan(angle: F64) -> F64,

            // Logarithmic
            log(value: F64) -> F64,
            log10(value: F64) -> F64,
            exp(value: F64) -> F64,

            // Rounding
            floor(value: F64) -> F64,
            ceil(value: F64) -> F64,
            round(value: F64) -> F64,

            // Comparison
            min(a: F64, b: F64) -> F64,
            max(a: F64, b: F64) -> F64,
        );

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
