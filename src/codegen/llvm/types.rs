use super::{symbols, LLVMCompiler, StructTypeInfo, Type};
use crate::ast::{self, AstType};
use crate::error::CompileError;
use crate::stdlib_types::StdlibTypeRegistry;
use inkwell::{
    types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum},
    AddressSpace,
};
use std::collections::HashMap;

impl<'ctx> LLVMCompiler<'ctx> {
    pub fn to_llvm_type(&mut self, type_: &AstType) -> Result<Type<'ctx>, CompileError> {
        let result = match type_ {
            AstType::I8 => Ok(Type::Basic(self.context.i8_type().into())),
            AstType::I16 => Ok(Type::Basic(self.context.i16_type().into())),
            AstType::I32 => Ok(Type::Basic(self.context.i32_type().into())),
            AstType::I64 => Ok(Type::Basic(self.context.i64_type().into())),
            AstType::U8 => Ok(Type::Basic(self.context.i8_type().into())),
            AstType::U16 => Ok(Type::Basic(self.context.i16_type().into())),
            AstType::U32 => Ok(Type::Basic(self.context.i32_type().into())),
            AstType::U64 => Ok(Type::Basic(self.context.i64_type().into())),
            AstType::Usize => Ok(Type::Basic(self.context.i64_type().into())), // usize as i64 on 64-bit systems
            AstType::F32 => Ok(Type::Basic(self.context.f32_type().into())),
            AstType::F64 => Ok(Type::Basic(self.context.f64_type().into())),
            AstType::Bool => Ok(Type::Basic(self.context.bool_type().into())),
            AstType::StaticLiteral | AstType::StaticString => Ok(Type::Basic(
                self.context.ptr_type(AddressSpace::default()).into(),
            )),
            // Note: String structs are now handled by the normal AstType::Struct branch below
            // since String is registered in struct_types via register_builtin_enums()
            AstType::Void => Ok(Type::Void),
            // Handle all pointer types (Ptr, MutPtr, RawPtr) - they're all the same in LLVM
            t if t.is_ptr_type() => {
                if let Some(inner) = t.ptr_inner() {
                    let inner_type = self.to_llvm_type(inner)?;
                    match inner_type {
                        Type::Basic(_) | Type::Struct(_) | Type::Void => Ok(Type::Basic(
                            self.context.ptr_type(AddressSpace::default()).into(),
                        )),
                        _ => Err(CompileError::UnsupportedFeature(
                            "Unsupported pointer type".to_string(),
                            None,
                        )),
                    }
                } else {
                    Err(CompileError::UnsupportedFeature(
                        "Invalid pointer type".to_string(),
                        None,
                    ))
                }
            }
            AstType::Struct { name, fields: _ } => {
                // Try to ensure the struct type is registered (might come from stdlib)
                self.ensure_struct_type(name)?;
                let struct_info = self.struct_types.get(name).ok_or_else(|| {
                    CompileError::TypeError(
                        format!("Undefined struct type: {}", name),
                        self.get_current_span(),
                    )
                })?;
                Ok(Type::Struct(struct_info.llvm_type))
            }
            AstType::Array(inner) => {
                let inner_type = self.to_llvm_type(inner)?;
                match inner_type {
                    Type::Basic(basic_type) => Ok(Type::Basic(basic_type)), // Dynamic array (pointer)
                    _ => Ok(Type::Basic(self.context.i8_type().array_type(0).into())), // Default to array of bytes
                }
            }
            AstType::FixedArray { element_type, size } => {
                let elem_type = self.to_llvm_type(element_type)?;
                match elem_type {
                    Type::Basic(basic_type) => {
                        // Create an LLVM array type with the specified size
                        let array_type = match basic_type {
                            BasicTypeEnum::IntType(int_type) => {
                                int_type.array_type(*size as u32).into()
                            }
                            BasicTypeEnum::FloatType(float_type) => {
                                float_type.array_type(*size as u32).into()
                            }
                            BasicTypeEnum::PointerType(ptr_type) => {
                                ptr_type.array_type(*size as u32).into()
                            }
                            BasicTypeEnum::StructType(struct_type) => {
                                struct_type.array_type(*size as u32).into()
                            }
                            BasicTypeEnum::ArrayType(arr_type) => {
                                arr_type.array_type(*size as u32).into()
                            }
                            BasicTypeEnum::VectorType(vec_type) => {
                                vec_type.array_type(*size as u32).into()
                            }
                            BasicTypeEnum::ScalableVectorType(_) => {
                                // For now, use a default array of i8
                                self.context.i8_type().array_type(*size as u32).into()
                            }
                        };
                        Ok(Type::Basic(array_type))
                    }
                    _ => Ok(Type::Basic(
                        self.context.i8_type().array_type(*size as u32).into(),
                    )), // Default to array of bytes
                }
            }
            AstType::Function { args, return_type } => {
                let return_llvm_type = self.to_llvm_type(return_type)?;
                let arg_llvm_types: Result<Vec<BasicTypeEnum<'ctx>>, CompileError> = args
                    .iter()
                    .map(|arg| {
                        let arg_type = self.to_llvm_type(arg)?;
                        match arg_type {
                            Type::Basic(basic_type) => Ok(basic_type),
                            _ => Ok(self.context.i64_type().into()), // Default to i64 for complex types
                        }
                    })
                    .collect();
                let arg_llvm_types = arg_llvm_types?;

                // Convert BasicTypeEnum to BasicMetadataTypeEnum for function signatures
                let arg_metadata_types: Vec<BasicMetadataTypeEnum<'ctx>> =
                    arg_llvm_types.iter().map(|ty| (*ty).into()).collect();

                let function_type = match return_llvm_type {
                    Type::Basic(basic_type) => basic_type.fn_type(&arg_metadata_types, false),
                    _ => self.context.i64_type().fn_type(&arg_metadata_types, false),
                };
                Ok(Type::Function(function_type))
            }
            AstType::FunctionPointer {
                param_types,
                return_type,
            } => {
                // Function pointers are represented as pointers to functions
                let return_llvm_type = self.to_llvm_type(return_type)?;
                let arg_llvm_types: Result<Vec<BasicTypeEnum<'ctx>>, CompileError> = param_types
                    .iter()
                    .map(|arg| {
                        let arg_type = self.to_llvm_type(arg)?;
                        match arg_type {
                            Type::Basic(basic_type) => Ok(basic_type),
                            _ => Ok(self.context.i64_type().into()), // Default to i64 for complex types
                        }
                    })
                    .collect();
                let arg_llvm_types = arg_llvm_types?;

                // Convert BasicTypeEnum to BasicMetadataTypeEnum for function signatures
                let arg_metadata_types: Vec<BasicMetadataTypeEnum<'ctx>> =
                    arg_llvm_types.iter().map(|ty| (*ty).into()).collect();

                let _function_type = match return_llvm_type {
                    Type::Basic(basic_type) => basic_type.fn_type(&arg_metadata_types, false),
                    Type::Void => self.context.void_type().fn_type(&arg_metadata_types, false),
                    _ => self.context.i64_type().fn_type(&arg_metadata_types, false),
                };

                // Return a pointer to the function type
                Ok(Type::Basic(
                    self.context.ptr_type(AddressSpace::default()).into(),
                ))
            }
            AstType::Enum { name, variants: _ } => {
                // Look up the registered enum type
                if let Some(symbols::Symbol::EnumType(enum_info)) = self.symbols.lookup(name) {
                    Ok(Type::Struct(enum_info.llvm_type))
                } else {
                    // Fallback to a simple tag-only enum if not registered
                    // This should rarely happen as enums should be registered during declaration phase
                    let enum_struct_type = self.context.struct_type(
                        &[
                            self.context.i64_type().into(), // discriminant/tag only
                        ],
                        false,
                    );
                    Ok(Type::Struct(enum_struct_type))
                }
            }
            AstType::Ref(inner) => {
                // Ref<T> is represented as a pointer to T
                let inner_type = self.to_llvm_type(inner)?;
                match inner_type {
                    Type::Basic(basic_type) => Ok(Type::Basic(basic_type)),
                    _ => Ok(Type::Basic(
                        self.context.ptr_type(AddressSpace::default()).into(),
                    )),
                }
            }
            // Option and Result are now Generic types - they're handled in the Generic match above
            AstType::Range {
                start_type,
                end_type,
                inclusive: _,
            } => {
                // Range is represented as a struct with start, end, and inclusive values
                let _start_type = self.to_llvm_type(start_type)?;
                let _end_type = self.to_llvm_type(end_type)?;
                // For now, just use i64 for both start and end, and bool for inclusive
                let range_struct = self.context.struct_type(
                    &[
                        self.context.i64_type().into(),
                        self.context.i64_type().into(),
                        self.context.bool_type().into(), // Add inclusive field
                    ],
                    false,
                );
                Ok(Type::Struct(range_struct))
            }
            AstType::Vec { element_type, size } => {
                // Vec<T, size> - Fixed-size vector as struct containing array and length
                let elem_llvm_type = self.to_llvm_type(element_type)?;
                match elem_llvm_type {
                    Type::Basic(basic_type) => {
                        // Create struct: { [T; size], usize }
                        let array_type = basic_type.array_type(*size as u32);
                        let len_type = self.context.i64_type(); // Use i64 for length
                        let vec_struct = self.context.struct_type(
                            &[
                                array_type.into(), // data: [T; size]
                                len_type.into(),   // len: usize (current length)
                            ],
                            false,
                        );
                        Ok(Type::Struct(vec_struct))
                    }
                    Type::Struct(struct_type) => {
                        // Handle struct element types properly
                        let array_type = struct_type.array_type(*size as u32);
                        let len_type = self.context.i64_type(); // Use i64 for length
                        let vec_struct = self.context.struct_type(
                            &[
                                array_type.into(), // data: [T; size] where T is a struct
                                len_type.into(),   // len: usize (current length)
                            ],
                            false,
                        );
                        Ok(Type::Struct(vec_struct))
                    }
                    _ => {
                        // Fallback for other types (should not normally reach here)
                        let array_type = self.context.i8_type().array_type(*size as u32);
                        let len_type = self.context.i64_type();
                        let vec_struct = self
                            .context
                            .struct_type(&[array_type.into(), len_type.into()], false);
                        Ok(Type::Struct(vec_struct))
                    }
                }
            }
            AstType::DynVec {
                element_types,
                allocator_type: _,
            } => {
                // DynVec<T> - Dynamic vector as struct containing pointer, length, and capacity
                // For mixed variant types, use a union or tagged union approach
                if element_types.len() == 1 {
                    // Single type DynVec: { ptr, len, capacity }
                    let ptr_type = self.context.ptr_type(AddressSpace::default());
                    let len_type = self.context.i64_type();
                    let cap_type = self.context.i64_type();
                    let dynvec_struct = self.context.struct_type(
                        &[
                            ptr_type.into(), // data: Ptr<T>
                            len_type.into(), // len: usize
                            cap_type.into(), // capacity: usize
                        ],
                        false,
                    );
                    Ok(Type::Struct(dynvec_struct))
                } else {
                    // Mixed variant DynVec: { ptr, len, capacity, discriminants }
                    let ptr_type = self.context.ptr_type(AddressSpace::default());
                    let len_type = self.context.i64_type();
                    let cap_type = self.context.i64_type();
                    let discriminant_ptr = self.context.ptr_type(AddressSpace::default()); // Pointer to discriminant array
                    let dynvec_struct = self.context.struct_type(
                        &[
                            ptr_type.into(),         // data: Ptr<union>
                            len_type.into(),         // len: usize
                            cap_type.into(),         // capacity: usize
                            discriminant_ptr.into(), // discriminants: Ptr<u8> for variant tracking
                        ],
                        false,
                    );
                    Ok(Type::Struct(dynvec_struct))
                }
            }
            AstType::Generic { name, type_args } => {
                if name.is_empty() {
                    return Ok(Type::Basic(self.context.i32_type().into()));
                }

                if name.len() == 1
                    && name.chars().next().unwrap().is_uppercase()
                    && type_args.is_empty()
                {
                    let placeholder = self.context.struct_type(&[], false);
                    return Ok(Type::Struct(placeholder));
                }

                // Well-known types: Option and Result use tagged union representation
                if self.well_known.is_result(name) || self.well_known.is_option(name) {
                    let enum_struct_type = self.context.struct_type(
                        &[
                            self.context.i64_type().into(), // discriminant
                            self.context
                                .ptr_type(inkwell::AddressSpace::default())
                                .into(), // payload pointer
                        ],
                        false,
                    );
                    return Ok(Type::Struct(enum_struct_type));
                }

                // Try to get struct definition from StdlibTypeRegistry
                let registry = crate::stdlib_types::stdlib_types();
                if let Some(struct_type) = registry.get_struct_type(name) {
                    return self.to_llvm_type(&struct_type);
                }

                // Check registered enum types in symbol table
                if self.well_known.is_option(name) || self.well_known.is_result(name) {
                    // Look up the registered enum type
                    if let Some(symbols::Symbol::EnumType(enum_info)) = self.symbols.lookup(name) {
                        return Ok(Type::Struct(enum_info.llvm_type));
                    } else {
                        // Fallback to standard enum struct if not registered (shouldn't happen)
                        let enum_struct_type = self.context.struct_type(
                            &[
                                self.context.i64_type().into(), // discriminant
                                self.context
                                    .ptr_type(inkwell::AddressSpace::default())
                                    .into(), // payload pointer
                            ],
                            false,
                        );
                        return Ok(Type::Struct(enum_struct_type));
                    }
                }

                // Check if this is actually a user-defined struct type
                if let Some(struct_info) = self.struct_types.get(name) {
                    Ok(Type::Struct(struct_info.llvm_type))
                } else if let Some(symbols::Symbol::EnumType(enum_info)) = self.symbols.lookup(name)
                {
                    // Check if it's an enum type that was parsed as Generic
                    Ok(Type::Struct(enum_info.llvm_type))
                } else if name == "DynVec" && type_args.len() == 1 {
                    let dynvec_type = AstType::DynVec {
                        element_types: type_args.clone(),
                        allocator_type: None,
                    };
                    self.to_llvm_type(&dynvec_type)
                } else if type_args
                    .iter()
                    .any(|t| matches!(t, AstType::Generic { .. }))
                {
                    let placeholder = self.context.struct_type(&[], false);
                    Ok(Type::Struct(placeholder))
                } else {
                    Err(CompileError::InternalError(
                        format!("Unresolved generic type '{}' found after monomorphization. This is a compiler bug.", name),
                        self.get_current_span()
                    ))
                }
            }
            AstType::EnumType { name } => {
                // EnumType is used when an enum is referenced as a type constructor
                // Look up the registered enum type
                if let Some(symbols::Symbol::EnumType(enum_info)) = self.symbols.lookup(name) {
                    Ok(Type::Struct(enum_info.llvm_type))
                } else {
                    // Fallback to a default enum structure if not registered
                    let enum_struct_type = self.context.struct_type(
                        &[
                            self.context.i64_type().into(), // discriminant/tag
                            self.context.i64_type().into(), // payload (simplified)
                        ],
                        false,
                    );
                    Ok(Type::Struct(enum_struct_type))
                }
            }
            AstType::StdModule => {
                // StdModule is a marker type for imported stdlib modules (like math, io)
                // It's represented as an i64 in LLVM (storing module identifier)
                Ok(Type::Basic(self.context.i64_type().into()))
            }
        };
        result
    }
    pub fn expect_basic_type<'a>(&self, t: Type<'a>) -> Result<BasicTypeEnum<'a>, CompileError> {
        match t {
            Type::Basic(ty) => Ok(ty),
            Type::Struct(struct_type) => Ok(struct_type.as_basic_type_enum()),
            _ => Err(CompileError::UnsupportedFeature(
                "Expected basic type, got non-basic type (e.g., function type)".to_string(),
                self.get_current_span(),
            )),
        }
    }

    /// Parse comma-separated types from a string, handling nested generics
    pub fn parse_comma_separated_types(&self, type_str: &str) -> Vec<AstType> {
        crate::parser::parse_type_args_from_string(type_str).unwrap_or_default()
    }

    /// Parse a single type string into an AstType
    pub fn parse_type_string(&self, type_str: &str) -> AstType {
        crate::parser::parse_type_from_string(type_str).unwrap_or(AstType::I32)
    }

    pub fn register_struct_type(
        &mut self,
        struct_def: &ast::StructDefinition,
    ) -> Result<(), CompileError> {
        let mut field_types = Vec::new();
        let mut fields = HashMap::new();

        for (index, field) in struct_def.fields.iter().enumerate() {
            let llvm_type = match &field.type_ {
                AstType::I8 => self.context.i8_type().as_basic_type_enum(),
                AstType::I16 => self.context.i16_type().as_basic_type_enum(),
                AstType::I32 => self.context.i32_type().as_basic_type_enum(),
                AstType::I64 => self.context.i64_type().as_basic_type_enum(),
                AstType::U8 => self.context.i8_type().as_basic_type_enum(),
                AstType::U16 => self.context.i16_type().as_basic_type_enum(),
                AstType::U32 => self.context.i32_type().as_basic_type_enum(),
                AstType::U64 => self.context.i64_type().as_basic_type_enum(),
                AstType::Usize => self.context.i64_type().as_basic_type_enum(),
                AstType::F32 => self.context.f32_type().as_basic_type_enum(),
                AstType::F64 => self.context.f64_type().as_basic_type_enum(),
                AstType::Bool => self.context.bool_type().as_basic_type_enum(),
                AstType::StaticLiteral | AstType::StaticString => self
                    .context
                    .ptr_type(AddressSpace::default())
                    .as_basic_type_enum(),
                AstType::Struct { name, .. } if StdlibTypeRegistry::is_string_type(name) => self
                    .context
                    .ptr_type(AddressSpace::default())
                    .as_basic_type_enum(),
                AstType::Void => {
                    return Err(CompileError::TypeError(
                        "Void type not allowed in struct fields".to_string(),
                        None,
                    ))
                }
                t if t.is_ptr_type() => {
                    // Ptr<T> and MutPtr<T> are enums with { i64 discriminant, ptr payload } (16 bytes)
                    // RawPtr<T> is just a plain pointer (8 bytes)
                    if t.is_raw_ptr() {
                        self.context
                            .ptr_type(AddressSpace::default())
                            .as_basic_type_enum()
                    } else {
                        // Ptr<T> and MutPtr<T> are enums: { i64 discriminant, ptr payload }
                        self.context.struct_type(
                            &[
                                self.context.i64_type().into(),
                                self.context.ptr_type(AddressSpace::default()).into(),
                            ],
                            false,
                        ).as_basic_type_enum()
                    }
                }
                AstType::Generic { name, .. } => {
                    if let Some(struct_info) = self.struct_types.get(name) {
                        struct_info.llvm_type.as_basic_type_enum()
                    } else {
                        self.context
                            .ptr_type(AddressSpace::default())
                            .as_basic_type_enum()
                    }
                }
                AstType::Struct { name, .. } => {
                    if let Some(struct_info) = self.struct_types.get(name) {
                        struct_info.llvm_type.as_basic_type_enum()
                    } else {
                        return Err(CompileError::TypeError(
                            format!("Struct '{}' not yet registered. This may be a forward reference issue. Structs should be defined before use, or the typechecker should resolve Generic types to Struct types.", name),
                            None
                        ));
                    }
                }
                AstType::FunctionPointer { .. } => self
                    .context
                    .ptr_type(AddressSpace::default())
                    .as_basic_type_enum(),
                _ => {
                    return Err(CompileError::TypeError(
                        format!("Unsupported type in struct: {:?}", field.type_),
                        None,
                    ))
                }
            };

            field_types.push(llvm_type);
            fields.insert(field.name.clone(), (index, field.type_.clone()));
        }

        let struct_type = self.context.struct_type(&field_types, false);

        let struct_info = StructTypeInfo {
            llvm_type: struct_type,
            fields,
        };

        self.struct_types
            .insert(struct_def.name.clone(), struct_info);

        Ok(())
    }

    /// Try to get a struct type, registering from stdlib if not found locally
    pub fn ensure_struct_type(&mut self, name: &str) -> Result<bool, CompileError> {
        // Already registered locally
        if self.struct_types.contains_key(name) {
            return Ok(true);
        }

        // Try to get from stdlib
        let registry = crate::stdlib_types::stdlib_types();
        if let Some(struct_def) = registry.get_struct_definition(name) {
            // Clone the struct_def to avoid borrow issues
            let struct_def = struct_def.clone();
            self.register_struct_type(&struct_def)?;
            return Ok(true);
        }

        Ok(false)
    }

    pub fn register_enum_type(
        &mut self,
        enum_def: &ast::EnumDefinition,
    ) -> Result<(), CompileError> {
        let mut variant_indices = HashMap::new();
        let mut max_payload_size = 0u32;
        let mut has_payloads = false;

        for (index, variant) in enum_def.variants.iter().enumerate() {
            variant_indices.insert(variant.name.clone(), index as u64);

            if let Some(payload_type) = &variant.payload {
                if !matches!(payload_type, AstType::Void) {
                    has_payloads = true;
                    let payload_size = match payload_type {
                        AstType::I8 | AstType::U8 | AstType::Bool => 8,
                        AstType::I16 | AstType::U16 => 16,
                        AstType::I32 | AstType::U32 | AstType::F32 => 32,
                        AstType::I64 | AstType::U64 | AstType::F64 | AstType::Usize => 64,
                        AstType::StaticLiteral | AstType::StaticString => 64,
                        AstType::Struct { name, .. } if StdlibTypeRegistry::is_string_type(name) => 64,
                        t if t.is_ptr_type() => 64,
                        AstType::Struct { .. } | AstType::Generic { .. } => 64,
                        AstType::Void => 0,
                        _ => 64,
                    };
                    max_payload_size = max_payload_size.max(payload_size);
                }
            }
        }

        let enum_struct_type = if has_payloads {
            let ptr_type = self.context.ptr_type(AddressSpace::default());

            self.context
                .struct_type(&[self.context.i64_type().into(), ptr_type.into()], false)
        } else {
            self.context
                .struct_type(&[self.context.i64_type().into()], false)
        };

        let enum_info = symbols::EnumInfo {
            llvm_type: enum_struct_type,
            variant_indices,
            variants: enum_def.variants.clone(),
        };

        self.symbols
            .insert(&enum_def.name, symbols::Symbol::EnumType(enum_info));

        Ok(())
    }
}
