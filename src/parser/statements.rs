// Loop syntax is simplified - only conditional and infinite loops are supported.
// Range and iterator loops have been removed in favor of functional iteration.
use super::core::Parser;
use crate::ast::{Declaration, Expression, Program, Statement, VariableDeclarationType};
use crate::error::{CompileError, Result};
use crate::lexer::Token;

impl<'a> Parser<'a> {
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
                    let saved_current = self.current_token.clone();
                    let saved_peek = self.peek_token.clone();
                    let saved_lex_state = self.lexer.save_state();

                    self.next_token();
                    self.next_token();

                    let is_module_import = self.is_module_import_after_colon_assign();

                    self.current_token = saved_current;
                    self.peek_token = saved_peek;
                    self.lexer.restore_state(saved_lex_state);

                    if is_module_import {
                        self.next_token();
                        self.next_token();
                        declarations.push(self.parse_module_import_after_colon_assign(var_name)?);
                    } else {
                        declarations.push(self.parse_constant_from_statement()?);
                    }
                } else if self.peek_token == Token::Operator("::".to_string()) {
                    let saved_state = self.lexer.save_state();
                    let saved_current_token = self.current_token.clone();
                    let saved_peek_token = self.peek_token.clone();

                    self.next_token();
                    self.next_token();

                    let is_function = self.current_token == Token::Symbol('(');

                    self.lexer.restore_state(saved_state);
                    self.current_token = saved_current_token;
                    self.peek_token = saved_peek_token;

                    if is_function {
                        declarations.push(Declaration::Function(self.parse_function()?));
                    } else {
                        let var_name = if let Token::Identifier(n) = &self.current_token {
                            n.clone()
                        } else {
                            unreachable!()
                        };
                        self.next_token();
                        declarations.push(self.parse_top_level_mutable_var(var_name)?);
                    }
                } else if self.peek_token == Token::Symbol(':')
                    || self.peek_token == Token::Operator("<".to_string())
                {
                    // Check if it's a struct, enum, or function definition
                    let _name = if let Token::Identifier(name) = &self.current_token {
                        name.clone()
                    } else {
                        unreachable!()
                    };

                    // Look ahead to see what type of declaration this is
                    let saved_state = self.lexer.save_state();
                    let saved_current_token = self.current_token.clone();
                    let saved_peek_token = self.peek_token.clone();

                    // If generics, need to look ahead to determine struct vs function
                    if self.peek_token == Token::Operator("<".to_string()) {
                        // Look ahead to see if it's a struct or a function with generics
                        let saved_inner_state = self.lexer.save_state();
                        let saved_current = self.current_token.clone();
                        let saved_peek = self.peek_token.clone();
                        let saved_span = self.current_span.clone();
                        let saved_peek_span = self.peek_span.clone();

                        // Skip past the generics to see what follows
                        self.next_token(); // Move to <
                        self.next_token(); // Move past <
                        let mut depth = 1;
                        while depth > 0 && self.current_token != Token::Eof {
                            if self.current_token == Token::Operator("<".to_string()) {
                                depth += 1;
                            } else if self.current_token == Token::Operator(">".to_string()) {
                                depth -= 1;
                            }
                            if depth > 0 {
                                self.next_token();
                            }
                        }

                        if depth == 0 {
                            self.next_token(); // Move past >

                            // FIRST check if this is an impl block: Type<T>.impl = { ... }
                            if self.current_token == Token::Symbol('.') {
                                // Save state before checking
                                let saved_after_generics_state = self.lexer.save_state();
                                let saved_after_generics_current = self.current_token.clone();
                                let saved_after_generics_peek = self.peek_token.clone();

                                self.next_token();
                                if let Token::Identifier(method_name) = &self.current_token {
                                    let method_name = method_name.clone();

                                    // Check if this is a method definition: Type<T>.method = ...
                                    if self.peek_token == Token::Operator("=".to_string()) {
                                        // This is a method definition! Restore and parse properly
                                        self.lexer.restore_state(saved_inner_state);
                                        self.current_token = saved_current.clone();
                                        self.peek_token = saved_peek.clone();
                                        self.current_span = saved_span.clone();
                                        self.peek_span = saved_peek_span.clone();

                                        // Parse type name with generics
                                        let mut type_name =
                                            if let Token::Identifier(name) = &self.current_token {
                                                name.clone()
                                            } else {
                                                return Err(CompileError::SyntaxError(
                                                    "Expected type name".to_string(),
                                                    Some(self.current_span.clone()),
                                                ));
                                            };
                                        self.next_token(); // consume name
                                        self.next_token(); // consume '<'
                                        type_name.push('<');
                                        loop {
                                            if let Token::Identifier(param) = &self.current_token {
                                                type_name.push_str(param);
                                                self.next_token();
                                            } else {
                                                break;
                                            }
                                            if self.current_token == Token::Symbol(',') {
                                                type_name.push(',');
                                                self.next_token();
                                            } else if self.current_token
                                                == Token::Operator(">".to_string())
                                            {
                                                type_name.push('>');
                                                self.next_token();
                                                break;
                                            } else {
                                                break;
                                            }
                                        }

                                        // Now at '.', consume it
                                        self.next_token(); // consume '.'

                                        if method_name == "impl" {
                                            // impl block: Type<T>.impl = { ... }
                                            self.next_token(); // consume 'impl'
                                            if self.current_token
                                                == Token::Operator("=".to_string())
                                            {
                                                self.next_token();
                                                declarations.push(Declaration::ImplBlock(
                                                    self.parse_impl_block(type_name)?,
                                                ));
                                                continue;
                                            }
                                            return Err(CompileError::SyntaxError(
                                                "Expected '=' after 'impl'".to_string(),
                                                Some(self.current_span.clone()),
                                            ));
                                        } else {
                                            // Method definition: Type<T>.method = (params) ReturnType { ... }
                                            let full_function_name =
                                                format!("{}.{}", type_name, method_name);
                                            let mut func = self.parse_function()?;
                                            func.name = full_function_name;
                                            declarations.push(Declaration::Function(func));
                                            continue;
                                        }
                                    }
                                }

                                // Not a recognized pattern - restore and continue
                                self.lexer.restore_state(saved_after_generics_state);
                                self.current_token = saved_after_generics_current.clone();
                                self.peek_token = saved_after_generics_peek.clone();
                            }

                            // Check what comes after the generics
                            let is_struct = self.current_token == Token::Symbol(':')
                                && self.peek_token == Token::Symbol('{');
                            let is_enum = self.current_token == Token::Symbol(':')
                                && (matches!(&self.peek_token, Token::Identifier(_))
                                    || self.peek_token == Token::Symbol('.')); // Support .Variant syntax (enums use commas, not pipes)
                                                                               // Generic function: name<T>(params) return_type { ... }
                                                                               // or name<T> = (params) return_type { ... }
                            let is_generic_function = self.current_token == Token::Symbol('(');
                            let is_equals_function = self.current_token
                                == Token::Operator("=".to_string())
                                && self.peek_token == Token::Symbol('(');
                            let is_function = (self.current_token == Token::Symbol(':')
                                && self.peek_token == Token::Symbol('('))
                                || is_generic_function
                                || is_equals_function;
                            let is_behavior = self.current_token == Token::Symbol(':')
                                && matches!(&self.peek_token, Token::Identifier(name) if name == "behavior");

                            // Restore lexer state
                            self.lexer.restore_state(saved_inner_state);
                            self.current_token = saved_current;
                            self.peek_token = saved_peek;
                            self.current_span = saved_span;
                            self.peek_span = saved_peek_span;

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
                            self.lexer.restore_state(saved_inner_state);
                            self.current_token = saved_current;
                            self.peek_token = saved_peek;
                            self.current_span = saved_span;
                            self.peek_span = saved_peek_span;
                            declarations.push(Declaration::Struct(self.parse_struct()?));
                        }
                    } else {
                        // FIRST check if this is an impl block: Type.impl or Type<T>.impl
                        // Check if peek_token is '.' or '<' (indicating impl block)
                        if self.peek_token == Token::Symbol('.')
                            || self.peek_token == Token::Operator("<".to_string())
                        {
                            // Save state before trying to parse impl
                            let saved_before_impl_state = self.lexer.save_state();
                            let saved_before_impl_current = self.current_token.clone();
                            let saved_before_impl_peek = self.peek_token.clone();

                            // Try to parse as impl block
                            if let Token::Identifier(name) = &self.current_token {
                                let mut type_name = name.clone();
                                self.next_token(); // consume type name

                                // Check for generic parameters
                                if self.current_token == Token::Operator("<".to_string()) {
                                    type_name.push('<');
                                    self.next_token();
                                    loop {
                                        if let Token::Identifier(param) = &self.current_token {
                                            type_name.push_str(param);
                                            self.next_token();
                                        } else {
                                            break;
                                        }
                                        if self.current_token == Token::Symbol(',') {
                                            type_name.push(',');
                                            self.next_token();
                                        } else if self.current_token
                                            == Token::Operator(">".to_string())
                                        {
                                            type_name.push('>');
                                            self.next_token();
                                            break;
                                        } else {
                                            break;
                                        }
                                    }
                                }

                                // Check if next is '.' followed by 'impl'
                                if self.current_token == Token::Symbol('.') {
                                    self.next_token();
                                    if let Token::Identifier(method_name) = &self.current_token {
                                        if method_name == "impl" {
                                            // This IS an impl block!
                                            self.next_token();
                                            if self.current_token
                                                == Token::Operator("=".to_string())
                                            {
                                                self.next_token();
                                                declarations.push(Declaration::ImplBlock(
                                                    self.parse_impl_block(type_name)?,
                                                ));
                                                continue; // Skip to next declaration
                                            }
                                        }
                                    }
                                }
                            }

                            // Not an impl block, restore state
                            self.lexer.restore_state(saved_before_impl_state);
                            self.current_token = saved_before_impl_current.clone();
                            self.peek_token = saved_before_impl_peek.clone();
                        }

                        // Need to look ahead to determine if it's a struct, enum, behavior, trait, or function
                        self.next_token(); // Move to ':'
                        self.next_token(); // Move past ':' to see what comes after

                        // Check what comes after ':'
                        // Enums use commas to separate variants, structs use {}
                        let is_enum = matches!(&self.current_token, Token::Identifier(_))
                            || matches!(&self.current_token, Token::Symbol('.')); // Support .Variant syntax
                        let is_function = matches!(&self.current_token, Token::Symbol('('));
                        let is_behavior = matches!(&self.current_token, Token::Identifier(name) if name == "behavior");

                        // Check if it's a trait or struct (both start with '{')
                        let (is_struct, is_trait) =
                            if matches!(&self.current_token, Token::Symbol('{')) {
                                // Look ahead to see if it contains method signatures (trait) or fields (struct)
                                let saved_trait_state = self.lexer.save_state();
                                let saved_trait_current = self.current_token.clone();
                                let saved_trait_peek = self.peek_token.clone();

                                self.next_token(); // consume '{'

                                // Look for the pattern: identifier ':' '(' which indicates a trait method
                                let looks_like_trait =
                                    if let Token::Identifier(_) = &self.current_token {
                                        self.next_token();
                                        self.current_token == Token::Symbol(':') && {
                                            self.next_token();
                                            self.current_token == Token::Symbol('(')
                                        }
                                    } else {
                                        false
                                    };

                                // Restore state
                                self.lexer.restore_state(saved_trait_state);
                                self.current_token = saved_trait_current;
                                self.peek_token = saved_trait_peek;

                                if looks_like_trait {
                                    (false, true)
                                } else {
                                    (true, false)
                                }
                            } else {
                                (false, false)
                            };

                        // Restore lexer state
                        self.lexer.restore_state(saved_state);
                        self.current_token = saved_current_token;
                        self.peek_token = saved_peek_token;

                        if is_behavior {
                            declarations.push(Declaration::Behavior(self.parse_behavior()?));
                        } else if is_trait {
                            declarations.push(Declaration::Trait(self.parse_trait()?));
                        } else if is_enum {
                            declarations.push(Declaration::Enum(self.parse_enum()?));
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
                    let type_name = if let Token::Identifier(name) = &self.current_token {
                        let mut full_name = name.clone();
                        self.next_token(); // consume type name

                        // Check for generic parameters: Type<T, U>
                        if self.current_token == Token::Operator("<".to_string()) {
                            full_name.push('<');
                            self.next_token(); // consume '<'

                            // Parse type parameters
                            loop {
                                if let Token::Identifier(param) = &self.current_token {
                                    full_name.push_str(param);
                                    self.next_token();
                                } else {
                                    break;
                                }

                                if self.current_token == Token::Symbol(',') {
                                    full_name.push(',');
                                    self.next_token();
                                } else if self.current_token == Token::Operator(">".to_string()) {
                                    full_name.push('>');
                                    self.next_token();
                                    break;
                                } else {
                                    break;
                                }
                            }
                        }

                        full_name
                    } else {
                        unreachable!()
                    };

                    // Save state for potential backtrack
                    let saved_impl_state = self.lexer.save_state();
                    let saved_current_token = self.current_token.clone();
                    let saved_peek_token = self.peek_token.clone();

                    // Now check for '.'
                    if self.current_token != Token::Symbol('.') {
                        // Not an impl block, restore and continue
                        self.lexer.restore_state(saved_impl_state);
                        self.current_token = saved_current_token;
                        self.peek_token = saved_peek_token;
                        // Continue parsing as something else
                        declarations.push(Declaration::Function(self.parse_function()?));
                        continue;
                    }

                    self.next_token(); // consume '.'

                    if let Token::Identifier(method_name) = &self.current_token {
                        if method_name == "impl" {
                            // This is an impl block: Type.impl = { ... }
                            self.next_token(); // consume 'impl'

                            // Expect '='
                            if self.current_token != Token::Operator("=".to_string()) {
                                return Err(CompileError::SyntaxError(
                                    format!(
                                        "Expected '=' after 'impl', got {:?}",
                                        self.current_token
                                    ),
                                    Some(self.current_span.clone()),
                                ));
                            }
                            self.next_token(); // consume '='

                            declarations
                                .push(Declaration::ImplBlock(self.parse_impl_block(type_name)?));
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
                                // Not a recognized method and not a method definition, restore and error
                                self.lexer.restore_state(saved_impl_state);
                                self.current_token = saved_current_token;
                                self.peek_token = saved_peek_token;

                                return Err(CompileError::SyntaxError(
                                    format!(
                                        "Expected 'implements' or 'requires' after '{}.'",
                                        type_name
                                    ),
                                    Some(self.current_span.clone()),
                                ));
                            }
                        }
                    } else {
                        // Not an identifier, restore and error
                        self.lexer.restore_state(saved_impl_state);
                        self.current_token = saved_current_token;
                        self.peek_token = saved_peek_token;

                        return Err(CompileError::SyntaxError(
                            format!("Expected 'implements' or 'requires' after '{}.'", type_name),
                            Some(self.current_span.clone()),
                        ));
                    }
                } else if self.peek_token == Token::Symbol('(') {
                    // Could be an external function declaration
                    declarations.push(Declaration::ExternalFunction(
                        self.parse_external_function()?,
                    ));
                } else if self.peek_token == Token::Operator(":=".to_string()) {
                    // Top-level constant declaration: name := value
                    let name = if let Token::Identifier(name) = &self.current_token {
                        name.clone()
                    } else {
                        unreachable!()
                    };
                    self.next_token(); // consume identifier
                    self.next_token(); // consume ':='
                    let value = self.parse_expression()?;

                    // Skip optional semicolon
                    if self.current_token == Token::Symbol(';') {
                        self.next_token();
                    }

                    // Create a constant declaration
                    declarations.push(Declaration::Constant {
                        name,
                        value,
                        type_: None, // Type will be inferred
                    });
                } else if self.peek_token == Token::Operator("=".to_string()) {
                    // Could be either:
                    // 1. Function declaration: name = (params) returnType { ... }
                    // 2. Variable declaration: name = value
                    // Need to look ahead to distinguish

                    // Save state for restoration
                    let saved_fn_state = self.lexer.save_state();
                    let saved_current_token = self.current_token.clone();
                    let saved_peek_token = self.peek_token.clone();

                    let name = if let Token::Identifier(n) = &self.current_token {
                        n.clone()
                    } else {
                        unreachable!()
                    };

                    // Look ahead past =
                    self.next_token(); // move to =
                    self.next_token(); // move past =

                    // Check if it's a function (starts with '(')
                    let is_function = self.current_token == Token::Symbol('(');

                    // Restore state
                    self.lexer.restore_state(saved_fn_state);
                    self.current_token = saved_current_token;
                    self.peek_token = saved_peek_token;

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
                                    });
                                }
                            }
                        } else {
                            // Regular variable declaration
                            let value = self.parse_expression()?;

                            // Skip optional semicolon
                            if self.current_token == Token::Symbol(';') {
                                self.next_token();
                            }

                            // Create a top-level constant declaration (immutable by default at top level)
                            declarations.push(Declaration::Constant {
                                name,
                                value,
                                type_: None, // Type will be inferred
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
                if self.current_token == Token::Symbol(';') {
                    self.next_token();
                }
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
                self.next_token();
                let label = if let Token::Identifier(label_name) = &self.current_token {
                    let label_name = label_name.clone();
                    self.next_token();
                    Some(label_name)
                } else {
                    None
                };
                if self.current_token == Token::Symbol(';') {
                    self.next_token();
                }
                Ok(Statement::Break { label })
            }
            Token::Identifier(id) if id == "defer" => {
                self.next_token(); // consume 'defer'

                // Parse the statement or block to defer
                let deferred_stmt = if self.current_token == Token::Symbol('{') {
                    // Parse block
                    self.next_token(); // consume '{'
                    let mut statements = Vec::new();
                    while self.current_token != Token::Symbol('}')
                        && self.current_token != Token::Eof
                    {
                        statements.push(self.parse_statement()?);
                    }
                    if self.current_token != Token::Symbol('}') {
                        return Err(CompileError::SyntaxError(
                            "Expected '}' to close defer block".to_string(),
                            Some(self.current_span.clone()),
                        ));
                    }
                    self.next_token(); // consume '}'

                    // Wrap multiple statements in a single defer by using the first as the deferred action
                    // In a real implementation, we'd need a Block statement type
                    if statements.is_empty() {
                        Statement::Expression {
                            expr: Expression::Boolean(true),
                            span: None,
                        } // No-op
                    } else if statements.len() == 1 {
                        statements.into_iter().next().unwrap()
                    } else {
                        // For now, only support single statement in defer
                        // TODO: Add Block statement type to support multiple statements
                        statements.into_iter().next().unwrap()
                    }
                } else {
                    // Single statement
                    self.parse_statement()?
                };

                Ok(Statement::Defer(Box::new(deferred_stmt)))
            }
            Token::Identifier(id) if id == "continue" => {
                self.next_token();
                let label = if let Token::Identifier(label_name) = &self.current_token {
                    let label_name = label_name.clone();
                    self.next_token();
                    Some(label_name)
                } else {
                    None
                };
                if self.current_token == Token::Symbol(';') {
                    self.next_token();
                }
                Ok(Statement::Continue { label })
            }
            Token::Identifier(id) if id == "comptime" => {
                // Parse comptime block as statement
                let span = Some(self.current_span.clone());
                self.next_token(); // consume 'comptime'
                if self.current_token != Token::Symbol('{') {
                    // It's a comptime expression, not a block
                    let expr = Expression::Comptime(Box::new(self.parse_expression()?));
                    if self.current_token == Token::Symbol(';') {
                        self.next_token();
                    }
                    return Ok(Statement::Expression { expr, span });
                }
                self.next_token(); // consume '{'

                let mut statements = vec![];
                while self.current_token != Token::Symbol('}') && self.current_token != Token::Eof {
                    statements.push(self.parse_statement()?);
                }

                if self.current_token != Token::Symbol('}') {
                    return Err(CompileError::SyntaxError(
                        "Expected '}' to close comptime block".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
                self.next_token(); // consume '}'
                Ok(Statement::ComptimeBlock(statements))
            }
            Token::AtThis => {
                // Parse @this.defer() statement
                self.next_token(); // consume '@this'

                if self.current_token != Token::Symbol('.') {
                    return Err(CompileError::SyntaxError(
                        "Expected '.' after '@this'".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
                self.next_token(); // consume '.'

                if let Token::Identifier(id) = &self.current_token {
                    if id == "defer" {
                        self.next_token(); // consume 'defer'

                        if self.current_token != Token::Symbol('(') {
                            return Err(CompileError::SyntaxError(
                                "Expected '(' after '@this.defer'".to_string(),
                                Some(self.current_span.clone()),
                            ));
                        }
                        self.next_token(); // consume '('

                        // Parse the expression to defer
                        let expr = self.parse_expression()?;

                        if self.current_token != Token::Symbol(')') {
                            return Err(CompileError::SyntaxError(
                                "Expected ')' after defer expression".to_string(),
                                Some(self.current_span.clone()),
                            ));
                        }
                        self.next_token(); // consume ')'

                        // Optional semicolon
                        if self.current_token == Token::Symbol(';') {
                            self.next_token();
                        }

                        Ok(Statement::ThisDefer(expr))
                    } else {
                        return Err(CompileError::SyntaxError(
                            "Expected 'defer' after '@this.' - only 'defer' is supported"
                                .to_string(),
                            Some(self.current_span.clone()),
                        ));
                    }
                } else {
                    return Err(CompileError::SyntaxError(
                        "Expected 'defer' after '@this.' - only 'defer' is supported".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
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
                        // Parse the left-hand side expression first
                        let span = Some(self.current_span.clone());
                        let lhs = self.parse_expression()?;

                        // Check if it's followed by an assignment
                        if self.current_token == Token::Operator("=".to_string()) {
                            self.next_token(); // consume '='
                            let value = self.parse_expression()?;
                            if self.current_token == Token::Symbol(';') {
                                self.next_token();
                            }
                            // Use PointerAssignment for member field assignments and array element assignments
                            Ok(Statement::PointerAssignment {
                                pointer: lhs,
                                value,
                            })
                        } else {
                            // Just an expression statement
                            if self.current_token == Token::Symbol(';') {
                                self.next_token();
                            }
                            Ok(Statement::Expression { expr: lhs, span })
                        }
                    }
                    _ => {
                        // Not a variable declaration, treat as expression
                        let span = Some(self.current_span.clone());
                        let expr = self.parse_expression()?;
                        if self.current_token == Token::Symbol(';') {
                            self.next_token();
                        }
                        Ok(Statement::Expression { expr, span })
                    }
                }
            }
            Token::Symbol('?') => {
                // Parse conditional expression
                let span = Some(self.current_span.clone());
                let expr = self.parse_expression()?;
                if self.current_token == Token::Symbol(';') {
                    self.next_token();
                }
                Ok(Statement::Expression { expr, span })
            }
            Token::Symbol('(') => {
                // Parse parenthesized expression as statement
                let span = Some(self.current_span.clone());
                let expr = self.parse_expression()?;
                if self.current_token == Token::Symbol(';') {
                    self.next_token();
                }
                Ok(Statement::Expression { expr, span })
            }
            Token::Symbol('{') => {
                // Parse destructuring import: { io, maths } = @std
                self.parse_destructuring_import()
            }
            // Handle literal expressions as valid statements
            Token::Integer(_) | Token::Float(_) | Token::StringLiteral(_) => {
                let span = Some(self.current_span.clone());
                let expr = self.parse_expression()?;
                if self.current_token == Token::Symbol(';') {
                    self.next_token();
                }
                Ok(Statement::Expression { expr, span })
            }
            // Handle @builtin and @std as expression statements (for calls like @builtin.store())
            Token::AtBuiltin | Token::AtStd => {
                let span = Some(self.current_span.clone());
                let expr = self.parse_expression()?;
                if self.current_token == Token::Symbol(';') {
                    self.next_token();
                }
                Ok(Statement::Expression { expr, span })
            }
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

        if self.current_token == Token::Symbol(';') {
            self.next_token();
        }

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

        // Opening brace
        if self.current_token != Token::Symbol('{') {
            return Err(CompileError::SyntaxError(
                "Expected '{' for loop body".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();

        // Parse loop body
        let mut body = vec![];
        while self.current_token != Token::Symbol('}') && self.current_token != Token::Eof {
            body.push(self.parse_statement()?);
        }

        if self.current_token != Token::Symbol('}') {
            return Err(CompileError::SyntaxError(
                "Expected '}' to close loop body".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();

        Ok(Statement::Loop { kind, label, body })
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

        // Consume semicolon if present
        if self.current_token == Token::Symbol(';') {
            self.next_token();
        }

        // Return a VariableAssignment statement - the typechecker will determine if this is valid
        // (i.e., if the variable was declared as mutable with ::=)
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

    /// Parse type name with optional generic parameters (e.g., "Type" or "Type<T, U>")
    /// Returns the full type name as a string.
    fn parse_type_name_with_generics(&mut self) -> Result<String> {
        let mut type_name = if let Token::Identifier(name) = &self.current_token {
            name.clone()
        } else {
            return Err(CompileError::SyntaxError(
                "Expected type name".to_string(),
                Some(self.current_span.clone()),
            ));
        };
        self.next_token(); // consume name

        // Check for generic parameters
        if self.current_token == Token::Operator("<".to_string()) {
            type_name.push('<');
            self.next_token(); // consume '<'

            loop {
                if let Token::Identifier(param) = &self.current_token {
                    type_name.push_str(param);
                    self.next_token();
                } else {
                    break;
                }

                if self.current_token == Token::Symbol(',') {
                    type_name.push(',');
                    self.next_token();
                } else if self.current_token == Token::Operator(">".to_string()) {
                    type_name.push('>');
                    self.next_token();
                    break;
                } else {
                    break;
                }
            }
        }

        Ok(type_name)
    }

    /// Parse an impl block: Type.impl = { methods... } or Type<T>.impl = { methods... }
    fn parse_impl_block_declaration(&mut self) -> Result<Declaration> {
        let type_name = self.parse_type_name_with_generics()?;

        // Expect '.'
        if self.current_token != Token::Symbol('.') {
            return Err(CompileError::SyntaxError(
                "Expected '.' after type name for impl block".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token(); // consume '.'

        // Expect 'impl'
        if !matches!(&self.current_token, Token::Identifier(name) if name == "impl") {
            return Err(CompileError::SyntaxError(
                "Expected 'impl' after '.'".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token(); // consume 'impl'

        // Expect '='
        if self.current_token != Token::Operator("=".to_string()) {
            return Err(CompileError::SyntaxError(
                "Expected '=' after 'impl'".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token(); // consume '='

        Ok(Declaration::ImplBlock(self.parse_impl_block(type_name)?))
    }

    /// Parse a method definition: Type.method = (params) ReturnType { body }
    fn parse_method_definition(&mut self) -> Result<Declaration> {
        let type_name = self.parse_type_name_with_generics()?;

        // Expect '.'
        if self.current_token != Token::Symbol('.') {
            return Err(CompileError::SyntaxError(
                "Expected '.' after type name for method definition".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token(); // consume '.'

        // Get method name
        let method_name = if let Token::Identifier(name) = &self.current_token {
            name.clone()
        } else {
            return Err(CompileError::SyntaxError(
                "Expected method name after '.'".to_string(),
                Some(self.current_span.clone()),
            ));
        };

        // Build full function name "Type.method"
        let full_function_name = format!("{}.{}", type_name, method_name);

        // Parse the function (current token is at method name)
        let mut func = self.parse_function()?;
        func.name = full_function_name;

        Ok(Declaration::Function(func))
    }

    /// Parse a top-level constant: name := value
    fn parse_top_level_constant(&mut self, name: String) -> Result<Declaration> {
        self.next_token(); // consume ':='
        let value = self.parse_expression()?;

        // Skip optional semicolon
        if self.current_token == Token::Symbol(';') {
            self.next_token();
        }

        Ok(Declaration::Constant {
            name,
            value,
            type_: None,
        })
    }

    /// Parse a module import: name = @std.module or name = @std.build.import("module")
    fn parse_module_import_declaration(&mut self, name: String) -> Result<Declaration> {
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
            if full_path.ends_with(".import") && self.current_token == Token::Symbol('(') {
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

                    return Ok(Declaration::ModuleImport {
                        alias: name,
                        module_path: format!("@std.{}", imported_module),
                    });
                } else {
                    return Err(CompileError::SyntaxError(
                        "Expected string literal in import()".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
            }

            // Direct module import: name = @std.module
            Ok(Declaration::ModuleImport {
                alias: name,
                module_path: full_path,
            })
        } else {
            Err(CompileError::SyntaxError(
                "Expected module path".to_string(),
                Some(self.current_span.clone()),
            ))
        }
    }

    // ========================================================================
    // END HELPER FUNCTIONS
    // ========================================================================

    fn parse_destructuring_import(&mut self) -> Result<Statement> {
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

        // Consume semicolon if present
        if self.current_token == Token::Symbol(';') {
            self.next_token();
        }

        // Convert to DestructuringImport statement
        Ok(Statement::DestructuringImport { names, source })
    }
}
