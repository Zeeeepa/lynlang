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
            // If error already has a span, keep it
            other => other,
        }
    }

    // ============================================================================
    // PATTERN MATCHING
    // Basic pattern matching implementation for common cases
    // ============================================================================

    /// Compile a pattern test, returning (matches: i1, bindings)
    pub fn compile_pattern_test_with_type(
        &mut self,
        scrutinee: &BasicValueEnum<'ctx>,
        pattern: &ast::Pattern,
        _scrutinee_type: Option<&AstType>,
    ) -> Result<(inkwell::values::IntValue<'ctx>, Vec<(String, BasicValueEnum<'ctx>)>), CompileError> {
        match pattern {
            // Literal patterns - compare scrutinee with the literal value
            ast::Pattern::Literal(expr) => {
                match expr {
                    ast::Expression::Boolean(b) => {
                        // Compare boolean scrutinee with literal
                        // Note: comparisons in Zen are zero-extended to i64, so we handle both i1 and i64
                        if let BasicValueEnum::IntValue(scrutinee_int) = scrutinee {
                            let bit_width = scrutinee_int.get_type().get_bit_width();
                            let literal_val = if bit_width == 1 {
                                self.context.bool_type().const_int(*b as u64, false)
                            } else {
                                // Scrutinee is i64 (zero-extended boolean comparison result)
                                scrutinee_int.get_type().const_int(*b as u64, false)
                            };
                            let matches = self.builder.build_int_compare(
                                inkwell::IntPredicate::EQ,
                                *scrutinee_int,
                                literal_val,
                                "bool_match",
                            )?;
                            Ok((matches, vec![]))
                        } else {
                            // Scrutinee is not a boolean - no match
                            Ok((self.context.bool_type().const_int(0, false), vec![]))
                        }
                    }
                    ast::Expression::Integer32(n) => {
                        if let BasicValueEnum::IntValue(scrutinee_int) = scrutinee {
                            let literal_val = self.context.i32_type().const_int(*n as u64, false);
                            let matches = self.builder.build_int_compare(
                                inkwell::IntPredicate::EQ,
                                *scrutinee_int,
                                literal_val,
                                "i32_match",
                            )?;
                            Ok((matches, vec![]))
                        } else {
                            Ok((self.context.bool_type().const_int(0, false), vec![]))
                        }
                    }
                    ast::Expression::Integer64(n) => {
                        if let BasicValueEnum::IntValue(scrutinee_int) = scrutinee {
                            let literal_val = self.context.i64_type().const_int(*n as u64, false);
                            let matches = self.builder.build_int_compare(
                                inkwell::IntPredicate::EQ,
                                *scrutinee_int,
                                literal_val,
                                "i64_match",
                            )?;
                            Ok((matches, vec![]))
                        } else {
                            Ok((self.context.bool_type().const_int(0, false), vec![]))
                        }
                    }
                    _ => Err(CompileError::UnsupportedFeature(
                        format!("Unsupported literal pattern type: {:?}", expr),
                        self.current_span.clone(),
                    ))
                }
            }
            // Wildcard pattern - always matches
            ast::Pattern::Wildcard => {
                Ok((self.context.bool_type().const_int(1, false), vec![]))
            }
            // Identifier pattern - always matches and binds the value
            ast::Pattern::Identifier(name) => {
                Ok((self.context.bool_type().const_int(1, false), vec![(name.clone(), *scrutinee)]))
            }
            // EnumLiteral pattern - match enum variant by tag and extract payload bindings
            ast::Pattern::EnumLiteral { variant, payload } => {
                // The scrutinee should be a pointer to an enum struct
                // The first field (index 0) is the tag, field 1 is the payload
                if let BasicValueEnum::PointerValue(scrutinee_ptr) = scrutinee {
                    // Find the enum type and variant tag from the symbol table
                    let mut found_tag: Option<u64> = None;
                    let mut found_struct_type: Option<inkwell::types::StructType<'ctx>> = None;

                    // Search all enums for one containing this variant
                    for symbol in self.symbols.all_symbols() {
                        if let symbols::Symbol::EnumType(info) = symbol {
                            if let Some(&tag) = info.variant_indices.get(variant) {
                                found_tag = Some(tag);
                                found_struct_type = Some(info.llvm_type);
                                break;
                            }
                        }
                    }

                    if let (Some(tag), Some(struct_type)) = (found_tag, found_struct_type) {
                        // Load the tag field (field 0) from the enum struct
                        let tag_ptr = self.builder.build_struct_gep(
                            struct_type,
                            *scrutinee_ptr,
                            0,
                            "tag_ptr",
                        )?;
                        let tag_val = self.builder.build_load(
                            self.context.i64_type(),
                            tag_ptr,
                            "tag_val",
                        )?;

                        // Compare with expected variant tag
                        let expected_tag = self.context.i64_type().const_int(tag, false);
                        let matches = self.builder.build_int_compare(
                            inkwell::IntPredicate::EQ,
                            tag_val.into_int_value(),
                            expected_tag,
                            "enum_match",
                        )?;

                        // Extract payload bindings if there's a payload pattern
                        // IMPORTANT: We only compute the GEP here (which is safe).
                        // The actual loads/dereferences happen in apply_pattern_bindings
                        // which runs in the body block AFTER the pattern match is confirmed.
                        let bindings = if let Some(payload_pattern) = payload {
                            // Get the GEP to the payload field (field 1)
                            // We return this pointer so apply_pattern_bindings can dereference it
                            let payload_ptr_ptr = self.builder.build_struct_gep(
                                struct_type,
                                *scrutinee_ptr,
                                1,
                                "payload_ptr_ptr",
                            )?;

                            // For enum payloads from a GEP, we need TWO loads:
                            // 1. Load the payload pointer from the struct field
                            // 2. Dereference that pointer to get the actual value
                            // We use prefix __enum_payload_gep__ to indicate this
                            match payload_pattern.as_ref() {
                                ast::Pattern::Identifier(name) => {
                                    vec![(format!("__enum_payload_gep__{}", name), payload_ptr_ptr.into())]
                                }
                                ast::Pattern::Wildcard => {
                                    vec![]
                                }
                                _ => {
                                    vec![]
                                }
                            }
                        } else {
                            vec![]
                        };

                        Ok((matches, bindings))
                    } else {
                        Err(CompileError::UnsupportedFeature(
                            format!("Enum variant '{}' not found in symbol table", variant),
                            self.current_span.clone(),
                        ))
                    }
                } else if let BasicValueEnum::IntValue(scrutinee_int) = scrutinee {
                    // Simple enum represented as an integer
                    // Find the variant tag from symbol table
                    let mut found_tag: Option<u64> = None;
                    for symbol in self.symbols.all_symbols() {
                        if let symbols::Symbol::EnumType(info) = symbol {
                            if let Some(&tag) = info.variant_indices.get(variant) {
                                found_tag = Some(tag);
                                break;
                            }
                        }
                    }

                    if let Some(tag) = found_tag {
                        let expected_tag = self.context.i64_type().const_int(tag, false);
                        // Extend the scrutinee to i64 if needed
                        let scrutinee_ext = if scrutinee_int.get_type().get_bit_width() < 64 {
                            self.builder.build_int_z_extend(*scrutinee_int, self.context.i64_type(), "ext")?
                        } else {
                            *scrutinee_int
                        };
                        let matches = self.builder.build_int_compare(
                            inkwell::IntPredicate::EQ,
                            scrutinee_ext,
                            expected_tag,
                            "enum_match",
                        )?;
                        Ok((matches, vec![]))
                    } else {
                        Err(CompileError::UnsupportedFeature(
                            format!("Enum variant '{}' not found in symbol table", variant),
                            self.current_span.clone(),
                        ))
                    }
                } else if let BasicValueEnum::StructValue(scrutinee_struct) = scrutinee {
                    // Enum loaded as a struct value - extract the tag field (field 0)
                    // Find the variant tag from symbol table
                    let mut found_tag: Option<u64> = None;
                    for symbol in self.symbols.all_symbols() {
                        if let symbols::Symbol::EnumType(info) = symbol {
                            if let Some(&tag) = info.variant_indices.get(variant) {
                                found_tag = Some(tag);
                                break;
                            }
                        }
                    }

                    if let Some(tag) = found_tag {
                        // Extract the tag field from the struct value
                        let tag_val = self.builder.build_extract_value(
                            *scrutinee_struct,
                            0,
                            "tag_val",
                        )?;

                        let expected_tag = self.context.i64_type().const_int(tag, false);
                        let matches = self.builder.build_int_compare(
                            inkwell::IntPredicate::EQ,
                            tag_val.into_int_value(),
                            expected_tag,
                            "enum_match",
                        )?;

                        // Extract payload bindings if there's a payload pattern
                        // For StructValue, we extract the pointer field but defer dereferencing
                        let bindings = if let Some(payload_pattern) = payload {
                            // Extract the payload pointer (field 1) from the struct value
                            // This is safe - just extracting a register value, not dereferencing
                            let payload_ptr = self.builder.build_extract_value(
                                *scrutinee_struct,
                                1,
                                "payload_ptr",
                            )?;

                            // For enum payloads from extract_value, we have the pointer directly
                            // We only need ONE load to dereference it
                            // We use prefix __enum_payload_ptr__ to indicate this
                            match payload_pattern.as_ref() {
                                ast::Pattern::Identifier(name) => {
                                    vec![(format!("__enum_payload_ptr__{}", name), payload_ptr)]
                                }
                                ast::Pattern::Wildcard => {
                                    vec![]
                                }
                                _ => {
                                    vec![]
                                }
                            }
                        } else {
                            vec![]
                        };

                        Ok((matches, bindings))
                    } else {
                        Err(CompileError::UnsupportedFeature(
                            format!("Enum variant '{}' not found in symbol table", variant),
                            self.current_span.clone(),
                        ))
                    }
                } else {
                    Err(CompileError::UnsupportedFeature(
                        format!("Unsupported scrutinee type for enum pattern: {:?}", scrutinee),
                        self.current_span.clone(),
                    ))
                }
            }
            _ => Err(CompileError::UnsupportedFeature(
                format!("Pattern type not yet implemented: {:?}", pattern),
                self.current_span.clone(),
            ))
        }
    }

    /// Apply pattern bindings to the current scope
    pub fn apply_pattern_bindings(&mut self, bindings: &[(String, BasicValueEnum<'ctx>)]) {
        for (name, value) in bindings {
            // Check for GEP-based enum payload (needs two loads)
            if let Some(var_name) = name.strip_prefix("__enum_payload_gep__") {
                if value.is_pointer_value() {
                    let gep_ptr = value.into_pointer_value();

                    // First load: get the payload pointer from the struct field
                    let payload_ptr = if let Ok(loaded) = self.builder.build_load(
                        self.context.ptr_type(inkwell::AddressSpace::default()),
                        gep_ptr,
                        "payload_ptr",
                    ) {
                        loaded.into_pointer_value()
                    } else {
                        continue;
                    };

                    // Determine the payload type from tracked generic types
                    let payload_ast_type = self.generic_type_context.get("Option_Some_Type")
                        .or_else(|| self.generic_type_context.get("Result_Ok_Type"))
                        .or_else(|| self.generic_type_context.get("Result_Err_Type"))
                        .cloned();

                    let payload_llvm_type = payload_ast_type.as_ref()
                        .and_then(|ast_type| self.to_llvm_type(ast_type).ok())
                        .and_then(|t| self.expect_basic_type(t).ok());

                    // Second load: dereference the payload pointer to get the actual value
                    let payload_val = if let Some(llvm_type) = payload_llvm_type {
                        if let Ok(val) = self.builder.build_load(llvm_type, payload_ptr, "payload_val") {
                            val
                        } else {
                            continue;
                        }
                    } else {
                        if let Ok(val) = self.builder.build_load(self.context.i64_type(), payload_ptr, "payload_val") {
                            val
                        } else {
                            continue;
                        }
                    };

                    let ast_type = payload_ast_type.unwrap_or(AstType::I64);
                    let val_type = payload_val.get_type();
                    if let Ok(alloca) = self.builder.build_alloca(val_type, var_name) {
                        let _ = self.builder.build_store(alloca, payload_val);
                        self.variables.insert(var_name.to_string(), VariableInfo {
                            pointer: alloca,
                            ast_type,
                            is_mutable: false,
                            is_initialized: true,
                        });
                    }
                }
                continue;
            }

            // Check for pointer-based enum payload (needs one load)
            if let Some(var_name) = name.strip_prefix("__enum_payload_ptr__") {
                if value.is_pointer_value() {
                    let payload_ptr = value.into_pointer_value();

                    // Determine the payload type from tracked generic types
                    let payload_ast_type = self.generic_type_context.get("Option_Some_Type")
                        .or_else(|| self.generic_type_context.get("Result_Ok_Type"))
                        .or_else(|| self.generic_type_context.get("Result_Err_Type"))
                        .cloned();

                    let payload_llvm_type = payload_ast_type.as_ref()
                        .and_then(|ast_type| self.to_llvm_type(ast_type).ok())
                        .and_then(|t| self.expect_basic_type(t).ok());

                    // Single load: dereference the payload pointer to get the actual value
                    let payload_val = if let Some(llvm_type) = payload_llvm_type {
                        if let Ok(val) = self.builder.build_load(llvm_type, payload_ptr, "payload_val") {
                            val
                        } else {
                            continue;
                        }
                    } else {
                        if let Ok(val) = self.builder.build_load(self.context.i64_type(), payload_ptr, "payload_val") {
                            val
                        } else {
                            continue;
                        }
                    };

                    let ast_type = payload_ast_type.unwrap_or(AstType::I64);
                    let val_type = payload_val.get_type();
                    if let Ok(alloca) = self.builder.build_alloca(val_type, var_name) {
                        let _ = self.builder.build_store(alloca, payload_val);
                        self.variables.insert(var_name.to_string(), VariableInfo {
                            pointer: alloca,
                            ast_type,
                            is_mutable: false,
                            is_initialized: true,
                        });
                    }
                }
                continue;
            }

            // Regular binding - store the value directly
            let llvm_type = value.get_type();
            if let Ok(alloca) = self.builder.build_alloca(llvm_type, name) {
                let _ = self.builder.build_store(alloca, *value);
                // Infer AST type from the LLVM type
                let ast_type = match value {
                    BasicValueEnum::IntValue(iv) => {
                        let bit_width = iv.get_type().get_bit_width();
                        match bit_width {
                            1 => AstType::Bool,
                            32 => AstType::I32,
                            64 => AstType::I64,
                            _ => AstType::I32,
                        }
                    }
                    BasicValueEnum::FloatValue(fv) => {
                        let ty = fv.get_type();
                        if ty == self.context.f32_type() {
                            AstType::F32
                        } else {
                            AstType::F64
                        }
                    }
                    BasicValueEnum::PointerValue(_) => AstType::raw_ptr(AstType::Void),
                    _ => AstType::I32, // Fallback
                };
                self.variables.insert(name.clone(), VariableInfo {
                    pointer: alloca,
                    ast_type,
                    is_mutable: false,
                    is_initialized: true,
                });
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
                ast::Declaration::ModuleImport { alias, module_path } => {
                    // Handle module imports like { io } = @std or { Option, Some, None } = @std
                    // We just register these as compile-time symbols
                    // The actual variables will be created when needed in functions

                    // Extract the module name from the path (e.g., "@std.io" -> "io")
                    let module_name = if let Some(last_part) = module_path.split('.').last() {
                        last_part
                    } else {
                        alias
                    };

                    // Handle specific std types and modules
                    if self.well_known.is_option(module_name) || self.well_known.is_option_variant(module_name) {
                        self.module_imports.insert(alias.clone(), 100);
                    } else if self.well_known.is_result(module_name) || self.well_known.is_result_variant(module_name) {
                        self.module_imports.insert(alias.clone(), 101);
                    } else {
                        match module_name {
                            // Collections
                            "HashMap" | "HashSet" | "DynVec" | "Array" | "Vec" => {
                                self.module_imports.insert(alias.clone(), 102);
                            }
                            // Allocator types
                            "Allocator" | "get_default_allocator" => {
                                self.module_imports.insert(alias.clone(), 103);
                            }
                            // Math functions
                            "min" | "max" | "abs" | "sqrt" | "pow" | "sin" | "cos" | "tan" => {
                                self.module_imports.insert(alias.clone(), 104);
                            }
                            // Regular modules
                            "io" => {
                                self.module_imports.insert(alias.clone(), 1);
                            }
                            "math" => {
                                self.module_imports.insert(alias.clone(), 2);
                            }
                            "core" => {
                                self.module_imports.insert(alias.clone(), 3);
                            }
                            "GPA" => {
                                self.module_imports.insert(alias.clone(), 4);
                            }
                            "AsyncPool" => {
                                self.module_imports.insert(alias.clone(), 5);
                            }
                            "build" => {
                                self.module_imports.insert(alias.clone(), 7);
                            }
                            _ => {
                                self.module_imports.insert(alias.clone(), 0);
                            }
                        }
                    }
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

        // First pass: Declare all functions
        for declaration in &program.declarations {
            if let ast::Declaration::Function(func) = declaration {
                self.declare_function(func)?;
            }
        }

        // Second pass: Define and compile all functions
        for declaration in &program.declarations {
            if let ast::Declaration::Function(func) = declaration {
                self.compile_function_body(func)?;
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
