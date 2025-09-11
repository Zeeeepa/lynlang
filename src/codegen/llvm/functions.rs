use super::{LLVMCompiler, Type};
use crate::ast::{self, AstType};
use crate::error::CompileError;
use inkwell::{
    types::{BasicType, BasicTypeEnum, BasicMetadataTypeEnum},
    values::{BasicValueEnum, FunctionValue},
};
use inkwell::module::Linkage;

impl<'ctx> LLVMCompiler<'ctx> {
    /// Declares an external function (C FFI)
    pub fn declare_external_function(&mut self, ext_func: &ast::ExternalFunction) -> Result<(), CompileError> {
        let ret_type = self.to_llvm_type(&ext_func.return_type)?;
        
        // First get the basic types for the parameters
        let param_basic_types: Result<Vec<BasicTypeEnum>, CompileError> = ext_func.args
            .iter()
            .map(|t| self.to_llvm_type(t).and_then(|t| t.into_basic_type()))
            .collect();
        
        // Convert basic types to metadata types for the function signature
        let param_metadata: Result<Vec<BasicMetadataTypeEnum>, CompileError> = param_basic_types?
            .into_iter()
            .map(|ty| {
                Ok(match ty {
                    BasicTypeEnum::ArrayType(t) => t.into(),
                    BasicTypeEnum::FloatType(t) => t.into(),
                    BasicTypeEnum::IntType(t) => t.into(),
                    BasicTypeEnum::PointerType(t) => t.into(),
                    BasicTypeEnum::StructType(t) => t.into(),
                    BasicTypeEnum::VectorType(t) => t.into(),
                    BasicTypeEnum::ScalableVectorType(t) => t.into(),
                })
            })
            .collect();
            
        let param_metadata = param_metadata?;

        // Create the function type with the metadata types
        let function_type = match ret_type {
            Type::Basic(b) => {
                match b {
                    BasicTypeEnum::ArrayType(t) => t.fn_type(&param_metadata, ext_func.is_varargs),
                    BasicTypeEnum::FloatType(t) => t.fn_type(&param_metadata, ext_func.is_varargs),
                    BasicTypeEnum::IntType(t) => t.fn_type(&param_metadata, ext_func.is_varargs),
                    BasicTypeEnum::PointerType(t) => t.fn_type(&param_metadata, ext_func.is_varargs),
                    BasicTypeEnum::StructType(t) => t.fn_type(&param_metadata, ext_func.is_varargs),
                    BasicTypeEnum::VectorType(t) => t.fn_type(&param_metadata, ext_func.is_varargs),
                    BasicTypeEnum::ScalableVectorType(t) => t.fn_type(&param_metadata, ext_func.is_varargs),
                }
            },
            Type::Function(f) => f,
            Type::Void => self.context.void_type().fn_type(&param_metadata, ext_func.is_varargs),
            Type::Pointer(_) => {
                return Err(CompileError::UnsupportedFeature(
                    "Cannot use pointer type as function return type".to_string(),
                    None,
                ));
            }
            Type::Struct(st) => {
                st.fn_type(&param_metadata, false)
            }
        };

        // Only declare if not already declared
        if self.module.get_function(&ext_func.name).is_none() {
            self.module.add_function(&ext_func.name, function_type, None);
        }
        Ok(())
    }

    /// Defines and compiles a function in one step
    pub fn declare_function(&mut self, function: &ast::Function) -> Result<FunctionValue<'ctx>, CompileError> {
        
        // First, get the return type
        let return_type = self.to_llvm_type(&function.return_type)?;
        
        // Get parameter basic types with their names
        let param_basic_types: Result<Vec<BasicTypeEnum>, CompileError> = function
            .args
            .iter()
            .map(|(_name, t)| {
                self.to_llvm_type(t)
                    .and_then(|zen_type| {
                        self.expect_basic_type(zen_type)
                    })
            })
            .collect();
            
        // Convert basic types to metadata types for the function signature
        let param_metadata: Result<Vec<BasicMetadataTypeEnum>, CompileError> = param_basic_types?
            .into_iter()
            .map(|ty| {
                Ok(match ty {
                    BasicTypeEnum::ArrayType(t) => t.into(),
                    BasicTypeEnum::FloatType(t) => t.into(),
                    BasicTypeEnum::IntType(t) => t.into(),
                    BasicTypeEnum::PointerType(t) => t.into(),
                    BasicTypeEnum::StructType(t) => t.into(),
                    BasicTypeEnum::VectorType(t) => t.into(),
                    BasicTypeEnum::ScalableVectorType(t) => t.into(),
                })
            })
            .collect();
            
        let param_metadata = param_metadata?;
        
        // Create the function type with the metadata types
        let function_type = match return_type {
            Type::Basic(b) => {
                match b {
                    BasicTypeEnum::ArrayType(t) => t.fn_type(&param_metadata, false),
                    BasicTypeEnum::FloatType(t) => t.fn_type(&param_metadata, false),
                    BasicTypeEnum::IntType(t) => t.fn_type(&param_metadata, false),
                    BasicTypeEnum::PointerType(t) => t.fn_type(&param_metadata, false),
                    BasicTypeEnum::StructType(t) => t.fn_type(&param_metadata, false),
                    BasicTypeEnum::VectorType(t) => t.fn_type(&param_metadata, false),
                    BasicTypeEnum::ScalableVectorType(t) => t.fn_type(&param_metadata, false),
                }
            },
            Type::Function(f) => f,
            Type::Void => self.context.void_type().fn_type(&param_metadata, false),
            Type::Pointer(_) => {
                return Err(CompileError::UnsupportedFeature(
                    "Cannot use pointer type as function return type".to_string(),
                    None,
                ));
            }
            Type::Struct(st) => {
                st.fn_type(&param_metadata, false)
            }
        };
        
        // Check if function already declared
        if let Some(func) = self.module.get_function(&function.name) {
            return Ok(func);
        }
        
        // Declare the function (this creates a declaration)
        let function_value = self.module.add_function(&function.name, function_type, None);
        
        // Set the function linkage to external so it can be linked
        function_value.set_linkage(Linkage::External);
        
        // Store the function for later use
        self.functions.insert(function.name.clone(), function_value);
        // Store the return type for type inference
        self.function_types.insert(function.name.clone(), function.return_type.clone());
        
        Ok(function_value)
    }
    
    pub fn compile_function_body(&mut self, function: &ast::Function) -> Result<(), CompileError> {
        // Get the already-declared function
        let function_value = self.module.get_function(&function.name)
            .ok_or_else(|| CompileError::InternalError(
                format!("Function {} not declared", function.name),
                None
            ))?;
        
        // Set names for all arguments
        for (i, (arg_name, _)) in function.args.iter().enumerate() {
            if let Some(param) = function_value.get_nth_param(i as u32) {
                param.set_name(arg_name);
            }
        }
        
        // Add to symbol table
        self.symbols.insert(
            function.name.clone(),
            super::symbols::Symbol::Function(function_value),
        );
        
        // Now compile the function body
        let entry_block = self.context.append_basic_block(function_value, "entry");
        self.builder.position_at_end(entry_block);
        self.current_function = Some(function_value);

        // Clear variables from previous function by entering a new scope
        self.symbols.enter_scope();

        // Store function parameters in variables
        for (i, (name, type_)) in function.args.iter().enumerate() {
            let param = function_value.get_nth_param(i as u32).unwrap();
            // Get the LLVM type for this parameter
            let llvm_type = self.to_llvm_type(type_)?;
            let basic_type = self.expect_basic_type(llvm_type)?;
            let alloca = self.builder.build_alloca(basic_type, name)?;
            self.builder.build_store(alloca, param)?;
            // Register the parameter in the variables map
            self.variables.insert(name.clone(), (alloca, type_.clone()));
        }

        for statement in &function.body {
            self.compile_statement(statement)?;
        }

        // Check if we need to add a return statement
        if let Some(block) = self.builder.get_insert_block() {
            if block.get_terminator().is_none() {
                // Check if the last statement was an expression that should be returned
                if let Some(last_stmt) = function.body.last() {
                    match last_stmt {
                        ast::Statement::Expression(expr) => {
                            // For non-void functions, treat trailing expressions as return values
                            if !matches!(function.return_type, AstType::Void) {
                                let value = self.compile_expression(expr)?;
                                // Cast to the correct return type if needed
                                let return_type = self.to_llvm_type(&function.return_type)?;
                                let return_basic_type = self.expect_basic_type(return_type)?;
                                let casted_value = self.cast_value_to_type(value, return_basic_type)?;
                                self.builder.build_return(Some(&casted_value))?;
                            } else {
                                // For void functions, just return void
                                self.builder.build_return(None)?;
                            }
                        }
                        ast::Statement::ComptimeBlock(statements) => {
                            // ComptimeBlock with expressions should be treated as a return value
                            if !matches!(function.return_type, AstType::Void) {
                                // Find the last expression in the comptime block
                                if let Some(ast::Statement::Expression(expr)) = statements.last() {
                                    // Evaluate the comptime expression and return it
                                    let value = self.compile_expression(&ast::Expression::Comptime(Box::new(expr.clone())))?;
                                    // Cast to the correct return type if needed
                                    let return_type = self.to_llvm_type(&function.return_type)?;
                                    let return_basic_type = self.expect_basic_type(return_type)?;
                                    let casted_value = self.cast_value_to_type(value, return_basic_type)?;
                                    self.builder.build_return(Some(&casted_value))?;
                                } else {
                                    return Err(CompileError::MissingReturnStatement(function.name.clone(), None));
                                }
                            } else {
                                // For void functions, just return void
                                self.builder.build_return(None)?;
                            }
                        }
                        _ => {
                            // Not a trailing expression, handle normally
                            if let AstType::Void = function.return_type {
                                self.builder.build_return(None)?;
                            } else {
                                return Err(CompileError::MissingReturnStatement(function.name.clone(), None));
                            }
                        }
                    }
                } else {
                    // No statements in function body
                    if let AstType::Void = function.return_type {
                        self.builder.build_return(None)?;
                    } else {
                        return Err(CompileError::MissingReturnStatement(function.name.clone(), None));
                    }
                }
            } else {
            }
        } else {
        }

        self.current_function = None;
        Ok(())
    }
    
    // Backward compatibility wrapper
    pub fn define_and_compile_function(&mut self, function: &ast::Function) -> Result<(), CompileError> {
        self.declare_function(function)?;
        self.compile_function_body(function)
    }

    pub fn compile_function_call(&mut self, name: &str, args: &[ast::Expression]) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Check if this is a stdlib function call (e.g., io.print)
        if name.contains('.') {
            let parts: Vec<&str> = name.splitn(2, '.').collect();
            if parts.len() == 2 {
                let module = parts[0];
                let func = parts[1];
                
                // Handle stdlib function calls
                if module == "io" {
                    match func {
                        "print" => return self.compile_io_print(args),
                        "println" => return self.compile_io_println(args),
                        "print_int" => return self.compile_io_print_int(args),
                        "print_float" => return self.compile_io_print_float(args),
                        _ => {}
                    }
                } else if module == "math" {
                    // Handle math module functions
                    return self.compile_math_function(func, args);
                } else if module == "core" {
                    // Handle core module functions
                    match func {
                        "assert" => return self.compile_core_assert(args),
                        "panic" => return self.compile_core_panic(args),
                        _ => {}
                    }
                }
            }
        }
        
        // First check if this is a direct function call
        if let Some(function) = self.module.get_function(name) {
            // Direct function call
            let mut compiled_args = Vec::with_capacity(args.len());
            let param_types = function.get_type().get_param_types();
            
            for (i, arg) in args.iter().enumerate() {
                let mut val = self.compile_expression(arg)?;
                
                // Cast integer arguments to match expected parameter type if needed
                if i < param_types.len() {
                    let expected_type = param_types[i];
                    if val.is_int_value() && expected_type.is_int_type() {
                        let int_val = val.into_int_value();
                        let expected_int_type = expected_type.into_int_type();
                        if int_val.get_type().get_bit_width() != expected_int_type.get_bit_width() {
                            // Need to cast
                            if int_val.get_type().get_bit_width() < expected_int_type.get_bit_width() {
                                // Sign extend
                                val = self.builder.build_int_s_extend(int_val, expected_int_type, "extend")?.into();
                            } else {
                                // Truncate
                                val = self.builder.build_int_truncate(int_val, expected_int_type, "trunc")?.into();
                            }
                        }
                    }
                }
                
                compiled_args.push(val);
            }
            let args_metadata: Vec<inkwell::values::BasicMetadataValueEnum> = compiled_args.iter()
                .map(|arg| inkwell::values::BasicMetadataValueEnum::try_from(*arg).map_err(|_| 
                    CompileError::InternalError("Failed to convert argument to metadata".to_string(), None)
                ))
                .collect::<Result<Vec<_>, _>>()?;
            let call = self.builder.build_call(function, &args_metadata, "calltmp")?;
            Ok(call.try_as_basic_value().left().ok_or_else(||
                CompileError::InternalError("Function call did not return a value".to_string(), None)
            )?)
        } else if let Ok((alloca, var_type)) = self.get_variable(name) {
            // Function pointer call - load the function pointer from variable
            let function_ptr = self.builder.build_load(
                alloca.get_type(),
                alloca,
                "func_ptr"
            )?;
            
            // Get function type from the variable type
            let function_type = match &var_type {
                AstType::Function { args, return_type } => {
                    let param_types_basic: Result<Vec<BasicTypeEnum>, CompileError> = args
                        .iter()
                        .map(|ty| {
                            let llvm_ty = self.to_llvm_type(ty)?;
                            match llvm_ty {
                                Type::Basic(b) => Ok(b),
                                _ => Err(CompileError::InternalError("Function argument type must be a basic type".to_string(), None)),
                            }
                        })
                        .collect();
                    let param_types_basic = param_types_basic?;
                    let param_metadata: Vec<BasicMetadataTypeEnum> = param_types_basic.iter().map(|ty| (*ty).into()).collect();
                    let ret_type = self.to_llvm_type(return_type)?;
                    match ret_type {
                        Type::Basic(b) => b.fn_type(&param_metadata, false),
                        Type::Void => self.context.void_type().fn_type(&param_metadata, false),
                        _ => return Err(CompileError::InternalError("Function return type must be a basic type or void".to_string(), None)),
                    }
                },
                AstType::FunctionPointer { param_types, return_type } => {
                    let param_types_basic: Result<Vec<BasicTypeEnum>, CompileError> = param_types
                        .iter()
                        .map(|ty| {
                            let llvm_ty = self.to_llvm_type(ty)?;
                            match llvm_ty {
                                Type::Basic(b) => Ok(b),
                                _ => Err(CompileError::InternalError("Function argument type must be a basic type".to_string(), None)),
                            }
                        })
                        .collect();
                    let param_types_basic = param_types_basic?;
                    let param_metadata: Vec<BasicMetadataTypeEnum> = param_types_basic.iter().map(|ty| (*ty).into()).collect();
                    let ret_type = self.to_llvm_type(return_type)?;
                    match ret_type {
                        Type::Basic(b) => b.fn_type(&param_metadata, false),
                        Type::Void => self.context.void_type().fn_type(&param_metadata, false),
                        _ => return Err(CompileError::InternalError("Function return type must be a basic type or void".to_string(), None)),
                    }
                },
                AstType::Pointer(inner) if matches!(**inner, AstType::FunctionPointer { .. }) => {
                    let inner_llvm_type = self.to_llvm_type(inner)?;
                    match inner_llvm_type {
                        Type::Basic(inkwell::types::BasicTypeEnum::PointerType(_ptr_type)) => {
                            // For function pointers, we need to get the function type
                            // Since we can't get it directly from the pointer type in newer LLVM,
                            // we'll create a function type based on the AST type
                            if let AstType::FunctionPointer { param_types, return_type } = &**inner {
                                let param_types_basic: Result<Vec<BasicTypeEnum>, CompileError> = param_types
                                    .iter()
                                    .map(|ty| {
                                        let llvm_ty = self.to_llvm_type(ty)?;
                                        match llvm_ty {
                                            Type::Basic(b) => Ok(b),
                                            _ => Err(CompileError::InternalError("Function argument type must be a basic type".to_string(), None)),
                                        }
                                    })
                                    .collect();
                                let param_types_basic = param_types_basic?;
                                let param_metadata: Vec<BasicMetadataTypeEnum> = param_types_basic.iter().map(|ty| (*ty).into()).collect();
                                let ret_type = self.to_llvm_type(return_type)?;
                                match ret_type {
                                    Type::Basic(b) => b.fn_type(&param_metadata, false),
                                    Type::Void => self.context.void_type().fn_type(&param_metadata, false),
                                    _ => return Err(CompileError::InternalError("Function return type must be a basic type or void".to_string(), None)),
                                }
                            } else {
                                return Err(CompileError::InternalError("Expected function pointer type in pointer".to_string(), None));
                            }
                        },
                        _ => return Err(CompileError::TypeMismatch {
                            expected: "function pointer".to_string(),
                            found: format!("{:?}", inner_llvm_type),
                            span: None,
                        }),
                    }
                },
                _ => return Err(CompileError::TypeMismatch {
                    expected: "function pointer".to_string(),
                    found: format!("{:?}", var_type),
                    span: None,
                }),
            };
            
            // Compile arguments
            let mut compiled_args = Vec::with_capacity(args.len());
            for arg in args {
                let val = self.compile_expression(arg)?;
                compiled_args.push(val);
            }
            let args_metadata: Vec<inkwell::values::BasicMetadataValueEnum> = compiled_args.iter()
                .map(|arg| inkwell::values::BasicMetadataValueEnum::try_from(*arg).map_err(|_| 
                    CompileError::InternalError("Failed to convert argument to metadata".to_string(), None)
                ))
                .collect::<Result<Vec<_>, _>>()?;
            
            // Cast the loaded pointer to the correct function type
            let casted_function_ptr = self.builder.build_pointer_cast(
                function_ptr.into_pointer_value(),
                self.context.ptr_type(inkwell::AddressSpace::default()),
                "casted_func_ptr"
            )?;
            
            // Make indirect call using build_indirect_call for function pointers
            let call = self.builder.build_indirect_call(
                function_type,
                casted_function_ptr,
                &args_metadata,
                "indirect_call"
            )?;
            Ok(call.try_as_basic_value().left().ok_or_else(||
                CompileError::InternalError("Function call did not return a value".to_string(), None)
            )?)
        } else {
            // Function not found
            Err(CompileError::UndeclaredFunction(name.to_string(), None))
        }
    }
    
    /// Compile io.print function call
    fn compile_io_print(&mut self, args: &[ast::Expression]) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if args.len() != 1 {
            return Err(CompileError::TypeError(
                format!("io.print expects 1 argument, got {}", args.len()),
                None,
            ));
        }
        
        // Get or declare printf
        let printf_fn = self.module.get_function("printf").unwrap_or_else(|| {
            let i32_type = self.context.i32_type();
            let i8_ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
            let fn_type = i32_type.fn_type(&[i8_ptr_type.into()], true);
            self.module.add_function("printf", fn_type, None)
        });
        
        // Compile the string argument
        let arg_value = self.compile_expression(&args[0])?;
        
        // Call printf
        let _call = self.builder.build_call(
            printf_fn,
            &[arg_value.into()],
            "printf_call"
        )?;
        
        // Return void as unit value
        Ok(self.context.i32_type().const_zero().into())
    }
    
    /// Compile io.println function call
    fn compile_io_println(&mut self, args: &[ast::Expression]) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if args.len() != 1 {
            return Err(CompileError::TypeError(
                format!("io.println expects 1 argument, got {}", args.len()),
                None,
            ));
        }
        
        // Get or declare puts (which adds a newline automatically)
        let puts_fn = self.module.get_function("puts").unwrap_or_else(|| {
            let i32_type = self.context.i32_type();
            let i8_ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
            let fn_type = i32_type.fn_type(&[i8_ptr_type.into()], false);
            self.module.add_function("puts", fn_type, None)
        });
        
        // Compile the string argument
        let arg_value = self.compile_expression(&args[0])?;
        
        // Call puts
        let _call = self.builder.build_call(
            puts_fn,
            &[arg_value.into()],
            "puts_call"
        )?;
        
        // Return void as unit value
        Ok(self.context.i32_type().const_zero().into())
    }
    
    /// Compile io.print_int function call
    fn compile_io_print_int(&mut self, args: &[ast::Expression]) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if args.len() != 1 {
            return Err(CompileError::TypeError(
                format!("io.print_int expects 1 argument, got {}", args.len()),
                None,
            ));
        }
        
        // Get or declare printf
        let printf_fn = self.module.get_function("printf").unwrap_or_else(|| {
            let i32_type = self.context.i32_type();
            let i8_ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
            let fn_type = i32_type.fn_type(&[i8_ptr_type.into()], true);
            self.module.add_function("printf", fn_type, None)
        });
        
        // Create format string for integer
        let format_str = self.builder.build_global_string_ptr("%d\n", "int_format")?;
        
        // Compile the integer argument
        let arg_value = self.compile_expression(&args[0])?;
        
        // Call printf
        let _call = self.builder.build_call(
            printf_fn,
            &[format_str.as_pointer_value().into(), arg_value.into()],
            "printf_int_call"
        )?;
        
        // Return void as unit value
        Ok(self.context.i32_type().const_zero().into())
    }
    
    /// Compile io.print_float function call
    fn compile_io_print_float(&mut self, args: &[ast::Expression]) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if args.len() != 1 {
            return Err(CompileError::TypeError(
                format!("io.print_float expects 1 argument, got {}", args.len()),
                None,
            ));
        }
        
        // Get or declare printf
        let printf_fn = self.module.get_function("printf").unwrap_or_else(|| {
            let i32_type = self.context.i32_type();
            let i8_ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
            let fn_type = i32_type.fn_type(&[i8_ptr_type.into()], true);
            self.module.add_function("printf", fn_type, None)
        });
        
        // Create format string for float
        let format_str = self.builder.build_global_string_ptr("%.6f\n", "float_format")?;
        
        // Compile the float argument
        let arg_value = self.compile_expression(&args[0])?;
        
        // Convert to f64 if needed
        let float_value = match arg_value {
            BasicValueEnum::FloatValue(f) => f,
            BasicValueEnum::IntValue(i) => {
                // Convert int to float
                self.builder.build_signed_int_to_float(i, self.context.f64_type(), "int_to_float")?
            }
            _ => {
                return Err(CompileError::TypeError(
                    "io.print_float expects a numeric argument".to_string(),
                    None,
                ));
            }
        };
        
        // Call printf
        let _call = self.builder.build_call(
            printf_fn,
            &[format_str.as_pointer_value().into(), float_value.into()],
            "printf_float_call"
        )?;
        
        // Return void as unit value
        Ok(self.context.i32_type().const_zero().into())
    }
    
    /// Compile math module function calls
    fn compile_math_function(&mut self, func_name: &str, args: &[ast::Expression]) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Handle min and max functions
        if func_name == "min" || func_name == "max" {
            if args.len() != 2 {
                return Err(CompileError::TypeError(
                    format!("math.{} expects 2 arguments, got {}", func_name, args.len()),
                    None,
                ));
            }
            
            let left = self.compile_expression(&args[0])?;
            let right = self.compile_expression(&args[1])?;
            
            // Handle integer min/max
            if left.is_int_value() && right.is_int_value() {
                let left_int = left.into_int_value();
                let right_int = right.into_int_value();
                
                // Make sure both integers are the same type
                let (left_int, right_int) = if left_int.get_type() != right_int.get_type() {
                    // Promote to i64 if types differ
                    let i64_type = self.context.i64_type();
                    let left_promoted = if left_int.get_type().get_bit_width() < 64 {
                        self.builder.build_int_s_extend(left_int, i64_type, "extend_left")?
                    } else {
                        left_int
                    };
                    let right_promoted = if right_int.get_type().get_bit_width() < 64 {
                        self.builder.build_int_s_extend(right_int, i64_type, "extend_right")?
                    } else {
                        right_int
                    };
                    (left_promoted, right_promoted)
                } else {
                    (left_int, right_int)
                };
                
                let cmp = if func_name == "min" {
                    self.builder.build_int_compare(
                        inkwell::IntPredicate::SLT,
                        left_int,
                        right_int,
                        "lt"
                    )?
                } else {
                    self.builder.build_int_compare(
                        inkwell::IntPredicate::SGT,
                        left_int,
                        right_int,
                        "gt"
                    )?
                };
                
                let result = self.builder.build_select(cmp, left_int, right_int, func_name)?;
                return Ok(result.try_into().unwrap());
            }
            
            // Handle float min/max or mixed types
            let left_float = match left {
                BasicValueEnum::FloatValue(f) => f,
                BasicValueEnum::IntValue(i) => {
                    self.builder.build_signed_int_to_float(i, self.context.f64_type(), "int_to_float")?
                }
                _ => {
                    return Err(CompileError::TypeError(
                        format!("math.{} expects numeric arguments", func_name),
                        None,
                    ));
                }
            };
            
            let right_float = match right {
                BasicValueEnum::FloatValue(f) => f,
                BasicValueEnum::IntValue(i) => {
                    self.builder.build_signed_int_to_float(i, self.context.f64_type(), "int_to_float")?
                }
                _ => {
                    return Err(CompileError::TypeError(
                        format!("math.{} expects numeric arguments", func_name),
                        None,
                    ));
                }
            };
            
            // Use fmin/fmax intrinsics for floats
            let intrinsic_name = if func_name == "min" { "llvm.minnum.f64" } else { "llvm.maxnum.f64" };
            let intrinsic = self.module.get_function(intrinsic_name).unwrap_or_else(|| {
                let f64_type = self.context.f64_type();
                let fn_type = f64_type.fn_type(&[f64_type.into(), f64_type.into()], false);
                self.module.add_function(intrinsic_name, fn_type, None)
            });
            
            let result = self.builder.build_call(
                intrinsic,
                &[left_float.into(), right_float.into()],
                func_name
            )?;
            
            return Ok(result.try_as_basic_value().left().unwrap());
        }
        
        // Handle abs specially for integer types
        if func_name == "abs" {
            if args.len() != 1 {
                return Err(CompileError::TypeError(
                    format!("math.abs expects 1 argument, got {}", args.len()),
                    None,
                ));
            }
            
            let val = self.compile_expression(&args[0])?;
            return match val {
                BasicValueEnum::IntValue(i) => {
                    // For integers, generate abs using conditional
                    let zero = i.get_type().const_zero();
                    let is_negative = self.builder.build_int_compare(
                        inkwell::IntPredicate::SLT,
                        i,
                        zero,
                        "is_negative"
                    )?;
                    let neg = self.builder.build_int_neg(i, "neg")?;
                    let abs_val = self.builder.build_select(is_negative, neg, i, "abs")?;
                    Ok(abs_val.try_into().unwrap())
                }
                BasicValueEnum::FloatValue(f) => {
                    // For floats, use fabs
                    let fabs_fn = self.module.get_function("fabs").unwrap_or_else(|| {
                        let fn_type = self.context.f64_type().fn_type(&[
                            self.context.f64_type().into(),
                        ], false);
                        self.module.add_function("fabs", fn_type, None)
                    });
                    let call = self.builder.build_call(
                        fabs_fn,
                        &[f.into()],
                        "fabs_call"
                    )?;
                    Ok(call.try_as_basic_value().left().unwrap())
                }
                _ => Err(CompileError::TypeError(
                    "math.abs expects a numeric argument".to_string(),
                    None,
                ))
            };
        }
        
        // Map math function names to their C math library equivalents
        let c_func_name = match func_name {
            "sqrt" => "sqrt",
            "sin" => "sin",
            "cos" => "cos",
            "tan" => "tan",
            "asin" => "asin",
            "acos" => "acos",
            "atan" => "atan",
            "exp" => "exp",
            "log" => "log",
            "log10" => "log10",
            "log2" => "log2",
            "pow" => "pow",
            "floor" => "floor",
            "ceil" => "ceil",
            "round" => "round",
            "fabs" => "fabs",
            _ => {
                return Err(CompileError::UndeclaredFunction(
                    format!("math.{}", func_name),
                    None,
                ));
            }
        };
        
        // Determine function signature based on the function
        let (expected_args, fn_type) = match func_name {
            "pow" | "atan2" | "fmod" | "hypot" => {
                // Two-argument functions
                (2, self.context.f64_type().fn_type(&[
                    self.context.f64_type().into(),
                    self.context.f64_type().into(),
                ], false))
            }
            _ => {
                // Single-argument functions
                (1, self.context.f64_type().fn_type(&[
                    self.context.f64_type().into(),
                ], false))
            }
        };
        
        // Check argument count
        if args.len() != expected_args {
            return Err(CompileError::TypeError(
                format!("math.{} expects {} argument(s), got {}", func_name, expected_args, args.len()),
                None,
            ));
        }
        
        // Get or declare the math function
        let math_fn = self.module.get_function(c_func_name).unwrap_or_else(|| {
            self.module.add_function(c_func_name, fn_type, None)
        });
        
        // Compile arguments
        let mut compiled_args = Vec::new();
        for arg in args {
            let val = self.compile_expression(arg)?;
            // Convert to f64 if needed
            let f64_val = match val {
                BasicValueEnum::FloatValue(f) => f,
                BasicValueEnum::IntValue(i) => {
                    // Cast int to float
                    self.builder.build_signed_int_to_float(i, self.context.f64_type(), "int_to_float")?
                }
                _ => {
                    return Err(CompileError::TypeError(
                        format!("math.{} expects numeric arguments", func_name),
                        None,
                    ));
                }
            };
            compiled_args.push(f64_val.into());
        }
        
        // Call the math function
        let call_result = self.builder.build_call(
            math_fn,
            &compiled_args,
            &format!("{}_call", c_func_name),
        )?;
        
        // Return the result
        Ok(call_result.try_as_basic_value().left().unwrap_or_else(|| {
            self.context.f64_type().const_zero().into()
        }))
    }
    
    /// Compile core.assert function call
    fn compile_core_assert(&mut self, args: &[ast::Expression]) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if args.len() != 1 {
            return Err(CompileError::TypeError(
                format!("core.assert expects 1 argument, got {}", args.len()),
                None,
            ));
        }
        
        let condition = self.compile_expression(&args[0])?;
        let condition = if condition.is_int_value() {
            condition.into_int_value()
        } else {
            return Err(CompileError::TypeError(
                "core.assert requires a boolean condition".to_string(),
                None,
            ));
        };
        
        // Create basic blocks for assertion
        let current_fn = self.current_function.ok_or_else(|| 
            CompileError::InternalError("No current function".to_string(), None)
        )?;
        
        let then_block = self.context.append_basic_block(current_fn, "assert_pass");
        let else_block = self.context.append_basic_block(current_fn, "assert_fail");
        
        // Check condition
        self.builder.build_conditional_branch(condition, then_block, else_block)?;
        
        // Assert fail block - call abort() or exit(1)
        self.builder.position_at_end(else_block);
        
        // Get or declare exit function
        let exit_fn = self.module.get_function("exit").unwrap_or_else(|| {
            let i32_type = self.context.i32_type();
            let fn_type = self.context.void_type().fn_type(&[i32_type.into()], false);
            self.module.add_function("exit", fn_type, Some(inkwell::module::Linkage::External))
        });
        
        // Call exit(1)
        let exit_code = self.context.i32_type().const_int(1, false);
        self.builder.build_call(exit_fn, &[exit_code.into()], "exit_call")?;
        self.builder.build_unreachable()?;
        
        // Continue in pass block
        self.builder.position_at_end(then_block);
        
        // Return void value
        Ok(self.context.i32_type().const_int(0, false).into())
    }
    
    /// Compile core.panic function call
    fn compile_core_panic(&mut self, args: &[ast::Expression]) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if args.len() != 1 {
            return Err(CompileError::TypeError(
                format!("core.panic expects 1 argument, got {}", args.len()),
                None,
            ));
        }
        
        // First print the panic message if it's a string
        if let ast::Expression::String(msg) = &args[0] {
            // Get or declare puts function
            let puts = self.module.get_function("puts").unwrap_or_else(|| {
                let i8_ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_type = self.context.i32_type().fn_type(&[i8_ptr_type.into()], false);
                self.module.add_function("puts", fn_type, Some(inkwell::module::Linkage::External))
            });
            
            // Create panic message with "panic: " prefix
            let panic_msg = format!("panic: {}", msg);
            let string_value = self.builder.build_global_string_ptr(&panic_msg, "panic_msg")?;
            self.builder.build_call(puts, &[string_value.as_pointer_value().into()], "puts_call")?;
        }
        
        // Get or declare exit function
        let exit_fn = self.module.get_function("exit").unwrap_or_else(|| {
            let i32_type = self.context.i32_type();
            let fn_type = self.context.void_type().fn_type(&[i32_type.into()], false);
            self.module.add_function("exit", fn_type, Some(inkwell::module::Linkage::External))
        });
        
        // Call exit(1)
        let exit_code = self.context.i32_type().const_int(1, false);
        self.builder.build_call(exit_fn, &[exit_code.into()], "exit_call")?;
        self.builder.build_unreachable()?;
        
        // Create a new unreachable block to satisfy type system
        let current_fn = self.current_function.ok_or_else(|| 
            CompileError::InternalError("No current function".to_string(), None)
        )?;
        let unreachable_block = self.context.append_basic_block(current_fn, "after_panic");
        self.builder.position_at_end(unreachable_block);
        
        // Return a dummy value (this code is unreachable)
        Ok(self.context.i32_type().const_int(0, false).into())
    }
} 