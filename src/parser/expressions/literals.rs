//! Literal expression parsing: integers, floats, strings
//! Extracted from primary.rs

use super::super::core::Parser;
use crate::ast::Expression;
use crate::error::{CompileError, Result};
use crate::lexer::Token;

/// Parse integer literal with UFC support
pub fn parse_integer_literal(parser: &mut Parser, value_str: &str) -> Result<Expression> {
    let value = value_str.parse::<i64>().map_err(|_| {
        CompileError::SyntaxError(
            format!("Invalid integer: {}", value_str),
            Some(parser.current_span.clone()),
        )
    })?;
    parser.next_token();
    // Default to Integer32 unless out of range
    let mut expr = if value <= i32::MAX as i64 && value >= i32::MIN as i64 {
        Expression::Integer32(value as i32)
    } else {
        Expression::Integer64(value)
    };

    // Handle UFC on integer literals: 5.double()
    loop {
        match &parser.current_token {
            Token::Symbol('.') => {
                parser.next_token(); // consume '.'

                let member = match &parser.current_token {
                    Token::Identifier(name) => name.clone(),
                    _ => {
                        return Err(CompileError::SyntaxError(
                            "Expected identifier after '.'".to_string(),
                            Some(parser.current_span.clone()),
                        ));
                    }
                };
                parser.next_token();

                // Check if it's a method call
                if parser.current_token == Token::Symbol('(') {
                    return super::calls::parse_call_expression_with_object(parser, expr, member);
                } else {
                    // For now, just member access (though unlikely on literals)
                    expr = Expression::MemberAccess {
                        object: Box::new(expr),
                        member,
                    };
                }
            }
            _ => break,
        }
    }

    Ok(expr)
}

/// Parse float literal with UFC support
pub fn parse_float_literal(parser: &mut Parser, value_str: &str) -> Result<Expression> {
    let value = value_str.parse::<f64>().map_err(|_| {
        CompileError::SyntaxError(
            format!("Invalid float: {}", value_str),
            Some(parser.current_span.clone()),
        )
    })?;
    parser.next_token();
    let mut expr = Expression::Float64(value);

    // Handle UFC on float literals
    loop {
        match &parser.current_token {
            Token::Symbol('.') => {
                parser.next_token(); // consume '.'

                let member = match &parser.current_token {
                    Token::Identifier(name) => name.clone(),
                    _ => {
                        return Err(CompileError::SyntaxError(
                            "Expected identifier after '.'".to_string(),
                            Some(parser.current_span.clone()),
                        ));
                    }
                };
                parser.next_token();

                // Check if it's a method call
                if parser.current_token == Token::Symbol('(') {
                    return super::calls::parse_call_expression_with_object(parser, expr, member);
                } else {
                    expr = Expression::MemberAccess {
                        object: Box::new(expr),
                        member,
                    };
                }
            }
            _ => break,
        }
    }

    Ok(expr)
}

/// Parse string literal with interpolation and UFC support
pub fn parse_string_literal(parser: &mut Parser, value: &str) -> Result<Expression> {
    let value = value.to_string();
    parser.next_token();

    // Check if the string contains interpolation markers
    let mut expr = if value.contains('\x01') {
        super::blocks::parse_interpolated_string(parser, value)?
    } else {
        Expression::String(value)
    };

    // Handle UFC on string literals
    loop {
        match &parser.current_token {
            Token::Symbol('.') => {
                parser.next_token(); // consume '.'

                let member = match &parser.current_token {
                    Token::Identifier(name) => name.clone(),
                    _ => {
                        return Err(CompileError::SyntaxError(
                            "Expected identifier after '.'".to_string(),
                            Some(parser.current_span.clone()),
                        ));
                    }
                };
                parser.next_token();

                // Check if it's a method call
                if parser.current_token == Token::Symbol('(') {
                    return super::calls::parse_call_expression_with_object(parser, expr, member);
                } else {
                    expr = Expression::MemberAccess {
                        object: Box::new(expr),
                        member,
                    };
                }
            }
            _ => break,
        }
    }

    Ok(expr)
}

/// Parse shorthand enum variant syntax: .VariantName or .VariantName(payload)
pub fn parse_shorthand_enum_variant(parser: &mut Parser) -> Result<Expression> {
    parser.next_token(); // consume '.'

    let variant = match &parser.current_token {
        Token::Identifier(v) => v.clone(),
        _ => {
            return Err(CompileError::SyntaxError(
                "Expected variant name after '.'".to_string(),
                Some(parser.current_span.clone()),
            ));
        }
    };
    parser.next_token();

    // Check for variant payload
    let payload = if parser.current_token == Token::Symbol('(') {
        parser.next_token(); // consume '('
        let expr = parser.parse_expression()?;
        if parser.current_token != Token::Symbol(')') {
            return Err(CompileError::SyntaxError(
                "Expected ')' after enum variant payload".to_string(),
                Some(parser.current_span.clone()),
            ));
        }
        parser.next_token(); // consume ')'
        Some(Box::new(expr))
    } else {
        None
    };

    // Use EnumLiteral for shorthand syntax
    Ok(Expression::EnumLiteral { variant, payload })
}
