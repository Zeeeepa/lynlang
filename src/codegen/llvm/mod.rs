#![allow(dead_code)]

use crate::ast::{self, AstType};
use crate::comptime;
use crate::error::{CompileError, Span};
use crate::well_known::WellKnownTypes;
use inkwell::{
    basic_block::BasicBlock,
    builder::Builder,
    context::Context,
    module::Module,
    types::{BasicType, BasicTypeEnum, FunctionType, StructType},
    values::{BasicValueEnum, FunctionValue, PointerValue},
};
use std::collections::HashMap;

mod behaviors;
mod binary_ops;
mod builtins;
mod control_flow;
mod expressions;
mod functions;
mod generics;
mod literals;
mod pointers;
mod statements;
mod stdlib_codegen;
mod strings;
mod structs;
mod symbols;
mod types;
mod vec_support;

#[derive(Debug, Clone)]
pub enum Type<'ctx> {
    Basic(BasicTypeEnum<'ctx>),
    Pointer(Box<Type<'ctx>>),
    Struct(StructType<'ctx>),
    Function(FunctionType<'ctx>),
    Void,
}

impl<'ctx> Type<'ctx> {
    pub fn into_basic_type(self) -> Result<BasicTypeEnum<'ctx>, CompileError> {
        match self {
            Type::Basic(t) => Ok(t),
            // Note: This method doesn't have access to current_span, so we leave it as None
            // The caller should use add_span_to_error if needed
            _ => Err(CompileError::TypeMismatch {
                expected: "basic type".to_string(),
                found: format!("{:?}", self),
                span: None,
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StructTypeInfo<'ctx> {
    pub llvm_type: StructType<'ctx>,
    pub fields: HashMap<String, (usize, AstType)>,
}

// Variable information with mutability tracking
#[derive(Debug, Clone)]
pub struct VariableInfo<'ctx> {
    pub pointer: PointerValue<'ctx>,
    pub ast_type: AstType,
    pub is_mutable: bool,
    pub is_initialized: bool,
    /// The source location where this variable was defined.
    /// Used for "previous declaration was here" diagnostics.
    pub definition_span: Option<Span>,
}

pub struct LLVMCompiler<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub variables: HashMap<String, VariableInfo<'ctx>>,
    pub functions: HashMap<String, FunctionValue<'ctx>>,
    pub function_types: HashMap<String, AstType>,
    pub current_function: Option<FunctionValue<'ctx>>,
    pub symbols: symbols::SymbolTable<'ctx>,
    pub struct_types: HashMap<String, StructTypeInfo<'ctx>>,
    pub loop_stack: Vec<(BasicBlock<'ctx>, BasicBlock<'ctx>)>,
    pub defer_stack: Vec<ast::Expression>,
    pub comptime_evaluator: comptime::ComptimeInterpreter,
    pub behavior_codegen: Option<behaviors::BehaviorCodegen<'ctx>>,
    pub current_impl_type: Option<String>,
    pub inline_counter: usize,
    pub load_counter: usize,
    pub generic_type_context: HashMap<String, AstType>,
    pub generic_tracker: generics::GenericTypeTracker,
    pub module_imports: HashMap<String, u64>,
    pub current_span: Option<Span>,
    pub well_known: WellKnownTypes,
}

impl<'ctx> LLVMCompiler<'ctx> {
    // ============================================================================
    // SPAN TRACKING HELPERS
    // These methods help propagate source location information to error messages
    // ============================================================================

    /// Set the current span for error reporting
    pub fn set_span(&mut self, span: Option<Span>) {
        self.current_span = span;
    }

    /// Get the current span for error reporting
    pub fn get_current_span(&self) -> Option<Span> {
        self.current_span.clone()
    }

    /// Create an error with the current span context
    pub fn error_with_span(&self, error: CompileError) -> CompileError {
        if self.current_span.is_some() {
            self.add_span_to_error(error)
        } else {
            error
        }
    }

    /// Add the current span to an error if it doesn't already have one
    fn add_span_to_error(&self, error: CompileError) -> CompileError {
        match error {
            CompileError::UndeclaredVariable(name, None) => {
                CompileError::UndeclaredVariable(name, self.current_span.clone())
            }
            CompileError::UndeclaredFunction(name, None) => {
                CompileError::UndeclaredFunction(name, self.current_span.clone())
            }
            CompileError::TypeMismatch {
                expected,
                found,
                span: None,
            } => CompileError::TypeMismatch {
                expected,
                found,
                span: self.current_span.clone(),
            },
            CompileError::InternalError(msg, None) => {
                CompileError::InternalError(msg, self.current_span.clone())
            }
            CompileError::UnsupportedFeature(msg, None) => {
                CompileError::UnsupportedFeature(msg, self.current_span.clone())
            }
            CompileError::SyntaxError(msg, None) => {
                CompileError::SyntaxError(msg, self.current_span.clone())
            }
            CompileError::TypeError(msg, None) => {
                CompileError::TypeError(msg, self.current_span.clone())
            }
            // If error already has a span, keep it
            other => other,
        }
    }

    // ============================================================================
    // PATTERN MATCHING
    // Basic pattern matching implementation for common cases
    // ============================================================================

    /// Find enum variant tag from symbol table
    fn find_enum_variant_tag(&self, variant: &str) -> Option<(u64, Option<inkwell::types::StructType<'ctx>>)> {
        for symbol in self.symbols.all_symbols() {
            if let symbols::Symbol::EnumType(info) = symbol {
                if let Some(&tag) = info.variant_indices.get(variant) {
                    return Some((tag, Some(info.llvm_type)));
                }
            }
        }
        None
    }

    /// Compare int value with expected tag
    fn compare_with_tag(
        &self,
        int_val: inkwell::values::IntValue<'ctx>,
        expected_tag: u64,
    ) -> Result<inkwell::values::IntValue<'ctx>, CompileError> {
        let tag_const = self.context.i64_type().const_int(expected_tag, false);
        let int_extended = if int_val.get_type().get_bit_width() < 64 {
            self.builder.build_int_z_extend(int_val, self.context.i64_type(), "ext")?
        } else {
            int_val
        };
        Ok(self.builder.build_int_compare(inkwell::IntPredicate::EQ, int_extended, tag_const, "enum_match")?)
    }

    /// Extract payload bindings from pattern (returns binding name with prefix or empty)
    fn extract_payload_binding(payload: &Option<Box<ast::Pattern>>, prefix: &str, value: BasicValueEnum<'ctx>) -> Vec<(String, BasicValueEnum<'ctx>)> {
        match payload.as_ref().map(|p| p.as_ref()) {
            Some(ast::Pattern::Identifier(name)) => vec![(format!("{}{}", prefix, name), value)],
            Some(ast::Pattern::Wildcard) | None => vec![],
            _ => vec![],
        }
    }

    /// Compare integer scrutinee with literal value
    fn compare_int_literal(
        &self,
        scrutinee_int: inkwell::values::IntValue<'ctx>,
        literal_val: inkwell::values::IntValue<'ctx>,
        name: &str,
    ) -> Result<(inkwell::values::IntValue<'ctx>, Vec<(String, BasicValueEnum<'ctx>)>), CompileError> {
        let matches = self.builder.build_int_compare(inkwell::IntPredicate::EQ, scrutinee_int, literal_val, name)?;
        Ok((matches, vec![]))
    }

    /// Compile a pattern test, returning (matches: i1, bindings)
    pub fn compile_pattern_test_with_type(
        &mut self,
        scrutinee: &BasicValueEnum<'ctx>,
        pattern: &ast::Pattern,
        _scrutinee_type: Option<&AstType>,
    ) -> Result<(inkwell::values::IntValue<'ctx>, Vec<(String, BasicValueEnum<'ctx>)>), CompileError> {
        let true_val = || self.context.bool_type().const_int(1, false);

        match pattern {
            ast::Pattern::Literal(expr) => self.compile_literal_pattern(scrutinee, expr),
            ast::Pattern::Wildcard => Ok((true_val(), vec![])),
            ast::Pattern::Identifier(name) => Ok((true_val(), vec![(name.clone(), *scrutinee)])),
            ast::Pattern::EnumLiteral { variant, payload } => {
                self.compile_enum_pattern(scrutinee, variant, payload)
            }
            ast::Pattern::Tuple(patterns) => {
                let mut all_bindings = vec![];
                for pattern in patterns {
                    let (_, bindings) = self.compile_pattern_test_with_type(scrutinee, pattern, None)?;
                    all_bindings.extend(bindings);
                }
                Ok((true_val(), all_bindings))
            }
            _ => Err(CompileError::UnsupportedFeature(
                format!("Pattern type not yet implemented: {:?}", pattern),
                self.current_span.clone(),
            ))
        }
    }

    /// Compile literal pattern matching
    fn compile_literal_pattern(
        &self,
        scrutinee: &BasicValueEnum<'ctx>,
        expr: &ast::Expression,
    ) -> Result<(inkwell::values::IntValue<'ctx>, Vec<(String, BasicValueEnum<'ctx>)>), CompileError> {
        let false_val = self.context.bool_type().const_int(0, false);

        let BasicValueEnum::IntValue(scrutinee_int) = scrutinee else {
            return Ok((false_val, vec![]));
        };

        match expr {
            ast::Expression::Boolean(b) => {
                let bit_width = scrutinee_int.get_type().get_bit_width();
                let literal_val = if bit_width == 1 {
                    self.context.bool_type().const_int(*b as u64, false)
                } else {
                    scrutinee_int.get_type().const_int(*b as u64, false)
                };
                self.compare_int_literal(*scrutinee_int, literal_val, "bool_match")
            }
            ast::Expression::Integer32(n) => {
                let literal_val = self.context.i32_type().const_int(*n as u64, false);
                self.compare_int_literal(*scrutinee_int, literal_val, "i32_match")
            }
            ast::Expression::Integer64(n) => {
                let literal_val = self.context.i64_type().const_int(*n as u64, false);
                self.compare_int_literal(*scrutinee_int, literal_val, "i64_match")
            }
            _ => Err(CompileError::UnsupportedFeature(
                format!("Unsupported literal pattern type: {:?}", expr),
                self.current_span.clone(),
            ))
        }
    }

    /// Compile enum pattern matching
    fn compile_enum_pattern(
        &mut self,
        scrutinee: &BasicValueEnum<'ctx>,
        variant: &str,
        payload: &Option<Box<ast::Pattern>>,
    ) -> Result<(inkwell::values::IntValue<'ctx>, Vec<(String, BasicValueEnum<'ctx>)>), CompileError> {
        match scrutinee {
            BasicValueEnum::PointerValue(ptr) => self.compile_enum_pattern_ptr(*ptr, variant, payload),
            BasicValueEnum::IntValue(int_val) => self.compile_enum_pattern_int(*int_val, variant),
            BasicValueEnum::StructValue(struct_val) => self.compile_enum_pattern_struct(*struct_val, variant, payload),
            _ => Err(CompileError::UnsupportedFeature(
                format!("Unsupported scrutinee type for enum pattern: {:?}", scrutinee),
                self.current_span.clone(),
            ))
        }
    }

    /// Compile enum pattern for pointer scrutinee
    fn compile_enum_pattern_ptr(
        &mut self,
        scrutinee_ptr: PointerValue<'ctx>,
        variant: &str,
        payload: &Option<Box<ast::Pattern>>,
    ) -> Result<(inkwell::values::IntValue<'ctx>, Vec<(String, BasicValueEnum<'ctx>)>), CompileError> {
        let Some((tag, Some(struct_type))) = self.find_enum_variant_tag(variant) else {
            return Err(CompileError::UnsupportedFeature(
                format!("Enum variant '{}' not found in symbol table", variant),
                self.current_span.clone(),
            ));
        };

        let tag_ptr = self.builder.build_struct_gep(struct_type, scrutinee_ptr, 0, "tag_ptr")?;
        let tag_val = self.builder.build_load(self.context.i64_type(), tag_ptr, "tag_val")?;
        let matches = self.compare_with_tag(tag_val.into_int_value(), tag)?;

        let bindings = if payload.is_some() {
            let payload_ptr = self.builder.build_struct_gep(struct_type, scrutinee_ptr, 1, "payload_ptr_ptr")?;
            Self::extract_payload_binding(payload, "__enum_payload_gep__", payload_ptr.into())
        } else {
            vec![]
        };

        Ok((matches, bindings))
    }

    /// Compile enum pattern for integer scrutinee (simple enum)
    fn compile_enum_pattern_int(
        &self,
        scrutinee_int: inkwell::values::IntValue<'ctx>,
        variant: &str,
    ) -> Result<(inkwell::values::IntValue<'ctx>, Vec<(String, BasicValueEnum<'ctx>)>), CompileError> {
        let Some((tag, _)) = self.find_enum_variant_tag(variant) else {
            return Err(CompileError::UnsupportedFeature(
                format!("Enum variant '{}' not found in symbol table", variant),
                self.current_span.clone(),
            ));
        };
        let matches = self.compare_with_tag(scrutinee_int, tag)?;
        Ok((matches, vec![]))
    }

    /// Compile enum pattern for struct value scrutinee
    fn compile_enum_pattern_struct(
        &self,
        scrutinee_struct: inkwell::values::StructValue<'ctx>,
        variant: &str,
        payload: &Option<Box<ast::Pattern>>,
    ) -> Result<(inkwell::values::IntValue<'ctx>, Vec<(String, BasicValueEnum<'ctx>)>), CompileError> {
        let Some((tag, _)) = self.find_enum_variant_tag(variant) else {
            return Err(CompileError::UnsupportedFeature(
                format!("Enum variant '{}' not found in symbol table", variant),
                self.current_span.clone(),
            ));
        };

        let tag_val = self.builder.build_extract_value(scrutinee_struct, 0, "tag_val")?;
        let matches = self.compare_with_tag(tag_val.into_int_value(), tag)?;

        let bindings = if payload.is_some() {
            let payload_ptr = self.builder.build_extract_value(scrutinee_struct, 1, "payload_ptr")?;
            Self::extract_payload_binding(payload, "__enum_payload_ptr__", payload_ptr)
        } else {
            vec![]
        };

        Ok((matches, bindings))
    }

    /// Get module ID for import tracking
    fn get_module_id(&self, module_name: &str) -> u64 {
        if self.well_known.is_option(module_name) || self.well_known.is_option_variant(module_name) {
            100
        } else if self.well_known.is_result(module_name) || self.well_known.is_result_variant(module_name) {
            101
        } else {
            match module_name {
                "HashMap" | "HashSet" | "DynVec" | "Array" | "Vec" => 102,
                "Allocator" | "get_default_allocator" => 103,
                "min" | "max" | "abs" | "sqrt" | "pow" | "sin" | "cos" | "tan" => 104,
                "io" => 1, "math" => 2, "core" => 3, "GPA" => 4, "AsyncPool" => 5, "build" => 7,
                _ => 0,
            }
        }
    }

    /// Get payload AST type from tracked generic types
    fn get_payload_ast_type(&self) -> Option<AstType> {
        self.generic_type_context.get("Option_Some_Type")
            .or_else(|| self.generic_type_context.get("Result_Ok_Type"))
            .or_else(|| self.generic_type_context.get("Result_Err_Type"))
            .cloned()
    }

    /// Load a value from pointer with fallback to i64
    fn load_with_type_or_i64(&self, ptr: PointerValue<'ctx>, llvm_type: Option<BasicTypeEnum<'ctx>>) -> Option<BasicValueEnum<'ctx>> {
        if let Some(ty) = llvm_type {
            self.builder.build_load(ty, ptr, "payload_val").ok()
        } else {
            self.builder.build_load(self.context.i64_type(), ptr, "payload_val").ok()
        }
    }

    /// Store a binding as a variable
    fn store_binding(&mut self, var_name: &str, value: BasicValueEnum<'ctx>, ast_type: AstType) {
        if let Ok(alloca) = self.builder.build_alloca(value.get_type(), var_name) {
            let _ = self.builder.build_store(alloca, value);
            self.variables.insert(var_name.to_string(), VariableInfo {
                pointer: alloca,
                ast_type,
                is_mutable: false,
                is_initialized: true,
                definition_span: self.current_span.clone(),
            });
        }
    }

    /// Infer AST type from LLVM value
    fn infer_ast_type(&self, value: &BasicValueEnum<'ctx>) -> AstType {
        match value {
            BasicValueEnum::IntValue(iv) => match iv.get_type().get_bit_width() {
                1 => AstType::Bool,
                32 => AstType::I32,
                _ => AstType::I64,
            },
            BasicValueEnum::FloatValue(fv) => {
                if fv.get_type() == self.context.f32_type() { AstType::F32 } else { AstType::F64 }
            }
            BasicValueEnum::PointerValue(_) => AstType::raw_ptr(AstType::Void),
            _ => AstType::I32,
        }
    }

    /// Apply pattern bindings to the current scope
    pub fn apply_pattern_bindings(&mut self, bindings: &[(String, BasicValueEnum<'ctx>)]) {
        for (name, value) in bindings {
            // Handle enum payload bindings (GEP needs 2 loads, ptr needs 1)
            let (prefix, needs_extra_load) = if name.starts_with("__enum_payload_gep__") {
                ("__enum_payload_gep__", true)
            } else if name.starts_with("__enum_payload_ptr__") {
                ("__enum_payload_ptr__", false)
            } else {
                // Regular binding
                let ast_type = self.infer_ast_type(value);
                self.store_binding(name, *value, ast_type);
                continue;
            };

            let var_name = &name[prefix.len()..];
            if !value.is_pointer_value() { continue; }

            let payload_ptr = if needs_extra_load {
                // GEP-based: first load the pointer from struct field
                let gep_ptr = value.into_pointer_value();
                let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                match self.builder.build_load(ptr_type, gep_ptr, "payload_ptr") {
                    Ok(loaded) => loaded.into_pointer_value(),
                    Err(_) => continue,
                }
            } else {
                value.into_pointer_value()
            };

            let payload_ast_type = self.get_payload_ast_type();
            let payload_llvm_type = payload_ast_type.as_ref()
                .and_then(|t| self.to_llvm_type(t).ok())
                .and_then(|t| self.expect_basic_type(t).ok());

            if let Some(payload_val) = self.load_with_type_or_i64(payload_ptr, payload_llvm_type) {
                let ast_type = payload_ast_type.unwrap_or(AstType::I64);
                self.store_binding(var_name, payload_val, ast_type);
            }
        }
    }

    /// Helper to track generic types in both old and new systems
    pub fn track_generic_type(&mut self, key: String, type_: AstType) {
        self.generic_type_context.insert(key.clone(), type_.clone());
        self.generic_tracker.insert(key, type_);
    }

    /// Helper to track complex generic types recursively
    pub fn track_complex_generic(&mut self, type_: &AstType, prefix: &str) {
        self.generic_tracker.track_generic_type(type_, prefix);

        // Also update the old system for backwards compatibility
        match type_ {
            AstType::Generic { name, type_args } => {
                if self.well_known.is_result(name) && type_args.len() == 2 {
                    self.generic_type_context
                        .insert(format!("{}_Ok_Type", prefix), type_args[0].clone());
                    self.generic_type_context
                        .insert(format!("{}_Err_Type", prefix), type_args[1].clone());
                } else if self.well_known.is_option(name) && type_args.len() == 1 {
                    self.generic_type_context
                        .insert(format!("{}_Some_Type", prefix), type_args[0].clone());
                }
            }
            _ => {}
        }
    }

    pub fn new(context: &'ctx Context) -> Self {
        let module = context.create_module("main");
        let builder = context.create_builder();
        let mut symbols = symbols::SymbolTable::new();
        let comptime_evaluator = comptime::ComptimeInterpreter::new();

        let i64_type = context.i64_type();
        let i32_type = context.i32_type();
        let float_type = context.f64_type();
        let bool_type = context.bool_type();

        symbols.insert("i64", symbols::Symbol::Type(i64_type.as_basic_type_enum()));
        symbols.insert("i32", symbols::Symbol::Type(i32_type.as_basic_type_enum()));
        symbols.insert(
            "f64",
            symbols::Symbol::Type(float_type.as_basic_type_enum()),
        );
        symbols.insert(
            "bool",
            symbols::Symbol::Type(bool_type.as_basic_type_enum()),
        );

        let mut compiler = Self {
            context,
            module,
            builder,
            variables: HashMap::new(),
            functions: HashMap::new(),
            function_types: HashMap::new(),
            current_function: None,
            symbols,
            struct_types: HashMap::new(),
            loop_stack: Vec::new(),
            defer_stack: Vec::new(),
            comptime_evaluator,
            behavior_codegen: Some(behaviors::BehaviorCodegen::new()),
            current_impl_type: None,
            inline_counter: 0,
            load_counter: 0,
            generic_type_context: HashMap::new(),
            generic_tracker: generics::GenericTypeTracker::new(),
            module_imports: HashMap::new(),
            current_span: None,
            well_known: WellKnownTypes::new(),
        };

        // Declare standard library functions
        compiler.declare_stdlib_functions();

        // Register built-in Option and Result enums
        compiler.register_builtin_enums();

        compiler
    }

    pub fn get_type(&self, name: &str) -> Result<BasicTypeEnum<'ctx>, CompileError> {
        self.symbols
            .lookup(name)
            .and_then(|sym| match sym {
                symbols::Symbol::Type(ty) => Some(*ty),
                _ => None,
            })
            .ok_or_else(|| {
                CompileError::UndeclaredVariable(name.to_string(), self.current_span.clone())
            })
    }

    // ============================================================================
    // TYPE-SAFE IR GENERATION HELPERS
    // These catch type mismatches at compile time instead of causing runtime segfaults
    // ============================================================================

    /// Type-safe store that verifies the value type matches the expected type.
    /// This prevents bugs like storing i64 into an i32 alloca.
    pub fn verified_store(
        &self,
        value: BasicValueEnum<'ctx>,
        ptr: PointerValue<'ctx>,
        expected_type: BasicTypeEnum<'ctx>,
        context: &str, // For error messages, e.g., "variable 'x'" or "struct field 'name'"
    ) -> Result<(), CompileError> {
        let value_type = value.get_type();

        // Check for type mismatch
        let mismatch = match (value_type, expected_type) {
            (BasicTypeEnum::IntType(vt), BasicTypeEnum::IntType(et)) => {
                vt.get_bit_width() != et.get_bit_width()
            }
            (BasicTypeEnum::FloatType(vt), BasicTypeEnum::FloatType(et)) => {
                // Compare by checking if they're the same type
                vt != et
            }
            (BasicTypeEnum::PointerType(_), BasicTypeEnum::PointerType(_)) => {
                // Opaque pointers are always compatible
                false
            }
            (BasicTypeEnum::StructType(vt), BasicTypeEnum::StructType(et)) => {
                // Struct types should match exactly
                vt != et
            }
            (BasicTypeEnum::ArrayType(vt), BasicTypeEnum::ArrayType(et)) => vt != et,
            (BasicTypeEnum::VectorType(vt), BasicTypeEnum::VectorType(et)) => vt != et,
            // Different type categories = mismatch
            _ => {
                // Special case: pointer and int can be compatible in some contexts
                // But generally different categories are a mismatch
                !matches!(
                    (&value_type, &expected_type),
                    (BasicTypeEnum::PointerType(_), BasicTypeEnum::IntType(_))
                        | (BasicTypeEnum::IntType(_), BasicTypeEnum::PointerType(_))
                )
            }
        };

        if mismatch {
            return Err(CompileError::InternalError(
                format!(
                    "LLVM IR type mismatch in store for {}: value has type {:?} but storage expects {:?}. \
                     This is a compiler bug - please report it.",
                    context,
                    value_type,
                    expected_type
                ),
                None,
            ));
        }

        self.builder
            .build_store(ptr, value)
            .map_err(CompileError::from)?;
        Ok(())
    }

    /// Type-safe store with automatic type coercion for integers.
    /// If value is an integer and sizes don't match, it will truncate or extend as needed.
    /// Returns the (possibly coerced) value that was stored.
    pub fn coercing_store(
        &self,
        value: BasicValueEnum<'ctx>,
        ptr: PointerValue<'ctx>,
        expected_type: BasicTypeEnum<'ctx>,
        _context: &str,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        let final_value = if let BasicValueEnum::IntValue(int_val) = value {
            if let BasicTypeEnum::IntType(expected_int_type) = expected_type {
                let val_bits = int_val.get_type().get_bit_width();
                let expected_bits = expected_int_type.get_bit_width();

                if val_bits > expected_bits {
                    // Truncate
                    self.builder
                        .build_int_truncate(int_val, expected_int_type, "trunc")
                        .map_err(CompileError::from)?
                        .into()
                } else if val_bits < expected_bits {
                    // Zero-extend
                    self.builder
                        .build_int_z_extend(int_val, expected_int_type, "zext")
                        .map_err(CompileError::from)?
                        .into()
                } else {
                    value
                }
            } else {
                value
            }
        } else {
            value
        };

        self.builder
            .build_store(ptr, final_value)
            .map_err(CompileError::from)?;
        Ok(final_value)
    }

    /// Type-safe load that returns a value with the correct type.
    /// Uses a unique name counter to avoid LLVM naming conflicts.
    pub fn verified_load(
        &mut self,
        ptr: PointerValue<'ctx>,
        expected_type: BasicTypeEnum<'ctx>,
        name_hint: &str,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        self.load_counter += 1;
        let name = format!("{}_{}", name_hint, self.load_counter);

        self.builder
            .build_load(expected_type, ptr, &name)
            .map_err(CompileError::from)
    }

    /// Debug helper: Print type information for troubleshooting IR generation issues
    #[allow(dead_code)]
    pub fn debug_type_info(&self, label: &str, value: BasicValueEnum<'ctx>) {
        if std::env::var("DEBUG_TYPES").is_ok() {
            eprintln!("[DEBUG_TYPES] {}: {:?}", label, value.get_type());
        }
    }

    pub fn declare_variable(
        &mut self,
        name: &str,
        _ty: AstType,
        ptr: PointerValue<'ctx>,
    ) -> Result<(), CompileError> {
        let symbol = symbols::Symbol::Variable(ptr);
        if self.symbols.exists_in_current_scope(name) {
            return Err(CompileError::UndeclaredVariable(
                name.to_string(),
                self.current_span.clone(),
            ));
        }
        self.symbols.insert(name, symbol);
        Ok(())
    }

    pub fn get_variable(&self, name: &str) -> Result<(PointerValue<'ctx>, AstType), CompileError> {
        // First check the HashMap-based variables (main storage)
        if let Some(var_info) = self.variables.get(name) {
            return Ok((var_info.pointer, var_info.ast_type.clone()));
        }

        // Then check the SymbolTable (used in trait methods and other contexts)
        if let Some(symbols::Symbol::Variable(ptr)) = self.symbols.lookup(name) {
            // We don't have type info in symbols, so use a generic type
            // This is primarily for 'self' in trait methods
            let ty = if name == "self" {
                // For 'self', we should have the struct type
                // This is a workaround - ideally we'd store the type in symbols
                AstType::Struct {
                    name: String::new(), // Will be resolved in context
                    fields: vec![],
                }
            } else {
                AstType::Void // Generic fallback
            };
            return Ok((*ptr, ty));
        }

        // Check if it's a function
        if let Some(function) = self.module.get_function(name) {
            let ptr = function.as_global_value().as_pointer_value();
            let ty = AstType::ptr(AstType::Function {
                args: vec![],
                return_type: Box::new(AstType::Void),
            });
            return Ok((ptr, ty));
        }

        Err(CompileError::UndeclaredVariable(
            name.to_string(),
            self.current_span.clone(),
        ))
    }

    pub fn compile_program(&mut self, program: &ast::Program) -> Result<(), CompileError> {
        // First pass: register all struct types (may have forward references)
        // We do this in two sub-passes:
        // 1. Register all structs with their names (so they can be looked up)
        // 2. Then resolve field types (which may reference other structs)

        // Sub-pass 1: Register all struct names first
        let struct_defs: Vec<_> = program
            .declarations
            .iter()
            .filter_map(|d| {
                if let ast::Declaration::Struct(struct_def) = d {
                    Some(struct_def)
                } else {
                    None
                }
            })
            .collect();

        // Sub-pass 2: Now register structs with resolved field types
        for struct_def in &struct_defs {
            self.register_struct_type(struct_def)?;
        }

        // Register enum types
        for declaration in &program.declarations {
            if let ast::Declaration::Enum(enum_def) = declaration {
                self.register_enum_type(enum_def)?;
            }
        }

        for declaration in &program.declarations {
            match declaration {
                ast::Declaration::ExternalFunction(ext_func) => {
                    self.declare_external_function(ext_func)?;
                }
                ast::Declaration::Function(_) => {}
                ast::Declaration::Struct(_) => {} // Already handled above
                ast::Declaration::Enum(_) => {}   // Already handled above
                ast::Declaration::Export { .. } => {
                    // Exports are handled at module level, no codegen needed
                }
                ast::Declaration::ModuleImport { alias, module_path, .. } => {
                    let module_name = module_path.split('.').last().unwrap_or(alias);
                    let module_id = self.get_module_id(module_name);
                    self.module_imports.insert(alias.clone(), module_id);
                }
                ast::Declaration::Behavior(_) => {} // Behaviors are interface definitions, no codegen needed
                ast::Declaration::Trait(_) => {} // Trait definitions are interface definitions, no direct codegen needed
                ast::Declaration::TraitImplementation(trait_impl) => {
                    self.compile_trait_implementation(trait_impl)?;
                }
                ast::Declaration::ImplBlock(impl_block) => {
                    self.compile_impl_block(impl_block)?;
                }
                ast::Declaration::TraitRequirement(_) => {
                    // Trait requirements are checked at compile time, no codegen needed
                }
                ast::Declaration::ComptimeBlock(statements) => {
                    // Evaluate comptime blocks and generate constants
                    for stmt in statements {
                        if let Err(e) = self.comptime_evaluator.execute_statement(stmt) {
                            return Err(CompileError::InternalError(
                                format!("Comptime evaluation error: {}", e),
                                None,
                            ));
                        }
                    }
                }
                ast::Declaration::TypeAlias(_) => {
                    // Type aliases are resolved at compile time, no codegen needed
                }
                ast::Declaration::Constant { name, value, .. } => {
                    // Evaluate the constant value and store it in the comptime environment
                    // This allows it to be used in subsequent code
                    if let Ok(comptime_value) = self.comptime_evaluator.evaluate_expression(value) {
                        self.comptime_evaluator
                            .set_variable(name.clone(), comptime_value);
                    }
                    // Constants are compile-time values, no runtime codegen needed
                }
            }
        }

        // Process top-level statements BEFORE function compilation
        // This ensures imported modules are available inside functions
        if !program.statements.is_empty() {
            // Create a temporary main block to process top-level statements
            let main_fn = if let Some(main) = self.module.get_function("main") {
                main
            } else {
                // Create a temporary function to process top-level statements
                let fn_type = self.context.i32_type().fn_type(&[], false);
                self.module.add_function("__temp_toplevel", fn_type, None)
            };

            let entry = self.context.append_basic_block(main_fn, "toplevel");
            let saved_block = self.builder.get_insert_block();
            self.builder.position_at_end(entry);

            for statement in &program.statements {
                self.compile_statement(statement)?;
            }

            // Restore the builder position
            if let Some(saved) = saved_block {
                self.builder.position_at_end(saved);
            }

            // Remove the temporary block if we created one
            if main_fn.get_name().to_str() == Ok("__temp_toplevel") {
                unsafe {
                    main_fn.delete();
                }
            }
        }

        // First pass: Declare all functions (skip generic functions - they're instantiated when called)
        for declaration in &program.declarations {
            if let ast::Declaration::Function(func) = declaration {
                if func.type_params.is_empty() {
                    self.declare_function(func)?;
                }
            }
        }

        // Second pass: Define and compile all functions (skip generic functions)
        for declaration in &program.declarations {
            if let ast::Declaration::Function(func) = declaration {
                if func.type_params.is_empty() {
                    self.compile_function_body(func)?;
                }
            }
        }

        Ok(())
    }

    pub fn cast_value_to_type(
        &self,
        value: BasicValueEnum<'ctx>,
        target_type: BasicTypeEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // If the types already match, no cast is needed
        if value.get_type() == target_type {
            return Ok(value);
        }

        // Handle casting between integer types
        if let (BasicValueEnum::IntValue(int_val), BasicTypeEnum::IntType(target_int_type)) =
            (value, target_type)
        {
            let source_width = int_val.get_type().get_bit_width();
            let target_width = target_int_type.get_bit_width();

            if source_width < target_width {
                // Sign extend or zero extend
                Ok(self
                    .builder
                    .build_int_s_extend(int_val, target_int_type, "cast")?
                    .into())
            } else if source_width > target_width {
                // Truncate
                Ok(self
                    .builder
                    .build_int_truncate(int_val, target_int_type, "cast")?
                    .into())
            } else {
                // Same width, just return as is
                Ok(int_val.into())
            }
        } else if let (
            BasicValueEnum::FloatValue(float_val),
            BasicTypeEnum::FloatType(target_float_type),
        ) = (value, target_type)
        {
            // Handle float casting
            let source_width = if float_val.get_type() == self.context.f32_type() {
                32
            } else {
                64
            };
            let target_width = if target_float_type == self.context.f32_type() {
                32
            } else {
                64
            };

            if source_width < target_width {
                Ok(self
                    .builder
                    .build_float_ext(float_val, target_float_type, "cast")?
                    .into())
            } else if source_width > target_width {
                Ok(self
                    .builder
                    .build_float_trunc(float_val, target_float_type, "cast")?
                    .into())
            } else {
                Ok(float_val.into())
            }
        } else {
            // For other types, return as is for now
            Ok(value)
        }
    }
}
