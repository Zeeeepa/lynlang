use super::{LLVMCompiler, Type};
use crate::ast::{self, AstType, ExternalFunction, Function};
use crate::error::CompileError;
use inkwell::{
    types::{BasicType, BasicTypeEnum, BasicMetadataTypeEnum},
    values::{BasicValueEnum, FunctionValue, PointerValue},
    AddressSpace,
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
            Type::Struct(_) => {
                return Err(CompileError::UnsupportedFeature(
                    "Cannot use struct type as function return type".to_string(),
                    None,
                ));
            }
        };

        // Only declare if not already declared
        if self.module.get_function(&ext_func.name).is_none() {
            self.module.add_function(&ext_func.name, function_type, None);
        }
        Ok(())
    }

    /// Defines and compiles a function in one step
    pub fn define_and_compile_function(&mut self, function: &ast::Function) -> Result<(), CompileError> {
        println!("DEBUG: Starting to define and compile function: {}", function.name);
        
        // First, get the return type
        let return_type = self.to_llvm_type(&function.return_type)?;
        println!("  Return type: {:?}", return_type);
        
        // Get parameter basic types with their names
        let param_basic_types: Result<Vec<BasicTypeEnum>, CompileError> = function
            .args
            .iter()
            .map(|(name, t)| {
                println!("  Param: {}: {:?}", name, t);
                self.to_llvm_type(t)
                    .and_then(|lyn_type| {
                        println!("    LLVM type: {:?}", lyn_type);
                        self.expect_basic_type(lyn_type)
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
        println!("  Param metadata types: {:?}", param_metadata);
        
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
            Type::Struct(_) => {
                return Err(CompileError::UnsupportedFeature(
                    "Cannot use struct type as function return type".to_string(),
                    None,
                ));
            }
        };
        
        // Define the function (this creates a definition, not a declaration)
        let function_value = self.module.add_function(&function.name, function_type, None);
        println!("  Defined function: {:?}", function_value);
        
        // Set the function linkage to external so it can be linked
        function_value.set_linkage(Linkage::External);
        
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
        println!("DEBUG: Created entry block and positioned builder");

        // Clear variables from previous function by entering a new scope
        self.symbols.enter_scope();

        // Store function parameters in variables
        for (i, (name, type_)) in function.args.iter().enumerate() {
            let param = function_value.get_nth_param(i as u32).unwrap();
            // Get the LLVM type for this parameter
            let llvm_type = self.to_llvm_type(type_)?;
            let basic_type = self.expect_basic_type(llvm_type)?;
            let alloca = self.builder.build_alloca(basic_type, name)?;
            eprintln!("[DEBUG] Storing param '{}' of type {:?} into alloca", name, param.get_type());
            self.builder.build_store(alloca, param)?;
            // Register the parameter in the variables map
            self.variables.insert(name.clone(), (alloca, type_.clone()));
        }

        println!("DEBUG: Compiling {} statements in function body", function.body.len());
        for (i, statement) in function.body.iter().enumerate() {
            println!("DEBUG: Compiling statement {}: {:?}", i, statement);
            self.compile_statement(statement)?;
        }

        // Check if we need to add a return statement
        if let Some(block) = self.builder.get_insert_block() {
            if block.get_terminator().is_none() {
                println!("DEBUG: No terminator found, adding return statement");
                // Check if the last statement was an expression that should be returned
                if let Some(last_stmt) = function.body.last() {
                    if let ast::Statement::Expression(expr) = last_stmt {
                        // For non-void functions, treat trailing expressions as return values
                        if !matches!(function.return_type, AstType::Void) {
                            println!("DEBUG: Compiling trailing expression as return value");
                            let value = self.compile_expression(expr)?;
                            self.builder.build_return(Some(&value))?;
                            println!("DEBUG: Added return statement with value");
                        } else {
                            // For void functions, just return void
                            self.builder.build_return(None)?;
                            println!("DEBUG: Added void return statement");
                        }
                    } else {
                        // Not a trailing expression, handle normally
                        if let AstType::Void = function.return_type {
                            self.builder.build_return(None)?;
                            println!("DEBUG: Added void return statement (no trailing expr)");
                        } else {
                            return Err(CompileError::MissingReturnStatement(function.name.clone(), None));
                        }
                    }
                } else {
                    // No statements in function body
                    if let AstType::Void = function.return_type {
                        self.builder.build_return(None)?;
                        println!("DEBUG: Added void return statement (empty function)");
                    } else {
                        return Err(CompileError::MissingReturnStatement(function.name.clone(), None));
                    }
                }
            } else {
                println!("DEBUG: Block already has terminator");
            }
        } else {
            println!("DEBUG: No insert block found");
        }

        self.current_function = None;
        println!("DEBUG: Finished defining and compiling function: {}", function.name);
        Ok(())
    }

    pub fn compile_function_call(&mut self, name: &str, args: &[ast::Expression]) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // First check if this is a direct function call
        if let Some(function) = self.module.get_function(name) {
            // Direct function call
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
                    let param_types: Result<Vec<BasicTypeEnum>, CompileError> = args
                        .iter()
                        .map(|ty| {
                            let llvm_ty = self.to_llvm_type(ty)?;
                            match llvm_ty {
                                Type::Basic(b) => Ok(b),
                                _ => Err(CompileError::InternalError("Function argument type must be a basic type".to_string(), None)),
                            }
                        })
                        .collect();
                    let param_types = param_types?;
                    let param_metadata: Vec<BasicMetadataTypeEnum> = param_types.iter().map(|ty| (*ty).into()).collect();
                    let ret_type = self.to_llvm_type(return_type)?;
                    match ret_type {
                        Type::Basic(b) => b.fn_type(&param_metadata, false),
                        Type::Void => self.context.void_type().fn_type(&param_metadata, false),
                        _ => return Err(CompileError::InternalError("Function return type must be a basic type or void".to_string(), None)),
                    }
                },
                AstType::Pointer(inner) if matches!(**inner, AstType::Function { .. }) => {
                    let inner_llvm_type = self.to_llvm_type(inner)?;
                    match inner_llvm_type {
                        Type::Basic(inkwell::types::BasicTypeEnum::PointerType(_ptr_type)) => {
                            // For function pointers, we need to get the function type
                            // Since we can't get it directly from the pointer type in newer LLVM,
                            // we'll create a function type based on the AST type
                            if let AstType::Function { args, return_type } = &**inner {
                                let param_types: Result<Vec<BasicTypeEnum>, CompileError> = args
                                    .iter()
                                    .map(|ty| {
                                        let llvm_ty = self.to_llvm_type(ty)?;
                                        match llvm_ty {
                                            Type::Basic(b) => Ok(b),
                                            _ => Err(CompileError::InternalError("Function argument type must be a basic type".to_string(), None)),
                                        }
                                    })
                                    .collect();
                                let param_types = param_types?;
                                let param_metadata: Vec<BasicMetadataTypeEnum> = param_types.iter().map(|ty| (*ty).into()).collect();
                                let ret_type = self.to_llvm_type(return_type)?;
                                match ret_type {
                                    Type::Basic(b) => b.fn_type(&param_metadata, false),
                                    Type::Void => self.context.void_type().fn_type(&param_metadata, false),
                                    _ => return Err(CompileError::InternalError("Function return type must be a basic type or void".to_string(), None)),
                                }
                            } else {
                                return Err(CompileError::InternalError("Expected function type in pointer".to_string(), None));
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
} 