use super::{LLVMCompiler, Type};
use crate::ast::AstType;
use crate::error::CompileError;
use inkwell::values::{BasicValueEnum};

impl<'ctx> LLVMCompiler<'ctx> {
    /// Compile Vec<T, N> method calls
    pub fn compile_vec_method(
        &mut self,
        obj_name: &str,
        method: &str,
        args: &[crate::ast::Expression],
        vec_ptr: inkwell::values::PointerValue<'ctx>,
        vec_value: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        match method {
            "push" => {
                // Vec.push(value) - adds element if not at capacity
                if args.len() != 1 {
                    return Err(CompileError::TypeError(
                        "push expects exactly 1 argument".to_string(),
                        None,
                    ));
                }

                let value = self.compile_expression(&args[0])?;

                // Vec is { [T; N], i64 } where array is first, len is second
                // Get len field pointer
                let len_ptr = self.builder.build_struct_gep(
                    vec_value.get_type(),
                    vec_ptr,
                    1,  // len is at index 1
                    "len_ptr",
                )?;
                
                let current_len = self
                    .builder
                    .build_load(self.context.i64_type(), len_ptr, "len")?
                    .into_int_value();

                // Get the capacity from the type
                let capacity = if let AstType::Vec { size, .. } = &self.variables.get(obj_name).unwrap().ast_type {
                    *size as u64
                } else {
                    return Err(CompileError::TypeError("Internal error: Vec type mismatch".to_string(), None));
                };

                // Check if we have room
                let at_capacity = self.builder.build_int_compare(
                    inkwell::IntPredicate::UGE,
                    current_len,
                    self.context.i64_type().const_int(capacity, false),
                    "at_capacity",
                )?;

                let current_fn = self.current_function.unwrap();
                let push_block = self.context.append_basic_block(current_fn, "vec_push");
                let full_block = self.context.append_basic_block(current_fn, "vec_full");
                let merge_block = self.context.append_basic_block(current_fn, "vec_push_merge");

                self.builder.build_conditional_branch(at_capacity, full_block, push_block)?;

                // Push block - add the element
                self.builder.position_at_end(push_block);
                
                // Get direct GEP to the array element
                // Vec struct is { [T; N], i64 }
                // We need indices: [0 (struct), 0 (array), current_len (element)]
                let indices = vec![
                    self.context.i32_type().const_int(0, false),  // Index into the struct pointer
                    self.context.i32_type().const_int(0, false),  // Index to the array field
                    current_len,  // Index to the element in the array
                ];
                
                let element_ptr = unsafe {
                    self.builder.build_in_bounds_gep(
                        vec_value.get_type(),
                        vec_ptr,
                        &indices,
                        "element_ptr",
                    )
                }?;

                // Store the value
                self.builder.build_store(element_ptr, value)?;

                // Increment length
                let new_len = self.builder.build_int_add(
                    current_len,
                    self.context.i64_type().const_int(1, false),
                    "new_len",
                )?;
                self.builder.build_store(len_ptr, new_len)?;
                self.builder.build_unconditional_branch(merge_block)?;

                // Full block - do nothing
                self.builder.position_at_end(full_block);
                self.builder.build_unconditional_branch(merge_block)?;

                // Merge block
                self.builder.position_at_end(merge_block);
                Ok(self.context.struct_type(&[], false).const_zero().into())
            }
            "get" => {
                // Vec.get(index) -> value at index
                if args.len() != 1 {
                    return Err(CompileError::TypeError(
                        "get expects exactly 1 argument".to_string(),
                        None,
                    ));
                }

                let index = self.compile_expression(&args[0])?;

                // Get the element type from the Vec type
                let element_ast_type = if let Some(var_info) = self.variables.get(obj_name) {
                    if let AstType::Vec { element_type, .. } = &var_info.ast_type {
                        element_type.as_ref().clone()
                    } else {
                        return Err(CompileError::TypeError("Internal error: Vec type mismatch".to_string(), None));
                    }
                } else {
                    return Err(CompileError::TypeError("Variable not found".to_string(), None));
                };
                
                let element_llvm_type = match self.to_llvm_type(&element_ast_type)? {
                    Type::Basic(basic_type) => basic_type,
                    _ => return Err(CompileError::TypeError("Vec element type must be basic type".to_string(), None)),
                };

                // Get direct GEP to the array element
                // Vec struct is { [T; N], i64 }
                let index_val = index.into_int_value();
                let indices = vec![
                    self.context.i32_type().const_int(0, false),  // Index into the struct pointer
                    self.context.i32_type().const_int(0, false),  // Index to the array field
                    index_val,  // Index to the element in the array
                ];
                
                let element_ptr = unsafe {
                    self.builder.build_in_bounds_gep(
                        vec_value.get_type(),
                        vec_ptr,
                        &indices,
                        "element_ptr",
                    )
                }?;

                // Load and return the value
                let value = self.builder.build_load(
                    element_llvm_type,
                    element_ptr,
                    "value",
                )?;
                
                Ok(value)
            }
            "len" => {
                // Vec.len() -> current length
                let len_ptr = self.builder.build_struct_gep(
                    vec_value.get_type(),
                    vec_ptr,
                    1,  // len is at index 1
                    "len_ptr",
                )?;
                
                let len = self.builder.build_load(
                    self.context.i64_type(),
                    len_ptr,
                    "len",
                )?;
                
                Ok(len)
            }
            "set" => {
                // Vec.set(index, value) -> void
                if args.len() != 2 {
                    return Err(CompileError::TypeError(
                        "set expects exactly 2 arguments".to_string(),
                        None,
                    ));
                }

                let index = self.compile_expression(&args[0])?;
                let value = self.compile_expression(&args[1])?;

                // Get direct GEP to the array element
                // Vec struct is { [T; N], i64 }
                let index_val = index.into_int_value();
                let indices = vec![
                    self.context.i32_type().const_int(0, false),  // Index into the struct pointer
                    self.context.i32_type().const_int(0, false),  // Index to the array field
                    index_val,  // Index to the element in the array
                ];
                
                let element_ptr = unsafe {
                    self.builder.build_in_bounds_gep(
                        vec_value.get_type(),
                        vec_ptr,
                        &indices,
                        "element_ptr",
                    )
                }?;

                // Store the value
                self.builder.build_store(element_ptr, value)?;
                
                Ok(self.context.struct_type(&[], false).const_zero().into())
            }
            "clear" => {
                // Vec.clear() -> void (resets len to 0)
                let len_ptr = self.builder.build_struct_gep(
                    vec_value.get_type(),
                    vec_ptr,
                    1,  // len is at index 1
                    "len_ptr",
                )?;
                
                self.builder.build_store(len_ptr, self.context.i64_type().const_int(0, false))?;
                
                Ok(self.context.struct_type(&[], false).const_zero().into())
            }
            "capacity" => {
                // Vec.capacity() -> fixed size from type
                let capacity = if let AstType::Vec { size, .. } = &self.variables.get(obj_name).unwrap().ast_type {
                    *size as u64
                } else {
                    return Err(CompileError::TypeError("Internal error: Vec type mismatch".to_string(), None));
                };
                
                Ok(self.context.i64_type().const_int(capacity, false).into())
            }
            _ => {
                Err(CompileError::TypeError(
                    format!("Unknown Vec method: {}", method),
                    None,
                ))
            }
        }
    }
}