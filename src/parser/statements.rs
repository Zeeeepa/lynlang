// Loop syntax is simplified - only conditional and infinite loops are supported.
// Range and iterator loops have been removed in favor of functional iteration.
use super::core::Parser;
use crate::ast::{Declaration, Expression, Program, Statement, VariableDeclarationType};
use crate::error::{CompileError, Result};
use crate::lexer::Token;

impl<'a> Parser<'a> {
    // ========================================================================
    // HELPER METHODS FOR PARSING COMMON PATTERNS
    // ========================================================================

    /// Parse a brace-enclosed block of statements: { stmt1; stmt2; ... }
    fn parse_brace_block(&mut self, context: &str) -> Result<Vec<Statement>> {
        self.expect_symbol('{')?;
        let mut statements = vec![];
        while self.current_token != Token::Symbol('}') && self.current_token != Token::Eof {
            statements.push(self.parse_statement()?);
        }
        if self.current_token != Token::Symbol('}') {
            return Err(self.syntax_error(format!("Expected '}}' to close {}", context)));
        }
        self.next_token();
        Ok(statements)
    }

    /// Wrap a list of statements as a single statement (for defer blocks, etc.)
    fn wrap_statements_as_stmt(&self, statements: Vec<Statement>) -> Statement {
        match statements.len() {
            0 => Statement::Expression {
                expr: Expression::Boolean(true),
                span: Some(self.current_span.clone()),
            },
            1 => statements.into_iter().next().unwrap(),
            _ => Statement::Block {
                statements,
                span: Some(self.current_span.clone()),
            },
        }
    }

    /// Parse an expression as a statement, handling optional semicolon
    fn parse_expression_statement(&mut self) -> Result<Statement> {
        let span = Some(self.current_span.clone());
        let expr = self.parse_expression()?;
        self.skip_optional_semicolon();
        Ok(Statement::Expression { expr, span })
    }

    /// Check if current function declaration has a body (for external fn detection)
    /// Assumes we're at '(' of parameter list
    fn function_has_body(&mut self) -> bool {
        // Skip past '(' and params
        let mut depth = 1;
        self.next_token(); // consume '('
        while depth > 0 && self.current_token != Token::Eof {
            match &self.current_token {
                Token::Symbol('(') => depth += 1,
                Token::Symbol(')') => depth -= 1,
                _ => {}
            }
            self.next_token();
        }

        // Now at return type, skip it (identifier with optional generics)
        while !matches!(self.current_token, Token::Symbol('{') | Token::Eof | Token::Identifier(_))
            || matches!(&self.current_token, Token::Identifier(_))
        {
            if matches!(self.current_token, Token::Symbol('{') | Token::Eof) {
                break;
            }
            self.next_token();
            // After identifier, check for generics
            if self.current_token == Token::Operator("<".to_string()) {
                self.skip_generic_params();
            } else {
                break;
            }
        }

        // Check for body: either '{' directly or '= {' (alternate syntax)
        self.current_token == Token::Symbol('{')
            || (self.current_token == Token::Operator("=".to_string())
                && self.peek_token == Token::Symbol('{'))
    }

    /// Check if a brace block looks like a trait (has method signatures) vs struct (has fields)
    /// Assumes we're at '{'
    fn looks_like_trait(&mut self) -> bool {
        self.next_token(); // consume '{'
        // Look for pattern: identifier ':' '(' which indicates a trait method
        if let Token::Identifier(_) = &self.current_token {
            self.next_token();
            self.current_token == Token::Symbol(':') && {
                self.next_token();
                self.current_token == Token::Symbol('(')
            }
        } else {
            false
        }
    }

    /// Detect what type of declaration follows after generics
    /// Returns (is_struct, is_enum, is_function, is_external_fn, is_behavior, is_trait)
    fn detect_declaration_type(&mut self) -> (bool, bool, bool, bool, bool, bool) {
        // We're past the generics, check what comes next
        let is_struct = self.current_token == Token::Symbol(':')
            && self.peek_token == Token::Symbol('{');
        let is_enum = self.current_token == Token::Symbol(':')
            && (matches!(&self.peek_token, Token::Identifier(_))
                || self.peek_token == Token::Symbol('.'));
        let is_generic_function = self.current_token == Token::Symbol('(');
        let is_equals_function = self.current_token == Token::Operator("=".to_string())
            && self.peek_token == Token::Symbol('(');
        let is_function = (self.current_token == Token::Symbol(':')
            && self.peek_token == Token::Symbol('('))
            || is_generic_function
            || is_equals_function;
        let is_behavior = self.current_token == Token::Symbol(':')
            && matches!(&self.peek_token, Token::Identifier(name) if name == "behavior");

        (is_struct, is_enum, is_function, false, is_behavior, false)
    }

    // ========================================================================
    // MAIN PARSING METHODS
    // ========================================================================

    pub fn parse_program(&mut self) -> Result<Program> {
        let mut declarations = vec![];
        while self.current_token != Token::Eof {
            // Check for @export { symbol1, symbol2, ... } or @export *
            if self.current_token == Token::AtExport {
                declarations.push(self.parse_export()?);
                continue;
            }

            // Check for destructuring import: { name, name } = @std
            if self.current_token == Token::Symbol('{') {
                declarations.extend(self.parse_destructuring_import_declaration()?);
                continue;
            }
            // Parse top-level declarations
            else if let Token::Identifier(name) = &self.current_token {
                // Handle special identifiers "type" and "comptime" first
                if name == "type" {
                    declarations.push(Declaration::TypeAlias(self.parse_type_alias()?));
                } else if name == "comptime" {
                    declarations.push(self.parse_comptime_block_declaration()?);
                } else if self.peek_token == Token::Operator(":=".to_string()) {
                    let var_name = name.clone();
                    let is_module_import = self.with_lookahead(|p| {
                        p.next_token();
                        p.next_token();
                        p.is_module_import_after_colon_assign()
                    });

                    if is_module_import {
                        self.next_token();
                        self.next_token();
                        declarations.push(self.parse_module_import_after_colon_assign(var_name)?);
                    } else {
                        declarations.push(self.parse_constant_from_statement()?);
                    }
                } else if self.peek_token == Token::Operator("::".to_string()) {
                    let is_function = self.with_lookahead(|p| {
                        p.next_token();
                        p.next_token();
                        p.current_token == Token::Symbol('(')
                    });

                    if is_function {
                        declarations.push(Declaration::Function(self.parse_function()?));
                    } else {
                        let var_name = self.current_identifier().unwrap();
                        self.next_token();
                        declarations.push(self.parse_top_level_mutable_var(var_name)?);
                    }
                } else if self.peek_token == Token::Symbol(':')
                    || self.peek_token == Token::Operator("<".to_string())
                {
                    // Check if it's a struct, enum, or function definition
                    // Look ahead to see what type of declaration this is
                    let saved_state = self.save_state();

                    // If generics, need to look ahead to determine struct vs function
                    if self.peek_token == Token::Operator("<".to_string()) {

                        // Skip past name and generics to see what follows
                        self.next_token(); // Move to <
                        let depth = self.skip_generic_params();

                        if depth >= 0 {
                            // FIRST check if this is an impl block: Type<T>.impl = { ... }
                            if self.current_token == Token::Symbol('.') {
                                let after_generics_state = self.save_state();
                                self.next_token();
                                if let Token::Identifier(method_name) = &self.current_token {
                                    let method_name = method_name.clone();
                                    // Check if this is a method definition: Type<T>.method = ...
                                    if self.peek_token == Token::Operator("=".to_string()) {
                                        // This is a method definition! Restore and parse properly
                                        self.restore_state(saved_state);
                                        // Parse type name with generics using helper
                                        let type_name = self.parse_type_name_with_generics()?;
                                        self.expect_symbol('.')?;

                                        if method_name == "impl" {
                                            self.next_token(); // consume 'impl'
                                            self.expect_operator("=")?;
                                            declarations.push(Declaration::ImplBlock(
                                                self.parse_impl_block(type_name)?,
                                            ));
                                            continue;
                                        } else {
                                            // Method definition: Type<T>.method = (params) ReturnType { ... }
                                            let full_function_name = format!("{}.{}", type_name, method_name);
                                            let mut func = self.parse_function()?;
                                            func.name = full_function_name;
                                            declarations.push(Declaration::Function(func));
                                            continue;
                                        }
                                    }
                                }
                                // Not a recognized pattern - restore and continue
                                self.restore_state(after_generics_state);
                            }

                            // Check what comes after the generics using helper
                            let (is_struct, is_enum, is_function, _, is_behavior, _) =
                                self.detect_declaration_type();

                            // Restore lexer state
                            self.restore_state(saved_state);

                            if is_behavior {
                                declarations.push(Declaration::Behavior(self.parse_behavior()?));
                            } else if is_enum {
                                declarations.push(Declaration::Enum(self.parse_enum()?));
                            } else if is_function {
                                declarations.push(Declaration::Function(self.parse_function()?));
                            } else if is_struct {
                                declarations.push(Declaration::Struct(self.parse_struct()?));
                            } else {
                                // Default to struct for backward compatibility
                                declarations.push(Declaration::Struct(self.parse_struct()?));
                            }
                        } else {
                            // Malformed generics, restore and try to parse as struct
                            self.restore_state(saved_state);
                            declarations.push(Declaration::Struct(self.parse_struct()?));
                        }
                    } else {
                        // FIRST check if this is an impl block: Type.impl or Type<T>.impl
                        if self.peek_token == Token::Symbol('.')
                            || self.peek_token == Token::Operator("<".to_string())
                        {
                            let impl_saved = self.save_state();
                            // Try to parse as impl block
                            if self.current_identifier().is_some() {
                                let type_name = self.parse_type_name_with_generics()?;
                                // Check if next is '.' followed by 'impl'
                                if self.try_consume_symbol('.') && self.is_keyword("impl") {
                                    self.next_token(); // consume 'impl'
                                    if self.try_consume_operator("=") {
                                        declarations.push(Declaration::ImplBlock(
                                            self.parse_impl_block(type_name)?,
                                        ));
                                        continue; // Skip to next declaration
                                    }
                                }
                            }
                            // Not an impl block, restore state
                            self.restore_state(impl_saved);
                        }

                        // Need to look ahead to determine if it's a struct, enum, behavior, trait, or function
                        self.next_token(); // Move to ':'
                        self.next_token(); // Move past ':' to see what comes after

                        // Check what comes after ':'
                        let is_enum = matches!(&self.current_token, Token::Identifier(_))
                            || matches!(&self.current_token, Token::Symbol('.'));
                        let is_function_or_external = matches!(&self.current_token, Token::Symbol('('));
                        let is_behavior = matches!(&self.current_token, Token::Identifier(name) if name == "behavior");

                        // If it looks like a function, check if it's external (no body) or regular
                        let (is_function, is_external_fn) = if is_function_or_external {
                            let has_body = self.with_lookahead(|p| p.function_has_body());
                            if has_body { (true, false) } else { (false, true) }
                        } else {
                            (false, false)
                        };

                        // Check if it's a trait or struct (both start with '{')
                        let (is_struct, is_trait) = if self.current_token == Token::Symbol('{') {
                            let looks_like_trait = self.with_lookahead(|p| p.looks_like_trait());
                            if looks_like_trait { (false, true) } else { (true, false) }
                        } else {
                            (false, false)
                        };

                        // Restore lexer state
                        self.restore_state(saved_state);

                        if is_behavior {
                            declarations.push(Declaration::Behavior(self.parse_behavior()?));
                        } else if is_trait {
                            declarations.push(Declaration::Trait(self.parse_trait()?));
                        } else if is_enum {
                            declarations.push(Declaration::Enum(self.parse_enum()?));
                        } else if is_external_fn {
                            declarations.push(Declaration::ExternalFunction(
                                self.parse_external_function()?,
                            ));
                        } else if is_function {
                            declarations.push(Declaration::Function(self.parse_function()?));
                        } else if is_struct {
                            declarations.push(Declaration::Struct(self.parse_struct()?));
                        } else {
                            // Try to parse as function (fallback)
                            declarations.push(Declaration::Function(self.parse_function()?));
                        }
                    }
                } else if self.peek_token == Token::Symbol('.')
                    || (self.peek_token == Token::Operator("<".to_string()))
                {
                    // Could be an impl block: Type.impl = { ... } or Type<T>.impl = { ... }
                    let type_name = self.parse_type_name_with_generics()?;

                    // Now check for '.'
                    if self.current_token != Token::Symbol('.') {
                        // Not an impl block, restore and parse as function
                        declarations.push(Declaration::Function(self.parse_function()?));
                        continue;
                    }

                    self.next_token(); // consume '.'

                    if let Token::Identifier(method_name) = &self.current_token {
                        if method_name == "impl" {
                            // This is an impl block: Type.impl = { ... }
                            self.next_token(); // consume 'impl'
                            self.expect_operator("=")?;
                            declarations.push(Declaration::ImplBlock(self.parse_impl_block(type_name)?));
                        } else if method_name == "implements" {
                            // This is a trait implementation: Type.implements(Trait, { ... })
                            self.next_token(); // consume 'implements'
                            declarations.push(Declaration::TraitImplementation(
                                self.parse_trait_implementation(type_name)?,
                            ));
                        } else if method_name == "requires" {
                            // This is a trait requirement: Type.requires(Trait)
                            self.next_token(); // consume 'requires'
                            declarations.push(Declaration::TraitRequirement(
                                self.parse_trait_requirement(type_name)?,
                            ));
                        } else {
                            // Check if this is a method definition: Type.method = (params) returnType { ... }
                            // Look ahead to see if next token is '='
                            if self.peek_token == Token::Operator("=".to_string()) {
                                // This is a method definition: Type.method = (params) returnType { ... }
                                // Build the compound function name "Type.method"
                                let full_function_name = format!("{}.{}", type_name, method_name);

                                // We're currently at the method name token
                                // We need to parse: method = (params) returnType { ... }
                                // But parse_function expects to start at the function name
                                // So we'll parse it normally - the current token is the method name
                                // and parse_function will consume it and the '=' after

                                // Actually, parse_function expects the name to be current_token
                                // and it will consume it. Since we're at "method" and need "Type.method",
                                // we need to handle this specially.

                                // Parse the function, but we need to construct the name as "Type.method"
                                // Let's parse it and then fix the name
                                let mut func = self.parse_function()?;
                                // Fix the function name to be the compound name
                                func.name = full_function_name;
                                declarations.push(Declaration::Function(func));
                            } else {
                                // Not a recognized method and not a method definition
                                return Err(self.syntax_error(
                                    format!("Expected 'impl', 'implements', 'requires', or method after '{}.'", type_name)
                                ));
                            }
                        }
                    } else {
                        // Not an identifier
                        return Err(self.syntax_error(
                            format!("Expected method name after '{}.'", type_name)
                        ));
                    }
                } else if self.peek_token == Token::Symbol(':') {
                    // Could be an external function declaration: name: (params) return_type
                    let is_external_fn = self.with_lookahead(|p| {
                        p.next_token(); // consume identifier
                        p.next_token(); // consume ':'
                        p.current_token == Token::Symbol('(')
                    });

                    if is_external_fn {
                        declarations.push(Declaration::ExternalFunction(
                            self.parse_external_function()?,
                        ));
                    } else {
                        declarations.push(Declaration::Function(self.parse_function()?));
                    }
                } else if self.peek_token == Token::Operator(":=".to_string()) {
                    // Top-level constant declaration: name := value
                    let name = self.current_identifier().unwrap();
                    self.next_token(); // consume identifier
                    self.next_token(); // consume ':='
                    let value = self.parse_expression()?;
                    self.skip_optional_semicolon();

                    declarations.push(Declaration::Constant {
                        name,
                        value,
                        type_: None,
                        span: Some(self.current_span.clone()),
                    });
                } else if self.peek_token == Token::Operator("=".to_string()) {
                    // Could be either:
                    // 1. Function declaration: name = (params) returnType { ... }
                    // 2. Variable declaration: name = value
                    // Look ahead to distinguish
                    let name = self.current_identifier().unwrap();
                    let is_function = self.with_lookahead(|p| {
                        p.next_token(); // move to =
                        p.next_token(); // move past =
                        p.current_token == Token::Symbol('(')
                    });

                    if is_function {
                        // Function declaration: name = (params) returnType { ... }
                        let func = self.parse_function()?;
                        declarations.push(Declaration::Function(func));
                    } else {
                        // Variable declaration: name = value
                        // Could be a module import: name = @std.module
                        self.next_token(); // consume identifier
                        self.next_token(); // consume '='

                        // Check if this is a module import (@std.module)
                        let is_module_import = if let Token::Identifier(id) = &self.current_token {
                            id.starts_with("@std")
                        } else {
                            false
                        };

                        if is_module_import {
                            // Parse as module import: name = @std.module
                            if let Token::Identifier(module_path) = &self.current_token {
                                let mut full_path = module_path.clone();
                                self.next_token();

                                // Handle member access for @std.module
                                while self.current_token == Token::Symbol('.') {
                                    self.next_token(); // consume '.'
                                    if let Token::Identifier(member) = &self.current_token {
                                        full_path.push('.');
                                        full_path.push_str(member);
                                        self.next_token();
                                    } else {
                                        break;
                                    }
                                }

                                // Check for .import() function call
                                if full_path.ends_with(".import")
                                    && self.current_token == Token::Symbol('(')
                                {
                                    self.next_token(); // consume '('
                                    if let Token::StringLiteral(module_name) = &self.current_token {
                                        let imported_module = module_name.clone();
                                        self.next_token(); // consume string
                                        if self.current_token != Token::Symbol(')') {
                                            return Err(CompileError::SyntaxError(
                                                "Expected ')' after module name".to_string(),
                                                Some(self.current_span.clone()),
                                            ));
                                        }
                                        self.next_token(); // consume ')'

                                        // Create ModuleImport with the imported module
                                        declarations.push(Declaration::ModuleImport {
                                            alias: name,
                                            module_path: format!("@std.{}", imported_module),
                                            span: Some(self.current_span.clone()),
                                        });
                                    } else {
                                        return Err(CompileError::SyntaxError(
                                            "Expected string literal in import()".to_string(),
                                            Some(self.current_span.clone()),
                                        ));
                                    }
                                } else {
                                    // Direct module import: name = @std.module
                                    declarations.push(Declaration::ModuleImport {
                                        alias: name,
                                        module_path: full_path,
                                        span: Some(self.current_span.clone()),
                                    });
                                }
                            }
                        } else {
                            // Regular variable declaration
                            let value = self.parse_expression()?;
                            self.skip_optional_semicolon();
                            declarations.push(Declaration::Constant {
                                name,
                                value,
                                type_: None,
                                span: Some(self.current_span.clone()),
                            });
                        }
                    }
                } else {
                    // Check if we're at EOF or if the identifier is alone (e.g., followed by comments)
                    if self.peek_token == Token::Eof {
                        // Skip standalone identifier at EOF (likely has trailing comments)
                        self.next_token();
                        continue;
                    }

                    // Skip standalone identifiers that aren't part of a declaration
                    // This can happen with malformed syntax or comments
                    // Try to recover by skipping to the next token
                    eprintln!("Warning: Unexpected standalone identifier '{}' at line {}, column {}. Skipping.", 
                        name, self.current_span.line, self.current_span.column);
                    self.next_token();
                    continue;
                }
            } else {
                return Err(CompileError::SyntaxError(
                    format!("Unexpected token at top level: {:?}", self.current_token),
                    Some(self.current_span.clone()),
                ));
            }
        }

        Ok(Program {
            declarations,
            statements: Vec::new(),
        })
    }

    pub fn parse_statement(&mut self) -> Result<Statement> {
        match &self.current_token {
            // Check for specific keywords first before generic identifier
            Token::Identifier(id) if id == "return" => {
                let span = Some(self.current_span.clone());
                self.next_token();
                let expr = self.parse_expression()?;
                self.skip_optional_semicolon();
                Ok(Statement::Return { expr, span })
            }
            Token::Identifier(id) if id == "loop" => {
                let span = Some(self.current_span.clone());
                // Check if this is loop() expression or loop{} statement
                if self.peek_token == Token::Symbol('(') {
                    // loop(() { ... }) - expression form
                    let expr = self.parse_expression()?;
                    Ok(Statement::Expression { expr, span })
                } else {
                    // loop { ... } - statement form
                    self.parse_loop_statement()
                }
            }
            Token::Identifier(id) if id == "break" => {
                let span = Some(self.current_span.clone());
                self.next_token();
                let label = self.current_identifier().map(|name| { self.next_token(); name });
                self.skip_optional_semicolon();
                Ok(Statement::Break { label, span })
            }
            Token::Identifier(id) if id == "defer" => {
                let span = Some(self.current_span.clone());
                self.next_token();
                let deferred_stmt = if self.current_token == Token::Symbol('{') {
                    let statements = self.parse_brace_block("defer block")?;
                    self.wrap_statements_as_stmt(statements)
                } else {
                    self.parse_statement()?
                };
                Ok(Statement::Defer { statement: Box::new(deferred_stmt), span })
            }
            Token::Identifier(id) if id == "continue" => {
                let span = Some(self.current_span.clone());
                self.next_token();
                let label = self.current_identifier().map(|name| { self.next_token(); name });
                self.skip_optional_semicolon();
                Ok(Statement::Continue { label, span })
            }
            Token::Identifier(id) if id == "comptime" => {
                let span = Some(self.current_span.clone());
                self.next_token(); // consume 'comptime'
                if self.current_token != Token::Symbol('{') {
                    let expr = Expression::Comptime(Box::new(self.parse_expression()?));
                    self.skip_optional_semicolon();
                    return Ok(Statement::Expression { expr, span });
                }
                let statements = self.parse_brace_block("comptime block")?;
                Ok(Statement::ComptimeBlock { statements, span })
            }
            Token::AtThis => {
                let span = Some(self.current_span.clone());
                self.next_token(); // consume '@this'
                self.expect_symbol('.')?;
                if !self.is_keyword("defer") {
                    return Err(self.syntax_error("Expected 'defer' after '@this.' - only 'defer' is supported"));
                }
                self.next_token(); // consume 'defer'
                self.expect_symbol('(')?;
                let expr = self.parse_expression()?;
                self.expect_symbol(')')?;
                self.skip_optional_semicolon();
                Ok(Statement::ThisDefer { expr, span })
            }
            // Generic identifier case (after all specific keywords)
            Token::Identifier(_name) => {
                // Check for variable declarations using peek tokens
                match &self.peek_token {
                    Token::Operator(op) if op == "=" => {
                        // Immutable assignment with = (per LANGUAGE_SPEC.zen)
                        self.parse_variable_declaration()
                    }
                    Token::Operator(op) if op == ":=" => {
                        // Constant with inferred type (per LANGUAGE_SPEC.zen line 21)
                        self.parse_variable_declaration()
                    }
                    Token::Operator(op) if op == "::=" => {
                        // Mutable assignment with ::=
                        self.parse_variable_declaration()
                    }
                    Token::Symbol(':') => {
                        // Type declaration: name : T or name : T = value
                        self.parse_variable_declaration()
                    }
                    Token::Operator(op) if op == "::" => {
                        // Mutable type declaration: name :: T or name :: T = value
                        self.parse_variable_declaration()
                    }
                    Token::Symbol('.') | Token::Symbol('[') => {
                        // Could be member access or array indexing followed by assignment
                        let span = Some(self.current_span.clone());
                        let lhs = self.parse_expression()?;

                        if self.current_token == Token::Operator("=".to_string()) {
                            self.next_token(); // consume '='
                            let value = self.parse_expression()?;
                            self.skip_optional_semicolon();
                            Ok(Statement::PointerAssignment { pointer: lhs, value, span })
                        } else {
                            self.skip_optional_semicolon();
                            Ok(Statement::Expression { expr: lhs, span })
                        }
                    }
                    _ => self.parse_expression_statement()
                }
            }
            Token::Symbol('?') | Token::Symbol('(') => self.parse_expression_statement(),
            Token::Symbol('{') => self.parse_destructuring_import(),
            Token::Integer(_) | Token::Float(_) | Token::StringLiteral(_) => {
                self.parse_expression_statement()
            }
            Token::AtBuiltin | Token::AtStd => self.parse_expression_statement(),
            _ => Err(CompileError::SyntaxError(
                format!("Unexpected token in statement: {:?}", self.current_token),
                Some(self.current_span.clone()),
            )),
        }
    }

    fn parse_variable_declaration(&mut self) -> Result<Statement> {
        // Capture span at start of declaration for error reporting
        let start_span = self.current_span.clone();

        let name = if let Token::Identifier(name) = &self.current_token {
            name.clone()
        } else {
            return Err(CompileError::SyntaxError(
                "Expected variable name".to_string(),
                Some(self.current_span.clone()),
            ));
        };
        self.next_token();

        let (is_mutable, declaration_type, type_) = match &self.current_token {
            Token::Operator(op) if op == "=" => {
                self.next_token();
                (false, VariableDeclarationType::InferredImmutable, None)
            }
            Token::Operator(op) if op == ":=" => {
                self.next_token();
                (false, VariableDeclarationType::InferredImmutable, None)
            }
            Token::Operator(op) if op == "::=" => {
                self.next_token();
                (true, VariableDeclarationType::InferredMutable, None)
            }
            Token::Symbol(':') => {
                // Check if it's : T (with type) or just : followed by something else
                self.next_token();

                // Check for :: T (mutable with type)
                if self.current_token == Token::Symbol(':') {
                    // Explicit mutable: name :: T = value
                    self.next_token();
                    let type_ = self.parse_type()?;
                    if self.current_token != Token::Operator("=".to_string()) {
                        return Err(CompileError::SyntaxError(
                            "Expected '=' after type".to_string(),
                            Some(self.current_span.clone()),
                        ));
                    }
                    self.next_token();
                    (true, VariableDeclarationType::ExplicitMutable, Some(type_))
                } else {
                    // Explicit immutable: name : T = value (or just name : T without initializer)
                    let type_ = self.parse_type()?;
                    // Check if there's an initializer
                    if self.current_token == Token::Operator("=".to_string()) {
                        self.next_token();
                        (
                            false,
                            VariableDeclarationType::ExplicitImmutable,
                            Some(type_),
                        )
                    } else {
                        // Just type annotation without initializer - still return for now
                        // The initializer will be None in the calling code
                        return Ok(Statement::VariableDeclaration {
                            name,
                            type_: Some(type_),
                            initializer: None,
                            is_mutable: false,
                            declaration_type: VariableDeclarationType::ExplicitImmutable,
                            span: Some(start_span),
                        });
                    }
                }
            }
            Token::Operator(op) if op == "::" => {
                // This might be :: T directly (for mutable with type)
                self.next_token();
                let type_ = self.parse_type()?;

                // Check if there's an initializer
                if self.current_token == Token::Operator("=".to_string()) {
                    self.next_token();
                    (true, VariableDeclarationType::ExplicitMutable, Some(type_))
                } else {
                    // Forward declaration without initializer
                    return Ok(Statement::VariableDeclaration {
                        name,
                        type_: Some(type_),
                        initializer: None,
                        is_mutable: true,
                        declaration_type: VariableDeclarationType::ExplicitMutable,
                        span: Some(start_span),
                    });
                }
            }
            _ => {
                return Err(CompileError::SyntaxError(
                    format!(
                        "Expected variable declaration operator, got: {:?}",
                        self.current_token
                    ),
                    Some(self.current_span.clone()),
                ));
            }
        };

        let initializer = self.parse_expression()?;

        // For inferred declarations (:= and ::=), leave type_ as None
        // For explicit declarations (: T = and :: T =), use the parsed type
        let final_type = type_;
        self.skip_optional_semicolon();

        Ok(Statement::VariableDeclaration {
            name,
            type_: final_type,
            initializer: Some(initializer),
            is_mutable,
            declaration_type,
            span: Some(start_span),
        })
    }

    fn parse_loop_statement(&mut self) -> Result<Statement> {
        use crate::ast::LoopKind;
        let span = Some(self.current_span.clone());

        // Skip 'loop' keyword
        self.next_token();

        // Check for optional label
        let label = if self.current_token == Token::Symbol(':') {
            self.next_token();
            if let Token::Identifier(label_name) = &self.current_token {
                let label_name = label_name.clone();
                self.next_token();
                Some(label_name)
            } else {
                return Err(CompileError::SyntaxError(
                    "Expected label name after ':'".to_string(),
                    Some(self.current_span.clone()),
                ));
            }
        } else {
            None
        };

        // Determine the loop kind - only support infinite and condition loops now
        let kind = if self.current_token == Token::Symbol('{') {
            // No condition - infinite loop: loop { }
            LoopKind::Infinite
        } else {
            // Parse a general condition expression
            let condition = self.parse_expression()?;
            LoopKind::Condition(condition)
        };

        // Parse loop body using helper
        let body = self.parse_brace_block("loop body")?;
        Ok(Statement::Loop { kind, label, body, span })
    }

    #[allow(dead_code)]
    fn parse_variable_assignment(&mut self) -> Result<Statement> {
        // Capture span at start of assignment for error reporting
        let start_span = self.current_span.clone();

        // Parse as either declaration or assignment - typechecker will determine which
        let name = if let Token::Identifier(name) = &self.current_token {
            name.clone()
        } else {
            return Err(CompileError::SyntaxError(
                "Expected variable name".to_string(),
                Some(self.current_span.clone()),
            ));
        };
        self.next_token(); // consume identifier

        // Consume the '=' operator
        if self.current_token != Token::Operator("=".to_string()) {
            return Err(CompileError::SyntaxError(
                "Expected '=' for assignment".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();

        // Parse the value expression
        let value = self.parse_expression()?;

        self.skip_optional_semicolon();

        // Return a VariableAssignment statement - the typechecker will determine if this is valid
        Ok(Statement::VariableAssignment {
            name,
            value,
            span: Some(start_span),
        })
    }

    // ========================================================================
    // HELPER FUNCTIONS FOR parse_program
    // ========================================================================

    /// Parse a comptime block: comptime { statements... }
    fn parse_comptime_block_declaration(&mut self) -> Result<Declaration> {
        self.next_token(); // consume 'comptime'
        if self.current_token != Token::Symbol('{') {
            return Err(CompileError::SyntaxError(
                "Expected '{' after comptime".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token(); // consume '{'

        let mut statements = vec![];
        while self.current_token != Token::Symbol('}') && self.current_token != Token::Eof {
            let stmt = self.parse_statement()?;

            // Check for imports in comptime blocks (not allowed)
            if let Statement::VariableDeclaration {
                initializer, name, ..
            } = &stmt
            {
                if let Some(expr) = initializer {
                    let is_import = match expr {
                        Expression::MemberAccess { object, .. } => {
                            if let Expression::Identifier(id) = &**object {
                                id.starts_with("@std") || id == "@std"
                            } else {
                                false
                            }
                        }
                        Expression::FunctionCall { name, .. } => {
                            name.contains("import") || name == "build.import"
                        }
                        Expression::Identifier(id) => id.starts_with("@std"),
                        _ => false,
                    };

                    if is_import {
                        return Err(CompileError::SyntaxError(
                            format!(
                                "Import '{}' not allowed inside comptime block. Imports must be at module level.",
                                name
                            ),
                            Some(self.current_span.clone()),
                        ));
                    }
                }
            }

            statements.push(stmt);
        }

        if self.current_token != Token::Symbol('}') {
            return Err(CompileError::SyntaxError(
                "Expected '}' to close comptime block".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token(); // consume '}'

        Ok(Declaration::ComptimeBlock(statements))
    }

    fn parse_destructuring_import(&mut self) -> Result<Statement> {
        let span = Some(self.current_span.clone());
        // Parse { io, maths } = @std
        self.next_token(); // consume '{'

        let mut names = vec![];

        // Parse the list of identifiers
        while self.current_token != Token::Symbol('}') {
            if let Token::Identifier(name) = &self.current_token {
                names.push(name.clone());
                self.next_token();

                // Check for comma
                if self.current_token == Token::Symbol(',') {
                    self.next_token();
                } else if self.current_token != Token::Symbol('}') {
                    return Err(CompileError::SyntaxError(
                        "Expected ',' or '}' in destructuring import".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
            } else {
                return Err(CompileError::SyntaxError(
                    "Expected identifier in destructuring import".to_string(),
                    Some(self.current_span.clone()),
                ));
            }
        }

        self.next_token(); // consume '}'

        // Expect '='
        if self.current_token != Token::Operator("=".to_string()) {
            return Err(CompileError::SyntaxError(
                "Expected '=' after destructuring pattern".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();

        // Parse the source (should be @std or @std.something)
        let source = self.parse_expression()?;
        self.skip_optional_semicolon();
        Ok(Statement::DestructuringImport { names, source, span })
    }
}
