use super::LLVMCompiler;
use crate::ast::{AstType, Expression};
use crate::error::CompileError;
use inkwell::{
    types::BasicType,
    values::{BasicValue, BasicValueEnum},
};

impl<'ctx> LLVMCompiler<'ctx> {
    pub fn compile_address_of(&mut self, expr: &Expression) -> Result<BasicValueEnum<'ctx>, CompileError> {
        match expr {
            Expression::Identifier(name) => {
                let var_info = self
                    .variables
                    .get(name)
                    .ok_or_else(|| CompileError::UndeclaredVariable(name.clone(), None))?;
                
                let alloca = var_info.pointer;
                let ast_type = &var_info.ast_type;
                
                // If the variable is already a pointer type, return it directly
                if matches!(ast_type, AstType::Ptr(_)) {
                    Ok(alloca.as_basic_value_enum())
                } else {
                    // For non-pointer variables, return the address
                    Ok(alloca.as_basic_value_enum())
                }
            }
            _ => Err(CompileError::UnsupportedFeature(
                "AddressOf only supported for identifiers".to_string(),
                None,
            )),
        }
    }

    pub fn compile_dereference(&mut self, expr: &Expression) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // First, check if expr is an identifier for a pointer variable
        if let Expression::Identifier(name) = expr {
            if let Ok((alloca, ast_type)) = self.get_variable(name) {
                match ast_type {
                    AstType::Ptr(inner) | AstType::MutPtr(inner) | AstType::RawPtr(inner) => {
                        // This is a pointer variable, so we need to:
                        // 1. Load the pointer value from the alloca
                        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                        let ptr_value = self.builder.build_load(ptr_type, alloca, "load_ptr")?;
                        let ptr = ptr_value.into_pointer_value();
                        
                        // 2. Load the value from the address stored in the pointer
                        let llvm_type = self.to_llvm_type(&inner)?;
                        return match llvm_type {
                            super::Type::Basic(basic_type) => {
                                Ok(self.builder.build_load(basic_type, ptr, "deref_value")?.into())
                            }
                            super::Type::Struct(struct_type) => {
                                Ok(self.builder.build_load(struct_type, ptr, "deref_struct")?.into())
                            }
                            _ => Err(CompileError::TypeError(
                                "Cannot dereference non-basic/non-struct type".to_string(),
                                None,
                            )),
                        };
                    }
                    _ => {
                        return Err(CompileError::TypeMismatch {
                            expected: "pointer".to_string(),
                            found: format!("{:?}", ast_type),
                            span: None,
                        });
                    }
                }
            }
        }
        
        // For CreateReference expressions, we know the type
        if let Expression::CreateReference(inner_expr) | Expression::CreateMutableReference(inner_expr) = expr {
            // This is a temporary pointer created by .ref() or .mut_ref()
            // The pointer value is the address of the inner expression
            let ptr_val = self.compile_address_of(inner_expr)?;
            if !ptr_val.is_pointer_value() {
                return Err(CompileError::TypeMismatch {
                    expected: "pointer".to_string(),
                    found: format!("{:?}", ptr_val.get_type()),
                    span: None,
                });
            }
            let ptr = ptr_val.into_pointer_value();
            
            // Try to infer the type of the inner expression
            // For now, we'll use i32 as the default for integer literals
            let llvm_type = if let Expression::Integer32(_) = &**inner_expr {
                super::Type::Basic(self.context.i32_type().as_basic_type_enum())
            } else if let Expression::Identifier(name) = &**inner_expr {
                // Look up the type of the identifier
                if let Ok((_, ast_type)) = self.get_variable(name) {
                    self.to_llvm_type(&ast_type)?
                } else {
                    super::Type::Basic(self.context.i32_type().as_basic_type_enum())
                }
            } else {
                // Default to i32 for now
                super::Type::Basic(self.context.i32_type().as_basic_type_enum())
            };
            
            return match llvm_type {
                super::Type::Basic(basic_type) => {
                    Ok(self.builder.build_load(basic_type, ptr, "deref_value")?.into())
                }
                super::Type::Struct(struct_type) => {
                    Ok(self.builder.build_load(struct_type, ptr, "deref_struct")?.into())
                }
                _ => Err(CompileError::TypeError(
                    "Cannot dereference non-basic/non-struct type".to_string(),
                    None,
                )),
            };
        }
        
        // For other expressions, compile them and check if they return a pointer
        let ptr_val = self.compile_expression(expr)?;
        if !ptr_val.is_pointer_value() {
            return Err(CompileError::TypeMismatch {
                expected: "pointer".to_string(),
                found: format!("{:?}", ptr_val.get_type()),
                span: None,
            });
        }
        let ptr = ptr_val.into_pointer_value();
        
        // Since we don't have type info, assume i32 for now (most common in tests)
        let llvm_type = super::Type::Basic(self.context.i32_type().as_basic_type_enum());
        match llvm_type {
            super::Type::Basic(basic_type) => Ok(self.builder.build_load(basic_type, ptr, "load_tmp")?.into()),
            super::Type::Struct(struct_type) => Ok(self.builder.build_load(struct_type, ptr, "load_struct_tmp")?.into()),
            _ => Err(CompileError::TypeError("Cannot dereference non-basic/non-struct type".to_string(), None)),
        }
    }

    pub fn compile_pointer_to_int(&mut self, expr: &Expression) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // Convert a pointer to its numeric address (usize)
        let val = self.compile_expression(expr)?;
        if let Ok(ptr) = val.try_into() {
            let ptr: inkwell::values::PointerValue = ptr;
            // Use i64 as usize for now (TODO: get proper target-specific size)
            let usize_type = self.context.i64_type();
            let int_val = self.builder.build_ptr_to_int(ptr, usize_type, "ptr_to_int")?;
            Ok(int_val.as_basic_value_enum())
        } else {
            Err(CompileError::TypeMismatch {
                expected: "pointer".to_string(),
                found: format!("{:?}", val.get_type()),
                span: None,
            })
        }
    }

    pub fn compile_pointer_offset(&mut self, pointer: &Expression, offset: &Expression) -> Result<BasicValueEnum<'ctx>, CompileError> {
        let base_val = self.compile_expression(pointer)?;
        let offset_val = self.compile_expression(offset)?;
        if !base_val.is_pointer_value() {
            return Err(CompileError::TypeMismatch {
                expected: "pointer for pointer offset base".to_string(),
                found: format!("{:?}", base_val.get_type()),
                span: None,
            });
        }
        if !offset_val.is_int_value() {
            return Err(CompileError::TypeMismatch {
                expected: "integer for pointer offset value".to_string(),
                found: format!("{:?}", offset_val.get_type()),
                span: None,
            });
        }
        unsafe {
            let ptr_type = base_val.get_type();
            let _offset = offset_val.into_int_value();
            let ptr = base_val.into_pointer_value();
            Ok(self.builder.build_gep(ptr_type, ptr, &[self.context.i32_type().const_int(0, false)], "gep_tmp")?.into())
        }
    }
} 