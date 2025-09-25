//! # Abstract Syntax Tree
//!
//! The `ast` module defines the data structures that represent the code in a structured way.
//! The parser will produce these structures, and the compiler will consume them.

mod declarations;
mod expressions;
mod patterns;
mod span;
mod statements;
mod types;

// Re-export everything for backward compatibility
pub use declarations::*;
pub use expressions::*;
pub use patterns::*;
// pub use span::{Position, Span}; // Currently unused
pub use statements::*;
pub use types::*;

/// Root AST node representing a complete Zen program
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Program {
    pub declarations: Vec<Declaration>,
    pub statements: Vec<Statement>, // Top-level statements (for testing/REPL)
}

// Convenience methods for backward compatibility
impl Program {
    #[allow(dead_code)]
    pub fn new(declarations: Vec<Declaration>) -> Self {
        Self {
            declarations,
            statements: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn from_functions(functions: Vec<Function>) -> Self {
        Self {
            declarations: functions.into_iter().map(Declaration::Function).collect(),
            statements: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn functions(&self) -> impl Iterator<Item = &Function> {
        self.declarations.iter().filter_map(|decl| {
            if let Declaration::Function(func) = decl {
                Some(func)
            } else {
                None
            }
        })
    }
}
