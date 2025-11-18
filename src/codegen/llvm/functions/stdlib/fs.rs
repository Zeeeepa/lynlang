//! File system module codegen - read_file, write_file, etc.

use super::super::super::LLVMCompiler;
use super::helpers;
use crate::ast;
use crate::error::CompileError;
use inkwell::values::BasicValueEnum;

/// Compile fs.read_file function call
pub fn compile_fs_read_file<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 1 {
        return Err(CompileError::TypeError(
            format!("fs.read_file expects 1 argument, got {}", args.len()),
            None,
        ));
    }

    // Get or declare fopen
    let fopen_fn = compiler.module.get_function("fopen").unwrap_or_else(|| {
        let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
        compiler.module.add_function("fopen", fn_type, None)
    });

    // Get or declare fclose
    let fclose_fn = compiler.module.get_function("fclose").unwrap_or_else(|| {
        let i32_type = compiler.context.i32_type();
        let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
        let fn_type = i32_type.fn_type(&[ptr_type.into()], false);
        compiler.module.add_function("fclose", fn_type, None)
    });

    // Get or declare fseek
    let fseek_fn = compiler.module.get_function("fseek").unwrap_or_else(|| {
        let i32_type = compiler.context.i32_type();
        let i64_type = compiler.context.i64_type();
        let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
        let fn_type =
            i32_type.fn_type(&[ptr_type.into(), i64_type.into(), i32_type.into()], false);
        compiler.module.add_function("fseek", fn_type, None)
    });

    // Get or declare ftell
    let ftell_fn = compiler.module.get_function("ftell").unwrap_or_else(|| {
        let i64_type = compiler.context.i64_type();
        let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
        let fn_type = i64_type.fn_type(&[ptr_type.into()], false);
        compiler.module.add_function("ftell", fn_type, None)
    });

    // Get or declare fread
    let fread_fn = compiler.module.get_function("fread").unwrap_or_else(|| {
        let i64_type = compiler.context.i64_type();
        let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
        let fn_type = i64_type.fn_type(
            &[
                ptr_type.into(),
                i64_type.into(),
                i64_type.into(),
                ptr_type.into(),
            ],
            false,
        );
        compiler.module.add_function("fread", fn_type, None)
    });

    // Compile the path argument
    let path_value = compiler.compile_expression(&args[0])?;
    let path_ptr = path_value.into_pointer_value();

    // Create mode string "r"
    let mode_str = compiler.builder.build_global_string_ptr("r", "read_mode")?;

    // Call fopen
    let file_ptr = compiler
        .builder
        .build_call(
            fopen_fn,
            &[path_ptr.into(), mode_str.as_pointer_value().into()],
            "fopen_call",
        )?
        .try_as_basic_value()
        .left()
        .unwrap();

    // Check if file opened successfully
    let is_null = compiler
        .builder
        .build_is_null(file_ptr.into_pointer_value(), "is_null")?;
    let current_fn = compiler.current_function.unwrap();
    let success_block = compiler.context.append_basic_block(current_fn, "file_opened");
    let error_block = compiler.context.append_basic_block(current_fn, "file_error");
    let merge_block = compiler.context.append_basic_block(current_fn, "merge");

    compiler.builder
        .build_conditional_branch(is_null, error_block, success_block)?;

    // Success block: read file
    compiler.builder.position_at_end(success_block);

    // Seek to end to get file size
    let seek_end = compiler.context.i32_type().const_int(2, false); // SEEK_END
    compiler.builder.build_call(
        fseek_fn,
        &[
            file_ptr.into(),
            compiler.context.i64_type().const_zero().into(),
            seek_end.into(),
        ],
        "fseek_end",
    )?;

    // Get file size
    let file_size = compiler
        .builder
        .build_call(ftell_fn, &[file_ptr.into()], "ftell_call")?
        .try_as_basic_value()
        .left()
        .unwrap();

    // Seek back to beginning
    let seek_set = compiler.context.i32_type().const_int(0, false); // SEEK_SET
    compiler.builder.build_call(
        fseek_fn,
        &[
            file_ptr.into(),
            compiler.context.i64_type().const_zero().into(),
            seek_set.into(),
        ],
        "fseek_start",
    )?;

    // Allocate buffer for file contents
    let malloc_fn = compiler.module.get_function("malloc").unwrap();
    let buffer = compiler
        .builder
        .build_call(malloc_fn, &[file_size.into()], "malloc_buffer")?
        .try_as_basic_value()
        .left()
        .unwrap();

    // Read file contents
    compiler.builder.build_call(
        fread_fn,
        &[
            buffer.into(),
            compiler.context.i64_type().const_int(1, false).into(),
            file_size.into(),
            file_ptr.into(),
        ],
        "fread_call",
    )?;

    // Close file
    compiler.builder
        .build_call(fclose_fn, &[file_ptr.into()], "fclose_call")?;

    // Create Result.Ok with the buffer
    let result_ok = helpers::create_result_ok(compiler, buffer)?;
    compiler.builder.build_unconditional_branch(merge_block)?;
    let success_value = result_ok;

    // Error block: return Result.Err
    compiler.builder.position_at_end(error_block);
    let error_msg = compiler
        .builder
        .build_global_string_ptr("Failed to open file", "file_error_msg")?;
    let result_err = helpers::create_result_err(compiler, error_msg.as_pointer_value().into())?;
    compiler.builder.build_unconditional_branch(merge_block)?;
    let error_value = result_err;

    // Merge block
    compiler.builder.position_at_end(merge_block);
    let phi = compiler
        .builder
        .build_phi(success_value.get_type(), "result_phi")?;
    phi.add_incoming(&[(&success_value, success_block), (&error_value, error_block)]);

    Ok(phi.as_basic_value())
}

/// Compile fs.write_file function call
pub fn compile_fs_write_file<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 2 {
        return Err(CompileError::TypeError(
            format!("fs.write_file expects 2 arguments, got {}", args.len()),
            None,
        ));
    }

    // Get or declare fopen
    let fopen_fn = compiler.module.get_function("fopen").unwrap_or_else(|| {
        let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
        compiler.module.add_function("fopen", fn_type, None)
    });

    // Get or declare fwrite
    let fwrite_fn = compiler.module.get_function("fwrite").unwrap_or_else(|| {
        let i64_type = compiler.context.i64_type();
        let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
        let fn_type = i64_type.fn_type(
            &[
                ptr_type.into(),
                i64_type.into(),
                i64_type.into(),
                ptr_type.into(),
            ],
            false,
        );
        compiler.module.add_function("fwrite", fn_type, None)
    });

    // Get or declare fclose
    let fclose_fn = compiler.module.get_function("fclose").unwrap_or_else(|| {
        let i32_type = compiler.context.i32_type();
        let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
        let fn_type = i32_type.fn_type(&[ptr_type.into()], false);
        compiler.module.add_function("fclose", fn_type, None)
    });

    // Get or declare strlen
    let strlen_fn = compiler.module.get_function("strlen").unwrap_or_else(|| {
        let i64_type = compiler.context.i64_type();
        let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
        let fn_type = i64_type.fn_type(&[ptr_type.into()], false);
        compiler.module.add_function("strlen", fn_type, None)
    });

    // Compile arguments
    let path_value = compiler.compile_expression(&args[0])?;
    let path_ptr = path_value.into_pointer_value();
    let content_value = compiler.compile_expression(&args[1])?;
    let content_ptr = content_value.into_pointer_value();

    // Create mode string "w"
    let mode_str = compiler.builder.build_global_string_ptr("w", "write_mode")?;

    // Call fopen
    let file_ptr = compiler
        .builder
        .build_call(
            fopen_fn,
            &[path_ptr.into(), mode_str.as_pointer_value().into()],
            "fopen_call",
        )?
        .try_as_basic_value()
        .left()
        .unwrap();

    // Check if file opened successfully
    let is_null = compiler
        .builder
        .build_is_null(file_ptr.into_pointer_value(), "is_null")?;
    let current_fn = compiler.current_function.unwrap();
    let success_block = compiler.context.append_basic_block(current_fn, "file_opened");
    let error_block = compiler.context.append_basic_block(current_fn, "file_error");
    let merge_block = compiler.context.append_basic_block(current_fn, "merge");

    compiler.builder
        .build_conditional_branch(is_null, error_block, success_block)?;

    // Success block: write file
    compiler.builder.position_at_end(success_block);

    // Get content length
    let content_len = compiler
        .builder
        .build_call(strlen_fn, &[content_ptr.into()], "strlen_call")?
        .try_as_basic_value()
        .left()
        .unwrap();

    // Write content
    compiler.builder.build_call(
        fwrite_fn,
        &[
            content_ptr.into(),
            compiler.context.i64_type().const_int(1, false).into(),
            content_len.into(),
            file_ptr.into(),
        ],
        "fwrite_call",
    )?;

    // Close file
    compiler.builder
        .build_call(fclose_fn, &[file_ptr.into()], "fclose_call")?;

    // Create Result.Ok(void)
    let result_ok = helpers::create_result_ok_void(compiler)?;
    compiler.builder.build_unconditional_branch(merge_block)?;
    let success_value = result_ok;

    // Error block: return Result.Err
    compiler.builder.position_at_end(error_block);
    let error_msg = compiler
        .builder
        .build_global_string_ptr("Failed to write file", "write_error_msg")?;
    let result_err = helpers::create_result_err(compiler, error_msg.as_pointer_value().into())?;
    compiler.builder.build_unconditional_branch(merge_block)?;
    let error_value = result_err;

    // Merge block
    compiler.builder.position_at_end(merge_block);
    let phi = compiler
        .builder
        .build_phi(success_value.get_type(), "result_phi")?;
    phi.add_incoming(&[(&success_value, success_block), (&error_value, error_block)]);

    Ok(phi.as_basic_value())
}

/// Compile fs.exists function call
pub fn compile_fs_exists<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 1 {
        return Err(CompileError::TypeError(
            format!("fs.exists expects 1 argument, got {}", args.len()),
            None,
        ));
    }

    // Get or declare access
    let access_fn = compiler.module.get_function("access").unwrap_or_else(|| {
        let i32_type = compiler.context.i32_type();
        let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
        let fn_type = i32_type.fn_type(&[ptr_type.into(), i32_type.into()], false);
        compiler.module.add_function("access", fn_type, None)
    });

    // Compile the path argument
    let path_value = compiler.compile_expression(&args[0])?;
    let path_ptr = path_value.into_pointer_value();

    // F_OK = 0 (check for existence)
    let f_ok = compiler.context.i32_type().const_int(0, false);

    // Call access
    let result = compiler
        .builder
        .build_call(access_fn, &[path_ptr.into(), f_ok.into()], "access_call")?
        .try_as_basic_value()
        .left()
        .unwrap();

    // Compare result with 0 (success)
    let zero = compiler.context.i32_type().const_int(0, false);
    let exists = compiler.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        result.into_int_value(),
        zero,
        "exists",
    )?;

    Ok(exists.into())
}

/// Compile fs.remove_file function call
pub fn compile_fs_remove_file<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 1 {
        return Err(CompileError::TypeError(
            format!("fs.remove_file expects 1 argument, got {}", args.len()),
            None,
        ));
    }

    // Get or declare unlink
    let unlink_fn = compiler.module.get_function("unlink").unwrap_or_else(|| {
        let i32_type = compiler.context.i32_type();
        let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
        let fn_type = i32_type.fn_type(&[ptr_type.into()], false);
        compiler.module.add_function("unlink", fn_type, None)
    });

    // Compile the path argument
    let path_value = compiler.compile_expression(&args[0])?;
    let path_ptr = path_value.into_pointer_value();

    // Call unlink
    let result = compiler
        .builder
        .build_call(unlink_fn, &[path_ptr.into()], "unlink_call")?
        .try_as_basic_value()
        .left()
        .unwrap();

    // Check if successful (result == 0)
    let zero = compiler.context.i32_type().const_int(0, false);
    let is_success = compiler.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        result.into_int_value(),
        zero,
        "is_success",
    )?;

    let current_fn = compiler.current_function.unwrap();
    let success_block = compiler
        .context
        .append_basic_block(current_fn, "remove_success");
    let error_block = compiler.context.append_basic_block(current_fn, "remove_error");
    let merge_block = compiler.context.append_basic_block(current_fn, "merge");

    compiler.builder
        .build_conditional_branch(is_success, success_block, error_block)?;

    // Success block
    compiler.builder.position_at_end(success_block);
    let result_ok = helpers::create_result_ok_void(compiler)?;
    compiler.builder.build_unconditional_branch(merge_block)?;
    let success_value = result_ok;

    // Error block
    compiler.builder.position_at_end(error_block);
    let error_msg = compiler
        .builder
        .build_global_string_ptr("Failed to remove file", "remove_error_msg")?;
    let result_err = helpers::create_result_err(compiler, error_msg.as_pointer_value().into())?;
    compiler.builder.build_unconditional_branch(merge_block)?;
    let error_value = result_err;

    // Merge block
    compiler.builder.position_at_end(merge_block);
    let phi = compiler
        .builder
        .build_phi(success_value.get_type(), "result_phi")?;
    phi.add_incoming(&[(&success_value, success_block), (&error_value, error_block)]);

    Ok(phi.as_basic_value())
}

/// Compile fs.create_dir function call
pub fn compile_fs_create_dir<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 1 {
        return Err(CompileError::TypeError(
            format!("fs.create_dir expects 1 argument, got {}", args.len()),
            None,
        ));
    }

    // Get or declare mkdir
    let mkdir_fn = compiler.module.get_function("mkdir").unwrap_or_else(|| {
        let i32_type = compiler.context.i32_type();
        let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
        let fn_type = i32_type.fn_type(&[ptr_type.into(), i32_type.into()], false);
        compiler.module.add_function("mkdir", fn_type, None)
    });

    // Compile the path argument
    let path_value = compiler.compile_expression(&args[0])?;
    let path_ptr = path_value.into_pointer_value();

    // Mode = 0755 (rwxr-xr-x)
    let mode = compiler.context.i32_type().const_int(0o755, false);

    // Call mkdir
    let result = compiler
        .builder
        .build_call(mkdir_fn, &[path_ptr.into(), mode.into()], "mkdir_call")?
        .try_as_basic_value()
        .left()
        .unwrap();

    // Check if successful (result == 0)
    let zero = compiler.context.i32_type().const_int(0, false);
    let is_success = compiler.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        result.into_int_value(),
        zero,
        "is_success",
    )?;

    let current_fn = compiler.current_function.unwrap();
    let success_block = compiler.context.append_basic_block(current_fn, "mkdir_success");
    let error_block = compiler.context.append_basic_block(current_fn, "mkdir_error");
    let merge_block = compiler.context.append_basic_block(current_fn, "merge");

    compiler.builder
        .build_conditional_branch(is_success, success_block, error_block)?;

    // Success block
    compiler.builder.position_at_end(success_block);
    let result_ok = helpers::create_result_ok_void(compiler)?;
    compiler.builder.build_unconditional_branch(merge_block)?;
    let success_value = result_ok;

    // Error block
    compiler.builder.position_at_end(error_block);
    let error_msg = compiler
        .builder
        .build_global_string_ptr("Failed to create directory", "mkdir_error_msg")?;
    let result_err = helpers::create_result_err(compiler, error_msg.as_pointer_value().into())?;
    compiler.builder.build_unconditional_branch(merge_block)?;
    let error_value = result_err;

    // Merge block
    compiler.builder.position_at_end(merge_block);
    let phi = compiler
        .builder
        .build_phi(success_value.get_type(), "result_phi")?;
    phi.add_incoming(&[(&success_value, success_block), (&error_value, error_block)]);

    Ok(phi.as_basic_value())
}
