//! # Abstract Syntax Tree
//!
//! The `ast` module defines the data structures that represent the code in a structured way.
//! The parser will produce these structures, and the compiler will consume them.

mod span;
mod types;
mod expressions;
mod statements;
mod declarations;
mod patterns;

// Re-export everything for backward compatibility
pub use span::{Position, Span};
pub use types::*;
pub use expressions::*;
pub use statements::*;
pub use declarations::*;
pub use patterns::*;

/// Root AST node representing a complete Zen program
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Program {
    pub declarations: Vec<Declaration>,
    pub statements: Vec<Statement>, // Top-level statements (for testing/REPL)
}

// Convenience methods for backward compatibility
impl Program {
    pub fn new(declarations: Vec<Declaration>) -> Self {
        Self {
            declarations,
            statements: Vec::new(),
        }
    }
    
    pub fn from_functions(functions: Vec<Function>) -> Self {
        Self {
            declarations: functions.into_iter().map(Declaration::Function).collect(),
            statements: Vec::new(),
        }
    }

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