use super::LLVMCompiler;
use crate::ast::{BinaryOperator, Expression};
use crate::error::CompileError;
use inkwell::values::BasicValueEnum;
use inkwell::AddressSpace;
use inkwell::{FloatPredicate, IntPredicate};

impl<'ctx> LLVMCompiler<'ctx> {
    /// Helper function to cast integers to the same type (prefer wider type)
    fn normalize_int_types(
        &mut self,
        left_int: inkwell::values::IntValue<'ctx>,
        right_int: inkwell::values::IntValue<'ctx>,
    ) -> Result<(inkwell::values::IntValue<'ctx>, inkwell::values::IntValue<'ctx>), CompileError> {
        if left_int.get_type() != right_int.get_type() {
            let left_width = left_int.get_type().get_bit_width();
            let right_width = right_int.get_type().get_bit_width();
            
            if left_width > right_width {
                // Cast right to left's type
                let right_cast = self.builder.build_int_s_extend(
                    right_int,
                    left_int.get_type(),
                    "ext_right",
                )?;
                Ok((left_int, right_cast))
            } else {
                // Cast left to right's type
                let left_cast = self.builder.build_int_s_extend(
                    left_int,
                    right_int.get_type(),
                    "ext_left",
                )?;
                Ok((left_cast, right_int))
            }
        } else {
            Ok((left_int, right_int))
        }
    }
    
    pub fn compile_binary_operation(
        &mut self,
        op: &BinaryOperator,
        left: &Expression,
        right: &Expression,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        let left_val = self.compile_expression(left)?;
        let right_val = self.compile_expression(right)?;

        match op {
            BinaryOperator::Add => self.compile_add(left_val, right_val),
            BinaryOperator::Subtract => self.compile_subtract(left_val, right_val),
            BinaryOperator::Multiply => self.compile_multiply(left_val, right_val),
            BinaryOperator::Divide => self.compile_divide(left_val, right_val),
            BinaryOperator::Equals => self.compile_equals(left_val, right_val),
            BinaryOperator::NotEquals => self.compile_not_equals(left_val, right_val),
            BinaryOperator::LessThan => self.compile_less_than(left_val, right_val),
            BinaryOperator::GreaterThan => self.compile_greater_than(left_val, right_val),
            BinaryOperator::LessThanEquals => self.compile_less_than_equals(left_val, right_val),
            BinaryOperator::GreaterThanEquals => {
                self.compile_greater_than_equals(left_val, right_val)
            }
            BinaryOperator::StringConcat => self.compile_string_concat(left_val, right_val),
            BinaryOperator::Modulo => self.compile_modulo(left_val, right_val),
            BinaryOperator::And => self.compile_and(left_val, right_val),
            BinaryOperator::Or => self.compile_or(left_val, right_val),
        }
    }

    fn compile_add(
        &mut self,
        left_val: BasicValueEnum<'ctx>,
        right_val: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if left_val.is_int_value() && right_val.is_int_value() {
            let left_int = left_val.into_int_value();
            let right_int = right_val.into_int_value();

            // Cast to same type if needed (prefer wider type)
            let (left_final, right_final) = if left_int.get_type() != right_int.get_type() {
                let left_width = left_int.get_type().get_bit_width();
                let right_width = right_int.get_type().get_bit_width();

                if left_width > right_width {
                    // Cast right to left's type
                    let right_cast = self.builder.build_int_s_extend(
                        right_int,
                        left_int.get_type(),
                        "ext_right",
                    )?;
                    (left_int, right_cast)
                } else {
                    // Cast left to right's type
                    let left_cast = self.builder.build_int_s_extend(
                        left_int,
                        right_int.get_type(),
                        "ext_left",
                    )?;
                    (left_cast, right_int)
                }
            } else {
                (left_int, right_int)
            };

            let result = self
                .builder
                .build_int_add(left_final, right_final, "addtmp")?;
            Ok(result.into())
        } else if left_val.is_float_value() && right_val.is_float_value() {
            let result = self.builder.build_float_add(
                left_val.into_float_value(),
                right_val.into_float_value(),
                "addtmp",
            )?;
            Ok(result.into())
        } else if left_val.is_int_value() && right_val.is_float_value() {
            // Convert int to float
            let left_float = self.builder.build_signed_int_to_float(
                left_val.into_int_value(),
                self.context.f64_type(),
                "int_to_float",
            )?;
            let result = self.builder.build_float_add(
                left_float,
                right_val.into_float_value(),
                "addtmp",
            )?;
            Ok(result.into())
        } else if left_val.is_float_value() && right_val.is_int_value() {
            // Convert int to float  
            let right_float = self.builder.build_signed_int_to_float(
                right_val.into_int_value(),
                self.context.f64_type(),
                "int_to_float",
            )?;
            let result = self.builder.build_float_add(
                left_val.into_float_value(),
                right_float,
                "addtmp",
            )?;
            Ok(result.into())
        } else {
            // Check for specific type mismatches
            let left_is_pointer = left_val.is_pointer_value();
            let right_is_pointer = right_val.is_pointer_value();

            if left_is_pointer && right_val.is_int_value() {
                Err(CompileError::TypeMismatch {
                    expected: "i64".to_string(),
                    found: "String".to_string(),
                    span: None,
                })
            } else if right_is_pointer && left_val.is_int_value() {
                Err(CompileError::TypeMismatch {
                    expected: "i64".to_string(),
                    found: "String".to_string(),
                    span: None,
                })
            } else {
                Err(CompileError::TypeMismatch {
                    expected: "int or float".to_string(),
                    found: "mixed types".to_string(),
                    span: None,
                })
            }
        }
    }

    fn compile_subtract(
        &mut self,
        left_val: BasicValueEnum<'ctx>,
        right_val: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if left_val.is_int_value() && right_val.is_int_value() {
            let left_int = left_val.into_int_value();
            let right_int = right_val.into_int_value();

            // Cast to same type if needed (prefer wider type)
            let (left_final, right_final) = if left_int.get_type() != right_int.get_type() {
                let left_width = left_int.get_type().get_bit_width();
                let right_width = right_int.get_type().get_bit_width();

                if left_width > right_width {
                    let right_cast = self.builder.build_int_s_extend(
                        right_int,
                        left_int.get_type(),
                        "ext_right",
                    )?;
                    (left_int, right_cast)
                } else {
                    let left_cast = self.builder.build_int_s_extend(
                        left_int,
                        right_int.get_type(),
                        "ext_left",
                    )?;
                    (left_cast, right_int)
                }
            } else {
                (left_int, right_int)
            };

            let result = self
                .builder
                .build_int_sub(left_final, right_final, "subtmp")?;
            Ok(result.into())
        } else if left_val.is_float_value() && right_val.is_float_value() {
            let result = self.builder.build_float_sub(
                left_val.into_float_value(),
                right_val.into_float_value(),
                "subtmp",
            )?;
            Ok(result.into())
        } else if left_val.is_int_value() && right_val.is_float_value() {
            // Convert int to float
            let left_float = self.builder.build_signed_int_to_float(
                left_val.into_int_value(),
                self.context.f64_type(),
                "int_to_float",
            )?;
            let result = self.builder.build_float_sub(
                left_float,
                right_val.into_float_value(),
                "subtmp",
            )?;
            Ok(result.into())
        } else if left_val.is_float_value() && right_val.is_int_value() {
            // Convert int to float
            let right_float = self.builder.build_signed_int_to_float(
                right_val.into_int_value(),
                self.context.f64_type(),
                "int_to_float",
            )?;
            let result = self.builder.build_float_sub(
                left_val.into_float_value(),
                right_float,
                "subtmp",
            )?;
            Ok(result.into())
        } else {
            Err(CompileError::TypeMismatch {
                expected: "int or float".to_string(),
                found: "mixed types".to_string(),
                span: None,
            })
        }
    }

    fn compile_multiply(
        &mut self,
        left_val: BasicValueEnum<'ctx>,
        right_val: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if left_val.is_int_value() && right_val.is_int_value() {
            let left_int = left_val.into_int_value();
            let right_int = right_val.into_int_value();

            // Cast to same type if needed (prefer wider type)
            let (left_final, right_final) = if left_int.get_type() != right_int.get_type() {
                let left_width = left_int.get_type().get_bit_width();
                let right_width = right_int.get_type().get_bit_width();

                if left_width > right_width {
                    let right_cast = self.builder.build_int_s_extend(
                        right_int,
                        left_int.get_type(),
                        "ext_right",
                    )?;
                    (left_int, right_cast)
                } else {
                    let left_cast = self.builder.build_int_s_extend(
                        left_int,
                        right_int.get_type(),
                        "ext_left",
                    )?;
                    (left_cast, right_int)
                }
            } else {
                (left_int, right_int)
            };

            let result = self
                .builder
                .build_int_mul(left_final, right_final, "multmp")?;
            Ok(result.into())
        } else if left_val.is_float_value() && right_val.is_float_value() {
            let result = self.builder.build_float_mul(
                left_val.into_float_value(),
                right_val.into_float_value(),
                "multmp",
            )?;
            Ok(result.into())
        } else if left_val.is_int_value() && right_val.is_float_value() {
            // Convert int to float
            let left_float = self.builder.build_signed_int_to_float(
                left_val.into_int_value(),
                self.context.f64_type(),
                "int_to_float",
            )?;
            let result = self.builder.build_float_mul(
                left_float,
                right_val.into_float_value(),
                "multmp",
            )?;
            Ok(result.into())
        } else if left_val.is_float_value() && right_val.is_int_value() {
            // Convert int to float
            let right_float = self.builder.build_signed_int_to_float(
                right_val.into_int_value(),
                self.context.f64_type(),
                "int_to_float",
            )?;
            let result = self.builder.build_float_mul(
                left_val.into_float_value(),
                right_float,
                "multmp",
            )?;
            Ok(result.into())
        } else {
            Err(CompileError::TypeMismatch {
                expected: "int or float".to_string(),
                found: "mixed types".to_string(),
                span: None,
            })
        }
    }

    fn compile_divide(
        &mut self,
        left_val: BasicValueEnum<'ctx>,
        right_val: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if left_val.is_int_value() && right_val.is_int_value() {
            let left_int = left_val.into_int_value();
            let right_int = right_val.into_int_value();

            // Cast to same type if needed (prefer wider type)
            let (left_final, right_final) = if left_int.get_type() != right_int.get_type() {
                let left_width = left_int.get_type().get_bit_width();
                let right_width = right_int.get_type().get_bit_width();

                if left_width > right_width {
                    let right_cast = self.builder.build_int_s_extend(
                        right_int,
                        left_int.get_type(),
                        "ext_right",
                    )?;
                    (left_int, right_cast)
                } else {
                    let left_cast = self.builder.build_int_s_extend(
                        left_int,
                        right_int.get_type(),
                        "ext_left",
                    )?;
                    (left_cast, right_int)
                }
            } else {
                (left_int, right_int)
            };

            let result = self
                .builder
                .build_int_signed_div(left_final, right_final, "divtmp")?;
            Ok(result.into())
        } else if left_val.is_float_value() && right_val.is_float_value() {
            let result = self.builder.build_float_div(
                left_val.into_float_value(),
                right_val.into_float_value(),
                "divtmp",
            )?;
            Ok(result.into())
        } else if left_val.is_int_value() && right_val.is_float_value() {
            // Convert int to float
            let left_float = self.builder.build_signed_int_to_float(
                left_val.into_int_value(),
                self.context.f64_type(),
                "int_to_float",
            )?;
            let result = self.builder.build_float_div(
                left_float,
                right_val.into_float_value(),
                "divtmp",
            )?;
            Ok(result.into())
        } else if left_val.is_float_value() && right_val.is_int_value() {
            // Convert int to float
            let right_float = self.builder.build_signed_int_to_float(
                right_val.into_int_value(),
                self.context.f64_type(),
                "int_to_float",
            )?;
            let result = self.builder.build_float_div(
                left_val.into_float_value(),
                right_float,
                "divtmp",
            )?;
            Ok(result.into())
        } else {
            Err(CompileError::TypeMismatch {
                expected: "int or float".to_string(),
                found: "mixed types".to_string(),
                span: None,
            })
        }
    }

    fn compile_equals(
        &mut self,
        left_val: BasicValueEnum<'ctx>,
        right_val: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if left_val.is_int_value() && right_val.is_int_value() {
            let left_int = left_val.into_int_value();
            let right_int = right_val.into_int_value();
            
            // Cast to same type if needed (prefer wider type)
            let (left_final, right_final) = if left_int.get_type() != right_int.get_type() {
                let left_width = left_int.get_type().get_bit_width();
                let right_width = right_int.get_type().get_bit_width();
                
                if left_width > right_width {
                    // Cast right to left's type
                    let right_cast = self.builder.build_int_s_extend(
                        right_int,
                        left_int.get_type(),
                        "ext_right",
                    )?;
                    (left_int, right_cast)
                } else {
                    // Cast left to right's type
                    let left_cast = self.builder.build_int_s_extend(
                        left_int,
                        right_int.get_type(),
                        "ext_left",
                    )?;
                    (left_cast, right_int)
                }
            } else {
                (left_int, right_int)
            };
            
            let result = self.builder.build_int_compare(
                IntPredicate::EQ,
                left_final,
                right_final,
                "eqtmp",
            )?;
            Ok(result.into())
        } else if left_val.is_float_value() && right_val.is_float_value() {
            let result = self.builder.build_float_compare(
                FloatPredicate::OEQ,
                left_val.into_float_value(),
                right_val.into_float_value(),
                "eqtmp",
            )?;
            Ok(result.into())
        } else if left_val.is_int_value() && right_val.is_float_value() {
            // Convert int to float for comparison
            let left_float = self.builder.build_signed_int_to_float(
                left_val.into_int_value(),
                self.context.f64_type(),
                "int_to_float",
            )?;
            let result = self.builder.build_float_compare(
                FloatPredicate::OEQ,
                left_float,
                right_val.into_float_value(),
                "eqtmp",
            )?;
            Ok(result.into())
        } else if left_val.is_float_value() && right_val.is_int_value() {
            // Convert int to float for comparison
            let right_float = self.builder.build_signed_int_to_float(
                right_val.into_int_value(),
                self.context.f64_type(),
                "int_to_float",
            )?;
            let result = self.builder.build_float_compare(
                FloatPredicate::OEQ,
                left_val.into_float_value(),
                right_float,
                "eqtmp",
            )?;
            Ok(result.into())
        } else if left_val.is_pointer_value() && right_val.is_pointer_value() {
            // String comparison: call strcmp and check for zero
            let strcmp_fn = match self.module.get_function("strcmp") {
                Some(f) => f,
                None => {
                    let i8_ptr_type = self.context.ptr_type(AddressSpace::default());
                    let fn_type = self
                        .context
                        .i32_type()
                        .fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
                    self.module.add_function("strcmp", fn_type, None)
                }
            };
            let left_ptr = left_val.into_pointer_value();
            let right_ptr = right_val.into_pointer_value();
            let call = self.builder.build_call(
                strcmp_fn,
                &[left_ptr.into(), right_ptr.into()],
                "strcmp_call",
            )?;
            let cmp_result = call
                .try_as_basic_value()
                .left()
                .ok_or_else(|| {
                    CompileError::InternalError("strcmp did not return a value".to_string(), None)
                })?
                .into_int_value();
            let zero = self.context.i32_type().const_int(0, false);
            let result =
                self.builder
                    .build_int_compare(IntPredicate::EQ, cmp_result, zero, "strcmp_eq")?;
            Ok(result.into())
        } else {
            Err(CompileError::TypeMismatch {
                expected: "int or float or string".to_string(),
                found: "mixed types".to_string(),
                span: None,
            })
        }
    }

    fn compile_not_equals(
        &mut self,
        left_val: BasicValueEnum<'ctx>,
        right_val: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if left_val.is_int_value() && right_val.is_int_value() {
            let left_int = left_val.into_int_value();
            let right_int = right_val.into_int_value();
            
            // Cast to same type if needed (prefer wider type)
            let (left_final, right_final) = if left_int.get_type() != right_int.get_type() {
                let left_width = left_int.get_type().get_bit_width();
                let right_width = right_int.get_type().get_bit_width();
                
                if left_width > right_width {
                    // Cast right to left's type
                    let right_cast = self.builder.build_int_s_extend(
                        right_int,
                        left_int.get_type(),
                        "ext_right",
                    )?;
                    (left_int, right_cast)
                } else {
                    // Cast left to right's type
                    let left_cast = self.builder.build_int_s_extend(
                        left_int,
                        right_int.get_type(),
                        "ext_left",
                    )?;
                    (left_cast, right_int)
                }
            } else {
                (left_int, right_int)
            };
            
            let result = self.builder.build_int_compare(
                IntPredicate::NE,
                left_final,
                right_final,
                "netmp",
            )?;
            Ok(result.into())
        } else if left_val.is_float_value() && right_val.is_float_value() {
            let result = self.builder.build_float_compare(
                FloatPredicate::ONE,
                left_val.into_float_value(),
                right_val.into_float_value(),
                "netmp",
            )?;
            Ok(result.into())
        } else if left_val.is_pointer_value() && right_val.is_pointer_value() {
            // String comparison: call strcmp and check for nonzero
            let strcmp_fn = match self.module.get_function("strcmp") {
                Some(f) => f,
                None => {
                    let i8_ptr_type = self.context.ptr_type(AddressSpace::default());
                    let fn_type = self
                        .context
                        .i32_type()
                        .fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
                    self.module.add_function("strcmp", fn_type, None)
                }
            };
            let left_ptr = left_val.into_pointer_value();
            let right_ptr = right_val.into_pointer_value();
            let call = self.builder.build_call(
                strcmp_fn,
                &[left_ptr.into(), right_ptr.into()],
                "strcmp_call",
            )?;
            let cmp_result = call
                .try_as_basic_value()
                .left()
                .ok_or_else(|| {
                    CompileError::InternalError("strcmp did not return a value".to_string(), None)
                })?
                .into_int_value();
            let zero = self.context.i32_type().const_int(0, false);
            let result =
                self.builder
                    .build_int_compare(IntPredicate::NE, cmp_result, zero, "strcmp_ne")?;
            Ok(result.into())
        } else {
            Err(CompileError::TypeMismatch {
                expected: "int or float or string".to_string(),
                found: "mixed types".to_string(),
                span: None,
            })
        }
    }

    fn compile_less_than(
        &mut self,
        left_val: BasicValueEnum<'ctx>,
        right_val: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if left_val.is_int_value() && right_val.is_int_value() {
            let left_int = left_val.into_int_value();
            let right_int = right_val.into_int_value();
            
            // Normalize types
            let (left_final, right_final) = self.normalize_int_types(left_int, right_int)?;
            
            let result = self.builder.build_int_compare(
                IntPredicate::SLT,
                left_final,
                right_final,
                "lttmp",
            )?;
            // Zero-extend i1 to i64 for test compatibility
            let zext =
                self.builder
                    .build_int_z_extend(result, self.context.i64_type(), "zext_lt")?;
            Ok(zext.into())
        } else if left_val.is_float_value() && right_val.is_float_value() {
            let result = self.builder.build_float_compare(
                FloatPredicate::OLT,
                left_val.into_float_value(),
                right_val.into_float_value(),
                "lttmp",
            )?;
            // Zero-extend i1 to i64 for test compatibility
            let zext =
                self.builder
                    .build_int_z_extend(result, self.context.i64_type(), "zext_lt")?;
            Ok(zext.into())
        } else {
            Err(CompileError::TypeMismatch {
                expected: "int or float".to_string(),
                found: "mixed types".to_string(),
                span: None,
            })
        }
    }

    fn compile_greater_than(
        &mut self,
        left_val: BasicValueEnum<'ctx>,
        right_val: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if left_val.is_int_value() && right_val.is_int_value() {
            let left_int = left_val.into_int_value();
            let right_int = right_val.into_int_value();
            
            // Normalize types
            let (left_final, right_final) = self.normalize_int_types(left_int, right_int)?;
            
            let result = self.builder.build_int_compare(
                IntPredicate::SGT,
                left_final,
                right_final,
                "gttmp",
            )?;
            Ok(result.into())
        } else if left_val.is_float_value() && right_val.is_float_value() {
            let result = self.builder.build_float_compare(
                FloatPredicate::OGT,
                left_val.into_float_value(),
                right_val.into_float_value(),
                "gttmp",
            )?;
            Ok(result.into())
        } else {
            Err(CompileError::TypeMismatch {
                expected: "int or float".to_string(),
                found: "mixed types".to_string(),
                span: None,
            })
        }
    }

    fn compile_less_than_equals(
        &mut self,
        left_val: BasicValueEnum<'ctx>,
        right_val: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if left_val.is_int_value() && right_val.is_int_value() {
            let left_int = left_val.into_int_value();
            let right_int = right_val.into_int_value();
            
            // Normalize types
            let (left_final, right_final) = self.normalize_int_types(left_int, right_int)?;
            
            let result = self.builder.build_int_compare(
                IntPredicate::SLE,
                left_final,
                right_final,
                "letmp",
            )?;
            Ok(result.into())
        } else if left_val.is_float_value() && right_val.is_float_value() {
            let result = self.builder.build_float_compare(
                FloatPredicate::OLE,
                left_val.into_float_value(),
                right_val.into_float_value(),
                "letmp",
            )?;
            Ok(result.into())
        } else {
            Err(CompileError::TypeMismatch {
                expected: "int or float".to_string(),
                found: "mixed types".to_string(),
                span: None,
            })
        }
    }

    fn compile_greater_than_equals(
        &mut self,
        left_val: BasicValueEnum<'ctx>,
        right_val: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if left_val.is_int_value() && right_val.is_int_value() {
            let left_int = left_val.into_int_value();
            let right_int = right_val.into_int_value();
            
            // Normalize types
            let (left_final, right_final) = self.normalize_int_types(left_int, right_int)?;
            
            let result = self.builder.build_int_compare(
                IntPredicate::SGE,
                left_final,
                right_final,
                "getmp",
            )?;
            Ok(result.into())
        } else if left_val.is_float_value() && right_val.is_float_value() {
            let result = self.builder.build_float_compare(
                FloatPredicate::OGE,
                left_val.into_float_value(),
                right_val.into_float_value(),
                "getmp",
            )?;
            Ok(result.into())
        } else {
            Err(CompileError::TypeMismatch {
                expected: "int or float".to_string(),
                found: "mixed types".to_string(),
                span: None,
            })
        }
    }

    fn compile_string_concat(
        &mut self,
        _left_val: BasicValueEnum<'ctx>,
        _right_val: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // String concatenation requires an allocator - return error for now
        // This will be replaced with proper allocator-based concatenation
        Err(CompileError::InternalError(
            "String concatenation requires an allocator. Use String.concat() or String.append() from stdlib instead.".to_string(),
            None,
        ))
    }

    fn compile_modulo(
        &mut self,
        left_val: BasicValueEnum<'ctx>,
        right_val: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if left_val.is_int_value() && right_val.is_int_value() {
            let left_int = left_val.into_int_value();
            let right_int = right_val.into_int_value();

            // Cast to same type if needed (prefer wider type)
            let (left_final, right_final) = if left_int.get_type() != right_int.get_type() {
                let left_width = left_int.get_type().get_bit_width();
                let right_width = right_int.get_type().get_bit_width();

                if left_width > right_width {
                    let right_cast = self.builder.build_int_s_extend(
                        right_int,
                        left_int.get_type(),
                        "ext_right",
                    )?;
                    (left_int, right_cast)
                } else {
                    let left_cast = self.builder.build_int_s_extend(
                        left_int,
                        right_int.get_type(),
                        "ext_left",
                    )?;
                    (left_cast, right_int)
                }
            } else {
                (left_int, right_int)
            };

            let result = self
                .builder
                .build_int_signed_rem(left_final, right_final, "modtmp")?;
            Ok(result.into())
        } else {
            Err(CompileError::TypeMismatch {
                expected: "int".to_string(),
                found: "mixed types".to_string(),
                span: None,
            })
        }
    }

    fn compile_and(
        &mut self,
        left_val: BasicValueEnum<'ctx>,
        right_val: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if left_val.is_int_value() && right_val.is_int_value() {
            let left_int = left_val.into_int_value();
            let right_int = right_val.into_int_value();

            // Cast to same type if needed (prefer wider type)
            let (left_final, right_final) = if left_int.get_type() != right_int.get_type() {
                let left_width = left_int.get_type().get_bit_width();
                let right_width = right_int.get_type().get_bit_width();

                if left_width > right_width {
                    // Cast right to left's type
                    let right_cast = if right_width == 1 {
                        // Zero-extend boolean to match left's type
                        self.builder.build_int_z_extend(
                            right_int,
                            left_int.get_type(),
                            "zext_right",
                        )?
                    } else {
                        self.builder.build_int_s_extend(
                            right_int,
                            left_int.get_type(),
                            "ext_right",
                        )?
                    };
                    (left_int, right_cast)
                } else {
                    // Cast left to right's type
                    let left_cast = if left_width == 1 {
                        // Zero-extend boolean to match right's type
                        self.builder.build_int_z_extend(
                            left_int,
                            right_int.get_type(),
                            "zext_left",
                        )?
                    } else {
                        self.builder.build_int_s_extend(
                            left_int,
                            right_int.get_type(),
                            "ext_left",
                        )?
                    };
                    (left_cast, right_int)
                }
            } else {
                (left_int, right_int)
            };

            let result = self.builder.build_and(
                left_final,
                right_final,
                "andtmp",
            )?;
            Ok(result.into())
        } else {
            Err(CompileError::TypeMismatch {
                expected: "int".to_string(),
                found: "mixed types".to_string(),
                span: None,
            })
        }
    }

    fn compile_or(
        &mut self,
        left_val: BasicValueEnum<'ctx>,
        right_val: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        if left_val.is_int_value() && right_val.is_int_value() {
            let left_int = left_val.into_int_value();
            let right_int = right_val.into_int_value();

            // Cast to same type if needed (prefer wider type)
            let (left_final, right_final) = if left_int.get_type() != right_int.get_type() {
                let left_width = left_int.get_type().get_bit_width();
                let right_width = right_int.get_type().get_bit_width();

                if left_width > right_width {
                    // Cast right to left's type
                    let right_cast = if right_width == 1 {
                        // Zero-extend boolean to match left's type
                        self.builder.build_int_z_extend(
                            right_int,
                            left_int.get_type(),
                            "zext_right",
                        )?
                    } else {
                        self.builder.build_int_s_extend(
                            right_int,
                            left_int.get_type(),
                            "ext_right",
                        )?
                    };
                    (left_int, right_cast)
                } else {
                    // Cast left to right's type
                    let left_cast = if left_width == 1 {
                        // Zero-extend boolean to match right's type
                        self.builder.build_int_z_extend(
                            left_int,
                            right_int.get_type(),
                            "zext_left",
                        )?
                    } else {
                        self.builder.build_int_s_extend(
                            left_int,
                            right_int.get_type(),
                            "ext_left",
                        )?
                    };
                    (left_cast, right_int)
                }
            } else {
                (left_int, right_int)
            };

            let result = self.builder.build_or(
                left_final,
                right_final,
                "ortmp",
            )?;
            Ok(result.into())
        } else {
            Err(CompileError::TypeMismatch {
                expected: "int".to_string(),
                found: "mixed types".to_string(),
                span: None,
            })
        }
    }
}
