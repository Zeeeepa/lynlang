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
                // Check for := operator (likely a module import or constant)
                if self.peek_token == Token::Operator(":=".to_string()) {
                    // Save the name and complete state
                    let var_name = name.clone();
                    let saved_current = self.current_token.clone();
                    let saved_peek = self.peek_token.clone();
                    let saved_lex_pos = self.lexer.position;
                    let saved_lex_read = self.lexer.read_position;
                    let saved_lex_char = self.lexer.current_char;

                    // Look ahead to see if this is a module import
                    self.next_token(); // Move to :=
                    self.next_token(); // Move past :=

                    // Check if the right side starts with @std or is a build.import call
                    let is_module_import = if self.current_token == Token::AtStd {
                        true
                    } else if let Token::Identifier(id) = &self.current_token {
                        if id.starts_with("@std") {
                            true
                        } else if id == "build" {
                            // Check for build.import pattern
                            let saved_pos = self.lexer.position;
                            let saved_read_pos = self.lexer.read_position;
                            let saved_char = self.lexer.current_char;
                            let saved_current = self.current_token.clone();
                            let saved_peek = self.peek_token.clone();

                            self.next_token();
                            let is_import = self.current_token == Token::Symbol('.')
                                && {
                                    self.next_token();
                                    matches!(&self.current_token, Token::Identifier(name) if name == "import")
                                };

                            // Restore position AND tokens
                            self.lexer.position = saved_pos;
                            self.lexer.read_position = saved_read_pos;
                            self.lexer.current_char = saved_char;
                            self.current_token = saved_current;
                            self.peek_token = saved_peek;

                            is_import
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    // Restore complete state
                    self.current_token = saved_current;
                    self.peek_token = saved_peek;
                    self.lexer.position = saved_lex_pos;
                    self.lexer.read_position = saved_lex_read;
                    self.lexer.current_char = saved_lex_char;

                    if is_module_import {
                        // Parse as module import
                        let alias = var_name;
                        self.next_token(); // consume name
                        self.next_token(); // consume :=

                        // Check if it's a build.import call
                        if let Token::Identifier(id) = &self.current_token {
                            if id == "build" {
                                // Handle build.import("module_name") pattern
                                self.next_token(); // consume 'build'
                                if self.current_token != Token::Symbol('.') {
                                    return Err(CompileError::SyntaxError(
                                        "Expected '.' after 'build'".to_string(),
                                        Some(self.current_span.clone()),
                                    ));
                                }
                                self.next_token(); // consume '.'

                                if !matches!(&self.current_token, Token::Identifier(name) if name == "import")
                                {
                                    return Err(CompileError::SyntaxError(
                                        "Expected 'import' after 'build.'".to_string(),
                                        Some(self.current_span.clone()),
                                    ));
                                }
                                self.next_token(); // consume 'import'

                                if self.current_token != Token::Symbol('(') {
                                    return Err(CompileError::SyntaxError(
                                        "Expected '(' after 'build.import'".to_string(),
                                        Some(self.current_span.clone()),
                                    ));
                                }
                                self.next_token(); // consume '('

                                // Get the module name from the string literal
                                let module_name =
                                    if let Token::StringLiteral(name) = &self.current_token {
                                        name.clone()
                                    } else {
                                        return Err(CompileError::SyntaxError(
                                            "Expected string literal for module name".to_string(),
                                            Some(self.current_span.clone()),
                                        ));
                                    };
                                self.next_token(); // consume string

                                if self.current_token != Token::Symbol(')') {
                                    return Err(CompileError::SyntaxError(
                                        "Expected ')' after module name".to_string(),
                                        Some(self.current_span.clone()),
                                    ));
                                }
                                self.next_token(); // consume ')'

                                // Create ModuleImport with std.module_name path
                                declarations.push(Declaration::ModuleImport {
                                    alias,
                                    module_path: format!("std.{}", module_name),
                                });
                            } else {
                                // Parse the module path (@std.module or similar)
                                let module_path = id.clone();
                                self.next_token();

                                // Handle member access for @std.module
                                let full_path = if self.current_token == Token::Symbol('.') {
                                    let mut path = module_path;
                                    while self.current_token == Token::Symbol('.') {
                                        self.next_token(); // consume '.'
                                        if let Token::Identifier(member) = &self.current_token {
                                            path.push('.');
                                            path.push_str(member);
                                            self.next_token();
                                        } else {
                                            break;
                                        }
                                    }
                                    path
                                } else {
                                    module_path
                                };

                                // Create ModuleImport declaration
                                declarations.push(Declaration::ModuleImport {
                                    alias,
                                    module_path: full_path,
                                });
                            }
                        } else {
                            return Err(CompileError::SyntaxError(
                                "Expected module path after :=".to_string(),
                                Some(self.current_span.clone()),
                            ));
                        }
                    } else {
                        // Parse as a constant declaration
                        let stmt = self.parse_statement()?;
                        // Convert statement to declaration
                        if let Statement::VariableDeclaration {
                            name,
                            type_,
                            initializer,
                            ..
                        } = stmt
                        {
                            if let Some(init) = initializer {
                                declarations.push(Declaration::Constant {
                                    name,
                                    type_,
                                    value: init,
                                });
                            } else {
                                return Err(CompileError::SyntaxError(
                                    "Constant declaration requires an initializer".to_string(),
                                    Some(self.current_span.clone()),
                                ));
                            }
                        } else {
                            return Err(CompileError::SyntaxError(
                                "Expected variable declaration".to_string(),
                                Some(self.current_span.clone()),
                            ));
                        }
                    }
                } else if self.peek_token == Token::Operator("::".to_string()) {
                    // Could be either:
                    // 1. Function with type annotation: name :: (params) -> returnType { ... }
                    // 2. Variable declaration: name :: Type = value
                    // Need to look ahead to distinguish

                    // Save state for restoration
                    let saved_position = self.lexer.position;
                    let saved_read_position = self.lexer.read_position;
                    let saved_current_char = self.lexer.current_char;
                    let saved_current_token = self.current_token.clone();
                    let saved_peek_token = self.peek_token.clone();

                    // Look ahead past ::
                    self.next_token(); // move to ::
                    self.next_token(); // move past ::

                    // Check if it's a function (starts with '(')
                    let is_function = self.current_token == Token::Symbol('(');

                    // Restore state
                    self.lexer.position = saved_position;
                    self.lexer.read_position = saved_read_position;
                    self.lexer.current_char = saved_current_char;
                    self.current_token = saved_current_token;
                    self.peek_token = saved_peek_token;

                    if is_function {
                        // Function with type annotation: name :: (params) -> returnType { ... }
                        declarations.push(Declaration::Function(self.parse_function()?));
                    } else {
                        // Variable declaration: name :: Type = value
                        // Parse as top-level variable declaration
                        let name = if let Token::Identifier(name) = &self.current_token {
                            name.clone()
                        } else {
                            unreachable!()
                        };
                        self.next_token(); // consume identifier
                        self.next_token(); // consume '::'

                        let type_ = self.parse_type()?;

                        if self.current_token != Token::Operator("=".to_string()) {
                            return Err(CompileError::SyntaxError(
                                "Expected '=' after type in mutable variable declaration"
                                    .to_string(),
                                Some(self.current_span.clone()),
                            ));
                        }
                        self.next_token(); // consume '='

                        let value = self.parse_expression()?;

                        // Skip optional semicolon
                        if self.current_token == Token::Symbol(';') {
                            self.next_token();
                        }

                        // Create a top-level mutable variable declaration
                        // For now, we'll treat it as a constant at the top level
                        declarations.push(Declaration::Constant {
                            name,
                            value,
                            type_: Some(type_),
                        });
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
                    let saved_position = self.lexer.position;
                    let saved_read_position = self.lexer.read_position;
                    let saved_current_char = self.lexer.current_char;
                    let saved_current_token = self.current_token.clone();
                    let saved_peek_token = self.peek_token.clone();

                    // If generics, need to look ahead to determine struct vs function
                    if self.peek_token == Token::Operator("<".to_string()) {
                        // Look ahead to see if it's a struct or a function with generics
                        let saved_position = self.lexer.position;
                        let saved_read_position = self.lexer.read_position;
                        let saved_current_char = self.lexer.current_char;
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
                                let saved_after_generics = (
                                    self.lexer.position,
                                    self.lexer.read_position,
                                    self.lexer.current_char,
                                    self.current_token.clone(),
                                    self.peek_token.clone(),
                                );
                                
                                self.next_token();
                                if let Token::Identifier(method_name) = &self.current_token {
                                    if method_name == "impl" {
                                        // This IS an impl block! Parse it
                                        // Restore to beginning and parse properly
                                        self.lexer.position = saved_position;
                                        self.lexer.read_position = saved_read_position;
                                        self.lexer.current_char = saved_current_char;
                                        self.current_token = saved_current.clone();
                                        self.peek_token = saved_peek.clone();
                                        self.current_span = saved_span.clone();
                                        self.peek_span = saved_peek_span.clone();
                                        
                                        // Parse type name with generics
                                        let mut type_name = if let Token::Identifier(name) = &self.current_token {
                                            name.clone()
                                        } else {
                                            return Err(CompileError::SyntaxError(
                                                "Expected type name for impl block".to_string(),
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
                                            } else if self.current_token == Token::Operator(">".to_string()) {
                                                type_name.push('>');
                                                self.next_token();
                                                break;
                                            } else {
                                                break;
                                            }
                                        }
                                        
                                        // Now parse .impl = { ... }
                                        if self.current_token == Token::Symbol('.') {
                                            self.next_token();
                                            if let Token::Identifier(method_name) = &self.current_token {
                                                if method_name == "impl" {
                                                    self.next_token();
                                                    if self.current_token == Token::Operator("=".to_string()) {
                                                        self.next_token();
                                                        declarations.push(Declaration::ImplBlock(
                                                            self.parse_impl_block(type_name)?,
                                                        ));
                                                        continue; // Skip to next declaration
                                                    }
                                                }
                                            }
                                        }
                                        
                                        return Err(CompileError::SyntaxError(
                                            "Expected '=' after 'impl'".to_string(),
                                            Some(self.current_span.clone()),
                                        ));
                                    }
                                }
                                
                                // Not an impl block - restore and skip struct/enum/function parsing
                                // (since '.' after generics is not valid for those)
                                self.lexer.position = saved_after_generics.0;
                                self.lexer.read_position = saved_after_generics.1;
                                self.lexer.current_char = saved_after_generics.2;
                                self.current_token = saved_after_generics.3.clone();
                                self.peek_token = saved_after_generics.4.clone();
                                
                                // Skip to next declaration - '.' after generics without 'impl' is invalid
                                continue;
                            }

                            // Check what comes after the generics
                            let is_struct = self.current_token == Token::Symbol(':')
                                && self.peek_token == Token::Symbol('{');
                            let is_enum = self.current_token == Token::Symbol(':')
                                && (matches!(&self.peek_token, Token::Identifier(_))
                                    || self.peek_token == Token::Symbol('.')); // Support .Variant syntax (enums use commas, not pipes)
                                                                               // Generic function: name<T>(params) return_type { ... }
                            let is_generic_function = self.current_token == Token::Symbol('(');
                            let is_function = (self.current_token == Token::Symbol(':')
                                && self.peek_token == Token::Symbol('('))
                                || is_generic_function;
                            let is_behavior = self.current_token == Token::Symbol(':')
                                && matches!(&self.peek_token, Token::Identifier(name) if name == "behavior");

                            // Restore lexer state
                            self.lexer.position = saved_position;
                            self.lexer.read_position = saved_read_position;
                            self.lexer.current_char = saved_current_char;
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
                            self.lexer.position = saved_position;
                            self.lexer.read_position = saved_read_position;
                            self.lexer.current_char = saved_current_char;
                            self.current_token = saved_current;
                            self.peek_token = saved_peek;
                            self.current_span = saved_span;
                            self.peek_span = saved_peek_span;
                            declarations.push(Declaration::Struct(self.parse_struct()?));
                        }
                    } else {
                        // FIRST check if this is an impl block: Type.impl or Type<T>.impl
                        // Check if peek_token is '.' or '<' (indicating impl block)
                        if self.peek_token == Token::Symbol('.') || 
                           self.peek_token == Token::Operator("<".to_string()) {
                            // Save state before trying to parse impl
                            let saved_before_impl = (
                                self.lexer.position,
                                self.lexer.read_position,
                                self.lexer.current_char,
                                self.current_token.clone(),
                                self.peek_token.clone(),
                            );
                            
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
                                        } else if self.current_token == Token::Operator(">".to_string()) {
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
                                            if self.current_token == Token::Operator("=".to_string()) {
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
                            self.lexer.position = saved_before_impl.0;
                            self.lexer.read_position = saved_before_impl.1;
                            self.lexer.current_char = saved_before_impl.2;
                            self.current_token = saved_before_impl.3.clone();
                            self.peek_token = saved_before_impl.4.clone();
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
                                let saved = (
                                    self.lexer.position,
                                    self.lexer.read_position,
                                    self.lexer.current_char,
                                    self.current_token.clone(),
                                    self.peek_token.clone(),
                                );

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
                                self.lexer.position = saved.0;
                                self.lexer.read_position = saved.1;
                                self.lexer.current_char = saved.2;
                                self.current_token = saved.3;
                                self.peek_token = saved.4;

                                if looks_like_trait {
                                    (false, true)
                                } else {
                                    (true, false)
                                }
                            } else {
                                (false, false)
                            };

                        // Restore lexer state
                        self.lexer.position = saved_position;
                        self.lexer.read_position = saved_read_position;
                        self.lexer.current_char = saved_current_char;
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
                } else if self.peek_token == Token::Symbol('.') || 
                          (self.peek_token == Token::Operator("<".to_string())) {
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
                    let saved_position = self.lexer.position;
                    let saved_read_position = self.lexer.read_position;
                    let saved_current_char = self.lexer.current_char;
                    let saved_current_token = self.current_token.clone();
                    let saved_peek_token = self.peek_token.clone();

                    // Now check for '.'
                    if self.current_token != Token::Symbol('.') {
                        // Not an impl block, restore and continue
                        self.lexer.position = saved_position;
                        self.lexer.read_position = saved_read_position;
                        self.lexer.current_char = saved_current_char;
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
                                    format!("Expected '=' after 'impl', got {:?}", self.current_token),
                                    Some(self.current_span.clone()),
                                ));
                            }
                            self.next_token(); // consume '='
                            
                            declarations.push(Declaration::ImplBlock(
                                self.parse_impl_block(type_name)?,
                            ));
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
                            // Not a recognized method, restore and error
                            self.lexer.position = saved_position;
                            self.lexer.read_position = saved_read_position;
                            self.lexer.current_char = saved_current_char;
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
                    } else {
                        // Not an identifier, restore and error
                        self.lexer.position = saved_position;
                        self.lexer.read_position = saved_read_position;
                        self.lexer.current_char = saved_current_char;
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
                    let saved_position = self.lexer.position;
                    let saved_read_position = self.lexer.read_position;
                    let saved_current_char = self.lexer.current_char;
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
                    self.lexer.position = saved_position;
                    self.lexer.read_position = saved_read_position;
                    self.lexer.current_char = saved_current_char;
                    self.current_token = saved_current_token;
                    self.peek_token = saved_peek_token;

                    if is_function {
                        // Function declaration: name = (params) returnType { ... }
                        // Check if it's prefixed with 'pub'
                        let is_public = false; // Will be set by checking previous token if needed
                        let mut func = self.parse_function()?;
                        // Note: pub parsing will be handled in parse_function
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
            } else if let Token::Identifier(id) = &self.current_token {
                if id == "type" {
                    // Parse type alias: type Name = Type or type Name<T> = Type<T>
                    declarations.push(Declaration::TypeAlias(self.parse_type_alias()?));
                } else if id == "comptime" {
                    // Parse comptime block
                    self.next_token(); // consume 'comptime'
                    if self.current_token != Token::Symbol('{') {
                        return Err(CompileError::SyntaxError(
                            "Expected '{' after comptime".to_string(),
                            Some(self.current_span.clone()),
                        ));
                    }
                    self.next_token(); // consume '{'

                    let mut statements = vec![];
                    while self.current_token != Token::Symbol('}')
                        && self.current_token != Token::Eof
                    {
                        let stmt = self.parse_statement()?;

                        // Check if the parsed statement contains an import
                        if let Statement::VariableDeclaration {
                            initializer, name, ..
                        } = &stmt
                        {
                            if let Some(expr) = initializer {
                                // Check for @std patterns or build.import calls
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

                                // Reject imports inside comptime blocks
                                if is_import {
                                    return Err(CompileError::SyntaxError(
                                        format!("Import '{}' not allowed inside comptime block. Imports must be at module level.", name),
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

                    // Add comptime block to declarations
                    declarations.push(Declaration::ComptimeBlock(statements));
                } else {
                    return Err(CompileError::SyntaxError(
                        format!("Unexpected identifier at top level: {:?}", id),
                        Some(self.current_span.clone()),
                    ));
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
                self.next_token();
                let expr = self.parse_expression()?;
                if self.current_token == Token::Symbol(';') {
                    self.next_token();
                }
                Ok(Statement::Return(expr))
            }
            Token::Identifier(id) if id == "loop" => {
                // Check if this is loop() expression or loop{} statement
                if self.peek_token == Token::Symbol('(') {
                    // loop(() { ... }) - expression form
                    let expr = self.parse_expression()?;
                    Ok(Statement::Expression(expr))
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
                        Statement::Expression(Expression::Boolean(true)) // No-op
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
                self.next_token(); // consume 'comptime'
                if self.current_token != Token::Symbol('{') {
                    // It's a comptime expression, not a block
                    let expr = Expression::Comptime(Box::new(self.parse_expression()?));
                    if self.current_token == Token::Symbol(';') {
                        self.next_token();
                    }
                    return Ok(Statement::Expression(expr));
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
                            Ok(Statement::Expression(lhs))
                        }
                    }
                    _ => {
                        // Not a variable declaration, treat as expression
                        let expr = self.parse_expression()?;
                        if self.current_token == Token::Symbol(';') {
                            self.next_token();
                        }
                        Ok(Statement::Expression(expr))
                    }
                }
            }
            Token::Symbol('?') => {
                // Parse conditional expression
                let expr = self.parse_expression()?;
                if self.current_token == Token::Symbol(';') {
                    self.next_token();
                }
                Ok(Statement::Expression(expr))
            }
            Token::Symbol('(') => {
                // Parse parenthesized expression as statement
                let expr = self.parse_expression()?;
                if self.current_token == Token::Symbol(';') {
                    self.next_token();
                }
                Ok(Statement::Expression(expr))
            }
            Token::Symbol('{') => {
                // Parse destructuring import: { io, maths } = @std
                self.parse_destructuring_import()
            }
            // Handle literal expressions as valid statements
            Token::Integer(_) | Token::Float(_) | Token::StringLiteral(_) => {
                let expr = self.parse_expression()?;
                if self.current_token == Token::Symbol(';') {
                    self.next_token();
                }
                Ok(Statement::Expression(expr))
            }
            _ => Err(CompileError::SyntaxError(
                format!("Unexpected token in statement: {:?}", self.current_token),
                Some(self.current_span.clone()),
            )),
        }
    }

    fn parse_variable_declaration(&mut self) -> Result<Statement> {
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
                // Immutable assignment: name = value (per LANGUAGE_SPEC.zen)
                self.next_token();
                (false, VariableDeclarationType::InferredImmutable, None)
            }
            Token::Operator(op) if op == "::=" => {
                // Inferred mutable: name ::= value
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
        Ok(Statement::VariableAssignment { name, value })
    }

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
