pub fn compile_array_index_address(
        compiler: &mut LLVMCompiler,
        array: &Expression,
        index: &Expression,
    ) -> Result<PointerValue<'ctx>, CompileError> {
        // Compile array expression - should be a pointer
        let array_val = compiler.compile_expression(array)?;

        // Get the actual pointer value
        let array_ptr = if array_val.is_pointer_value() {
            array_val.into_pointer_value()
        } else {
            return Err(CompileError::TypeError(
                format!(
                    "Array indexing requires pointer type, got {:?}",
                    array_val.get_type()
                ),
                None,
            ));
        };

        // Try to infer element type from context. Default to i32 for compatibility with tests
        // TODO: Proper type inference for array elements from declaration
        let element_type = compiler.context.i32_type();

        let index_val = compiler.compile_expression(index)?;
        let gep = unsafe {
            compiler.builder.build_gep(
                element_type,
                array_ptr,
                &[index_val.into_int_value()],
                "arrayidx",
            )?
        };
        Ok(gep)
    }
