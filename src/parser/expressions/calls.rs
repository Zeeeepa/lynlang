use super::super::core::Parser;
use crate::ast::Expression;
use crate::error::{CompileError, Result};
use crate::lexer::Token;

pub fn parse_call_expression(parser: &mut Parser, function_name: String) -> Result<Expression> {
        parser.next_token(); // consume '('
        let mut arguments = vec![];
        if parser.current_token != Token::Symbol(')') {
            loop {
                arguments.push(parser.parse_expression()?);
                if parser.current_token == Token::Symbol(')') {
                    break;
                }
                if parser.current_token != Token::Symbol(',') {
                    return Err(CompileError::SyntaxError(
                        "Expected ',' or ')' in function call".to_string(),
                        Some(parser.current_span.clone()),
                    ));
                }
                parser.next_token();
            }
        }
        parser.next_token(); // consume ')'

        // Check if this is an enum constructor (Some, None, Ok, Err)
        let mut expr = match function_name.as_str() {
            "Some" => {
                if arguments.len() != 1 {
                    return Err(CompileError::SyntaxError(
                        "Some constructor expects exactly one argument".to_string(),
                        Some(parser.current_span.clone()),
                    ));
                }
                Expression::EnumVariant {
                    enum_name: "Option".to_string(),
                    variant: "Some".to_string(),
                    payload: Some(Box::new(arguments.into_iter().next().unwrap())),
                }
            }
            "None" => {
                if !arguments.is_empty() {
                    return Err(CompileError::SyntaxError(
                        "None constructor expects no arguments".to_string(),
                        Some(parser.current_span.clone()),
                    ));
                }
                Expression::EnumVariant {
                    enum_name: "Option".to_string(),
                    variant: "None".to_string(),
                    payload: None,
                }
            }
            "Ok" => {
                if arguments.len() != 1 {
                    return Err(CompileError::SyntaxError(
                        "Ok constructor expects exactly one argument".to_string(),
                        Some(parser.current_span.clone()),
                    ));
                }
                Expression::EnumVariant {
                    enum_name: "Result".to_string(),
                    variant: "Ok".to_string(),
                    payload: Some(Box::new(arguments.into_iter().next().unwrap())),
                }
            }
            "Err" => {
                if arguments.len() != 1 {
                    return Err(CompileError::SyntaxError(
                        "Err constructor expects exactly one argument".to_string(),
                        Some(parser.current_span.clone()),
                    ));
                }
                Expression::EnumVariant {
                    enum_name: "Result".to_string(),
                    variant: "Err".to_string(),
                    payload: Some(Box::new(arguments.into_iter().next().unwrap())),
                }
            }
            _ => Expression::FunctionCall {
                name: function_name,
                args: arguments,
            },
        };

        // Continue parsing method chaining after function call
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

                    // Check for pointer-specific operations and method calls
                    if member == "val" {
                        // Pointer dereference: ptr.val
                        expr = Expression::PointerDereference(Box::new(expr));
                    } else if member == "addr" {
                        // Pointer address: expr.addr
                        expr = Expression::PointerAddress(Box::new(expr));
                    } else if member == "ref" && parser.current_token == Token::Symbol('(') {
                        // Reference creation: expr.ref()
                        parser.next_token(); // consume '('
                        if parser.current_token != Token::Symbol(')') {
                            return Err(CompileError::SyntaxError(
                                "Expected ')' after '.ref('".to_string(),
                                Some(parser.current_span.clone()),
                            ));
                        }
                        parser.next_token(); // consume ')'
                        expr = Expression::CreateReference(Box::new(expr));
                    } else if member == "mut_ref" && parser.current_token == Token::Symbol('(') {
                        // Mutable reference creation: expr.mut_ref()
                        parser.next_token(); // consume '('
                        if parser.current_token != Token::Symbol(')') {
                            return Err(CompileError::SyntaxError(
                                "Expected ')' after '.mut_ref('".to_string(),
                                Some(parser.current_span.clone()),
                            ));
                        }
                        parser.next_token(); // consume ')'
                        expr = Expression::CreateMutableReference(Box::new(expr));
                    } else if parser.current_token == Token::Symbol('(') {
                        return parse_call_expression_with_object(parser, expr, member);
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

pub fn parse_call_expression_with_object(
        parser: &mut Parser,
        object: Expression,
        method_name: String,
    ) -> Result<Expression> {
        parser.next_token(); // consume '('

        // Check if this is a module function call like io.print or math.sqrt
        // These should be treated as qualified function names, not method calls
        let is_module_call = match &object {
            Expression::Identifier(obj_name) => {
                // Common module names from @std
                obj_name == "io"
                    || obj_name == "math"
                    || obj_name == "core"
                    || obj_name == "compiler"
                    || obj_name == "net"
                    || obj_name == "os"
                    || obj_name == "fs"
                    || obj_name == "json"
                    || obj_name == "http"
                    || obj_name == "time"
            }
            // Handle @std.module.function syntax
            Expression::MemberAccess {
                object: base,
                member: _,
            } => {
                // Check if this is @std.module accessing a function
                if let Expression::StdReference = base.as_ref() {
                    // This is @std.module, so it's a module call
                    true
                } else {
                    false
                }
            }
            _ => false,
        };

        let mut arguments = vec![];
        if parser.current_token != Token::Symbol(')') {
            // Special handling for .loop() which takes a closure
            if method_name == "loop" && parser.current_token == Token::Symbol('(') {
                // Parse closure argument for .loop()
                parser.next_token(); // consume '('
                let mut params = vec![];

                // Parse closure parameters
                while parser.current_token != Token::Symbol(')') && parser.current_token != Token::Eof {
                    if let Token::Identifier(param_name) = &parser.current_token {
                        params.push((param_name.clone(), None));
                        parser.next_token();

                        // Check for optional second parameter (index)
                        if parser.current_token == Token::Symbol(',') {
                            parser.next_token();
                            if let Token::Identifier(param_name) = &parser.current_token {
                                params.push((param_name.clone(), None));
                                parser.next_token();
                            }
                        }
                        break; // .loop() only takes 1 or 2 params
                    } else {
                        return Err(CompileError::SyntaxError(
                            "Expected parameter name in closure".to_string(),
                            Some(parser.current_span.clone()),
                        ));
                    }
                }

                if parser.current_token != Token::Symbol(')') {
                    return Err(CompileError::SyntaxError(
                        "Expected ')' after closure parameters".to_string(),
                        Some(parser.current_span.clone()),
                    ));
                }
                parser.next_token(); // consume ')'

                // Parse closure body
                if parser.current_token != Token::Symbol('{') {
                    return Err(CompileError::SyntaxError(
                        "Expected '{' for closure body".to_string(),
                        Some(parser.current_span.clone()),
                    ));
                }

                let body = super::blocks::parse_block_expression(parser)?;
                arguments.push(Expression::Closure {
                    params,
                    return_type: None,
                    body: Box::new(body),
                });
            } else {
                // Regular argument parsing
                loop {
                    // Check for closure syntax: (params) { body }
                    if parser.current_token == Token::Symbol('(') {
                        // Try to parse as closure
                        let saved_pos = parser.lexer.position;
                        let saved_read_pos = parser.lexer.read_position;
                        let saved_char = parser.lexer.current_char;
                        let saved_current = parser.current_token.clone();
                        let saved_peek = parser.peek_token.clone();

                        // Try to parse closure
                        parser.next_token(); // consume '('
                        let mut params = vec![];
                        let mut is_closure = true;

                        while parser.current_token != Token::Symbol(')')
                            && parser.current_token != Token::Eof
                        {
                            if let Token::Identifier(param_name) = &parser.current_token {
                                params.push((param_name.clone(), None));
                                parser.next_token();
                                if parser.current_token == Token::Symbol(',') {
                                    parser.next_token();
                                }
                            } else {
                                // Not a closure, restore and parse as expression
                                is_closure = false;
                                break;
                            }
                        }

                        if is_closure && parser.current_token == Token::Symbol(')') {
                            parser.next_token(); // consume ')'
                            if parser.current_token == Token::Symbol('{') {
                                // It's a closure!
                                let body = super::blocks::parse_block_expression(parser)?;
                                arguments.push(Expression::Closure {
                                    params,
                                    return_type: None,
                                    body: Box::new(body),
                                });
                            } else {
                                // Not a closure, restore and parse as expression
                                parser.lexer.position = saved_pos;
                                parser.lexer.read_position = saved_read_pos;
                                parser.lexer.current_char = saved_char;
                                parser.current_token = saved_current;
                                parser.peek_token = saved_peek;
                                arguments.push(parser.parse_expression()?);
                            }
                        } else {
                            // Not a closure, restore and parse as expression
                            parser.lexer.position = saved_pos;
                            parser.lexer.read_position = saved_read_pos;
                            parser.lexer.current_char = saved_char;
                            parser.current_token = saved_current;
                            parser.peek_token = saved_peek;
                            arguments.push(parser.parse_expression()?);
                        }
                    } else {
                        arguments.push(parser.parse_expression()?);
                    }

                    if parser.current_token == Token::Symbol(')') {
                        break;
                    }
                    if parser.current_token != Token::Symbol(',') {
                        return Err(CompileError::SyntaxError(
                            "Expected ',' or ')' in function call".to_string(),
                            Some(parser.current_span.clone()),
                        ));
                    }
                    parser.next_token();
                }
            }
        }
        parser.next_token(); // consume ')'

        // UFCS implementation: Transform object.method(args) into method(object, args)
        // First, check if this is a stdlib method call (module function)
        let mut expr = if is_module_call {
            // This is a stdlib call like io.print or @std.io.println
            let module_name = match &object {
                Expression::Identifier(name) => name.clone(),
                Expression::MemberAccess {
                    object: base,
                    member,
                } => {
                    // Handle @std.module syntax
                    if let Expression::StdReference = base.as_ref() {
                        member.clone()
                    } else {
                        return Err(CompileError::SyntaxError(
                            "Expected module identifier for method call".to_string(),
                            Some(parser.current_span.clone()),
                        ));
                    }
                }
                _ => {
                    return Err(CompileError::SyntaxError(
                        "Expected identifier for object in method call".to_string(),
                        Some(parser.current_span.clone()),
                    ))
                }
            };
            Expression::FunctionCall {
                name: format!("{}.{}", module_name, method_name),
                args: arguments,
            }
        } else if method_name.contains('.') {
            // This shouldn't happen anymore but keep for compatibility
            Expression::FunctionCall {
                name: format!(
                    "{}.{}",
                    match &object {
                        Expression::Identifier(name) => name.clone(),
                        _ =>
                            return Err(CompileError::SyntaxError(
                                "Expected identifier for object in method call".to_string(),
                                Some(parser.current_span.clone()),
                            )),
                    },
                    method_name
                ),
                args: arguments,
            }
        } else {
            // Special handling for .loop() method on collections
            if method_name == "loop" {
                // Create a MethodCall expression for .loop()
                Expression::MethodCall {
                    object: Box::new(object),
                    method: method_name,
                    args: arguments,
                }
            } else {
                // This is UFC: any function can be called as a method
                // Create a MethodCall expression to be resolved during compilation
                Expression::MethodCall {
                    object: Box::new(object),
                    method: method_name,
                    args: arguments,
                }
            }
        };

        // Continue parsing method chaining after function call
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

                    // Check for pointer-specific operations and method calls
                    if member == "val" {
                        // Pointer dereference: ptr.val
                        expr = Expression::PointerDereference(Box::new(expr));
                    } else if member == "addr" {
                        // Pointer address: expr.addr
                        expr = Expression::PointerAddress(Box::new(expr));
                    } else if member == "ref" && parser.current_token == Token::Symbol('(') {
                        // Reference creation: expr.ref()
                        parser.next_token(); // consume '('
                        if parser.current_token != Token::Symbol(')') {
                            return Err(CompileError::SyntaxError(
                                "Expected ')' after '.ref('".to_string(),
                                Some(parser.current_span.clone()),
                            ));
                        }
                        parser.next_token(); // consume ')'
                        expr = Expression::CreateReference(Box::new(expr));
                    } else if member == "mut_ref" && parser.current_token == Token::Symbol('(') {
                        // Mutable reference creation: expr.mut_ref()
                        parser.next_token(); // consume '('
                        if parser.current_token != Token::Symbol(')') {
                            return Err(CompileError::SyntaxError(
                                "Expected ')' after '.mut_ref('".to_string(),
                                Some(parser.current_span.clone()),
                            ));
                        }
                        parser.next_token(); // consume ')'
                        expr = Expression::CreateMutableReference(Box::new(expr));
                    } else if parser.current_token == Token::Symbol('(') {
                        return parse_call_expression_with_object(parser, expr, member);
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
