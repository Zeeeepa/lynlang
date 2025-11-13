pub mod arrays;
pub mod runtime;
pub mod decl;
pub mod calls;
pub mod stdlib;

use super::LLVMCompiler;
use crate::ast;
use crate::error::CompileError;
use inkwell::{values::{BasicValueEnum, FunctionValue}};

impl<'ctx> LLVMCompiler<'ctx> {
    /// Helper to check if an expression is an allocator type
    fn is_allocator_type(&self, _expr: &ast::Expression) -> bool {
        arrays::is_allocator_type(self, _expr)
    }
    
    // Array methods
    pub fn compile_array_new(
        &mut self,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        arrays::compile_array_new(self, args)
    }
    
    // Note: These array methods are typically called from behaviors.rs with compiled values.
    // These wrappers are kept for API compatibility but may not be used directly.
    pub fn compile_array_push_by_ptr(
        &mut self,
        _args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // This should be called with compiled values, not expressions
        Err(CompileError::InternalError(
            "compile_array_push_by_ptr should be called with PointerValue and BasicValueEnum".to_string(),
            None,
        ))
    }
    
    pub fn compile_array_push(
        &mut self,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if args.len() < 1 {
            return Err(CompileError::TypeError(
                "Array.push expects at least 1 argument (array, value)".to_string(),
                None,
            ));
        }
        let array_val = self.compile_expression(&args[0])?;
        let value = self.compile_expression(&args[1])?;
        arrays::compile_array_push(self, array_val, value)
    }
    
    pub fn compile_array_get(
        &mut self,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if args.len() < 2 {
            return Err(CompileError::TypeError(
                "Array.get expects 2 arguments (array, index)".to_string(),
                None,
            ));
        }
        let array_val = self.compile_expression(&args[0])?;
        let index_val = self.compile_expression(&args[1])?;
        arrays::compile_array_get(self, array_val, index_val)
    }
    
    pub fn compile_array_len(
        &mut self,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if args.is_empty() {
            return Err(CompileError::TypeError(
                "Array.len expects 1 argument (array)".to_string(),
                None,
            ));
        }
        let array_val = self.compile_expression(&args[0])?;
        arrays::compile_array_len(self, array_val)
    }
    
    pub fn compile_array_set(
        &mut self,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if args.len() < 3 {
            return Err(CompileError::TypeError(
                "Array.set expects 3 arguments (array, index, value)".to_string(),
                None,
            ));
        }
        let array_val = self.compile_expression(&args[0])?;
        let index_val = self.compile_expression(&args[1])?;
        let value = self.compile_expression(&args[2])?;
        arrays::compile_array_set(self, array_val, index_val, value)
    }
    
    pub fn compile_array_pop_by_ptr(
        &mut self,
        _args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // This should be called with compiled values, not expressions
        Err(CompileError::InternalError(
            "compile_array_pop_by_ptr should be called with PointerValue".to_string(),
            None,
        ))
    }
    
    pub fn compile_array_pop(
        &mut self,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if args.is_empty() {
            return Err(CompileError::TypeError(
                "Array.pop expects 1 argument (array)".to_string(),
                None,
            ));
        }
        let array_val = self.compile_expression(&args[0])?;
        arrays::compile_array_pop(self, array_val)
    }
    
    // Runtime functions
    pub fn get_or_create_runtime_function(
        &mut self,
        name: &str,
    ) -> Result<FunctionValue<'ctx>, CompileError> {
        runtime::get_or_create_runtime_function(self, name)
    }
    
    // Function declaration/definition
    pub fn declare_external_function(
        &mut self,
        ext_func: &ast::ExternalFunction,
    ) -> Result<(), CompileError> {
        decl::declare_external_function(self, ext_func)
    }
    
    pub fn declare_function(
        &mut self,
        function: &ast::Function,
    ) -> Result<FunctionValue<'ctx>, CompileError> {
        decl::declare_function(self, function)
    }
    
    pub fn compile_function_body(
        &mut self,
        function: &ast::Function,
    ) -> Result<(), CompileError> {
        decl::compile_function_body(self, function)
    }
    
    pub fn define_and_compile_function(
        &mut self,
        function: &ast::Function,
    ) -> Result<FunctionValue<'ctx>, CompileError> {
        decl::define_and_compile_function(self, function)
    }
    
    // Function calls
    pub fn compile_function_call(
        &mut self,
        name: &str,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        calls::compile_function_call(self, name, args)
    }
    
    // Stdlib functions
    fn compile_io_print(
        &mut self,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        stdlib::compile_io_print(self, args)
    }
    
    fn compile_io_println(
        &mut self,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        stdlib::compile_io_println(self, args)
    }
    
    fn compile_io_print_int(
        &mut self,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        stdlib::compile_io_print_int(self, args)
    }
    
    fn compile_io_print_float(
        &mut self,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        stdlib::compile_io_print_float(self, args)
    }
    
    fn compile_math_function(
        &mut self,
        name: &str,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        stdlib::compile_math_function(self, name, args)
    }
    
    fn compile_core_assert(
        &mut self,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        stdlib::compile_core_assert(self, args)
    }
    
    fn compile_core_panic(
        &mut self,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        stdlib::compile_core_panic(self, args)
    }
    
    fn compile_fs_read_file(
        &mut self,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        stdlib::compile_fs_read_file(self, args)
    }
    
    fn compile_fs_write_file(
        &mut self,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        stdlib::compile_fs_write_file(self, args)
    }
    
    fn compile_fs_exists(
        &mut self,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        stdlib::compile_fs_exists(self, args)
    }
    
    fn compile_fs_remove_file(
        &mut self,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        stdlib::compile_fs_remove_file(self, args)
    }
    
    fn compile_fs_create_dir(
        &mut self,
        args: &[ast::Expression],
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        stdlib::compile_fs_create_dir(self, args)
    }
    
    fn create_result_ok(
        &mut self,
        value: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        stdlib::create_result_ok(self, value)
    }
    
    fn create_result_ok_void(&mut self) -> Result<BasicValueEnum<'ctx>, CompileError> {
        stdlib::create_result_ok_void(self)
    }
    
    fn create_result_err(
        &mut self,
        error: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        stdlib::create_result_err(self, error)
    }
}
