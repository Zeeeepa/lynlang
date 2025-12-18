use super::super::stdlib_codegen;
use super::super::LLVMCompiler;
use crate::ast;
use crate::error::CompileError;
use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum};
use inkwell::values::BasicValueEnum;

pub fn compile_function_call<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    name: &str,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // Check if this is Array.new() static method
    if name == "Array.new" {
        return compiler.compile_array_new(args);
    }

    // Check if this is a generic type constructor like HashMap<K,V>()
    if name.contains('<') && name.contains('>') {
        // Extract the base type name
        if let Some(angle_pos) = name.find('<') {
            let base_type = &name[..angle_pos];
            match base_type {
                "HashMap" => return stdlib_codegen::compile_hashmap_new(compiler, args),
                "HashSet" => return stdlib_codegen::compile_hashset_new(compiler, args),
                "DynVec" => return stdlib_codegen::compile_dynvec_new(compiler, args),
                _ => {
                    // Continue with regular function lookup
                }
            }
        }
    }

    // Check if this is HashMap.new() or HashSet.new()
    if name == "hashmap_new" {
        return stdlib_codegen::compile_hashmap_new(compiler, args);
    }
    if name == "hashset_new" {
        return stdlib_codegen::compile_hashset_new(compiler, args);
    }
    if name == "dynvec_new" {
        return stdlib_codegen::compile_dynvec_new(compiler, args);
    }

    // Check if this is a stdlib function call (e.g., io.print)
    if name.contains('.') {
        let parts: Vec<&str> = name.splitn(2, '.').collect();
        if parts.len() == 2 {
            let module = parts[0];
            let func = parts[1];

            // Handle stdlib function calls
            if module == "io" {
                match func {
                    "print" => return compiler.compile_io_print(args),
                    "println" => return compiler.compile_io_println(args),
                    "print_int" => return compiler.compile_io_print_int(args),
                    "print_float" => return compiler.compile_io_print_float(args),
                    "eprint" => return compiler.compile_io_eprint(args),
                    "eprintln" => return compiler.compile_io_eprintln(args),
                    "read_line" => return compiler.compile_io_read_line(args),
                    "read_input" => return compiler.compile_io_read_input(args),
                    _ => {}
                }
            } else if module == "math" {
                // Handle math module functions
                return compiler.compile_math_function(func, args);
            } else if module == "fs" {
                // Handle fs module functions
                match func {
                    "read_file" => return compiler.compile_fs_read_file(args),
                    "write_file" => return compiler.compile_fs_write_file(args),
                    "exists" => return compiler.compile_fs_exists(args),
                    "remove_file" => return compiler.compile_fs_remove_file(args),
                    "create_dir" => return compiler.compile_fs_create_dir(args),
                    _ => {}
                }
            } else if module == "core" {
                // Handle core module functions
                match func {
                    "assert" => return compiler.compile_core_assert(args),
                    "panic" => return compiler.compile_core_panic(args),
                    _ => {}
                }
            } else if module == "compiler" {
                let base_func = if let Some(angle_pos) = func.find('<') {
                    &func[..angle_pos]
                } else {
                    func
                };
                match base_func {
                    "inline_c" => return stdlib_codegen::compile_inline_c(compiler, args),
                    "raw_allocate" => return stdlib_codegen::compile_raw_allocate(compiler, args),
                    "raw_deallocate" => {
                        return stdlib_codegen::compile_raw_deallocate(compiler, args)
                    }
                    "raw_reallocate" => {
                        return stdlib_codegen::compile_raw_reallocate(compiler, args)
                    }
                    "raw_ptr_offset" => {
                        return stdlib_codegen::compile_raw_ptr_offset(compiler, args)
                    }
                    "raw_ptr_cast" => return stdlib_codegen::compile_raw_ptr_cast(compiler, args),
                    "call_external" => {
                        return stdlib_codegen::compile_call_external(compiler, args)
                    }
                    "load_library" => return stdlib_codegen::compile_load_library(compiler, args),
                    "get_symbol" => return stdlib_codegen::compile_get_symbol(compiler, args),
                    "unload_library" => {
                        return stdlib_codegen::compile_unload_library(compiler, args)
                    }
                    "null_ptr" | "nullptr" => {
                        return stdlib_codegen::compile_null_ptr(compiler, args)
                    }
                    "discriminant" => return stdlib_codegen::compile_discriminant(compiler, args),
                    "set_discriminant" => {
                        return stdlib_codegen::compile_set_discriminant(compiler, args)
                    }
                    "get_payload" => return stdlib_codegen::compile_get_payload(compiler, args),
                    "set_payload" => return stdlib_codegen::compile_set_payload(compiler, args),
                    "gep" => return stdlib_codegen::compile_gep(compiler, args),
                    "gep_struct" => return stdlib_codegen::compile_gep_struct(compiler, args),
                    "load" => {
                        // Extract type parameter from function name if present (e.g., compiler.load<u8>)
                        // For now, infer from context - type_arg will be None
                        return stdlib_codegen::compile_load(compiler, args, None);
                    }
                    "store" => {
                        // Extract type parameter from function name if present
                        return stdlib_codegen::compile_store(compiler, args, None);
                    }
                    "ptr_to_int" => return stdlib_codegen::compile_ptr_to_int(compiler, args),
                    "int_to_ptr" => return stdlib_codegen::compile_int_to_ptr(compiler, args),
                    _ => {}
                }
            } else if compiler.well_known.is_result(module) {
                // Handle Result enum constructors
                let payload = if !args.is_empty() {
                    Some(Box::new(args[0].clone()))
                } else {
                    None
                };
                return compiler.compile_enum_variant(module, func, &payload);
            } else if compiler.well_known.is_option(module) {
                // Handle Option enum constructors
                let payload = if !args.is_empty() {
                    Some(Box::new(args[0].clone()))
                } else {
                    None
                };
                return compiler.compile_enum_variant(module, func, &payload);
            }
        }
    }

    // Check if this is a direct call to math functions (available from @std)
    if name == "min" || name == "max" || name == "abs" {
        return compiler.compile_math_function(name, args);
    }

    // Check if this is the cast() builtin function
    if name == "cast" {
        return compile_cast_builtin(compiler, args);
    }

    // First check if this is a direct function call
    if let Some(function) = compiler.module.get_function(name) {
        // Direct function call
        let mut compiled_args = Vec::with_capacity(args.len());
        let param_types = function.get_type().get_param_types();

        for (i, arg) in args.iter().enumerate() {
            let mut val = compiler.compile_expression(arg)?;

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
                            val = compiler
                                .builder
                                .build_int_s_extend(int_val, expected_int_type, "extend")?
                                .into();
                        } else {
                            // Truncate
                            val = compiler
                                .builder
                                .build_int_truncate(int_val, expected_int_type, "trunc")?
                                .into();
                        }
                    }
                }
            }

            compiled_args.push(val);
        }
        let args_metadata: Vec<inkwell::values::BasicMetadataValueEnum> = compiled_args
            .iter()
            .map(|arg| {
                inkwell::values::BasicMetadataValueEnum::try_from(*arg).map_err(|_| {
                    CompileError::InternalError(
                        "Failed to convert argument to metadata".to_string(),
                        None,
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let call = compiler
            .builder
            .build_call(function, &args_metadata, "calltmp")?;

        // Update generic_type_context if this function returns Result<T,E> or Option<T>
        let generic_updates = if let Some(return_type) = compiler.function_types.get(name) {
            if let crate::ast::AstType::Generic {
                name: type_name,
                type_args,
            } = return_type
            {
                if compiler.well_known.is_result(type_name) && type_args.len() == 2 {
                    Some(vec![
                        ("Result_Ok_Type".to_string(), type_args[0].clone()),
                        ("Result_Err_Type".to_string(), type_args[1].clone()),
                    ])
                } else if compiler.well_known.is_option(type_name) && type_args.len() == 1 {
                    Some(vec![("Option_Some_Type".to_string(), type_args[0].clone())])
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if let Some(updates) = generic_updates {
            for (key, type_) in updates {
                compiler.track_generic_type(key, type_);
            }
        }

        // Check if the function returns void
        if function.get_type().get_return_type().is_none() {
            // Return a dummy value for void functions
            Ok(compiler.context.i32_type().const_zero().into())
        } else {
            Ok(call.try_as_basic_value().left().ok_or_else(|| {
                CompileError::InternalError(
                    "Function call did not return a value".to_string(),
                    None,
                )
            })?)
        }
    } else if let Ok((alloca, var_type)) = compiler.get_variable(name) {
        // Check if this is an imported math function
        if (name == "min" || name == "max" || name == "abs")
            && matches!(var_type, crate::ast::AstType::Function { .. })
        {
            return compiler.compile_math_function(name, args);
        }

        // Function pointer call - load the function pointer from variable
        let function_ptr = compiler
            .builder
            .build_load(alloca.get_type(), alloca, "func_ptr")?;

        // Get function type from the variable type
        let function_type = match &var_type {
            crate::ast::AstType::Function { args, return_type } => {
                let param_types_basic: Result<Vec<BasicTypeEnum>, CompileError> = args
                    .iter()
                    .map(|ty| {
                        let llvm_ty = compiler.to_llvm_type(ty)?;
                        match llvm_ty {
                            super::super::Type::Basic(b) => Ok(b),
                            super::super::Type::Struct(s) => Ok(s.into()),
                            _ => Err(CompileError::InternalError(
                                format!("Unsupported function argument type: {:?}", ty),
                                None,
                            )),
                        }
                    })
                    .collect();
                let param_types_basic = param_types_basic?;
                let param_metadata: Vec<BasicMetadataTypeEnum> =
                    param_types_basic.iter().map(|ty| (*ty).into()).collect();
                let ret_type = compiler.to_llvm_type(return_type)?;
                match ret_type {
                    super::super::Type::Basic(b) => match b {
                        BasicTypeEnum::IntType(t) => t.fn_type(&param_metadata, false),
                        BasicTypeEnum::FloatType(t) => t.fn_type(&param_metadata, false),
                        BasicTypeEnum::PointerType(t) => t.fn_type(&param_metadata, false),
                        BasicTypeEnum::StructType(t) => t.fn_type(&param_metadata, false),
                        BasicTypeEnum::ArrayType(t) => t.fn_type(&param_metadata, false),
                        BasicTypeEnum::VectorType(t) => t.fn_type(&param_metadata, false),
                        BasicTypeEnum::ScalableVectorType(t) => t.fn_type(&param_metadata, false),
                    },
                    super::super::Type::Struct(s) => s.fn_type(&param_metadata, false),
                    super::super::Type::Void => {
                        compiler.context.void_type().fn_type(&param_metadata, false)
                    }
                    _ => {
                        return Err(CompileError::InternalError(
                            "Function return type must be a basic type, struct or void".to_string(),
                            None,
                        ))
                    }
                }
            }
            crate::ast::AstType::FunctionPointer {
                param_types,
                return_type,
            } => {
                let param_types_basic: Result<Vec<BasicTypeEnum>, CompileError> = param_types
                    .iter()
                    .map(|ty| {
                        let llvm_ty = compiler.to_llvm_type(ty)?;
                        match llvm_ty {
                            super::super::Type::Basic(b) => Ok(b),
                            super::super::Type::Struct(s) => Ok(s.into()),
                            _ => Err(CompileError::InternalError(
                                format!("Unsupported function argument type: {:?}", ty),
                                None,
                            )),
                        }
                    })
                    .collect();
                let param_types_basic = param_types_basic?;
                let param_metadata: Vec<BasicMetadataTypeEnum> =
                    param_types_basic.iter().map(|ty| (*ty).into()).collect();
                let ret_type = compiler.to_llvm_type(return_type)?;
                match ret_type {
                    super::super::Type::Basic(b) => match b {
                        BasicTypeEnum::IntType(t) => t.fn_type(&param_metadata, false),
                        BasicTypeEnum::FloatType(t) => t.fn_type(&param_metadata, false),
                        BasicTypeEnum::PointerType(t) => t.fn_type(&param_metadata, false),
                        BasicTypeEnum::StructType(t) => t.fn_type(&param_metadata, false),
                        BasicTypeEnum::ArrayType(t) => t.fn_type(&param_metadata, false),
                        BasicTypeEnum::VectorType(t) => t.fn_type(&param_metadata, false),
                        BasicTypeEnum::ScalableVectorType(t) => t.fn_type(&param_metadata, false),
                    },
                    super::super::Type::Struct(s) => s.fn_type(&param_metadata, false),
                    super::super::Type::Void => {
                        compiler.context.void_type().fn_type(&param_metadata, false)
                    }
                    _ => {
                        return Err(CompileError::InternalError(
                            "Function return type must be a basic type, struct or void".to_string(),
                            None,
                        ))
                    }
                }
            }
            crate::ast::AstType::Ptr(inner)
                if matches!(**inner, crate::ast::AstType::FunctionPointer { .. }) =>
            {
                let inner_llvm_type = compiler.to_llvm_type(inner)?;
                match inner_llvm_type {
                    super::super::Type::Basic(inkwell::types::BasicTypeEnum::PointerType(
                        _ptr_type,
                    )) => {
                        // For function pointers, we need to get the function type
                        // Since we can't get it directly from the pointer type in newer LLVM,
                        // we'll create a function type based on the AST type
                        if let crate::ast::AstType::FunctionPointer {
                            param_types,
                            return_type,
                        } = &**inner
                        {
                            let param_types_basic: Result<Vec<BasicTypeEnum>, CompileError> =
                                param_types
                                    .iter()
                                    .map(|ty| {
                                        let llvm_ty = compiler.to_llvm_type(ty)?;
                                        match llvm_ty {
                                            super::super::Type::Basic(b) => Ok(b),
                                            super::super::Type::Struct(s) => Ok(s.into()),
                                            _ => Err(CompileError::InternalError(
                                                format!(
                                                    "Unsupported function argument type: {:?}",
                                                    ty
                                                ),
                                                None,
                                            )),
                                        }
                                    })
                                    .collect();
                            let param_types_basic = param_types_basic?;
                            let param_metadata: Vec<BasicMetadataTypeEnum> =
                                param_types_basic.iter().map(|ty| (*ty).into()).collect();
                            let ret_type = compiler.to_llvm_type(return_type)?;
                            match ret_type {
                                super::super::Type::Basic(b) => match b {
                                    BasicTypeEnum::IntType(t) => t.fn_type(&param_metadata, false),
                                    BasicTypeEnum::FloatType(t) => {
                                        t.fn_type(&param_metadata, false)
                                    }
                                    BasicTypeEnum::PointerType(t) => {
                                        t.fn_type(&param_metadata, false)
                                    }
                                    BasicTypeEnum::StructType(t) => {
                                        t.fn_type(&param_metadata, false)
                                    }
                                    BasicTypeEnum::ArrayType(t) => {
                                        t.fn_type(&param_metadata, false)
                                    }
                                    BasicTypeEnum::VectorType(t) => {
                                        t.fn_type(&param_metadata, false)
                                    }
                                    BasicTypeEnum::ScalableVectorType(t) => {
                                        t.fn_type(&param_metadata, false)
                                    }
                                },
                                super::super::Type::Struct(s) => s.fn_type(&param_metadata, false),
                                super::super::Type::Void => {
                                    compiler.context.void_type().fn_type(&param_metadata, false)
                                }
                                _ => {
                                    return Err(CompileError::InternalError(
                                        "Function return type must be a basic type, struct or void"
                                            .to_string(),
                                        None,
                                    ))
                                }
                            }
                        } else {
                            return Err(CompileError::InternalError(
                                "Expected function pointer type in pointer".to_string(),
                                None,
                            ));
                        }
                    }
                    _ => {
                        return Err(CompileError::TypeMismatch {
                            expected: "function pointer".to_string(),
                            found: format!("{:?}", inner_llvm_type),
                            span: None,
                        })
                    }
                }
            }
            _ => {
                return Err(CompileError::TypeMismatch {
                    expected: "function pointer".to_string(),
                    found: format!("{:?}", var_type),
                    span: None,
                })
            }
        };

        // Compile arguments
        let mut compiled_args = Vec::with_capacity(args.len());
        for arg in args {
            let val = compiler.compile_expression(arg)?;
            compiled_args.push(val);
        }
        let args_metadata: Vec<inkwell::values::BasicMetadataValueEnum> = compiled_args
            .iter()
            .map(|arg| {
                inkwell::values::BasicMetadataValueEnum::try_from(*arg).map_err(|_| {
                    CompileError::InternalError(
                        "Failed to convert argument to metadata".to_string(),
                        None,
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Cast the loaded pointer to the correct function type
        let casted_function_ptr = compiler.builder.build_pointer_cast(
            function_ptr.into_pointer_value(),
            compiler.context.ptr_type(inkwell::AddressSpace::default()),
            "casted_func_ptr",
        )?;

        // Make indirect call using build_indirect_call for function pointers
        let call = compiler.builder.build_indirect_call(
            function_type,
            casted_function_ptr,
            &args_metadata,
            "indirect_call",
        )?;

        // Update generic_type_context based on the return type of the function pointer
        match &var_type {
            crate::ast::AstType::Function { return_type, .. }
            | crate::ast::AstType::FunctionPointer { return_type, .. } => {
                if let crate::ast::AstType::Generic {
                    name: type_name,
                    type_args,
                } = return_type.as_ref()
                {
                    if compiler.well_known.is_result(type_name) && type_args.len() == 2 {
                        compiler
                            .track_generic_type("Result_Ok_Type".to_string(), type_args[0].clone());
                        compiler.track_generic_type(
                            "Result_Err_Type".to_string(),
                            type_args[1].clone(),
                        );
                    } else if compiler.well_known.is_option(type_name) && type_args.len() == 1 {
                        compiler.track_generic_type(
                            "Option_Some_Type".to_string(),
                            type_args[0].clone(),
                        );
                    }
                }
            }
            _ => {}
        }

        // Check if the function returns void
        if function_type.get_return_type().is_none() {
            // Return a dummy value for void functions
            Ok(compiler.context.i32_type().const_zero().into())
        } else {
            Ok(call.try_as_basic_value().left().ok_or_else(|| {
                CompileError::InternalError(
                    "Function call did not return a value".to_string(),
                    None,
                )
            })?)
        }
    } else {
        // Function not found
        Err(CompileError::UndeclaredFunction(
            name.to_string(),
            compiler.get_current_span(),
        ))
    }
}

/// Compile the cast(value, type) builtin function
/// Converts a value to a target type (integer/float conversions)
fn compile_cast_builtin<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 2 {
        return Err(CompileError::TypeError(
            format!(
                "cast() expects 2 arguments (value, type), got {}",
                args.len()
            ),
            None,
        ));
    }

    // Infer the source type BEFORE compiling (so we know signedness)
    let source_type =
        super::super::expressions::inference::infer_expression_type(compiler, &args[0])
            .unwrap_or(crate::ast::AstType::I32); // Default to signed i32 if inference fails

    // Compile the value to cast
    let value = compiler.compile_expression(&args[0])?;

    // The second argument should be a type identifier
    let target_type = match &args[1] {
        ast::Expression::Identifier(type_name) => match type_name.as_str() {
            "i8" => crate::ast::AstType::I8,
            "i16" => crate::ast::AstType::I16,
            "i32" => crate::ast::AstType::I32,
            "i64" => crate::ast::AstType::I64,
            "u8" => crate::ast::AstType::U8,
            "u16" => crate::ast::AstType::U16,
            "u32" => crate::ast::AstType::U32,
            "u64" => crate::ast::AstType::U64,
            "usize" => crate::ast::AstType::Usize,
            "f32" => crate::ast::AstType::F32,
            "f64" => crate::ast::AstType::F64,
            _ => {
                return Err(CompileError::TypeError(
                    format!(
                        "cast() target type '{}' is not a valid primitive type",
                        type_name
                    ),
                    None,
                ))
            }
        },
        _ => {
            return Err(CompileError::TypeError(
                "cast() second argument must be a type name (e.g., i32, f64)".to_string(),
                None,
            ))
        }
    };

    // Get the LLVM target type
    let llvm_target_type = compiler.to_llvm_type(&target_type)?;
    let target_basic_type = match llvm_target_type {
        super::super::Type::Basic(b) => b,
        _ => {
            return Err(CompileError::TypeError(
                "cast() target must be a basic type".to_string(),
                None,
            ))
        }
    };

    // Perform the cast based on source and target types
    match (value, target_basic_type) {
        // Int to Int
        (BasicValueEnum::IntValue(int_val), BasicTypeEnum::IntType(target_int)) => {
            let src_bits = int_val.get_type().get_bit_width();
            let dst_bits = target_int.get_bit_width();

            if src_bits == dst_bits {
                Ok(int_val.into())
            } else if src_bits < dst_bits {
                // Extend - use sign extend for signed source, zero extend for unsigned source
                if source_type.is_unsigned_integer() {
                    Ok(compiler
                        .builder
                        .build_int_z_extend(int_val, target_int, "cast_zext")?
                        .into())
                } else {
                    Ok(compiler
                        .builder
                        .build_int_s_extend(int_val, target_int, "cast_sext")?
                        .into())
                }
            } else {
                // Truncate
                Ok(compiler
                    .builder
                    .build_int_truncate(int_val, target_int, "cast_trunc")?
                    .into())
            }
        }
        // Float to Float
        (BasicValueEnum::FloatValue(float_val), BasicTypeEnum::FloatType(target_float)) => {
            let src_bits = if float_val.get_type() == compiler.context.f32_type() {
                32
            } else {
                64
            };
            let dst_bits = if target_float == compiler.context.f32_type() {
                32
            } else {
                64
            };

            if src_bits == dst_bits {
                Ok(float_val.into())
            } else if src_bits < dst_bits {
                Ok(compiler
                    .builder
                    .build_float_ext(float_val, target_float, "cast_fext")?
                    .into())
            } else {
                Ok(compiler
                    .builder
                    .build_float_trunc(float_val, target_float, "cast_ftrunc")?
                    .into())
            }
        }
        // Int to Float
        (BasicValueEnum::IntValue(int_val), BasicTypeEnum::FloatType(target_float)) => {
            // Use the inferred source type to determine signedness
            if source_type.is_unsigned_integer() {
                Ok(compiler
                    .builder
                    .build_unsigned_int_to_float(int_val, target_float, "cast_uitofp")?
                    .into())
            } else {
                Ok(compiler
                    .builder
                    .build_signed_int_to_float(int_val, target_float, "cast_sitofp")?
                    .into())
            }
        }
        // Float to Int
        (BasicValueEnum::FloatValue(float_val), BasicTypeEnum::IntType(target_int)) => {
            if target_type.is_unsigned_integer() {
                Ok(compiler
                    .builder
                    .build_float_to_unsigned_int(float_val, target_int, "cast_fptoui")?
                    .into())
            } else {
                Ok(compiler
                    .builder
                    .build_float_to_signed_int(float_val, target_int, "cast_fptosi")?
                    .into())
            }
        }
        _ => Err(CompileError::TypeError(
            format!(
                "Cannot cast {:?} to {:?}",
                value.get_type(),
                target_basic_type
            ),
            None,
        )),
    }
}
