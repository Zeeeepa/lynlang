use crate::ast::{self, AstType};
use crate::comptime;
use crate::error::CompileError;
use inkwell::{
    basic_block::BasicBlock,
    builder::Builder,
    context::Context,
    module::Module,
    types::{BasicType, BasicTypeEnum, FunctionType, StructType},
    values::{FunctionValue, PointerValue, BasicValueEnum},
};
use std::collections::HashMap;

mod behaviors;
mod binary_ops;
mod control_flow;
mod expressions;
mod functions;
mod literals;
mod patterns;
mod pointers;
mod statements;
mod strings;
mod structs;
mod symbols;
mod types;

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

pub struct LLVMCompiler<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub variables: HashMap<String, (PointerValue<'ctx>, AstType)>,
    pub functions: HashMap<String, FunctionValue<'ctx>>,
    pub function_types: HashMap<String, AstType>,  // Track function return types
    pub current_function: Option<FunctionValue<'ctx>>,
    pub symbols: symbols::SymbolTable<'ctx>,
    pub struct_types: HashMap<String, StructTypeInfo<'ctx>>,
    pub loop_stack: Vec<(BasicBlock<'ctx>, BasicBlock<'ctx>)>, // (continue_target, break_target)
    pub comptime_evaluator: comptime::ComptimeInterpreter,
    pub behavior_codegen: Option<behaviors::BehaviorCodegen<'ctx>>,
}

impl<'ctx> LLVMCompiler<'ctx> {
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
        symbols.insert("f64", symbols::Symbol::Type(float_type.as_basic_type_enum()));
        symbols.insert("bool", symbols::Symbol::Type(bool_type.as_basic_type_enum()));
        
        Self {
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
            comptime_evaluator,
            behavior_codegen: Some(behaviors::BehaviorCodegen::new()),
        }
    }

    pub fn get_type(&self, name: &str) -> Result<BasicTypeEnum<'ctx>, CompileError> {
        self.symbols.lookup(name)
            .and_then(|sym| match sym {
                symbols::Symbol::Type(ty) => Some(*ty),
                _ => None,
            })
            .ok_or_else(|| CompileError::UndeclaredVariable(name.to_string(), None))
    }

    pub fn declare_variable(&mut self, name: &str, _ty: AstType, ptr: PointerValue<'ctx>) -> Result<(), CompileError> {
        let symbol = symbols::Symbol::Variable(ptr);
        if self.symbols.exists_in_current_scope(name) {
            return Err(CompileError::UndeclaredVariable(name.to_string(), None));
        }
        self.symbols.insert(name, symbol);
        Ok(())
    }

    pub fn get_variable(&self, name: &str) -> Result<(PointerValue<'ctx>, AstType), CompileError> {
        if let Some(entry) = self.variables.get(name) {
            return Ok(entry.clone());
        }
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
        
        // First pass: register struct types
        for declaration in &program.declarations {
            if let ast::Declaration::Struct(struct_def) = declaration {
                self.register_struct_type(struct_def)?;
            }
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
                ast::Declaration::Enum(_) => {} // Already handled above
                ast::Declaration::ModuleImport { .. } => {}
                ast::Declaration::Behavior(_) => {} // Behaviors are interface definitions, no codegen needed
                ast::Declaration::TraitImplementation(trait_impl) => {
                    self.compile_trait_implementation(trait_impl)?;
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
                                None
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
                        self.comptime_evaluator.set_variable(name.clone(), comptime_value);
                    }
                    // Constants are compile-time values, no runtime codegen needed
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

    pub fn register_struct_type(&mut self, struct_def: &ast::StructDefinition) -> Result<(), CompileError> {
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
                AstType::String => self.context.ptr_type(inkwell::AddressSpace::default()).as_basic_type_enum(),
                AstType::Void => return Err(CompileError::TypeError("Void type not allowed in struct fields".to_string(), None)),
                AstType::Ptr(_inner) => {
                    // For pointer types in struct fields, we'll use a generic pointer type
                    // This is a simplification - in a full implementation we'd need to handle nested types
                    self.context.ptr_type(inkwell::AddressSpace::default()).as_basic_type_enum()
                },
                AstType::Struct { name, .. } => {
                    // Look up the previously registered struct type
                    if let Some(struct_info) = self.struct_types.get(name) {
                        struct_info.llvm_type.as_basic_type_enum()
                    } else {
                        return Err(CompileError::TypeError(
                            format!("Struct '{}' not yet defined. Structs must be defined before use in fields", name),
                            None
                        ))
                    }
                },
                _ => return Err(CompileError::TypeError(format!("Unsupported type in struct: {:?}", field.type_), None)),
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
        
        self.struct_types.insert(struct_def.name.clone(), struct_info);
        
        Ok(())
    }
    
    pub fn register_enum_type(&mut self, enum_def: &ast::EnumDefinition) -> Result<(), CompileError> {
        // Create variant index mapping
        let mut variant_indices = HashMap::new();
        for (index, variant) in enum_def.variants.iter().enumerate() {
            variant_indices.insert(variant.name.clone(), index as u64);
        }
        
        // For now, represent enums as a struct { tag: i64, payload: i64 }
        // In the future, this can be optimized based on the actual payload types
        let enum_struct_type = self.context.struct_type(&[
            self.context.i64_type().into(),  // tag
            self.context.i64_type().into(),  // payload (simplified for now)
        ], false);
        
        // Create enum info
        let enum_info = symbols::EnumInfo {
            llvm_type: enum_struct_type,
            variant_indices,
            variants: enum_def.variants.clone(),
        };
        
        // Register in symbol table
        self.symbols.insert(&enum_def.name, symbols::Symbol::EnumType(enum_info));
        
        Ok(())
    }
    
    pub fn cast_value_to_type(&self, value: BasicValueEnum<'ctx>, target_type: BasicTypeEnum<'ctx>) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // If the types already match, no cast is needed
        if value.get_type() == target_type {
            return Ok(value);
        }
        
        // Handle casting between integer types
        if let (BasicValueEnum::IntValue(int_val), BasicTypeEnum::IntType(target_int_type)) = (value, target_type) {
            let source_width = int_val.get_type().get_bit_width();
            let target_width = target_int_type.get_bit_width();
            
            if source_width < target_width {
                // Sign extend or zero extend
                Ok(self.builder.build_int_s_extend(int_val, target_int_type, "cast")?.into())
            } else if source_width > target_width {
                // Truncate
                Ok(self.builder.build_int_truncate(int_val, target_int_type, "cast")?.into())
            } else {
                // Same width, just return as is
                Ok(int_val.into())
            }
        } else if let (BasicValueEnum::FloatValue(float_val), BasicTypeEnum::FloatType(target_float_type)) = (value, target_type) {
            // Handle float casting
            let source_width = if float_val.get_type() == self.context.f32_type() { 32 } else { 64 };
            let target_width = if target_float_type == self.context.f32_type() { 32 } else { 64 };
            
            if source_width < target_width {
                Ok(self.builder.build_float_ext(float_val, target_float_type, "cast")?.into())
            } else if source_width > target_width {
                Ok(self.builder.build_float_trunc(float_val, target_float_type, "cast")?.into())
            } else {
                Ok(float_val.into())
            }
        } else {
            // For other types, return as is for now
            Ok(value)
        }
    }
} 