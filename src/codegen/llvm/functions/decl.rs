use super::super::LLVMCompiler;
use crate::ast::{self, AstType};
use crate::error::CompileError;
use inkwell::module::Linkage;
use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum};
use inkwell::values::FunctionValue;
use super::super::Type;

pub fn declare_external_function<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    ext_func: &ast::ExternalFunction,
) -> Result<(), CompileError> {
    let ret_type = compiler.to_llvm_type(&ext_func.return_type)?;

    // First get the basic types for the parameters
    let param_basic_types: Result<Vec<BasicTypeEnum>, CompileError> = ext_func
        .args
        .iter()
        .map(|t| compiler.to_llvm_type(t).and_then(|t| t.into_basic_type()))
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
        Type::Basic(b) => match b {
            BasicTypeEnum::ArrayType(t) => t.fn_type(&param_metadata, ext_func.is_varargs),
            BasicTypeEnum::FloatType(t) => t.fn_type(&param_metadata, ext_func.is_varargs),
            BasicTypeEnum::IntType(t) => t.fn_type(&param_metadata, ext_func.is_varargs),
            BasicTypeEnum::PointerType(t) => t.fn_type(&param_metadata, ext_func.is_varargs),
            BasicTypeEnum::StructType(t) => t.fn_type(&param_metadata, ext_func.is_varargs),
            BasicTypeEnum::VectorType(t) => t.fn_type(&param_metadata, ext_func.is_varargs),
            BasicTypeEnum::ScalableVectorType(t) => {
                t.fn_type(&param_metadata, ext_func.is_varargs)
            }
        },
        Type::Function(f) => f,
        Type::Void => compiler
            .context
            .void_type()
            .fn_type(&param_metadata, ext_func.is_varargs),
        Type::Pointer(_) => {
            return Err(CompileError::UnsupportedFeature(
                "Cannot use pointer type as function return type".to_string(),
                None,
            ));
        }
        Type::Struct(st) => st.fn_type(&param_metadata, false),
    };

    // Only declare if not already declared
    if compiler.module.get_function(&ext_func.name).is_none() {
        compiler.module
            .add_function(&ext_func.name, function_type, None);
    }
    Ok(())
}

pub fn declare_function<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    function: &ast::Function,
) -> Result<FunctionValue<'ctx>, CompileError> {
    // Special case for main function - C runtime expects int return
    let actual_return_type = if function.name == "main" {
        match &function.return_type {
            AstType::Void => AstType::I32,  // Convert void to i32
            AstType::Generic { name, .. } if name == "Result" => {
                // Main returning Result<T,E> is not supported in JIT mode
                eprintln!("Warning: main() returning Result<T,E> is not fully supported in JIT mode");
                eprintln!("The function will compile but may crash when executed.");
                eprintln!("Consider using 'void' or 'i32' as the return type for main()");
                // For now, keep the Result type and hope for the best
                function.return_type.clone()
            }
            _ => function.return_type.clone()
        }
    } else {
        function.return_type.clone()
    };

    // First, get the return type
    let return_type = compiler.to_llvm_type(&actual_return_type)?;

    // Get parameter basic types with their names
    let param_basic_types: Result<Vec<BasicTypeEnum>, CompileError> = function
        .args
        .iter()
        .map(|(_name, t)| {
            compiler.to_llvm_type(t)
                .and_then(|zen_type| compiler.expect_basic_type(zen_type))
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
        Type::Basic(b) => match b {
            BasicTypeEnum::ArrayType(t) => t.fn_type(&param_metadata, false),
            BasicTypeEnum::FloatType(t) => t.fn_type(&param_metadata, false),
            BasicTypeEnum::IntType(t) => t.fn_type(&param_metadata, false),
            BasicTypeEnum::PointerType(t) => t.fn_type(&param_metadata, false),
            BasicTypeEnum::StructType(t) => t.fn_type(&param_metadata, false),
            BasicTypeEnum::VectorType(t) => t.fn_type(&param_metadata, false),
            BasicTypeEnum::ScalableVectorType(t) => t.fn_type(&param_metadata, false),
        },
        Type::Function(f) => f,
        Type::Void => compiler.context.void_type().fn_type(&param_metadata, false),
        Type::Pointer(_) => {
            return Err(CompileError::UnsupportedFeature(
                "Cannot use pointer type as function return type".to_string(),
                None,
            ));
        }
        Type::Struct(st) => st.fn_type(&param_metadata, false),
    };

    // Check if function already declared
    if let Some(func) = compiler.module.get_function(&function.name) {
        return Ok(func);
    }

    // Declare the function (this creates a declaration)
    let function_value = compiler
        .module
        .add_function(&function.name, function_type, None);

    // Set the function linkage to external so it can be linked
    function_value.set_linkage(Linkage::External);

    // Store the function for later use
    compiler.functions.insert(function.name.clone(), function_value);
    // Store the return type for type inference (use actual_return_type which handles main() special case)
    compiler.function_types
        .insert(function.name.clone(), actual_return_type);

    Ok(function_value)
}

pub fn compile_function_body<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    function: &ast::Function,
) -> Result<(), CompileError> {
    // Get the already-declared function
    let function_value = compiler.module.get_function(&function.name).ok_or_else(|| {
        CompileError::InternalError(format!("Function {} not declared", function.name), None)
    })?;

    // Set names for all arguments
    for (i, (arg_name, _)) in function.args.iter().enumerate() {
        if let Some(param) = function_value.get_nth_param(i as u32) {
            param.set_name(arg_name);
        }
    }

    // Add to symbol table
    compiler.symbols.insert(
        function.name.clone(),
        super::super::symbols::Symbol::Function(function_value),
    );

    // Now compile the function body
    let entry_block = compiler.context.append_basic_block(function_value, "entry");
    compiler.builder.position_at_end(entry_block);
    compiler.current_function = Some(function_value);

    // Clear variables from previous function by entering a new scope
    compiler.symbols.enter_scope();
    compiler.variables.clear(); // Clear variables from previous function

    // Extract generic type information from the function's return type
    if let AstType::Generic { name, type_args } = &function.return_type {
        if name == "Result" && type_args.len() == 2 {
            compiler.track_generic_type("Result_Ok_Type".to_string(), type_args[0].clone());
            compiler.track_generic_type("Result_Err_Type".to_string(), type_args[1].clone());
            compiler.track_complex_generic(&function.return_type, "Result");
        } else if name == "Option" && type_args.len() == 1 {
            compiler.track_generic_type("Option_Some_Type".to_string(), type_args[0].clone());
            compiler.track_complex_generic(&function.return_type, "Option");
        }
    }

    // Create variables for module imports
    for (name, marker) in compiler.module_imports.clone() {
        let alloca = compiler
            .builder
            .build_alloca(compiler.context.i64_type(), &name)
            .map_err(|e| CompileError::from(e))?;

        compiler.builder
            .build_store(alloca, compiler.context.i64_type().const_int(marker, false))
            .map_err(|e| CompileError::from(e))?;

        compiler.variables.insert(
            name.clone(),
            super::super::VariableInfo {
                pointer: alloca,
                ast_type: AstType::StdModule,
                is_mutable: false,
                is_initialized: true,
            },
        );
    }

    // Store function parameters in variables
    for (i, (name, type_)) in function.args.iter().enumerate() {
        let param = function_value.get_nth_param(i as u32).unwrap();
        let llvm_type = compiler.to_llvm_type(type_)?;
        let basic_type = compiler.expect_basic_type(llvm_type)?;
        let alloca = compiler.builder.build_alloca(basic_type, name)?;
        compiler.builder.build_store(alloca, param)?;
        compiler.variables.insert(
            name.clone(),
            super::super::VariableInfo {
                pointer: alloca,
                ast_type: type_.clone(),
                is_mutable: false,
                is_initialized: true,
            },
        );
    }

    // Compile all statements
    let stmt_count = function.body.len();
    for (i, statement) in function.body.iter().enumerate() {
        if i == stmt_count - 1 {
            if let ast::Statement::Expression(expr) = statement {
                if !matches!(function.return_type, AstType::Void) {
                    let value = compiler.compile_expression(expr)?;
                    compiler.builder.build_return(Some(&value))?;
                    return Ok(());
                }
            }
        }
        compiler.compile_statement(statement)?;
    }

    // Handle return for void functions
    if matches!(function.return_type, AstType::Void) {
        if let Some(current_block) = compiler.builder.get_insert_block() {
            if current_block.get_terminator().is_none() {
                compiler.builder.build_return(None)?;
            }
        }
    } else {
        // Non-void function must have explicit return
        if let Some(current_block) = compiler.builder.get_insert_block() {
            if current_block.get_terminator().is_none() {
                return Err(CompileError::TypeError(
                    format!("Function '{}' must return a value", function.name),
                    None,
                ));
            }
        }
    }

    // Clean up
    compiler.symbols.exit_scope();
    compiler.variables.clear();
    compiler.generic_type_context.clear();
    compiler.generic_tracker = super::super::generics::GenericTypeTracker::new();
    compiler.current_function = None;
    Ok(())
}

pub fn define_and_compile_function<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    function: &ast::Function,
) -> Result<FunctionValue<'ctx>, CompileError> {
    let function_value = declare_function(compiler, function)?;
    compile_function_body(compiler, function)?;
    Ok(function_value)
}
