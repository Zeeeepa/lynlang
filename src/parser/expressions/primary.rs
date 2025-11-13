use super::super::core::Parser;
use crate::ast::{AstType, Expression, Statement};
use crate::error::{CompileError, Result};
use crate::lexer::Token;

pub fn parse_primary_expression(parser: &mut Parser) -> Result<Expression> {
        match &parser.current_token {
            Token::Identifier(id) if id == "loop" => {
                // Handle loop() function syntax
                parser.next_token(); // consume 'loop'
                if parser.current_token != Token::Symbol('(') {
                    return Err(CompileError::SyntaxError(
                        "Expected '(' after 'loop'".to_string(),
                        Some(parser.current_span.clone()),
                    ));
                }
                parser.next_token(); // consume '('

                // Parse the closure argument
                let loop_body = if parser.current_token == Token::Symbol('(') {
                    // Parse closure parameter list
                    parser.next_token(); // consume '('
                    let mut params = vec![];

                    // Handle empty parameter list
                    if parser.current_token != Token::Symbol(')') {
                        while parser.current_token != Token::Symbol(')')
                            && parser.current_token != Token::Eof
                        {
                            if let Token::Identifier(param_name) = &parser.current_token {
                                params.push((param_name.clone(), None));
                                parser.next_token();

                                if parser.current_token == Token::Symbol(',') {
                                    parser.next_token();
                                } else if parser.current_token != Token::Symbol(')') {
                                    return Err(CompileError::SyntaxError(
                                        "Expected ',' or ')' in closure parameters".to_string(),
                                        Some(parser.current_span.clone()),
                                    ));
                                }
                            } else {
                                return Err(CompileError::SyntaxError(
                                    "Expected parameter name in closure".to_string(),
                                    Some(parser.current_span.clone()),
                                ));
                            }
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
                    Expression::Closure {
                        params,
                        return_type: None,
                        body: Box::new(body),
                    }
                } else {
                    return Err(CompileError::SyntaxError(
                        "Expected closure as argument to loop()".to_string(),
                        Some(parser.current_span.clone()),
                    ));
                };

                if parser.current_token != Token::Symbol(')') {
                    return Err(CompileError::SyntaxError(
                        "Expected ')' after loop closure".to_string(),
                        Some(parser.current_span.clone()),
                    ));
                }
                parser.next_token(); // consume ')'

                // Return a Loop expression
                Ok(Expression::Loop {
                    body: Box::new(loop_body),
                })
            }
            Token::Identifier(id) if id == "break" => {
                // Parse break as an expression
                parser.next_token(); // consume 'break'
                                   // Check for optional label
                let mut label = None;
                let mut value = None;

                // Check if next token could be a label (identifier not followed by an operator)
                if let Token::Identifier(label_name) = &parser.current_token {
                    // Look ahead to see if this is a label or a value expression
                    if matches!(
                        &parser.peek_token,
                        Token::Symbol(_) | Token::Eof
                    ) {
                        label = Some(label_name.clone());
                        parser.next_token();
                    }
                }

                // Check if there's a value expression after break
                if !matches!(
                    &parser.current_token,
                    Token::Symbol('}') | Token::Eof | Token::Symbol(';')
                ) {
                    if label.is_none() {
                        // This must be a value expression
                        value = Some(Box::new(parser.parse_expression()?));
                    }
                }

                Ok(Expression::Break { label, value })
            }
            Token::Identifier(id) if id == "continue" => {
                // Parse continue as an expression
                parser.next_token(); // consume 'continue'
                let label = if let Token::Identifier(label_name) = &parser.current_token {
                    let label_name = label_name.clone();
                    parser.next_token();
                    Some(label_name)
                } else {
                    None
                };
                Ok(Expression::Continue { label })
            }
            Token::Identifier(id) if id == "return" => {
                // Parse return as an expression
                parser.next_token(); // consume 'return'
                let value = if !matches!(
                    &parser.current_token,
                    Token::Symbol('}') | Token::Eof | Token::Symbol(';')
                ) {
                    Box::new(parser.parse_expression()?)
                } else {
                    // Return void/unit if no expression
                    Box::new(Expression::Block(vec![]))
                };
                Ok(Expression::Return(value))
            }
            Token::Identifier(id) if id == "comptime" => {
                parser.next_token(); // consume 'comptime'
                let expr = parser.parse_expression()?;
                Ok(Expression::Comptime(Box::new(expr)))
            }
            Token::Integer(value_str) => {
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
            Token::Float(value_str) => {
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
            Token::StringLiteral(value) => {
                let value = value.clone();
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
            Token::Symbol('.') => {
                // Shorthand enum variant syntax: .VariantName or .VariantName(payload)
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
                return Ok(Expression::EnumLiteral { variant, payload });
            }
            Token::AtStd => {
                // Handle @std token
                parser.next_token();
                let mut expr = Expression::Identifier("@std".to_string());

                // Handle UFC on @std
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
            Token::AtThis => {
                // Handle @this token
                parser.next_token();
                let mut expr = Expression::Identifier("@this".to_string());

                // Handle UFC on @this
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
            Token::Identifier(name) => {
                let name = name.clone();
                parser.next_token();

                // Check for boolean and unit literals first (these don't chain)
                if name == "true" {
                    return Ok(Expression::Boolean(true));
                } else if name == "false" {
                    return Ok(Expression::Boolean(false));
                } else if name == "void" {
                    // void is a unit value - like () in other languages
                    return Ok(Expression::Unit);
                }

                // Check for Vec<T, size>() constructor
                if name == "Vec" && parser.current_token == Token::Operator("<".to_string()) {
                    return super::collections::parse_vec_constructor(parser);
                }

                // Check for DynVec<T>() or DynVec<T1, T2, ...>() constructor
                if name == "DynVec" && parser.current_token == Token::Operator("<".to_string()) {
                    return super::collections::parse_dynvec_constructor(parser);
                }

                // Check for Array<T>() constructor
                if name == "Array" && parser.current_token == Token::Operator("<".to_string()) {
                    return super::collections::parse_array_constructor(parser);
                }

                // Check for Option type constructors: Some(value) and None
                if name == "Some" && parser.current_token == Token::Symbol('(') {
                    parser.next_token(); // consume '('
                    let value = parser.parse_expression()?;
                    if parser.current_token != Token::Symbol(')') {
                        return Err(CompileError::SyntaxError(
                            "Expected ')' after Some value".to_string(),
                            Some(parser.current_span.clone()),
                        ));
                    }
                    parser.next_token(); // consume ')'
                    return Ok(Expression::Some(Box::new(value)));
                } else if name == "None" {
                    return Ok(Expression::None);
                }

                // Check for enum variant syntax: EnumName::VariantName
                if parser.current_token == Token::Operator("::".to_string()) {
                    parser.next_token(); // consume '::'

                    let variant = match &parser.current_token {
                        Token::Identifier(v) => v.clone(),
                        _ => {
                            return Err(CompileError::SyntaxError(
                                "Expected variant name after '::'".to_string(),
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

                    return Ok(Expression::EnumVariant {
                        enum_name: name,
                        variant,
                        payload,
                    });
                }

                // Check for generic type parameters
                // Could be:
                // 1. Generic struct literal: Vec<T> { ... }
                // 2. Generic function call: vec_new<i32>()
                // 3. Comparison: x < y
                // 4. Collection types: HashMap<K,V>.new()
                let (name_with_generics, consumed_generics) =
                    if parser.current_token == Token::Operator("<".to_string()) {
                        // Special handling for Array<T> which needs different treatment
                        if name == "Array" {
                            // Don't consume generics for Array, let it be handled later
                            (name.clone(), false)
                        } else if matches!(name.as_str(), "HashMap" | "HashSet" | "DynVec" | "Vec") {
                            // For collection types, we need to parse and preserve generics
                            // Parse the generic type arguments properly
                            parser.next_token(); // consume '<'
                            let mut type_args = Vec::new();
                            
                            // Parse comma-separated type arguments
                            loop {
                                type_args.push(parser.parse_type()?);
                                
                                if parser.current_token == Token::Symbol(',') {
                                    parser.next_token(); // consume ','
                                } else if parser.current_token == Token::Operator(">".to_string()) {
                                    parser.next_token(); // consume '>'
                                    break;
                                } else {
                                    return Err(CompileError::SyntaxError(
                                        "Expected ',' or '>' in generic type arguments".to_string(),
                                        Some(parser.current_span.clone()),
                                    ));
                                }
                            }
                            
                            // Format the name with the actual type arguments
                            let type_args_str = type_args.iter()
                                .map(|t| format!("{}", t))  // Use Display instead of Debug
                                .collect::<Vec<_>>()
                                .join(", ");
                            (format!("{}<{}>", name, type_args_str), true)
                        } else if super::collections::looks_like_generic_type_args(parser) {
                            // For other potential generic types
                            // Parse the generic type arguments properly
                            parser.next_token(); // consume '<'
                            let mut type_args = Vec::new();
                            
                            // Parse comma-separated type arguments
                            loop {
                                type_args.push(parser.parse_type()?);
                                
                                if parser.current_token == Token::Symbol(',') {
                                    parser.next_token(); // consume ','
                                } else if parser.current_token == Token::Operator(">".to_string()) {
                                    parser.next_token(); // consume '>'
                                    break;
                                } else {
                                    return Err(CompileError::SyntaxError(
                                        "Expected ',' or '>' in generic type arguments".to_string(),
                                        Some(parser.current_span.clone()),
                                    ));
                                }
                            }
                            
                            // Format the name with the actual type arguments
                            let type_args_str = type_args.iter()
                                .map(|t| format!("{}", t))  // Use Display instead of Debug
                                .collect::<Vec<_>>()
                                .join(", ");
                            (format!("{}<{}>", name, type_args_str), true)
                        } else {
                            // Not generic type args, probably a comparison
                            (name.clone(), false)
                        }
                    } else {
                        (name.clone(), false)
                    };

                // Check for struct literal syntax: Name { field: value, ... } or Name ( field: value, ... )
                if parser.current_token == Token::Symbol('{') {
                    return super::structs::parse_struct_literal(parser, name_with_generics, '{', '}');
                }
                
                // Check for struct literal with parentheses: Name ( field: value, ... )
                if parser.current_token == Token::Symbol('(') {
                    // Peek ahead to distinguish struct literal from function call
                    // Struct literal: Person ( age: 20, name: "John" )
                    // Function call: Person ( arg1, arg2 )
                    // We look for the pattern: identifier followed by ':'
                    let saved_pos = parser.lexer.position;
                    let saved_read_pos = parser.lexer.read_position;
                    let saved_char = parser.lexer.current_char;
                    let saved_token = parser.current_token.clone();
                    let saved_peek = parser.peek_token.clone();
                    
                    parser.next_token(); // consume '('
                    
                    // The lexer skips whitespace automatically, so we just check the next token
                    // Check if first token is identifier followed by ':'
                    let looks_like_struct_literal = if let Token::Identifier(_) = &parser.current_token {
                        parser.next_token();
                        parser.current_token == Token::Symbol(':')
                    } else {
                        false
                    };
                    
                    // Restore state
                    parser.lexer.position = saved_pos;
                    parser.lexer.read_position = saved_read_pos;
                    parser.lexer.current_char = saved_char;
                    parser.current_token = saved_token;
                    parser.peek_token = saved_peek;
                    
                    if looks_like_struct_literal {
                        return super::structs::parse_struct_literal(parser, name_with_generics, '(', ')');
                    }
                    // Otherwise, fall through to function call handling
                }

                // Check for function call with generics: vec_new<i32>()
                if consumed_generics && parser.current_token == Token::Symbol('(') {
                    return super::calls::parse_call_expression(parser, name_with_generics);
                }

                // Initialize expression based on special identifiers or regular names
                let mut expr = if name == "@std" {
                    Expression::StdReference
                } else if name == "@this" {
                    Expression::ThisReference
                } else if name == "None" {
                    // Handle None without parentheses
                    Expression::EnumVariant {
                        enum_name: "Option".to_string(),
                        variant: "None".to_string(),
                        payload: None,
                    }
                } else if consumed_generics {
                    // If we consumed generics, use the name with generics
                    // This handles cases like HashMap<K,V>.new()
                    Expression::Identifier(name_with_generics)
                } else {
                    Expression::Identifier(name)
                };

                // Handle member access, array indexing, and function calls
                loop {
                    match &parser.current_token {
                        Token::Symbol('.') => {
                            parser.next_token(); // consume '.'

                            // Handle special tokens that can appear after '.'
                            if let Token::Identifier(id) = &parser.current_token {
                                if id == "loop" {
                                    // Handle .loop() method on collections
                                    parser.next_token(); // consume 'loop'
                                    if parser.current_token != Token::Symbol('(') {
                                        return Err(CompileError::SyntaxError(
                                            "Expected '(' after '.loop'".to_string(),
                                            Some(parser.current_span.clone()),
                                        ));
                                    }
                                    parser.next_token(); // consume '('

                                    // Parse the closure parameter
                                    if parser.current_token != Token::Symbol('(') {
                                        return Err(CompileError::SyntaxError(
                                            "Expected '(' for loop closure parameter".to_string(),
                                            Some(parser.current_span.clone()),
                                        ));
                                    }
                                    parser.next_token(); // consume '('

                                    // Get the parameter name(s)
                                    let param = if let Token::Identifier(p) = &parser.current_token {
                                        p.clone()
                                    } else {
                                        return Err(CompileError::SyntaxError(
                                            "Expected parameter name in loop closure".to_string(),
                                            Some(parser.current_span.clone()),
                                        ));
                                    };
                                    parser.next_token();

                                    // Check for optional index parameter
                                    let index_param = if parser.current_token == Token::Symbol(',') {
                                        parser.next_token(); // consume ','
                                        if let Token::Identifier(idx) = &parser.current_token {
                                            let idx_name = idx.clone();
                                            parser.next_token();
                                            Some(idx_name)
                                        } else {
                                            return Err(CompileError::SyntaxError(
                                                "Expected index parameter name after ','"
                                                    .to_string(),
                                                Some(parser.current_span.clone()),
                                            ));
                                        }
                                    } else {
                                        None
                                    };

                                    if parser.current_token != Token::Symbol(')') {
                                        return Err(CompileError::SyntaxError(
                                            "Expected ')' after loop parameters".to_string(),
                                            Some(parser.current_span.clone()),
                                        ));
                                    }
                                    parser.next_token(); // consume ')'

                                    // Parse the body block
                                    if parser.current_token != Token::Symbol('{') {
                                        return Err(CompileError::SyntaxError(
                                            "Expected '{' for loop body".to_string(),
                                            Some(parser.current_span.clone()),
                                        ));
                                    }
                                    let body = super::blocks::parse_block_expression(parser)?;

                                    if parser.current_token != Token::Symbol(')') {
                                        return Err(CompileError::SyntaxError(
                                            "Expected ')' to close loop call".to_string(),
                                            Some(parser.current_span.clone()),
                                        ));
                                    }
                                    parser.next_token(); // consume ')'

                                    expr = Expression::CollectionLoop {
                                        collection: Box::new(expr),
                                        param,
                                        index_param,
                                        body: Box::new(body),
                                    };
                                    continue; // Continue the loop, no member to process
                                }
                            }

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

                            // Check for pointer-specific operations first
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
                            } else if member == "mut_ref"
                                && parser.current_token == Token::Symbol('(')
                            {
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
                            } else if member == "raise" && parser.current_token == Token::Symbol('(')
                            {
                                parser.next_token(); // consume '('
                                if parser.current_token != Token::Symbol(')') {
                                    return Err(CompileError::SyntaxError(
                                        "Expected ')' after 'raise('".to_string(),
                                        Some(parser.current_span.clone()),
                                    ));
                                }
                                parser.next_token(); // consume ')'
                                expr = Expression::Raise(Box::new(expr));
                            } else if member == "step" && parser.current_token == Token::Symbol('(') {
                                // Handle range.step(n) syntax for stepped ranges
                                parser.next_token(); // consume '('
                                let step_value = parser.parse_expression()?;
                                if parser.current_token != Token::Symbol(')') {
                                    return Err(CompileError::SyntaxError(
                                        "Expected ')' after step value".to_string(),
                                        Some(parser.current_span.clone()),
                                    ));
                                }
                                parser.next_token(); // consume ')'
                                                   // For now, treat as a method call - could be optimized later
                                expr = Expression::MethodCall {
                                    object: Box::new(expr),
                                    method: "step".to_string(),
                                    args: vec![step_value],
                                };
                            } else {
                                // Check if this could be an enum variant constructor (EnumName.Variant)
                                // First letter capitalized usually indicates a type/variant name
                                if member.chars().next().map_or(false, |c| c.is_uppercase()) {
                                    // Check if the base is an identifier (potential enum name)
                                    if let Expression::Identifier(enum_name) = &expr {
                                        // This looks like an enum variant constructor
                                        expr = Expression::EnumVariant {
                                            enum_name: enum_name.clone(),
                                            variant: member,
                                            payload: None,
                                        };
                                    } else {
                                        // Regular member access
                                        expr = Expression::MemberAccess {
                                            object: Box::new(expr),
                                            member,
                                        };
                                    }
                                } else {
                                    // Regular member access
                                    expr = Expression::MemberAccess {
                                        object: Box::new(expr),
                                        member,
                                    };
                                }
                            }
                        }
                        Token::Symbol('[') => {
                            // Array indexing
                            parser.next_token(); // consume '['
                            let index = parser.parse_expression()?;
                            if parser.current_token != Token::Symbol(']') {
                                return Err(CompileError::SyntaxError(
                                    "Expected ']' after array index".to_string(),
                                    Some(parser.current_span.clone()),
                                ));
                            }
                            parser.next_token(); // consume ']'
                            expr = Expression::ArrayIndex {
                                array: Box::new(expr),
                                index: Box::new(index),
                            };
                        }
                        Token::Symbol('(') => {
                            // Function call or enum variant constructor with payload
                            if let Expression::MemberAccess { object, member } = expr {
                                return super::calls::parse_call_expression_with_object(parser, *object, member);
                            } else if let Expression::Identifier(name) = expr {
                                return super::calls::parse_call_expression(parser, name);
                            } else if let Expression::EnumVariant {
                                enum_name,
                                variant,
                                payload: None,
                            } = expr
                            {
                                // This is an enum variant constructor with payload
                                parser.next_token(); // consume '('
                                let payload_expr = parser.parse_expression()?;
                                if parser.current_token != Token::Symbol(')') {
                                    return Err(CompileError::SyntaxError(
                                        "Expected ')' after enum variant payload".to_string(),
                                        Some(parser.current_span.clone()),
                                    ));
                                }
                                parser.next_token(); // consume ')'
                                expr = Expression::EnumVariant {
                                    enum_name,
                                    variant,
                                    payload: Some(Box::new(payload_expr)),
                                };
                            } else {
                                return Err(CompileError::SyntaxError(
                                    "Unexpected expression type for function call".to_string(),
                                    Some(parser.current_span.clone()),
                                ));
                            }
                        }
                        _ => break,
                    }
                }

                Ok(expr)
            }
            Token::Symbol('(') => {
                // Try to determine if this is a closure or a parenthesized expression
                // We'll use a simple heuristic: if we see (ident) { or (ident, ...) {
                // then it's likely a closure

                parser.next_token(); // consume '('

                // Check for empty closure: () { ... } or () => expr
                if parser.current_token == Token::Symbol(')') {
                    parser.next_token(); // consume ')'

                    // Check for arrow function syntax: () => expr
                    if parser.current_token == Token::Operator("=>".to_string()) {
                        parser.next_token(); // consume '=>'
                        let body_expr = parser.parse_expression()?;
                        // Convert expression to block that returns the value
                        let body = Expression::Block(vec![Statement::Expression(body_expr)]);
                        return Ok(Expression::Closure {
                            params: vec![],
                            return_type: None,
                            body: Box::new(body),
                        });
                    } else if parser.current_token == Token::Symbol('{') {
                        // It's a closure with no parameters: () { ... }
                        let body = super::blocks::parse_block_expression(parser)?;
                        return Ok(Expression::Closure {
                            params: vec![],
                            return_type: None,
                            body: Box::new(body),
                        });
                    }
                    // Check for return type: () ReturnType { ... }
                    else if matches!(&parser.current_token, Token::Identifier(_)) {
                        // This could be a return type
                        let return_type = parser.parse_type()?;
                        if parser.current_token == Token::Symbol('{') {
                            // It's a closure with return type: () ReturnType { ... }
                            let body = super::blocks::parse_block_expression(parser)?;
                            return Ok(Expression::Closure {
                                params: vec![],
                                return_type: Some(return_type),
                                body: Box::new(body),
                            });
                        } else {
                            return Err(CompileError::SyntaxError(
                                "Expected '{' after closure return type".to_string(),
                                Some(parser.current_span.clone()),
                            ));
                        }
                    } else {
                        // Empty parens are not valid as an expression
                        return Err(CompileError::SyntaxError(
                            "Empty parentheses are not a valid expression. Did you mean to write a closure? Use '() { ... }' or '() => expr'".to_string(),
                            Some(parser.current_span.clone()),
                        ));
                    }
                }

                // Try to parse as closure with parameters
                // A closure looks like: (param) { ... } or (param1, param2) { ... }
                // or with types: (param: Type) { ... }
                let mut is_closure = false;
                let mut closure_return_type: Option<AstType> = None;
                let mut params = vec![];

                // Check if this looks like a closure parameter list
                if let Token::Identifier(param_name) = &parser.current_token {
                    let first_param = param_name.clone();
                    parser.next_token();

                    // Check for optional type annotation
                    let param_type = if parser.current_token == Token::Symbol(':') {
                        parser.next_token(); // consume ':'
                        Some(parser.parse_type()?)
                    } else {
                        None
                    };

                    params.push((first_param, param_type));

                    // Check for more parameters
                    while parser.current_token == Token::Symbol(',') {
                        parser.next_token(); // consume ','

                        if let Token::Identifier(param_name) = &parser.current_token {
                            let param = param_name.clone();
                            parser.next_token();

                            // Check for optional type annotation
                            let param_type = if parser.current_token == Token::Symbol(':') {
                                parser.next_token(); // consume ':'
                                Some(parser.parse_type()?)
                            } else {
                                None
                            };

                            params.push((param, param_type));
                        } else {
                            // Not a valid parameter list, reset and parse as expression
                            break;
                        }
                    }

                    // Check if we have a valid closure pattern
                    if parser.current_token == Token::Symbol(')') {
                        parser.next_token(); // consume ')'

                        // Check for arrow function syntax: () => expr
                        if parser.current_token == Token::Operator("=>".to_string()) {
                            parser.next_token(); // consume '=>'
                                               // Parse the expression body
                            let body_expr = parser.parse_expression()?;
                            // Convert expression to block that returns the value
                            let body = Expression::Block(vec![Statement::Expression(body_expr)]);
                            return Ok(Expression::Closure {
                                params,
                                return_type: None,
                                body: Box::new(body),
                            });
                        }
                        // Check for the body
                        else if parser.current_token == Token::Symbol('{') {
                            is_closure = true;
                            closure_return_type = None;
                        }
                        // Check for return type: (params) ReturnType { ... }
                        else if matches!(&parser.current_token, Token::Identifier(_)) {
                            // Return type specified, check for body after
                            let return_type = parser.parse_type()?;
                            if parser.current_token == Token::Symbol('{') {
                                is_closure = true;
                                closure_return_type = Some(return_type);
                            }
                        }
                    }

                    if is_closure {
                        // Parse the closure body
                        let body = super::blocks::parse_block_expression(parser)?;
                        return Ok(Expression::Closure {
                            params,
                            return_type: closure_return_type,
                            body: Box::new(body),
                        });
                    } else {
                        // Not a closure, we have an issue with parsing
                        // Since we consumed tokens trying to parse as closure, we can't backtrack
                        return Err(CompileError::SyntaxError(
                            "Ambiguous syntax: Use explicit type annotations for closure parameters or parenthesize complex expressions".to_string(),
                            Some(parser.current_span.clone()),
                        ));
                    }
                } else {
                    // Not starting with an identifier, so it's definitely not a closure
                    // Parse as a parenthesized expression
                    let mut expr = parser.parse_expression()?;
                    if parser.current_token != Token::Symbol(')') {
                        return Err(CompileError::SyntaxError(
                            format!(
                                "Expected ')' after parenthesized expression, got {:?}",
                                parser.current_token
                            ),
                            Some(parser.current_span.clone()),
                        ));
                    }
                    parser.next_token(); // consume ')'

                    // Check for UFC method chaining after parenthesized expression
                    loop {
                        match &parser.current_token {
                            Token::Symbol('.') => {
                                // Member access or method call
                                parser.next_token();
                                if let Token::Identifier(member) = &parser.current_token.clone() {
                                    let member_clone = member.clone();
                                    parser.next_token();

                                    // Check if it's a method call
                                    if parser.current_token == Token::Symbol('(') {
                                        expr = super::calls::parse_call_expression_with_object(
                                            parser,
                                            expr,
                                            member_clone,
                                        )?;
                                    } else {
                                        // Member access
                                        expr = Expression::MemberAccess {
                                            object: Box::new(expr),
                                            member: member_clone,
                                        };
                                    }
                                } else {
                                    return Err(CompileError::SyntaxError(
                                        format!("Expected identifier after '.' in member access, got {:?}", parser.current_token),
                                        Some(parser.current_span.clone()),
                                    ));
                                }
                            }
                            _ => break,
                        }
                    }

                    Ok(expr)
                }
            }
            Token::Symbol('[') => {
                // Array literal: [expr, expr, ...]
                super::collections::parse_array_literal(parser)
            }
            Token::Symbol('{') => {
                // Block expression: { statements... }
                super::blocks::parse_block_expression(parser)
            }
            _ => Err(CompileError::SyntaxError(
                format!("Unexpected token: {:?}", parser.current_token),
                Some(parser.current_span.clone()),
            )),
        }
    }

