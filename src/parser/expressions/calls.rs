use crate::ast::Expression;
use crate::error::Result;
use crate::lexer::Token;
use crate::parser::core::Parser;

/// Parse a function call: `name(args...)`
pub fn parse_call_expression(parser: &mut Parser, function_name: String) -> Result<Expression> {
    let arguments = parse_argument_list(parser)?;

    let expr = Expression::FunctionCall {
        name: function_name,
        args: arguments,
    };

    parse_method_chain(parser, expr)
}

/// Parse a method call: `object.method(args...)`
pub fn parse_call_expression_with_object(
    parser: &mut Parser,
    object: Expression,
    method_name: String,
) -> Result<Expression> {
    parser.next_token(); // consume '('

    // Check for @std and @builtin syntax - these are syntactic constructs
    let is_builtin_syntax = match &object {
        Expression::BuiltinReference => true,
        Expression::MemberAccess { object: base, .. } => {
            matches!(base.as_ref(), Expression::StdReference | Expression::BuiltinReference)
        }
        _ => false,
    };

    let arguments = parse_arguments_until_close(parser)?;
    parser.next_token(); // consume ')'

    let expr = if is_builtin_syntax {
        build_builtin_call(&object, &method_name, arguments)
    } else {
        Expression::MethodCall {
            object: Box::new(object),
            method: method_name,
            args: arguments,
        }
    };

    parse_method_chain(parser, expr)
}

/// Parse argument list including the parentheses: `(arg1, arg2, ...)`
fn parse_argument_list(parser: &mut Parser) -> Result<Vec<Expression>> {
    parser.next_token(); // consume '('
    let args = parse_arguments_until_close(parser)?;
    parser.next_token(); // consume ')'
    Ok(args)
}

/// Parse arguments until ')' is reached (does not consume the ')')
fn parse_arguments_until_close(parser: &mut Parser) -> Result<Vec<Expression>> {
    let mut arguments = vec![];

    if parser.current_token == Token::Symbol(')') {
        return Ok(arguments);
    }

    loop {
        arguments.push(parse_argument(parser)?);

        if parser.current_token == Token::Symbol(')') {
            break;
        }
        if parser.current_token != Token::Symbol(',') {
            return Err(parser.syntax_error("Expected ',' or ')' in function call"));
        }
        parser.next_token(); // consume ','
    }

    Ok(arguments)
}

/// Parse a single argument - handles closures with `(params) { body }` syntax
fn parse_argument(parser: &mut Parser) -> Result<Expression> {
    // Check for closure syntax: (params) { body }
    if parser.current_token == Token::Symbol('(') {
        if let Some(closure) = try_parse_closure(parser)? {
            return Ok(closure);
        }
    }
    parser.parse_expression()
}

/// Try to parse a closure `(params) { body }`, returns None if not a closure
fn try_parse_closure(parser: &mut Parser) -> Result<Option<Expression>> {
    let saved_state = parser.lexer.save_state();
    let saved_current = parser.current_token.clone();
    let saved_peek = parser.peek_token.clone();

    parser.next_token(); // consume '('
    let mut params = vec![];

    // Try parsing parameter list
    while parser.current_token != Token::Symbol(')') && parser.current_token != Token::Eof {
        if let Token::Identifier(param_name) = &parser.current_token {
            let name = param_name.clone();
            parser.next_token();

            // Optional type annotation
            let param_type = if parser.current_token == Token::Symbol(':') {
                parser.next_token();
                Some(parser.parse_type()?)
            } else {
                None
            };

            params.push((name, param_type));

            if parser.current_token == Token::Symbol(',') {
                parser.next_token();
            }
        } else {
            // Not a valid parameter - restore and return None
            parser.lexer.restore_state(saved_state);
            parser.current_token = saved_current;
            parser.peek_token = saved_peek;
            return Ok(None);
        }
    }

    if parser.current_token != Token::Symbol(')') {
        parser.lexer.restore_state(saved_state);
        parser.current_token = saved_current;
        parser.peek_token = saved_peek;
        return Ok(None);
    }

    parser.next_token(); // consume ')'

    // Must be followed by '{' for closure body
    if parser.current_token != Token::Symbol('{') {
        parser.lexer.restore_state(saved_state);
        parser.current_token = saved_current;
        parser.peek_token = saved_peek;
        return Ok(None);
    }

    let body = super::blocks::parse_block_expression(parser)?;
    Ok(Some(Expression::Closure {
        params,
        return_type: None,
        body: Box::new(body),
    }))
}

/// Build a function call for @std or @builtin syntax
fn build_builtin_call(object: &Expression, method_name: &str, args: Vec<Expression>) -> Expression {
    match object {
        Expression::MemberAccess { object: base, member } => match base.as_ref() {
            Expression::StdReference => Expression::FunctionCall {
                name: format!("{}.{}", member, method_name),
                args,
            },
            Expression::BuiltinReference => Expression::FunctionCall {
                name: format!("builtin.{}.{}", member, method_name),
                args,
            },
            _ => unreachable!(),
        },
        Expression::BuiltinReference => Expression::FunctionCall {
            name: format!("builtin.{}", method_name),
            args,
        },
        _ => unreachable!(),
    }
}

/// Parse method chaining after an expression: `.member`, `.method()`, `.val`, etc.
/// Public so it can be reused by other expression parsers (literals, etc.)
pub fn parse_method_chain(parser: &mut Parser, mut expr: Expression) -> Result<Expression> {
    loop {
        if parser.current_token != Token::Symbol('.') {
            break;
        }
        parser.next_token(); // consume '.'

        let member = match &parser.current_token {
            Token::Identifier(name) => name.clone(),
            _ => return Err(parser.syntax_error("Expected identifier after '.'")),
        };
        parser.next_token();

        expr = parse_member_access(parser, expr, member)?;
    }

    Ok(expr)
}

/// Parse a single member access or method call
fn parse_member_access(
    parser: &mut Parser,
    expr: Expression,
    member: String,
) -> Result<Expression> {
    let is_call = parser.current_token == Token::Symbol('(');

    // Pointer operations (only when NOT followed by parentheses)
    if !is_call {
        match member.as_str() {
            "val" => return Ok(Expression::PointerDereference(Box::new(expr))),
            "addr" => return Ok(Expression::PointerAddress(Box::new(expr))),
            _ => {}
        }
    }

    // Reference operations (require empty parentheses)
    if is_call {
        match member.as_str() {
            "ref" => {
                parser.next_token(); // consume '('
                parser.expect_symbol(')')?; // validates and consumes ')'
                return Ok(Expression::CreateReference(Box::new(expr)));
            }
            "mut_ref" => {
                parser.next_token(); // consume '('
                parser.expect_symbol(')')?; // validates and consumes ')'
                return Ok(Expression::CreateMutableReference(Box::new(expr)));
            }
            _ => {}
        }
    }

    // Method call or member access
    if is_call {
        parse_call_expression_with_object(parser, expr, member)
    } else {
        Ok(Expression::MemberAccess {
            object: Box::new(expr),
            member,
        })
    }
}
