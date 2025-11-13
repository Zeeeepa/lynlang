pub mod primary;
pub mod operators;
pub mod calls;
pub mod structs;
pub mod patterns;
pub mod collections;
pub mod blocks;

use super::core::Parser;
use crate::ast::Expression;
use crate::error::Result;

impl<'a> Parser<'a> {
    pub fn parse_expression(&mut self) -> Result<Expression> {
        operators::parse_binary_expression(self, 0)
    }
}

