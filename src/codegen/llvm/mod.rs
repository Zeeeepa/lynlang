#![allow(dead_code)]

use crate::ast::{self, AstType};
use crate::comptime;
use crate::error::CompileError;
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
mod control_flow;
mod expressions;
mod functions;
mod generics;
mod literals;
mod patterns;
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
    pub function_types: HashMap<String, AstType>, // Track function return types
    pub current_function: Option<FunctionValue<'ctx>>,
    pub symbols: symbols::SymbolTable<'ctx>,
    pub struct_types: HashMap<String, StructTypeInfo<'ctx>>,
    pub loop_stack: Vec<(BasicBlock<'ctx>, BasicBlock<'ctx>)>, // (continue_target, break_target)
    pub defer_stack: Vec<ast::Expression>, // Stack of deferred expressions (LIFO order)
    pub comptime_evaluator: comptime::ComptimeInterpreter,
    pub behavior_codegen: Option<behaviors::BehaviorCodegen<'ctx>>,
    pub current_impl_type: Option<String>, // Track implementing type for trait methods
    pub inline_counter: usize,             // Counter for unique inline function names
    pub load_counter: usize,                // Counter for unique load instruction names
    pub generic_type_context: HashMap<String, AstType>, // Track instantiated generic types (legacy, kept for compatibility)
    pub generic_tracker: generics::GenericTypeTracker, // New improved generic type tracking
    pub module_imports: HashMap<String, u64>, // Track module imports (name -> marker value)
}

impl<'ctx> LLVMCompiler<'ctx> {
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
                if name == "Result" && type_args.len() == 2 {
                    self.generic_type_context.insert(format!("{}_Ok_Type", prefix), type_args[0].clone());
                    self.generic_type_context.insert(format!("{}_Err_Type", prefix), type_args[1].clone());
                } else if name == "Option" && type_args.len() == 1 {
                    self.generic_type_context.insert(format!("{}_Some_Type", prefix), type_args[0].clone());
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
        };

        // Declare standard library functions
        compiler.declare_stdlib_functions();

        // Register built-in Option and Result enums
        compiler.register_builtin_enums();

        compiler
    }

    fn register_builtin_enums(&mut self) {
        // Register Array<T> as a built-in type (not an enum)
        // Array has methods like new() and is a dynamic array type
        let array_struct_type = self.context.struct_type(
            &[
                self.context.ptr_type(inkwell::AddressSpace::default()).into(), // data pointer
                self.context.i64_type().into(), // length
                self.context.i64_type().into(), // capacity
            ],
            false,
        );
        
        // Register Array as a special built-in type
        // We'll use a struct info for now to make it available
        let array_info = StructTypeInfo {
            llvm_type: array_struct_type,
            fields: {
                let mut fields = HashMap::new();
                fields.insert("data".to_string(), (0, AstType::Ptr(Box::new(AstType::Void))));
                fields.insert("length".to_string(), (1, AstType::I64));
                fields.insert("capacity".to_string(), (2, AstType::I64));
                fields
            },
        };
        self.struct_types.insert("Array".to_string(), array_info);
        
        // Also register Array in symbols table so it can be used like a type
        self.symbols.insert("Array", symbols::Symbol::StructType(array_struct_type));
        
        // Register Option<T> and Result<T, E> as fallback until stdlib preloading is implemented
        // TODO: These should be loaded from stdlib/core/option.zen and stdlib/core/result.zen
        // For now, keep as fallback to ensure Option/Result work even without explicit imports
        // When stdlib/core/option.zen is loaded, it will override this registration via register_enum_type()
        
        // Register Option<T> enum
        // Option has Some(T) with payload and None without, so we need space for payload
        // Use pointer type for payload to handle any type (including strings and structs)
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let enum_struct_type = self.context.struct_type(
            &[
                self.context.i64_type().into(), // discriminant
                ptr_type.into(),                // payload (generic pointer that can hold any type)
            ],
            false,
        );

        let mut variant_indices = HashMap::new();
        variant_indices.insert("Some".to_string(), 0);
        variant_indices.insert("None".to_string(), 1);

        let option_info = symbols::EnumInfo {
            llvm_type: enum_struct_type,
            variant_indices: variant_indices.clone(),
            variants: vec![
                ast::EnumVariant {
                    name: "Some".to_string(),
                    payload: Some(AstType::Generic {
                        name: "T".to_string(),
                        type_args: vec![],
                    }),
                },
                ast::EnumVariant {
                    name: "None".to_string(),
                    payload: None,
                },
            ],
        };
        self.symbols
            .insert("Option", symbols::Symbol::EnumType(option_info));

        // Register Result<T, E> enum
        // Result always has payloads (Ok(T) or Err(E))
        let result_struct_type = self.context.struct_type(
            &[
                self.context.i64_type().into(), // discriminant
                ptr_type.into(),                // payload (generic pointer that can hold any type)
            ],
            false,
        );

        let mut result_variant_indices = HashMap::new();
        result_variant_indices.insert("Ok".to_string(), 0);
        result_variant_indices.insert("Err".to_string(), 1);

        let result_info = symbols::EnumInfo {
            llvm_type: result_struct_type,
            variant_indices: result_variant_indices,
            variants: vec![
                ast::EnumVariant {
                    name: "Ok".to_string(),
                    payload: Some(AstType::Generic {
                        name: "T".to_string(),
                        type_args: vec![],
                    }),
                },
                ast::EnumVariant {
                    name: "Err".to_string(),
                    payload: Some(AstType::Generic {
                        name: "E".to_string(),
                        type_args: vec![],
                    }),
                },
            ],
        };
        self.symbols
            .insert("Result", symbols::Symbol::EnumType(result_info));
    }

    fn declare_stdlib_functions(&mut self) {
        // Declare malloc: i8* @malloc(i64)
        if self.module.get_function("malloc").is_none() {
            let i64_type = self.context.i64_type();
            let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
            let malloc_type = ptr_type.fn_type(&[i64_type.into()], false);
            self.module.add_function("malloc", malloc_type, None);
        }

        // Declare free: void @free(i8*)
        if self.module.get_function("free").is_none() {
            let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
            let void_type = self.context.void_type();
            let free_type = void_type.fn_type(&[ptr_type.into()], false);
            self.module.add_function("free", free_type, None);
        }

        // Declare memcpy: void @memcpy(i8*, i8*, i64)
        if self.module.get_function("memcpy").is_none() {
            let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
            let i64_type = self.context.i64_type();
            let void_type = self.context.void_type();
            let memcpy_type =
                void_type.fn_type(&[ptr_type.into(), ptr_type.into(), i64_type.into()], false);
            self.module.add_function("memcpy", memcpy_type, None);
        }
        
        // Declare get_default_allocator: i8* @get_default_allocator()
        // Returns a pointer to the default system allocator
        if self.module.get_function("get_default_allocator").is_none() {
            let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
            let get_alloc_type = ptr_type.fn_type(&[], false);
            let func = self.module.add_function("get_default_allocator", get_alloc_type, None);
            
            // Implementation: return a simple marker that malloc/free will recognize
            let entry = self.context.append_basic_block(func, "entry");
            let current_block = self.builder.get_insert_block();
            self.builder.position_at_end(entry);
            
            // Return a non-null pointer (value 1) as a marker for default allocator
            // Create an integer with value 1 and cast to pointer
            let i64_type = self.context.i64_type();
            let marker_int = i64_type.const_int(1, false);
            let marker_ptr = self.builder.build_int_to_ptr(marker_int, ptr_type, "allocator_marker")
                .unwrap_or_else(|_| ptr_type.const_null());
            let _ = self.builder.build_return(Some(&marker_ptr));
            
            // Restore builder position
            if let Some(block) = current_block {
                self.builder.position_at_end(block);
            }
            
            // Register the function so it can be called from Zen code
            self.functions.insert("get_default_allocator".to_string(), func);
            // Register the return type
            self.function_types.insert(
                "get_default_allocator".to_string(),
                AstType::Ptr(Box::new(AstType::Void)), // Allocator is an opaque pointer
            );
        }
    }

    pub fn get_type(&self, name: &str) -> Result<BasicTypeEnum<'ctx>, CompileError> {
        self.symbols
            .lookup(name)
            .and_then(|sym| match sym {
                symbols::Symbol::Type(ty) => Some(*ty),
                _ => None,
            })
            .ok_or_else(|| CompileError::UndeclaredVariable(name.to_string(), None))
    }

    pub fn declare_variable(
        &mut self,
        name: &str,
        _ty: AstType,
        ptr: PointerValue<'ctx>,
    ) -> Result<(), CompileError> {
        let symbol = symbols::Symbol::Variable(ptr);
        if self.symbols.exists_in_current_scope(name) {
            return Err(CompileError::UndeclaredVariable(name.to_string(), None));
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
        if let Some(symbol) = self.symbols.lookup(name) {
            if let symbols::Symbol::Variable(ptr) = symbol {
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
        }

        // Check if it's a function
        if let Some(function) = self.module.get_function(name) {
            let ptr = function.as_global_value().as_pointer_value();
            let ty = AstType::Ptr(Box::new(AstType::Function {
                args: vec![],
                return_type: Box::new(AstType::Void),
            }));
            return Ok((ptr, ty));
        }

        Err(CompileError::UndeclaredVariable(name.to_string(), None))
    }

    pub fn compile_program(&mut self, program: &ast::Program) -> Result<(), CompileError> {
        // First pass: register all struct types (may have forward references)
        // We do this in two sub-passes:
        // 1. Register all structs with their names (so they can be looked up)
        // 2. Then resolve field types (which may reference other structs)
        
        // Sub-pass 1: Register all struct names first
        let struct_defs: Vec<_> = program.declarations
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
                    match module_name {
                        // Built-in types from @std
                        "Option" | "Some" | "None" => {
                            // Option types are already registered in register_builtin_types
                            // Just mark that these names are available as imports
                            self.module_imports.insert(alias.clone(), 100); // Special marker for Option types
                        }
                        "Result" | "Ok" | "Err" => {
                            // Result types are already registered in register_builtin_types
                            // Just mark that these names are available as imports
                            self.module_imports.insert(alias.clone(), 101); // Special marker for Result types
                        }
                        // Collections
                        "HashMap" | "HashSet" | "DynVec" | "Array" | "Vec" => {
                            self.module_imports.insert(alias.clone(), 102); // Special marker for collections
                        }
                        // Allocator types
                        "Allocator" | "get_default_allocator" => {
                            self.module_imports.insert(alias.clone(), 103); // Special marker for allocator
                        }
                        // Math functions
                        "min" | "max" | "abs" | "sqrt" | "pow" | "sin" | "cos" | "tan" => {
                            self.module_imports.insert(alias.clone(), 104); // Special marker for math functions
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
                            // Unknown import, store with marker 0
                            self.module_imports.insert(alias.clone(), 0);
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

    pub fn register_struct_type(
        &mut self,
        struct_def: &ast::StructDefinition,
    ) -> Result<(), CompileError> {
        // Convert field types to LLVM types
        let mut field_types = Vec::new();
        let mut fields = HashMap::new();

        for (index, field) in struct_def.fields.iter().enumerate() {
            let llvm_type = match &field.type_ {
                AstType::I8 => self.context.i8_type().as_basic_type_enum(),
                AstType::I16 => self.context.i16_type().as_basic_type_enum(),
                AstType::I32 => self.context.i32_type().as_basic_type_enum(),
                AstType::I64 => self.context.i64_type().as_basic_type_enum(),
                AstType::U8 => self.context.i8_type().as_basic_type_enum(),
                AstType::U16 => self.context.i16_type().as_basic_type_enum(),
                AstType::U32 => self.context.i32_type().as_basic_type_enum(),
                AstType::U64 => self.context.i64_type().as_basic_type_enum(),
                AstType::F32 => self.context.f32_type().as_basic_type_enum(),
                AstType::F64 => self.context.f64_type().as_basic_type_enum(),
                AstType::Bool => self.context.bool_type().as_basic_type_enum(),
                AstType::StaticLiteral | AstType::StaticString => self
                    .context
                    .ptr_type(inkwell::AddressSpace::default())
                    .as_basic_type_enum(),
                AstType::Struct { name, .. } if name == "String" => self
                    .context
                    .ptr_type(inkwell::AddressSpace::default())
                    .as_basic_type_enum(),
                AstType::Void => {
                    return Err(CompileError::TypeError(
                        "Void type not allowed in struct fields".to_string(),
                        None,
                    ))
                }
                AstType::Ptr(inner) | AstType::MutPtr(inner) | AstType::RawPtr(inner) => {
                    // For pointer types in struct fields, check if inner type is a registered struct
                    // This handles self-referential structs like Node { child: Ptr<Node> }
                    match inner.as_ref() {
                        AstType::Generic { name, .. } => {
                            // Check if this generic is actually a registered struct type
                            if let Some(struct_info) = self.struct_types.get(name) {
                                // Use pointer to the struct type
                                struct_info.llvm_type.ptr_type(inkwell::AddressSpace::default()).as_basic_type_enum()
                            } else {
                                // Generic type not registered yet (forward reference or self-reference)
                                // Use generic pointer type
                                self.context
                                    .ptr_type(inkwell::AddressSpace::default())
                                    .as_basic_type_enum()
                            }
                        }
                        AstType::Struct { name, .. } => {
                            // Look up the struct type and create a pointer to it
                            if let Some(struct_info) = self.struct_types.get(name) {
                                struct_info.llvm_type.ptr_type(inkwell::AddressSpace::default()).as_basic_type_enum()
                            } else {
                                // Struct not registered yet (forward reference)
                                self.context
                                    .ptr_type(inkwell::AddressSpace::default())
                                    .as_basic_type_enum()
                            }
                        }
                        _ => {
                            // For other pointer types, use generic pointer
                            self.context
                                .ptr_type(inkwell::AddressSpace::default())
                                .as_basic_type_enum()
                        }
                    }
                }
                AstType::Generic { name, .. } => {
                    // Check if this "generic" is actually a registered struct type
                    // (the parser sometimes represents struct types as generics)
                    if let Some(struct_info) = self.struct_types.get(name) {
                        struct_info.llvm_type.as_basic_type_enum()
                    } else {
                        // For real generic types, treat as pointers
                        // In a full implementation, we'd need generic instantiation
                        self.context
                            .ptr_type(inkwell::AddressSpace::default())
                            .as_basic_type_enum()
                    }
                }
                AstType::Struct { name, .. } => {
                    // Look up the previously registered struct type
                    if let Some(struct_info) = self.struct_types.get(name) {
                        // Use the struct type directly as a basic type enum
                        // This allows structs to be used as inline fields in other structs
                        struct_info.llvm_type.as_basic_type_enum()
                    } else {
                        // Forward reference - struct not yet registered
                        // This can happen when structs reference each other
                        // Return an error - forward references should be resolved by typechecker
                        return Err(CompileError::TypeError(
                            format!("Struct '{}' not yet registered. This may be a forward reference issue. Structs should be defined before use, or the typechecker should resolve Generic types to Struct types.", name),
                            None
                        ));
                    }
                }
                AstType::FunctionPointer { .. } => {
                    // Function pointers in struct fields are represented as generic pointers
                    self.context
                        .ptr_type(inkwell::AddressSpace::default())
                        .as_basic_type_enum()
                }
                _ => {
                    return Err(CompileError::TypeError(
                        format!("Unsupported type in struct: {:?}", field.type_),
                        None,
                    ))
                }
            };

            field_types.push(llvm_type);
            fields.insert(field.name.clone(), (index, field.type_.clone()));
        }

        // Create the LLVM struct type
        let struct_type = self.context.struct_type(&field_types, false);

        // Register the struct type
        let struct_info = StructTypeInfo {
            llvm_type: struct_type,
            fields,
        };

        self.struct_types
            .insert(struct_def.name.clone(), struct_info);

        Ok(())
    }

    pub fn register_enum_type(
        &mut self,
        enum_def: &ast::EnumDefinition,
    ) -> Result<(), CompileError> {
        // Create variant index mapping
        let mut variant_indices = HashMap::new();
        let mut max_payload_size = 0u32;
        let mut has_payloads = false;

        // Find the largest payload type to create a union-like structure
        for (index, variant) in enum_def.variants.iter().enumerate() {
            variant_indices.insert(variant.name.clone(), index as u64);

            if let Some(payload_type) = &variant.payload {
                // Skip void payloads - they don't need storage
                if !matches!(payload_type, AstType::Void) {
                    has_payloads = true;
                    // Calculate the size needed for this payload type
                    let payload_size = match payload_type {
                        AstType::I8 | AstType::U8 | AstType::Bool => 8,
                        AstType::I16 | AstType::U16 => 16,
                        AstType::I32 | AstType::U32 | AstType::F32 => 32,
                        AstType::I64 | AstType::U64 | AstType::F64 | AstType::Usize => 64,
                        AstType::StaticLiteral
                        | AstType::StaticString => 64, // pointer size
                        AstType::Struct { name, .. } if name == "String" => 64, // pointer size
                        AstType::Ptr(_)
                        | AstType::MutPtr(_)
                        | AstType::RawPtr(_) => 64, // pointer size
                        AstType::Struct { .. } | AstType::Generic { .. } => 64, // for now, use pointer size
                        AstType::Void => 0,                                     // void has no size
                        _ => 64, // default to 64 bits
                    };
                    max_payload_size = max_payload_size.max(payload_size);
                }
            }
        }

        // Create enum struct type based on actual payload needs
        let enum_struct_type = if has_payloads {
            // Use a generic pointer type for payloads to handle any type uniformly
            // This allows us to store strings, structs, and other complex types
            let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());

            self.context.struct_type(
                &[
                    self.context.i64_type().into(), // tag (discriminant)
                    ptr_type.into(),                // payload (generic pointer for any type)
                ],
                false,
            )
        } else {
            // For enums with no payloads (like unit enums), just use the tag
            self.context.struct_type(
                &[
                    self.context.i64_type().into(), // tag only
                ],
                false,
            )
        };

        // Create enum info with proper type information preserved
        let enum_info = symbols::EnumInfo {
            llvm_type: enum_struct_type,
            variant_indices,
            variants: enum_def.variants.clone(),
        };

        // Register in symbol table
        self.symbols
            .insert(&enum_def.name, symbols::Symbol::EnumType(enum_info));

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
    
    /// Parse comma-separated types from a string, handling nested generics
    pub fn parse_comma_separated_types(&self, type_str: &str) -> Vec<AstType> {
        let mut result = Vec::new();
        let mut current = String::new();
        let mut depth = 0;
        
        for ch in type_str.chars() {
            match ch {
                '<' => {
                    depth += 1;
                    current.push(ch);
                }
                '>' => {
                    depth -= 1;
                    current.push(ch);
                }
                ',' if depth == 0 => {
                    // End of current type
                    let parsed = self.parse_type_string(current.trim());
                    result.push(parsed);
                    current.clear();
                }
                _ => {
                    current.push(ch);
                }
            }
        }
        
        // Don't forget the last type
        if !current.is_empty() {
            let parsed = self.parse_type_string(current.trim());
            result.push(parsed);
        }
        
        result
    }
    
    /// Parse a single type string into an AstType
    pub fn parse_type_string(&self, type_str: &str) -> AstType {
        // Check for generic types
        if let Some(angle_pos) = type_str.find('<') {
            let base_type = &type_str[..angle_pos];
            let type_params_str = &type_str[angle_pos+1..type_str.len()-1];
            
            match base_type {
                "DynVec" => {
                    let element_types = self.parse_comma_separated_types(type_params_str);
                    AstType::DynVec {
                        element_types,
                        allocator_type: None,
                    }
                }
                "Vec" => {
                    // Vec<T, N> where N is the size
                    let parts = self.parse_comma_separated_types(type_params_str);
                    if parts.len() >= 1 {
                        // For now, default size to 10 if not specified
                        AstType::Vec {
                            element_type: Box::new(parts[0].clone()),
                            size: 10, // Default size
                        }
                    } else {
                        AstType::Vec {
                            element_type: Box::new(AstType::I32),
                            size: 10,
                        }
                    }
                }
                "Option" => {
                    let type_args = self.parse_comma_separated_types(type_params_str);
                    AstType::Generic {
                        name: "Option".to_string(),
                        type_args,
                    }
                }
                "Result" => {
                    let type_args = self.parse_comma_separated_types(type_params_str);
                    AstType::Generic {
                        name: "Result".to_string(),
                        type_args,
                    }
                }
                "HashMap" | "HashSet" => {
                    let type_args = self.parse_comma_separated_types(type_params_str);
                    AstType::Generic {
                        name: base_type.to_string(),
                        type_args,
                    }
                }
                _ => {
                    // Unknown generic type
                    let type_args = self.parse_comma_separated_types(type_params_str);
                    AstType::Generic {
                        name: base_type.to_string(),
                        type_args,
                    }
                }
            }
        } else {
            // Simple types
            match type_str {
                "i8" => AstType::I8,
                "i16" => AstType::I16,
                "i32" => AstType::I32,
                "i64" => AstType::I64,
                "u8" => AstType::U8,
                "u16" => AstType::U16,
                "u32" => AstType::U32,
                "u64" => AstType::U64,
                "f32" => AstType::F32,
                "f64" => AstType::F64,
                "bool" => AstType::Bool,
                "string" => AstType::StaticLiteral,
                "StaticString" => AstType::StaticString,
                "String" => crate::ast::resolve_string_struct_type(),
                "void" => AstType::Void,
                _ => AstType::I32, // Default fallback
            }
        }
    }
}
