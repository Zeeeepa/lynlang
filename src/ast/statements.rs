//! Statement nodes in the AST

use super::expressions::Expression;
use super::types::AstType;

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Expression(Expression),
    Return(Expression),
    // Enhanced variable declarations supporting all Zen syntax
    VariableDeclaration {
        name: String,
        type_: Option<AstType>, // None for inferred types
        initializer: Option<Expression>,
        is_mutable: bool, // true for ::= and :: T =, false for := and : T =
        declaration_type: VariableDeclarationType,
    },
    VariableAssignment {
        name: String,
        value: Expression,
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
    ModuleImport {
        alias: String,
        module_path: String,
    },
    // Defer statement for cleanup - traditional defer syntax
    Defer(Box<Statement>),
    // @this.defer() for scope-based cleanup
    ThisDefer(Expression),
    // Destructuring import: { io, maths } = @std
    DestructuringImport {
        names: Vec<String>,
        source: Expression,
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
