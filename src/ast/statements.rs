//! Statement nodes in the AST

use super::expressions::Expression;
use super::types::AstType;
use crate::error::Span;

/// A statement with optional source location information
#[derive(Debug, Clone, PartialEq)]
pub struct SpannedStatement {
    pub stmt: Statement,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Expression {
        expr: Expression,
        span: Option<Span>,
    },
    Return {
        expr: Expression,
        span: Option<Span>,
    },
    // Enhanced variable declarations supporting all Zen syntax
    VariableDeclaration {
        name: String,
        type_: Option<AstType>, // None for inferred types
        initializer: Option<Expression>,
        is_mutable: bool, // true for ::= and :: T =, false for := and : T =
        declaration_type: VariableDeclarationType,
        span: Option<Span>, // Source location for error reporting
    },
    #[allow(dead_code)]
    VariableAssignment {
        name: String,
        value: Expression,
        span: Option<Span>,
    },
    PointerAssignment {
        pointer: Expression,
        value: Expression,
    },
    // Loop construct supporting all Zen loop variations
    Loop {
        kind: LoopKind,
        label: Option<String>, // For labeled loops
        body: Vec<Statement>,
    },
    Break {
        label: Option<String>, // For labeled break
    },
    Continue {
        label: Option<String>, // For labeled continue
    },
    // New statements for enhanced features
    ComptimeBlock(Vec<Statement>),
    #[allow(dead_code)]
    ModuleImport {
        alias: String,
        module_path: String,
    },
    // Defer statement for cleanup - traditional defer syntax
    #[allow(dead_code)]
    Defer(Box<Statement>),
    // @this.defer() for scope-based cleanup
    #[allow(dead_code)]
    ThisDefer(Expression),
    // Destructuring import: { io, maths } = @std
    DestructuringImport {
        names: Vec<String>,
        source: Expression,
    },
    // Block of statements - used for defer blocks, etc.
    Block {
        statements: Vec<Statement>,
        span: Option<Span>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum VariableDeclarationType {
    InferredImmutable, // = (plain assignment creates immutable in Zen spec)
    InferredMutable,   // ::=
    ExplicitImmutable, // : T (with type annotation, immutable)
    ExplicitMutable,   // :: T (with type annotation, mutable)
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoopKind {
    // loop { } - infinite loop
    Infinite,
    // loop condition { } - while-like loop
    Condition(Expression),
}
