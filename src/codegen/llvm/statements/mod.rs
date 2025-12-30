pub mod control;
pub mod deferred;
pub mod variables;

use super::LLVMCompiler;
use crate::ast::Statement;
use crate::error::CompileError;

impl<'ctx> LLVMCompiler<'ctx> {
    pub fn compile_statement(&mut self, statement: &Statement) -> Result<(), CompileError> {
        match statement {
            Statement::Expression { expr, span } => {
                self.set_span(span.clone());
                variables::compile_expression_statement(self, expr)
            }
            Statement::Return { expr, span } => {
                self.set_span(span.clone());
                control::compile_return(self, expr)
            }
            Statement::VariableDeclaration { span, .. } => {
                self.set_span(span.clone());
                variables::compile_variable_declaration(self, statement)
            }
            Statement::VariableAssignment { span, .. } => {
                self.set_span(span.clone());
                variables::compile_assignment(self, statement)
            }
            Statement::PointerAssignment { .. } => variables::compile_assignment(self, statement),
            Statement::Loop { .. } => control::compile_loop(self, statement),
            Statement::Break { .. } => control::compile_break(self),
            Statement::Continue { .. } => control::compile_continue(self),
            Statement::Defer(_) => deferred::compile_defer(self, statement),
            Statement::ThisDefer(_) => deferred::compile_defer(self, statement),
            Statement::ComptimeBlock(_) => Ok(()),
            Statement::ModuleImport { .. } | Statement::DestructuringImport { .. } => Ok(()),
        }
    }

    pub fn execute_deferred_expressions(&mut self) -> Result<(), CompileError> {
        deferred::execute_deferred_expressions(self)
    }
}
