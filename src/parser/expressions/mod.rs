pub mod blocks;
pub mod calls;
pub mod collections;
pub mod control_flow;
pub mod literals;
pub mod operators;
pub mod patterns;
pub mod primary;
pub mod structs;

use super::core::Parser;
use crate::ast::Expression;
use crate::error::Result;

impl<'a> Parser<'a> {
    pub fn parse_expression(&mut self) -> Result<Expression> {
        operators::parse_binary_expression(self, 0)
    }
}
