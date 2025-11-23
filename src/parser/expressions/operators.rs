use super::super::core::Parser;
use crate::ast::{BinaryOperator, Expression};
use crate::error::Result;
use crate::lexer::Token;

pub fn parse_binary_expression(parser: &mut Parser, precedence: u8) -> Result<Expression> {
    let mut left = parse_unary_expression(parser)?;

    loop {
        // Check for ternary operator '?' first, before checking for other operators
        // This ensures that expressions like "x > y ?" are handled correctly
        // The ternary operator has precedence lower than comparison operators
        if parser.current_token == Token::Question {
            // Handle pattern matching with low precedence (but higher than assignment)
            // This ensures x < y ? ... parses as (x < y) ? ...
            if precedence < 1 {
                // Pattern match has very low precedence
                parser.next_token(); // consume '?'
                left = super::patterns::parse_pattern_match(parser, left)?;
            } else {
                // If we're in a recursive call with higher precedence, break
                // so the outer call (with lower precedence) can handle the '?'
                break;
            }
        // Check for 'as' identifier for type casting
        } else if let Token::Identifier(id) = &parser.current_token {
            if id == "as" {
                parser.next_token(); // consume 'as'
                let target_type = parser.parse_type()?;
                left = Expression::TypeCast {
                    expr: Box::new(left),
                    target_type,
                };
            } else {
                break;
            }
        } else if let Token::Operator(op) = &parser.current_token {
            let op_clone = op.clone();
            let next_prec = get_precedence(&op_clone);
            if next_prec > precedence {
                parser.next_token(); // advance past the operator

                // Handle range expressions specially
                if op_clone == ".." || op_clone == "..=" {
                    let right = parse_binary_expression(parser, next_prec)?;
                    left = Expression::Range {
                        start: Box::new(left),
                        end: Box::new(right),
                        inclusive: op_clone == "..=",
                    };
                } else {
                    // Parse right-hand side, but stop early if we encounter '?' 
                    // so the outer call can handle the ternary operator
                    let right = parse_binary_expression(parser, next_prec)?;
                    left = Expression::BinaryOp {
                        left: Box::new(left),
                        op: token_to_binary_operator(&op_clone)?,
                        right: Box::new(right),
                    };
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }

    Ok(left)
}

fn parse_unary_expression(parser: &mut Parser) -> Result<Expression> {
    match &parser.current_token {
        Token::Operator(op) if op == "-" => {
            parser.next_token();
            let expr = parse_unary_expression(parser)?;
            // For now, represent unary minus as BinaryOp with 0 - expr
            Ok(Expression::BinaryOp {
                left: Box::new(Expression::Integer32(0)),
                op: BinaryOperator::Subtract,
                right: Box::new(expr),
            })
        }
        Token::Operator(op) if op == "!" => {
            parser.next_token();
            let expr = parse_unary_expression(parser)?;
            // For now, represent logical not as a function call
            Ok(Expression::FunctionCall {
                name: "not".to_string(),
                args: vec![expr],
            })
        }
        _ => parse_postfix_expression(parser),
    }
}

fn parse_postfix_expression(parser: &mut Parser) -> Result<Expression> {
    super::primary::parse_primary_expression(parser)
}

pub fn get_precedence(op: &str) -> u8 {
    match op {
        ".." | "..=" => 1,            // Range has lowest precedence
        "||" => 2,                    // Logical OR
        "&&" => 3,                    // Logical AND
        "==" | "!=" => 4,             // Equality
        "<" | "<=" | ">" | ">=" => 5, // Comparison
        "+" | "-" => 6,               // Addition/Subtraction
        "*" | "/" | "%" => 7,         // Multiplication/Division/Modulo
        _ => 0,
    }
}

fn token_to_binary_operator(op: &str) -> Result<BinaryOperator> {
    match op {
        "+" => Ok(BinaryOperator::Add),
        "-" => Ok(BinaryOperator::Subtract),
        "*" => Ok(BinaryOperator::Multiply),
        "/" => Ok(BinaryOperator::Divide),
        "%" => Ok(BinaryOperator::Modulo),
        "==" => Ok(BinaryOperator::Equals),
        "!=" => Ok(BinaryOperator::NotEquals),
        "<" => Ok(BinaryOperator::LessThan),
        "<=" => Ok(BinaryOperator::LessThanEquals),
        ">" => Ok(BinaryOperator::GreaterThan),
        ">=" => Ok(BinaryOperator::GreaterThanEquals),
        "&&" => Ok(BinaryOperator::And),
        "||" => Ok(BinaryOperator::Or),
        _ => Err(crate::error::CompileError::SyntaxError(
            format!("Unknown binary operator: {}", op),
            None,
        )),
    }
}

