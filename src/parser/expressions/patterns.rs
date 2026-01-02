use crate::parser::core::Parser;
use crate::ast::{Expression, MatchArm, Pattern};
use crate::error::{CompileError, Result};
use crate::lexer::Token;

pub fn parse_pattern_match(parser: &mut Parser, scrutinee: Expression) -> Result<Expression> {
    // Parse: scrutinee ? | pattern => expr | pattern => expr ...
    // OR bool short form: scrutinee ? { block }
    let scrutinee = Box::new(scrutinee);

    // Check for bool pattern short form: expr ? { block }
    if parser.current_token == Token::Symbol('{') {
        // Bool pattern short form - execute block if scrutinee is true
        let body = super::blocks::parse_block_expression(parser)?;

        // Convert to standard pattern match with true pattern
        let arms = vec![
            MatchArm {
                pattern: Pattern::Literal(Expression::Boolean(true)),
                guard: None,
                body,
            },
            MatchArm {
                pattern: Pattern::Wildcard,
                guard: None,
                body: Expression::Block(vec![]), // Empty block for else case (returns void like the true case)
            },
        ];

        return Ok(Expression::QuestionMatch { scrutinee, arms });
    }

    // Standard pattern matching with | pattern => expr
    if parser.current_token != Token::Pipe {
        return Err(CompileError::SyntaxError(
            "Expected '|' to start pattern matching arms or '{' for bool pattern".to_string(),
            Some(parser.current_span.clone()),
        ));
    }

    let mut arms = vec![];

    // Parse arms: | pattern => expr | pattern => expr ...
    while parser.current_token == Token::Pipe {
        parser.next_token(); // consume '|'

        // Parse pattern - could be single or multiple (or patterns)
        let mut patterns = vec![parser.parse_pattern()?];

        // Check for additional patterns separated by |
        while parser.current_token == Token::Pipe &&
                  parser.peek_token != Token::Pipe && // Not start of next arm
                  parser.peek_token != Token::Eof
        {
            // This is an or pattern - consume the | and parse the next pattern
            parser.next_token();
            patterns.push(parser.parse_pattern()?);
        }

        // Create the final pattern
        let pattern = if patterns.len() == 1 {
            patterns.remove(0)
        } else {
            Pattern::Or(patterns)
        };

        // Check for destructuring/guard with ->
        let guard = if parser.current_token == Token::Operator("->".to_string()) {
            parser.next_token();
            // TODO: Properly handle destructuring vs guards
            // For now, treat it as a guard
            Some(parser.parse_expression()?)
        } else {
            None
        };

        // Parse body - can be either { block } or => expr (for compatibility)
        let body = if parser.current_token == Token::Symbol('{') {
            // Parse block as expression
            parser.next_token(); // consume '{'

            let mut statements = Vec::new();
            let mut final_expr = None;

            while parser.current_token != Token::Symbol('}') && parser.current_token != Token::Eof {
                // Check if this could be the final expression
                if parser.peek_token == Token::Symbol('}') {
                    // This might be the final expression (no semicolon)
                    let expr = parser.parse_expression()?;
                    if parser.current_token == Token::Symbol(';') {
                        // It's a statement, not the final expression
                        parser.next_token();
                        statements.push(crate::ast::Statement::Expression { expr, span: Some(parser.current_span.clone()) });
                    } else {
                        // It's the final expression
                        final_expr = Some(expr);
                    }
                } else {
                    // Parse as statement to handle variable declarations and assignments
                    let stmt = parser.parse_statement()?;
                    statements.push(stmt);
                }
            }

            if parser.current_token != Token::Symbol('}') {
                return Err(CompileError::SyntaxError(
                    "Expected '}' to close block in match arm".to_string(),
                    Some(parser.current_span.clone()),
                ));
            }
            parser.next_token(); // consume '}'

            // If we have statements, return a block expression
            // Otherwise return the final expression or an empty block
            if !statements.is_empty() || final_expr.is_some() {
                // If there's a final expression, add it to the statements
                if let Some(expr) = final_expr {
                    statements.push(crate::ast::Statement::Expression { expr, span: Some(parser.current_span.clone()) });
                }
                Expression::Block(statements)
            } else {
                // Empty block
                Expression::Block(vec![])
            }
        } else if parser.current_token == Token::Operator("=>".to_string()) {
            // Legacy => syntax for compatibility
            parser.next_token(); // consume '=>'
                                 // Handle return statement in pattern arm specially
            if let Token::Identifier(id) = &parser.current_token {
                if id == "return" {
                    parser.next_token(); // consume 'return'
                    let return_expr = parser.parse_expression()?;
                    // Wrap return in a special expression type or handle it differently
                    // For now, we'll just use the return expression directly
                    // In a full implementation, we'd need a Block expression type with statements
                    return_expr
                } else {
                    parser.parse_expression()?
                }
            } else {
                parser.parse_expression()?
            }
        } else {
            return Err(CompileError::SyntaxError(
                "Expected '{' or '=>' after pattern in match arm".to_string(),
                Some(parser.current_span.clone()),
            ));
        };

        arms.push(MatchArm {
            pattern,
            guard,
            body,
        });

        // Check if there are more arms
        if parser.current_token != Token::Pipe {
            break;
        }
    }
    Ok(Expression::QuestionMatch { scrutinee, arms })
}
