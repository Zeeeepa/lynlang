use super::core::Parser;
use crate::ast::{Expression, BinaryOperator, Pattern, MatchArm};
use crate::error::{CompileError, Result};
use crate::lexer::Token;

impl<'a> Parser<'a> {
    pub fn parse_expression(&mut self) -> Result<Expression> {
        self.parse_binary_expression(0)
    }

    fn parse_binary_expression(&mut self, precedence: u8) -> Result<Expression> {
        let mut left = self.parse_unary_expression()?;
        
        loop {
            // Check for 'as' identifier for type casting
            if let Token::Identifier(id) = &self.current_token {
                if id == "as" {
                    self.next_token(); // consume 'as'
                    let target_type = self.parse_type()?;
                    left = Expression::TypeCast {
                        expr: Box::new(left),
                        target_type,
                    };
                } else {
                    break;
                }
            } else if let Token::Operator(op) = &self.current_token {
                let op_clone = op.clone();
                let next_prec = self.get_precedence(&op_clone);
                if next_prec > precedence {
                    self.next_token(); // advance past the operator
                    
                    // Handle range expressions specially
                    if op_clone == ".." || op_clone == "..=" {
                        let right = self.parse_binary_expression(next_prec)?;
                        left = Expression::Range {
                            start: Box::new(left),
                            end: Box::new(right),
                            inclusive: op_clone == "..=",
                        };
                    } else {
                        let right = self.parse_binary_expression(next_prec)?;
                        left = Expression::BinaryOp {
                            left: Box::new(left),
                            op: self.token_to_binary_operator(&op_clone)?,
                            right: Box::new(right),
                        };
                    }
                } else {
                    break;
                }
            } else if self.current_token == Token::Question {
                // Handle pattern matching with low precedence (but higher than assignment)
                // This ensures x < y ? ... parses as (x < y) ? ...
                if precedence < 1 {  // Pattern match has very low precedence
                    self.next_token(); // consume '?'
                    left = self.parse_pattern_match(left)?;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        
        Ok(left)
    }

    fn parse_unary_expression(&mut self) -> Result<Expression> {
        match &self.current_token {
            Token::Operator(op) if op == "-" => {
                self.next_token();
                let expr = self.parse_unary_expression()?;
                // For now, represent unary minus as BinaryOp with 0 - expr
                Ok(Expression::BinaryOp {
                    left: Box::new(Expression::Integer32(0)),
                    op: BinaryOperator::Subtract,
                    right: Box::new(expr),
                })
            }
            Token::Operator(op) if op == "!" => {
                self.next_token();
                let expr = self.parse_unary_expression()?;
                // For now, represent logical not as a function call
                Ok(Expression::FunctionCall {
                    name: "not".to_string(),
                    args: vec![expr],
                })
            }
            _ => self.parse_postfix_expression(),
        }
    }

    fn parse_postfix_expression(&mut self) -> Result<Expression> {
        let expr = self.parse_primary_expression()?;
        
        // Pattern matching is now handled in binary expression parsing
        // to get correct precedence
        
        Ok(expr)
    }

    fn parse_primary_expression(&mut self) -> Result<Expression> {
        match &self.current_token {
            Token::Identifier(id) if id == "comptime" => {
                self.next_token(); // consume 'comptime'
                let expr = self.parse_expression()?;
                Ok(Expression::Comptime(Box::new(expr)))
            }
            Token::Integer(value_str) => {
                let value = value_str.parse::<i64>().map_err(|_| {
                    CompileError::SyntaxError(
                        format!("Invalid integer: {}", value_str),
                        Some(self.current_span.clone()),
                    )
                })?;
                self.next_token();
                // Default to Integer32 unless out of range
                if value <= i32::MAX as i64 && value >= i32::MIN as i64 {
                    Ok(Expression::Integer32(value as i32))
                } else {
                    Ok(Expression::Integer64(value))
                }
            }
            Token::Float(value_str) => {
                let value = value_str.parse::<f64>().map_err(|_| {
                    CompileError::SyntaxError(
                        format!("Invalid float: {}", value_str),
                        Some(self.current_span.clone()),
                    )
                })?;
                self.next_token();
                Ok(Expression::Float64(value))
            }
            Token::StringLiteral(value) => {
                let value = value.clone();
                self.next_token();
                
                // Check if the string contains interpolation syntax
                if value.contains("${") {
                    self.parse_interpolated_string(value)
                } else {
                    Ok(Expression::String(value))
                }
            }
            Token::Symbol('.') => {
                // Shorthand enum variant syntax: .VariantName or .VariantName(payload)
                self.next_token(); // consume '.'
                
                let variant = match &self.current_token {
                    Token::Identifier(v) => v.clone(),
                    _ => {
                        return Err(CompileError::SyntaxError(
                            "Expected variant name after '.'".to_string(),
                            Some(self.current_span.clone()),
                        ));
                    }
                };
                self.next_token();
                
                // Check for variant payload
                let payload = if self.current_token == Token::Symbol('(') {
                    self.next_token(); // consume '('
                    let expr = self.parse_expression()?;
                    if self.current_token != Token::Symbol(')') {
                        return Err(CompileError::SyntaxError(
                            "Expected ')' after enum variant payload".to_string(),
                            Some(self.current_span.clone()),
                        ));
                    }
                    self.next_token(); // consume ')'
                    Some(Box::new(expr))
                } else {
                    None
                };
                
                // Use EnumLiteral for shorthand syntax
                return Ok(Expression::EnumLiteral {
                    variant,
                    payload,
                });
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.next_token();
                
                // Check for special identifiers
                if name == "@std" {
                    return Ok(Expression::StdReference);
                } else if name == "@this" {
                    return Ok(Expression::ThisReference);
                }
                
                // Check for boolean literals
                if name == "true" {
                    return Ok(Expression::Boolean(true));
                } else if name == "false" {
                    return Ok(Expression::Boolean(false));
                } else if name == "break" {
                    // Parse break as an expression
                    let label = if let Token::Identifier(label_name) = &self.current_token {
                        let label_name = label_name.clone();
                        self.next_token();
                        Some(label_name)
                    } else {
                        None
                    };
                    return Ok(Expression::Break { label });
                } else if name == "continue" {
                    // Parse continue as an expression
                    let label = if let Token::Identifier(label_name) = &self.current_token {
                        let label_name = label_name.clone();
                        self.next_token();
                        Some(label_name)
                    } else {
                        None
                    };
                    return Ok(Expression::Continue { label });
                }
                
                // Check for loop() function syntax
                if name == "loop" && self.current_token == Token::Symbol('(') {
                    self.next_token(); // consume '('
                    
                    // Parse the closure argument
                    let loop_body = if self.current_token == Token::Symbol('(') {
                        // Parse closure parameter list
                        self.next_token(); // consume '('
                        let mut params = vec![];
                        
                        while self.current_token != Token::Symbol(')') && self.current_token != Token::Eof {
                            if let Token::Identifier(param_name) = &self.current_token {
                                params.push((param_name.clone(), None));
                                self.next_token();
                                
                                if self.current_token == Token::Symbol(',') {
                                    self.next_token();
                                } else if self.current_token != Token::Symbol(')') {
                                    return Err(CompileError::SyntaxError(
                                        "Expected ',' or ')' in closure parameters".to_string(),
                                        Some(self.current_span.clone()),
                                    ));
                                }
                            } else {
                                return Err(CompileError::SyntaxError(
                                    "Expected parameter name in closure".to_string(),
                                    Some(self.current_span.clone()),
                                ));
                            }
                        }
                        
                        if self.current_token != Token::Symbol(')') {
                            return Err(CompileError::SyntaxError(
                                "Expected ')' after closure parameters".to_string(),
                                Some(self.current_span.clone()),
                            ));
                        }
                        self.next_token(); // consume ')'
                        
                        // Parse closure body
                        if self.current_token != Token::Symbol('{') {
                            return Err(CompileError::SyntaxError(
                                "Expected '{' for closure body".to_string(),
                                Some(self.current_span.clone()),
                            ));
                        }
                        
                        let body = self.parse_block_expression()?;
                        Expression::Closure {
                            params,
                            body: Box::new(body),
                        }
                    } else {
                        return Err(CompileError::SyntaxError(
                            "Expected closure as argument to loop()".to_string(),
                            Some(self.current_span.clone()),
                        ));
                    };
                    
                    if self.current_token != Token::Symbol(')') {
                        return Err(CompileError::SyntaxError(
                            "Expected ')' after loop closure".to_string(),
                            Some(self.current_span.clone()),
                        ));
                    }
                    self.next_token(); // consume ')'
                    
                    // Return a Loop expression
                    return Ok(Expression::Loop {
                        body: Box::new(loop_body),
                    });
                }
                
                // Check for enum variant syntax: EnumName::VariantName
                if self.current_token == Token::Operator("::".to_string()) {
                    self.next_token(); // consume '::'
                    
                    let variant = match &self.current_token {
                        Token::Identifier(v) => v.clone(),
                        _ => {
                            return Err(CompileError::SyntaxError(
                                "Expected variant name after '::'".to_string(),
                                Some(self.current_span.clone()),
                            ));
                        }
                    };
                    self.next_token();
                    
                    // Check for variant payload
                    let payload = if self.current_token == Token::Symbol('(') {
                        self.next_token(); // consume '('
                        let expr = self.parse_expression()?;
                        if self.current_token != Token::Symbol(')') {
                            return Err(CompileError::SyntaxError(
                                "Expected ')' after enum variant payload".to_string(),
                                Some(self.current_span.clone()),
                            ));
                        }
                        self.next_token(); // consume ')'
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
                let (name_with_generics, consumed_generics) = if self.current_token == Token::Operator("<".to_string()) {
                    // Try to parse generic type arguments
                    // Save the position in case we need to backtrack
                    let _saved_pos = self.current_span.start;
                    
                    // Look ahead to determine if this is really generic syntax
                    if self.looks_like_generic_type_args() {
                        // Consume the generic type arguments
                        let mut depth = 1;
                        self.next_token(); // consume '<'
                        while depth > 0 && self.current_token != Token::Eof {
                            match &self.current_token {
                                Token::Operator(op) if op == "<" => depth += 1,
                                Token::Operator(op) if op == ">" => depth -= 1,
                                _ => {}
                            }
                            self.next_token();
                        }
                        // For now, we just track that we consumed generics
                        // In the future, we should preserve the actual type arguments
                        (format!("{}<>", name), true)
                    } else {
                        // Not generic type args, probably a comparison
                        (name.clone(), false)
                    }
                } else {
                    (name.clone(), false)
                };
                
                // Check for struct literal syntax: Name { field: value, ... }
                if self.current_token == Token::Symbol('{') {
                    return self.parse_struct_literal(name_with_generics);
                }
                
                // Check for function call with generics: vec_new<i32>()
                if consumed_generics && self.current_token == Token::Symbol('(') {
                    return self.parse_call_expression(name_with_generics);
                }
                
                let mut expr = Expression::Identifier(name);
                
                // Handle member access, array indexing, and function calls
                loop {
                    match &self.current_token {
                        Token::Symbol('.') => {
                            self.next_token(); // consume '.'
                            
                            let member = match &self.current_token {
                                Token::Identifier(name) => name.clone(),
                                _ => {
                                    return Err(CompileError::SyntaxError(
                                        "Expected identifier after '.'".to_string(),
                                        Some(self.current_span.clone()),
                                    ));
                                }
                            };
                            self.next_token();
                            
                            // Check for .raise() with empty parens
                            if member == "raise" && self.current_token == Token::Symbol('(') {
                                self.next_token(); // consume '('
                                if self.current_token != Token::Symbol(')') {
                                    return Err(CompileError::SyntaxError(
                                        "Expected ')' after 'raise('".to_string(),
                                        Some(self.current_span.clone()),
                                    ));
                                }
                                self.next_token(); // consume ')'
                                expr = Expression::Raise(Box::new(expr));
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
                            self.next_token(); // consume '['
                            let index = self.parse_expression()?;
                            if self.current_token != Token::Symbol(']') {
                                return Err(CompileError::SyntaxError(
                                    "Expected ']' after array index".to_string(),
                                    Some(self.current_span.clone()),
                                ));
                            }
                            self.next_token(); // consume ']'
                            expr = Expression::ArrayIndex {
                                array: Box::new(expr),
                                index: Box::new(index),
                            };
                        }
                        Token::Symbol('(') => {
                            // Function call
                            if let Expression::MemberAccess { object, member } = expr {
                                return self.parse_call_expression_with_object(*object, member);
                            } else if let Expression::Identifier(name) = expr {
                                return self.parse_call_expression(name);
                            } else {
                                return Err(CompileError::SyntaxError(
                                    "Unexpected expression type for function call".to_string(),
                                    Some(self.current_span.clone()),
                                ));
                            }
                        }
                        _ => break,
                    }
                }
                
                Ok(expr)
            }
            Token::Symbol('(') => {
                self.next_token();
                let mut expr = self.parse_expression()?;
                if self.current_token != Token::Symbol(')') {
                    return Err(CompileError::SyntaxError(
                        "Expected closing parenthesis".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
                self.next_token();
                
                // Handle member access, array indexing, and function calls after parenthesized expression
                loop {
                    match &self.current_token {
                        Token::Symbol('.') => {
                            self.next_token(); // consume '.'
                            
                            let member = match &self.current_token {
                                Token::Identifier(name) => name.clone(),
                                _ => {
                                    return Err(CompileError::SyntaxError(
                                        "Expected identifier after '.'".to_string(),
                                        Some(self.current_span.clone()),
                                    ));
                                }
                            };
                            self.next_token();
                            
                            // Check for .raise() with empty parens
                            if member == "raise" && self.current_token == Token::Symbol('(') {
                                self.next_token(); // consume '('
                                if self.current_token != Token::Symbol(')') {
                                    return Err(CompileError::SyntaxError(
                                        "Expected ')' after 'raise('".to_string(),
                                        Some(self.current_span.clone()),
                                    ));
                                }
                                self.next_token(); // consume ')'
                                expr = Expression::Raise(Box::new(expr));
                            } else {
                                expr = Expression::MemberAccess {
                                    object: Box::new(expr),
                                    member,
                                };
                            }
                        }
                        Token::Symbol('[') => {
                            // Array indexing
                            self.next_token(); // consume '['
                            let index = self.parse_expression()?;
                            if self.current_token != Token::Symbol(']') {
                                return Err(CompileError::SyntaxError(
                                    "Expected ']' after array index".to_string(),
                                    Some(self.current_span.clone()),
                                ));
                            }
                            self.next_token(); // consume ']'
                            expr = Expression::ArrayIndex {
                                array: Box::new(expr),
                                index: Box::new(index),
                            };
                        }
                        Token::Symbol('(') => {
                            // Function call
                            if let Expression::MemberAccess { object, member } = expr {
                                return self.parse_call_expression_with_object(*object, member);
                            } else {
                                return Err(CompileError::SyntaxError(
                                    "Cannot call non-identifier expression".to_string(),
                                    Some(self.current_span.clone()),
                                ));
                            }
                        }
                        _ => break,
                    }
                }
                
                Ok(expr)
            }
            Token::Symbol('[') => {
                // Array literal: [expr, expr, ...]
                self.parse_array_literal()
            }
            Token::Symbol('{') => {
                // Block expression: { statements... }
                self.parse_block_expression()
            }
            _ => Err(CompileError::SyntaxError(
                format!("Unexpected token: {:?}", self.current_token),
                Some(self.current_span.clone()),
            )),
        }
    }

    fn parse_call_expression(&mut self, function_name: String) -> Result<Expression> {
        self.next_token(); // consume '('
        let mut arguments = vec![];
        if self.current_token != Token::Symbol(')') {
            loop {
                arguments.push(self.parse_expression()?);
                if self.current_token == Token::Symbol(')') {
                    break;
                }
                if self.current_token != Token::Symbol(',') {
                    return Err(CompileError::SyntaxError(
                        "Expected ',' or ')' in function call".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
                self.next_token();
            }
        }
        self.next_token(); // consume ')'
        
        let mut expr = Expression::FunctionCall {
            name: function_name,
            args: arguments,
        };
        
        // Continue parsing method chaining after function call
        loop {
            match &self.current_token {
                Token::Symbol('.') => {
                    self.next_token(); // consume '.'
                    
                    let member = match &self.current_token {
                        Token::Identifier(name) => name.clone(),
                        _ => {
                            return Err(CompileError::SyntaxError(
                                "Expected identifier after '.'".to_string(),
                                Some(self.current_span.clone()),
                            ));
                        }
                    };
                    self.next_token();
                    
                    // Check for method call
                    if self.current_token == Token::Symbol('(') {
                        return self.parse_call_expression_with_object(expr, member);
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

    fn parse_call_expression_with_object(&mut self, object: Expression, method_name: String) -> Result<Expression> {
        self.next_token(); // consume '('
        
        // Check if this is a module function call like io.print or math.sqrt
        // These should be treated as qualified function names, not method calls
        let is_module_call = match &object {
            Expression::Identifier(obj_name) => {
                // Common module names from @std
                obj_name == "io" || obj_name == "math" || obj_name == "core" || 
                obj_name == "net" || obj_name == "os" || obj_name == "fs" ||
                obj_name == "json" || obj_name == "http" || obj_name == "time"
            }
            _ => false,
        };
        
        let mut arguments = vec![];
        if self.current_token != Token::Symbol(')') {
            // Special handling for .loop() which takes a closure
            if method_name == "loop" && self.current_token == Token::Symbol('(') {
                // Parse closure argument for .loop()
                self.next_token(); // consume '('
                let mut params = vec![];
                
                // Parse closure parameters
                while self.current_token != Token::Symbol(')') && self.current_token != Token::Eof {
                    if let Token::Identifier(param_name) = &self.current_token {
                        params.push((param_name.clone(), None));
                        self.next_token();
                        
                        // Check for optional second parameter (index)
                        if self.current_token == Token::Symbol(',') {
                            self.next_token();
                            if let Token::Identifier(param_name) = &self.current_token {
                                params.push((param_name.clone(), None));
                                self.next_token();
                            }
                        }
                        break; // .loop() only takes 1 or 2 params
                    } else {
                        return Err(CompileError::SyntaxError(
                            "Expected parameter name in closure".to_string(),
                            Some(self.current_span.clone()),
                        ));
                    }
                }
                
                if self.current_token != Token::Symbol(')') {
                    return Err(CompileError::SyntaxError(
                        "Expected ')' after closure parameters".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
                self.next_token(); // consume ')'
                
                // Parse closure body
                if self.current_token != Token::Symbol('{') {
                    return Err(CompileError::SyntaxError(
                        "Expected '{' for closure body".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
                
                let body = self.parse_block_expression()?;
                arguments.push(Expression::Closure {
                    params,
                    body: Box::new(body),
                });
            } else {
                // Regular argument parsing
                loop {
                    // Check for closure syntax: (params) { body }
                    if self.current_token == Token::Symbol('(') {
                        // Try to parse as closure
                        let saved_pos = self.lexer.position;
                        let saved_read_pos = self.lexer.read_position;
                        let saved_char = self.lexer.current_char;
                        let saved_current = self.current_token.clone();
                        let saved_peek = self.peek_token.clone();
                        
                        // Try to parse closure
                        self.next_token(); // consume '('
                        let mut params = vec![];
                        let mut is_closure = true;
                        
                        while self.current_token != Token::Symbol(')') && self.current_token != Token::Eof {
                            if let Token::Identifier(param_name) = &self.current_token {
                                params.push((param_name.clone(), None));
                                self.next_token();
                                if self.current_token == Token::Symbol(',') {
                                    self.next_token();
                                }
                            } else {
                                // Not a closure, restore and parse as expression
                                is_closure = false;
                                break;
                            }
                        }
                        
                        if is_closure && self.current_token == Token::Symbol(')') {
                            self.next_token(); // consume ')'
                            if self.current_token == Token::Symbol('{') {
                                // It's a closure!
                                let body = self.parse_block_expression()?;
                                arguments.push(Expression::Closure {
                                    params,
                                    body: Box::new(body),
                                });
                            } else {
                                // Not a closure, restore and parse as expression
                                self.lexer.position = saved_pos;
                                self.lexer.read_position = saved_read_pos;
                                self.lexer.current_char = saved_char;
                                self.current_token = saved_current;
                                self.peek_token = saved_peek;
                                arguments.push(self.parse_expression()?);
                            }
                        } else {
                            // Not a closure, restore and parse as expression
                            self.lexer.position = saved_pos;
                            self.lexer.read_position = saved_read_pos;
                            self.lexer.current_char = saved_char;
                            self.current_token = saved_current;
                            self.peek_token = saved_peek;
                            arguments.push(self.parse_expression()?);
                        }
                    } else {
                        arguments.push(self.parse_expression()?);
                    }
                    
                    if self.current_token == Token::Symbol(')') {
                        break;
                    }
                    if self.current_token != Token::Symbol(',') {
                        return Err(CompileError::SyntaxError(
                            "Expected ',' or ')' in function call".to_string(),
                            Some(self.current_span.clone()),
                        ));
                    }
                    self.next_token();
                }
            }
        }
        self.next_token(); // consume ')'
        
        // UFCS implementation: Transform object.method(args) into method(object, args)
        // First, check if this is a stdlib method call (module function)
        let mut expr = if is_module_call {
            // This is a stdlib call like io.print, keep the original format
            Expression::FunctionCall {
                name: format!("{}.{}", match &object {
                    Expression::Identifier(name) => name.clone(),
                    _ => return Err(CompileError::SyntaxError(
                        "Expected identifier for object in method call".to_string(),
                        Some(self.current_span.clone()),
                    )),
                }, method_name),
                args: arguments,
            }
        } else if method_name.contains('.') {
            // This shouldn't happen anymore but keep for compatibility
            Expression::FunctionCall {
                name: format!("{}.{}", match &object {
                    Expression::Identifier(name) => name.clone(),
                    _ => return Err(CompileError::SyntaxError(
                        "Expected identifier for object in method call".to_string(),
                        Some(self.current_span.clone()),
                    )),
                }, method_name),
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
                // This is UFCS: transform object.method(args) into method(object, args)
                // But preserve the full qualified name for debugging/display purposes
                let full_name = match &object {
                    Expression::Identifier(obj_name) => format!("{}.{}", obj_name, method_name),
                    _ => method_name.clone(),
                };
                
                let mut ufcs_args = vec![object];
                ufcs_args.extend(arguments);
                
                Expression::FunctionCall {
                    name: full_name,
                    args: ufcs_args,
                }
            }
        };
        
        // Continue parsing method chaining after function call
        loop {
            match &self.current_token {
                Token::Symbol('.') => {
                    self.next_token(); // consume '.'
                    
                    let member = match &self.current_token {
                        Token::Identifier(name) => name.clone(),
                        _ => {
                            return Err(CompileError::SyntaxError(
                                "Expected identifier after '.'".to_string(),
                                Some(self.current_span.clone()),
                            ));
                        }
                    };
                    self.next_token();
                    
                    // Check for method call
                    if self.current_token == Token::Symbol('(') {
                        return self.parse_call_expression_with_object(expr, member);
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
    
    fn parse_struct_literal(&mut self, name: String) -> Result<Expression> {
        self.next_token(); // consume '{'
        let mut fields = vec![];
        
        while self.current_token != Token::Symbol('}') {
            // Parse field name
            let field_name = match &self.current_token {
                Token::Identifier(name) => name.clone(),
                _ => {
                    return Err(CompileError::SyntaxError(
                        "Expected field name in struct literal".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
            };
            self.next_token();
            
            // Expect ':'
            if self.current_token != Token::Symbol(':') {
                return Err(CompileError::SyntaxError(
                    "Expected ':' after field name in struct literal".to_string(),
                    Some(self.current_span.clone()),
                ));
            }
            self.next_token();
            
            // Parse field value
            let field_value = self.parse_expression()?;
            fields.push((field_name, field_value));
            
            // Check for comma or end of struct
            if self.current_token == Token::Symbol(',') {
                self.next_token();
            } else if self.current_token != Token::Symbol('}') {
                return Err(CompileError::SyntaxError(
                    "Expected ',' or '}' in struct literal".to_string(),
                    Some(self.current_span.clone()),
                ));
            }
        }
        
        self.next_token(); // consume '}'
        Ok(Expression::StructLiteral { name, fields })
    }

    fn parse_pattern_match(&mut self, scrutinee: Expression) -> Result<Expression> {
        // Parse: scrutinee ? | pattern => expr | pattern => expr ...
        // OR bool short form: scrutinee ? { block }
        let scrutinee = Box::new(scrutinee);
        
        // Check for bool pattern short form: expr ? { block }
        if self.current_token == Token::Symbol('{') {
            // Bool pattern short form - execute block if scrutinee is true
            let body = self.parse_block_expression()?;
            
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
        if self.current_token != Token::Pipe {
            return Err(CompileError::SyntaxError(
                "Expected '|' to start pattern matching arms or '{' for bool pattern".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        
        let mut arms = vec![];
        
        // Parse arms: | pattern => expr | pattern => expr ...
        while self.current_token == Token::Pipe {
            self.next_token(); // consume '|'
            
            // Parse pattern - could be single or multiple (or patterns)
            let mut patterns = vec![self.parse_pattern()?];
            
            // Check for additional patterns separated by |
            while self.current_token == Token::Pipe && 
                  self.peek_token != Token::Pipe && // Not start of next arm
                  self.peek_token != Token::Eof {
                // This is an or pattern - consume the | and parse the next pattern
                self.next_token();
                patterns.push(self.parse_pattern()?);
            }
            
            // Create the final pattern
            let pattern = if patterns.len() == 1 {
                patterns.remove(0)
            } else {
                Pattern::Or(patterns)
            };
            
            // Check for destructuring/guard with ->
            let guard = if self.current_token == Token::Arrow {
                self.next_token();
                // TODO: Properly handle destructuring vs guards
                // For now, treat it as a guard
                Some(self.parse_expression()?)
            } else {
                None
            };
            
            // Parse body - can be either { block } or => expr (for compatibility)
            let body = if self.current_token == Token::Symbol('{') {
                // Parse block as expression
                self.next_token(); // consume '{'
                
                let mut statements = Vec::new();
                let mut final_expr = None;
                
                while self.current_token != Token::Symbol('}') && self.current_token != Token::Eof {
                    // Check if this could be the final expression
                    if self.peek_token == Token::Symbol('}') {
                        // This might be the final expression (no semicolon)
                        let expr = self.parse_expression()?;
                        if self.current_token == Token::Symbol(';') {
                            // It's a statement, not the final expression
                            self.next_token();
                            statements.push(crate::ast::Statement::Expression(expr));
                        } else {
                            // It's the final expression
                            final_expr = Some(expr);
                        }
                    } else {
                        // Parse as statement to handle variable declarations and assignments
                        let stmt = self.parse_statement()?;
                        statements.push(stmt);
                    }
                }
                
                if self.current_token != Token::Symbol('}') {
                    return Err(CompileError::SyntaxError(
                        "Expected '}' to close block in match arm".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
                self.next_token(); // consume '}'
                
                // If we have statements, return a block expression
                // Otherwise return the final expression or an empty block
                if !statements.is_empty() || final_expr.is_some() {
                    // If there's a final expression, add it to the statements
                    if let Some(expr) = final_expr {
                        statements.push(crate::ast::Statement::Expression(expr));
                    }
                    Expression::Block(statements)
                } else {
                    // Empty block
                    Expression::Block(vec![])
                }
            } else if self.current_token == Token::FatArrow {
                // Legacy => syntax for compatibility
                self.next_token(); // consume '=>'
                // Handle return statement in pattern arm specially
                if let Token::Identifier(id) = &self.current_token {
                    if id == "return" {
                        self.next_token(); // consume 'return'
                        let return_expr = self.parse_expression()?;
                        // Wrap return in a special expression type or handle it differently
                        // For now, we'll just use the return expression directly
                        // In a full implementation, we'd need a Block expression type with statements
                        return_expr
                    } else {
                        self.parse_expression()?
                    }
                } else {
                    self.parse_expression()?
                }
            } else {
                return Err(CompileError::SyntaxError(
                    "Expected '{' or '=>' after pattern in match arm".to_string(),
                    Some(self.current_span.clone()),
                ));
            };
            
            arms.push(MatchArm {
                pattern,
                guard,
                body,
            });
            
            // Check if there are more arms
            if self.current_token != Token::Pipe {
                break;
            }
        }
        Ok(Expression::QuestionMatch {
            scrutinee,
            arms,
        })
    }

    fn looks_like_generic_type_args(&self) -> bool {
        // Try to determine if this is generic type args Vec<T> or vec_new<i32> vs comparison x < y
        // Heuristics:
        // 1. If next token is a type-like identifier (uppercase or known types like i32), likely generic
        // 2. If we can see a > followed by { or (, it's definitely generic
        if let Token::Operator(op) = &self.current_token {
            if op == "<" {
                if let Token::Identifier(name) = &self.peek_token {
                    // Check for type-like names
                    let first_char = name.chars().next().unwrap_or('_');
                    
                    // Types typically start with uppercase or are primitive types
                    return first_char.is_uppercase() || 
                           name == "i8" || name == "i16" || name == "i32" || name == "i64" ||
                           name == "u8" || name == "u16" || name == "u32" || name == "u64" ||
                           name == "f32" || name == "f64" || name == "bool" || name == "string";
                }
            }
        }
        false
    }
    
    fn parse_array_literal(&mut self) -> Result<Expression> {
        // Consume '['
        self.next_token();
        
        let mut elements = Vec::new();
        
        // Handle empty array
        if self.current_token == Token::Symbol(']') {
            self.next_token();
            return Ok(Expression::ArrayLiteral(elements));
        }
        
        // Parse first element
        elements.push(self.parse_expression()?);
        
        // Parse remaining elements
        while self.current_token == Token::Symbol(',') {
            self.next_token(); // consume ','
            
            // Allow trailing comma
            if self.current_token == Token::Symbol(']') {
                break;
            }
            
            elements.push(self.parse_expression()?);
        }
        
        // Expect closing ']'
        if self.current_token != Token::Symbol(']') {
            return Err(CompileError::SyntaxError(
                format!("Expected ']' in array literal, got {:?}", self.current_token),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();
        
        Ok(Expression::ArrayLiteral(elements))
    }
    
    fn parse_block_expression(&mut self) -> Result<Expression> {
        self.next_token(); // consume '{'
        let mut statements = vec![];
        
        while self.current_token != Token::Symbol('}') && self.current_token != Token::Eof {
            statements.push(self.parse_statement()?);
        }
        
        if self.current_token != Token::Symbol('}') {
            return Err(CompileError::SyntaxError(
                "Expected '}' to close block expression".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token(); // consume '}'
        
        Ok(Expression::Block(statements))
    }

    pub(crate) fn token_to_binary_operator(&self, op: &str) -> Result<BinaryOperator> {
        match op {
            "+" => Ok(BinaryOperator::Add),
            "-" => Ok(BinaryOperator::Subtract),
            "*" => Ok(BinaryOperator::Multiply),
            "/" => Ok(BinaryOperator::Divide),
            "%" => Ok(BinaryOperator::Modulo),
            "==" => Ok(BinaryOperator::Equals),
            "!=" => Ok(BinaryOperator::NotEquals),
            "<" => Ok(BinaryOperator::LessThan),
            ">" => Ok(BinaryOperator::GreaterThan),
            "<=" => Ok(BinaryOperator::LessThanEquals),
            ">=" => Ok(BinaryOperator::GreaterThanEquals),
            "&&" => Ok(BinaryOperator::And),
            "||" => Ok(BinaryOperator::Or),
            _ => Err(CompileError::SyntaxError(
                format!("Unknown binary operator: {}", op),
                Some(self.current_span.clone()),
            )),
        }
    }

    fn get_precedence(&self, op: &str) -> u8 {
        match op {
            ".." | "..=" => 1,  // Range has lowest precedence
            "||" => 2,          // Logical OR
            "&&" => 3,          // Logical AND
            "==" | "!=" => 4,   // Equality
            "<" | "<=" | ">" | ">=" => 5,  // Comparison
            "+" | "-" => 6,     // Addition/Subtraction
            "*" | "/" | "%" => 7,  // Multiplication/Division/Modulo
            _ => 0,
        }
    }

    fn parse_interpolated_string(&mut self, input: String) -> Result<Expression> {
        use crate::ast::StringPart;
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut chars = input.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch == '$' && chars.peek() == Some(&'{') {
                // Found interpolation start
                chars.next(); // consume '{'
                
                // Save any literal part before this interpolation
                if !current.is_empty() {
                    parts.push(StringPart::Literal(current.clone()));
                    current.clear();
                }
                
                // Parse the interpolated expression
                let mut expr_str = String::new();
                let mut brace_count = 1;
                
                while let Some(ch) = chars.next() {
                    if ch == '{' {
                        brace_count += 1;
                        expr_str.push(ch);
                    } else if ch == '}' {
                        brace_count -= 1;
                        if brace_count == 0 {
                            break;
                        }
                        expr_str.push(ch);
                    } else {
                        expr_str.push(ch);
                    }
                }
                
                if brace_count != 0 {
                    return Err(CompileError::SyntaxError(
                        "Unmatched braces in string interpolation".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
                
                // Parse the expression string
                // We need to create a temporary parser for this expression
                let lexer = crate::lexer::Lexer::new(&expr_str);
                let mut temp_parser = crate::parser::Parser::new(lexer);
                let expr = temp_parser.parse_expression()?;
                parts.push(StringPart::Interpolation(expr));
            } else {
                current.push(ch);
            }
        }
        
        // Add any remaining literal part
        if !current.is_empty() {
            parts.push(StringPart::Literal(current));
        }
        
        // If we have interpolation parts, create an interpolated string expression
        if parts.iter().any(|p| matches!(p, StringPart::Interpolation(_))) {
            Ok(Expression::StringInterpolation { parts })
        } else {
            // No interpolation found, return a simple string
            Ok(Expression::String(input))
        }
    }
}
