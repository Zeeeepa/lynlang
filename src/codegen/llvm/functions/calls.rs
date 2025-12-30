use crate::ast::{self, AstType};
use crate::codegen::llvm::stdlib_codegen;
use crate::codegen::llvm::{LLVMCompiler, Type};
use crate::error::CompileError;
use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum, FunctionType};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum};
use inkwell::AddressSpace;

/// Build a function type from parameter types and return type
fn build_fn_type_from_params<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    param_types: &[AstType],
    return_type: &AstType,
) -> Result<FunctionType<'ctx>, CompileError> {
    let mut param_types_basic = Vec::with_capacity(param_types.len());
    for ty in param_types {
        let llvm_ty = compiler.to_llvm_type(ty)?;
        let basic = match llvm_ty {
            Type::Basic(b) => b,
            Type::Struct(s) => s.into(),
            _ => {
                return Err(CompileError::InternalError(
                    format!("Unsupported function argument type: {:?}", ty),
                    None,
                ))
            }
        };
        param_types_basic.push(basic);
    }

    let param_metadata: Vec<BasicMetadataTypeEnum> =
        param_types_basic.iter().map(|ty| (*ty).into()).collect();

    let ret_type = compiler.to_llvm_type(return_type)?;
    build_fn_type_from_ret(compiler, ret_type, &param_metadata)
}

/// Build a function type from an LLVM return type and parameter metadata
pub fn build_fn_type_from_ret<'ctx>(
    compiler: &LLVMCompiler<'ctx>,
    ret_type: Type<'ctx>,
    param_metadata: &[BasicMetadataTypeEnum<'ctx>],
) -> Result<FunctionType<'ctx>, CompileError> {
    match ret_type {
        Type::Basic(b) => Ok(match b {
            BasicTypeEnum::IntType(t) => t.fn_type(param_metadata, false),
            BasicTypeEnum::FloatType(t) => t.fn_type(param_metadata, false),
            BasicTypeEnum::PointerType(t) => t.fn_type(param_metadata, false),
            BasicTypeEnum::StructType(t) => t.fn_type(param_metadata, false),
            BasicTypeEnum::ArrayType(t) => t.fn_type(param_metadata, false),
            BasicTypeEnum::VectorType(t) => t.fn_type(param_metadata, false),
            BasicTypeEnum::ScalableVectorType(t) => t.fn_type(param_metadata, false),
        }),
        Type::Struct(s) => Ok(s.fn_type(param_metadata, false)),
        Type::Void => Ok(compiler.context.void_type().fn_type(param_metadata, false)),
        _ => Err(CompileError::InternalError(
            "Function return type must be a basic type, struct or void".to_string(),
            None,
        )),
    }
}

/// Compile arguments and convert types to match expected parameter types
fn compile_and_convert_args<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
    param_types: &[BasicMetadataTypeEnum<'ctx>],
) -> Result<Vec<BasicMetadataValueEnum<'ctx>>, CompileError> {
    let mut compiled_args = Vec::with_capacity(args.len());

    for (i, arg) in args.iter().enumerate() {
        let mut val = compiler.compile_expression(arg)?;

        if i < param_types.len() {
            let expected_type = param_types[i];

            // Convert string literal (pointer) to String struct if needed
            val = maybe_convert_ptr_to_string_struct(compiler, val, expected_type)?;

            // Cast integer arguments to match expected parameter type
            if let (true, BasicMetadataTypeEnum::IntType(expected_int_type)) =
                (val.is_int_value(), expected_type)
            {
                let int_val = val.into_int_value();
                let src_bits = int_val.get_type().get_bit_width();
                let dst_bits = expected_int_type.get_bit_width();

                if src_bits != dst_bits {
                    val = if src_bits < dst_bits {
                        compiler
                            .builder
                            .build_int_s_extend(int_val, expected_int_type, "extend")?
                            .into()
                    } else {
                        compiler
                            .builder
                            .build_int_truncate(int_val, expected_int_type, "trunc")?
                            .into()
                    };
                }
            }
        }

        compiled_args.push(val);
    }

    compiled_args
        .iter()
        .map(|arg| {
            BasicMetadataValueEnum::try_from(*arg).map_err(|_| {
                CompileError::InternalError(
                    "Failed to convert argument to metadata".to_string(),
                    None,
                )
            })
        })
        .collect()
}

/// Extract function type and return type reference from an AST type
fn get_function_type_from_ast<'a, 'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    var_type: &'a AstType,
) -> Result<(FunctionType<'ctx>, Option<&'a AstType>), CompileError> {
    match var_type {
        AstType::Function {
            args: param_types,
            return_type,
        } => {
            let fn_type = build_fn_type_from_params(compiler, param_types, return_type)?;
            Ok((fn_type, Some(return_type.as_ref())))
        }
        AstType::FunctionPointer {
            param_types,
            return_type,
        } => {
            let fn_type = build_fn_type_from_params(compiler, param_types, return_type)?;
            Ok((fn_type, Some(return_type.as_ref())))
        }
        t if t.is_ptr_type()
            && t.ptr_inner()
                .map(|inner| matches!(inner, AstType::FunctionPointer { .. }))
                .unwrap_or(false) =>
        {
            let inner = t.ptr_inner().unwrap();
            if let AstType::FunctionPointer {
                param_types,
                return_type,
            } = inner
            {
                let fn_type = build_fn_type_from_params(compiler, param_types, return_type)?;
                Ok((fn_type, Some(return_type.as_ref())))
            } else {
                Err(CompileError::InternalError(
                    "Expected function pointer type in pointer".to_string(),
                    None,
                ))
            }
        }
        _ => Err(CompileError::TypeMismatch {
            expected: "function pointer".to_string(),
            found: format!("{:?}", var_type),
            span: None,
        }),
    }
}

/// Track generic type context for Result<T, E> or Option<T> return types
fn track_generic_return_type(compiler: &mut LLVMCompiler, return_type: &AstType) {
    if let AstType::Generic {
        name: type_name,
        type_args,
    } = return_type
    {
        if compiler.well_known.is_result(type_name) && type_args.len() == 2 {
            compiler.track_generic_type("Result_Ok_Type".to_string(), type_args[0].clone());
            compiler.track_generic_type("Result_Err_Type".to_string(), type_args[1].clone());
        } else if compiler.well_known.is_option(type_name) && type_args.len() == 1 {
            compiler.track_generic_type("Option_Some_Type".to_string(), type_args[0].clone());
        }
    }
}

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
            } else if module == "compiler" || module == "builtin" || module == "@builtin" {
                // Both @std.compiler and @builtin route to the same intrinsics
                // @builtin is the raw intrinsic accessor (used only in compiler.zen)
                // @std.compiler will eventually route through compiler.zen
                // Handle "@builtin" with @ prefix from parsing
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
                    "dlerror" => return stdlib_codegen::compile_dlerror(compiler, args),
                    "null_ptr" | "nullptr" => {
                        return stdlib_codegen::compile_null_ptr(compiler, args)
                    }
                    "is_null" => return stdlib_codegen::compile_is_null(compiler, args),
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
                    "sizeof" => {
                        // Parse type argument from function name if present (e.g., sizeof<i32>)
                        let type_arg = if let Some(angle_pos) = func.find('<') {
                            let type_str = &func[angle_pos + 1..func.len() - 1];
                            crate::parser::parse_type_from_string(type_str).ok()
                        } else {
                            None
                        };
                        return stdlib_codegen::compile_sizeof(compiler, type_arg.as_ref());
                    }
                    "memset" => return stdlib_codegen::compile_memset(compiler, args),
                    "memcpy" => return stdlib_codegen::compile_memcpy(compiler, args),
                    "memmove" => return stdlib_codegen::compile_memmove(compiler, args),
                    "memcmp" => return stdlib_codegen::compile_memcmp(compiler, args),
                    "bswap16" => return stdlib_codegen::compile_bswap16(compiler, args),
                    "bswap32" => return stdlib_codegen::compile_bswap32(compiler, args),
                    "bswap64" => return stdlib_codegen::compile_bswap64(compiler, args),
                    "ctlz" => return stdlib_codegen::compile_ctlz(compiler, args),
                    "cttz" => return stdlib_codegen::compile_cttz(compiler, args),
                    "ctpop" => return stdlib_codegen::compile_ctpop(compiler, args),
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
        let param_types = function.get_type().get_param_types();
        let args_metadata = compile_and_convert_args(compiler, args, &param_types)?;

        let call = compiler
            .builder
            .build_call(function, &args_metadata, "calltmp")?;

        // Update generic_type_context if this function returns Result<T,E> or Option<T>
        if let Some(return_type) = compiler.function_types.get(name).cloned() {
            track_generic_return_type(compiler, &return_type);
        }

        // Check if the function returns void
        if function.get_type().get_return_type().is_none() {
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
            && matches!(var_type, AstType::Function { .. })
        {
            return compiler.compile_math_function(name, args);
        }

        // Function pointer call - load the function pointer from variable
        let function_ptr = compiler
            .builder
            .build_load(alloca.get_type(), alloca, "func_ptr")?;

        // Get function type and return type from the variable type
        let (function_type, return_type_ref) = get_function_type_from_ast(compiler, &var_type)?;

        // Compile arguments
        let param_types = function_type.get_param_types();
        let args_metadata = compile_and_convert_args(compiler, args, &param_types)?;

        // Cast the loaded pointer to the correct function type
        let casted_function_ptr = compiler.builder.build_pointer_cast(
            function_ptr.into_pointer_value(),
            compiler.context.ptr_type(AddressSpace::default()),
            "casted_func_ptr",
        )?;

        // Make indirect call
        let call = compiler.builder.build_indirect_call(
            function_type,
            casted_function_ptr,
            &args_metadata,
            "indirect_call",
        )?;

        // Update generic_type_context for Result/Option return types
        if let Some(ret_type) = return_type_ref {
            track_generic_return_type(compiler, ret_type);
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
        crate::codegen::llvm::expressions::inference::infer_expression_type(compiler, &args[0])
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
        crate::codegen::llvm::Type::Basic(b) => b,
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

/// Convert a pointer value (e.g., string literal) to a String struct if needed.
/// Returns the original value unchanged if conversion is not applicable.
///
/// This handles the case where a string literal is passed to a function expecting
/// a String struct. The String struct layout is: { ptr (data), i64 (len), i64 (capacity), ptr (allocator) }
fn maybe_convert_ptr_to_string_struct<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    val: BasicValueEnum<'ctx>,
    expected_type: BasicMetadataTypeEnum<'ctx>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // Only convert if value is a pointer
    if !val.is_pointer_value() {
        return Ok(val);
    }

    // Convert metadata type to basic type for struct check
    let basic_type: BasicTypeEnum = match expected_type {
        BasicMetadataTypeEnum::ArrayType(t) => t.into(),
        BasicMetadataTypeEnum::FloatType(t) => t.into(),
        BasicMetadataTypeEnum::IntType(t) => t.into(),
        BasicMetadataTypeEnum::PointerType(t) => t.into(),
        BasicMetadataTypeEnum::StructType(t) => t.into(),
        BasicMetadataTypeEnum::VectorType(t) => t.into(),
        BasicMetadataTypeEnum::ScalableVectorType(t) => t.into(),
        BasicMetadataTypeEnum::MetadataType(_) => return Ok(val),
    };

    if !basic_type.is_struct_type() {
        return Ok(val);
    }

    let struct_type = basic_type.into_struct_type();

    // Check if expected struct matches the registered String type
    let is_string_struct = if let Some(string_info) = compiler.struct_types.get("String") {
        string_info.llvm_type == struct_type
    } else {
        // Fallback: check struct layout matches String pattern (4 fields: ptr, i64, i64, ptr)
        if struct_type.count_fields() == 4 {
            let field_types = struct_type.get_field_types();
            field_types.len() == 4
                && field_types[0].is_pointer_type()
                && field_types[1].is_int_type()
                && field_types[2].is_int_type()
                && field_types[3].is_pointer_type()
        } else {
            false
        }
    };

    if !is_string_struct {
        return Ok(val);
    }

    // Convert pointer to String struct
    let ptr_val = val.into_pointer_value();

    // Get or declare strlen to compute the length
    let strlen_fn = compiler.module.get_function("strlen").unwrap_or_else(|| {
        let i64_type = compiler.context.i64_type();
        let ptr_type = compiler.context.ptr_type(AddressSpace::default());
        let fn_type = i64_type.fn_type(&[ptr_type.into()], false);
        compiler.module.add_function("strlen", fn_type, None)
    });

    // Call strlen to get the length
    let strlen_call = compiler.builder.build_call(
        strlen_fn,
        &[ptr_val.into()],
        "str_len"
    )?;
    let len_val = strlen_call.try_as_basic_value().left()
        .ok_or_else(|| CompileError::InternalError(
            "strlen should return a value".to_string(),
            None,
        ))?
        .into_int_value();

    // Build the String struct: { data, len, capacity, allocator }
    let null_ptr = compiler.context.ptr_type(AddressSpace::default()).const_null();

    // Allocate stack space for the struct and populate fields
    let struct_alloca = compiler.builder.build_alloca(struct_type, "string_struct")?;

    let data_ptr = compiler.builder.build_struct_gep(struct_type, struct_alloca, 0, "data_ptr")?;
    compiler.builder.build_store(data_ptr, ptr_val)?;

    let len_ptr = compiler.builder.build_struct_gep(struct_type, struct_alloca, 1, "len_ptr")?;
    compiler.builder.build_store(len_ptr, len_val)?;

    let cap_ptr = compiler.builder.build_struct_gep(struct_type, struct_alloca, 2, "cap_ptr")?;
    compiler.builder.build_store(cap_ptr, len_val)?;

    let alloc_ptr = compiler.builder.build_struct_gep(struct_type, struct_alloca, 3, "alloc_ptr")?;
    compiler.builder.build_store(alloc_ptr, null_ptr)?;

    // Load and return the complete struct
    Ok(compiler.builder.build_load(struct_type, struct_alloca, "string_val")?)
}
