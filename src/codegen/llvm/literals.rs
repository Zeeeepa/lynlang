use super::{LLVMCompiler, symbols};
use crate::ast::AstType;
use crate::error::CompileError;
use inkwell::values::BasicValueEnum;

impl<'ctx> LLVMCompiler<'ctx> {
    // Expression compilation methods for literals
    pub fn compile_integer_literal(&self, value: i64) -> Result<BasicValueEnum<'ctx>, CompileError> {
        Ok(self.context.i64_type().const_int(value as u64, false).into())
    }

    pub fn compile_float_literal(&self, value: f64) -> Result<BasicValueEnum<'ctx>, CompileError> {
        Ok(self.context.f64_type().const_float(value).into())
    }

    pub fn compile_string_literal(&mut self, val: &str) -> Result<BasicValueEnum<'ctx>, CompileError> {
        let ptr = self.builder.build_global_string_ptr(val, "str")?;
        let ptr_val = ptr.as_pointer_value();
        // Always return the pointer value, don't convert to integer
        // This fixes the issue where string literals were being converted to integers
        // when used as function arguments, breaking string operations
        Ok(ptr_val.into())
    }

    pub fn compile_identifier(&mut self, name: &str) -> Result<BasicValueEnum<'ctx>, CompileError> {
        eprintln!("DEBUG: compile_identifier for '{}'", name);
        eprintln!("DEBUG: Variables in scope: {:?}", self.variables.keys().collect::<Vec<_>>());
        
        // Check if this is an enum type name (will be handled later in member access)
        if let Some(symbols::Symbol::EnumType(_)) = self.symbols.lookup(name) {
            // Return a dummy value - this will be handled properly in member access
            // We can't create an enum variant without knowing which variant
            return Ok(self.context.i64_type().const_int(0, false).into());
        }
        
        // First check if this is a function name
        if let Some(function) = self.module.get_function(name) {
            // Return the function's address as a pointer value
            Ok(function.as_global_value().as_pointer_value().into())
        } else {
            // It's a variable, get the pointer
            eprintln!("DEBUG: Looking up variable '{}'", name);
            let (ptr, ast_type) = self.get_variable(name)?;
            
            // Load the value from the alloca based on type
            let loaded: BasicValueEnum = match &ast_type {
                AstType::Bool => {
                    // Booleans - load as bool_type (i1)
                    match self.builder.build_load(self.context.bool_type(), ptr, name) {
                        Ok(val) => val,
                        Err(e) => return Err(CompileError::InternalError(e.to_string(), None)),
                    }
                }
                AstType::I32 => {
                    // Load as i32
                    match self.builder.build_load(self.context.i32_type(), ptr, name) {
                        Ok(val) => val,
                        Err(e) => return Err(CompileError::InternalError(e.to_string(), None)),
                    }
                }
                AstType::I64 => {
                    // Load as i64  
                    match self.builder.build_load(self.context.i64_type(), ptr, name) {
                        Ok(val) => val,
                        Err(e) => return Err(CompileError::InternalError(e.to_string(), None)),
                    }
                }
                AstType::String => {
                    // Strings are stored as pointers
                    match self.builder.build_load(self.context.ptr_type(inkwell::AddressSpace::default()), ptr, name) {
                        Ok(val) => val.into(),
                        Err(e) => return Err(CompileError::InternalError(e.to_string(), None)),
                    }
                }
                AstType::Ptr(inner) => {
                    // For pointer types (including string interpolation results typed as *i8)
                    // We need to load the pointer value from the alloca
                    if matches!(**inner, AstType::I8) {
                        // This is likely a string (char pointer)
                        match self.builder.build_load(self.context.ptr_type(inkwell::AddressSpace::default()), ptr, name) {
                            Ok(val) => val.into(),
                            Err(e) => return Err(CompileError::InternalError(e.to_string(), None)),
                        }
                    } else if matches!(**inner, AstType::Function { .. }) {
                        match self.builder.build_load(self.context.ptr_type(inkwell::AddressSpace::default()), ptr, name) {
                            Ok(val) => val.into(),
                            Err(e) => return Err(CompileError::InternalError(e.to_string(), None)),
                        }
                    } else {
                        // Other pointer types - load the pointer value from the alloca
                        match self.builder.build_load(self.context.ptr_type(inkwell::AddressSpace::default()), ptr, name) {
                            Ok(val) => val.into(),
                            Err(e) => return Err(CompileError::InternalError(e.to_string(), None)),
                        }
                    }
                }
                AstType::Function { .. } => {
                    match self.builder.build_load(self.context.ptr_type(inkwell::AddressSpace::default()), ptr, name) {
                        Ok(val) => val.into(),
                        Err(e) => return Err(CompileError::InternalError(e.to_string(), None)),
                    }
                }
                AstType::EnumType { .. } => {
                    // For enum types, we need to load the struct value
                    // First get the enum struct type from the symbols
                    let enum_struct_type = self.context.struct_type(&[
                        self.context.i64_type().into(),
                        self.context.i64_type().into(),
                    ], false);
                    match self.builder.build_load(enum_struct_type, ptr, name) {
                        Ok(val) => val.into(),
                        Err(e) => return Err(CompileError::InternalError(e.to_string(), None)),
                    }
                }
                AstType::StdModule => {
                    // For stdlib module references, return the module marker value
                    // This value is used to identify which module is being referenced
                    match self.builder.build_load(self.context.i64_type(), ptr, name) {
                        Ok(val) => val.into(),
                        Err(e) => return Err(CompileError::InternalError(e.to_string(), None)),
                    }
                }
                _ => {
                    let elem_type = self.to_llvm_type(&ast_type)?;
                    let basic_type = self.expect_basic_type(elem_type)?;
                    match self.builder.build_load(basic_type, ptr, name) {
                        Ok(val) => val.into(),
                        Err(e) => return Err(CompileError::InternalError(e.to_string(), None)),
                    }
                }
            };
            Ok(loaded)
        }
    }
    
    pub fn compile_string_interpolation(&mut self, parts: &[crate::ast::StringPart]) -> Result<BasicValueEnum<'ctx>, CompileError> {
        use crate::ast::StringPart;
        use inkwell::AddressSpace;
        use inkwell::values::BasicMetadataValueEnum;
        
        // First, calculate the total size needed for the string
        // For now, we'll use a simple approach with sprintf for numeric values
        
        // Declare sprintf if not already declared
        let sprintf_fn = self.module.get_function("sprintf").unwrap_or_else(|| {
            let i32_type = self.context.i32_type();
            let ptr_type = self.context.ptr_type(AddressSpace::default());
            let fn_type = i32_type.fn_type(&[ptr_type.into(), ptr_type.into()], true);
            self.module.add_function("sprintf", fn_type, None)
        });
        
        // Build the format string and collect interpolated values
        let mut format_string = String::new();
        let mut values: Vec<BasicMetadataValueEnum> = Vec::new();
        
        for part in parts {
            match part {
                StringPart::Literal(s) => {
                    format_string.push_str(s);
                }
                StringPart::Interpolation(expr) => {
                    let val = self.compile_expression(expr)?;
                    
                    // Handle different value types for interpolation
                    let (format_spec, actual_val) = if val.is_struct_value() {
                        // This could be an enum (Option, Result, etc)
                        let struct_val = val.into_struct_value();
                        
                        // For enums, we check if it has a discriminant field
                        if struct_val.get_type().count_fields() >= 1 {
                            // Extract the discriminant (tag)
                            let tag = self.builder.build_extract_value(struct_val, 0, "enum_tag")?;
                            
                            // Check if this is Option type (Some=0, None=1)
                            let tag_int = tag.into_int_value();
                            let zero = tag_int.get_type().const_zero();
                            let is_some = self.builder.build_int_compare(
                                inkwell::IntPredicate::EQ,
                                tag_int,
                                zero,
                                "is_some"
                            )?;
                            
                            // Get the string representations
                            let some_str = if struct_val.get_type().count_fields() > 1 {
                                // Has payload - extract and format it
                                let payload = self.builder.build_extract_value(struct_val, 1, "payload")?;
                                // For now, format the payload as an integer
                                // In a full implementation, we'd recursively format the payload
                                let formatted = self.builder.build_global_string_ptr("Some(...)", "some_str")?;
                                formatted.as_pointer_value()
                            } else {
                                let formatted = self.builder.build_global_string_ptr("Some", "some_str")?;
                                formatted.as_pointer_value()
                            };
                            
                            let none_str = self.builder.build_global_string_ptr("None", "none_str")?;
                            
                            let result = self.builder.build_select(
                                is_some,
                                some_str,
                                none_str.as_pointer_value(),
                                "option_str"
                            )?;
                            
                            ("%s", result.into())
                        } else {
                            // Unknown struct - use a default representation
                            ("%s", self.builder.build_global_string_ptr("<struct>", "struct_str")?.as_pointer_value().into())
                        }
                    } else if val.is_int_value() {
                        let int_val = val.into_int_value();
                        let bit_width = int_val.get_type().get_bit_width();
                        match bit_width {
                            1 => {
                                // This is a boolean - convert to "true"/"false"
                                let true_str = self.builder.build_global_string_ptr("true", "true_str")?;
                                let false_str = self.builder.build_global_string_ptr("false", "false_str")?;
                                
                                let is_true = self.builder.build_int_compare(
                                    inkwell::IntPredicate::NE,
                                    int_val,
                                    int_val.get_type().const_zero(),
                                    "is_true"
                                )?;
                                
                                let result = self.builder.build_select(
                                    is_true,
                                    true_str.as_pointer_value(),
                                    false_str.as_pointer_value(),
                                    "bool_str"
                                )?;
                                
                                ("%s", result.into())
                            }
                            8 => {
                                // For 8-bit integers, extend to 32-bit to ensure proper printing
                                let extended = self.builder.build_int_z_extend(
                                    int_val,
                                    self.context.i32_type(),
                                    "i8_to_i32"
                                )?;
                                ("%d", extended.into())
                            }
                            32 => ("%d", val.into()),
                            64 => ("%lld", val.into()),
                            _ => ("%d", val.into()),
                        }
                    } else if val.is_float_value() {
                        ("%.6f", val.into())
                    } else if val.is_pointer_value() {
                        // Pointer values are strings - use %s
                        ("%s", val.into())
                    } else {
                        // Default to string format
                        ("%s", val.into())
                    };
                    
                    format_string.push_str(format_spec);
                    values.push(actual_val);
                }
            }
        }
        
        // Allocate buffer for the result (using a reasonable max size)
        let buffer_size = 1024;
        let buffer_type = self.context.i8_type().array_type(buffer_size);
        let buffer = self.builder.build_alloca(buffer_type, "str_buffer")?;
        let buffer_ptr = self.builder.build_pointer_cast(
            buffer, 
            self.context.ptr_type(AddressSpace::default()),
            "buffer_ptr"
        )?;
        
        // Build the format string
        let format_ptr = self.builder.build_global_string_ptr(&format_string, "format")?;
        
        // Build the sprintf call with all arguments
        let mut sprintf_args: Vec<BasicMetadataValueEnum> = vec![
            buffer_ptr.into(),
            format_ptr.as_pointer_value().into(),
        ];
        sprintf_args.extend(values);
        
        self.builder.build_call(sprintf_fn, &sprintf_args, "sprintf_call")?;
        
        // Return the buffer pointer
        Ok(buffer_ptr.into())
    }
} 