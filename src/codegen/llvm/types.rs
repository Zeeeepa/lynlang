use super::{LLVMCompiler, Type, symbols};
use crate::ast::AstType;
use crate::error::CompileError;
use inkwell::{
    types::{BasicType, BasicTypeEnum, BasicMetadataTypeEnum},
    AddressSpace,
};

impl<'ctx> LLVMCompiler<'ctx> {
    pub fn to_llvm_type(&mut self, type_: &AstType) -> Result<Type<'ctx>, CompileError> {
        // Debug empty Generic types
        if let AstType::Generic { name, .. } = type_ {
            if name.is_empty() {
                eprintln!("DEBUG: Empty generic type encountered: {:?}", type_);
            }
        }
        
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
            AstType::String => Ok(Type::Basic(self.context.ptr_type(AddressSpace::default()).into())),
            AstType::Void => Ok(Type::Void),
            AstType::Ptr(inner) => {
                let inner_type = self.to_llvm_type(inner)?;
                match inner_type {
                    Type::Basic(_) => Ok(Type::Basic(self.context.ptr_type(AddressSpace::default()).into())),
                    Type::Struct(_) => Ok(Type::Basic(self.context.ptr_type(AddressSpace::default()).into())),
                    Type::Void => {
                        // For void pointers, use i8* as the LLVM representation
                        Ok(Type::Basic(self.context.ptr_type(AddressSpace::default()).into()))
                    },
                    _ => Err(CompileError::UnsupportedFeature("Unsupported pointer type".to_string(), None)),
                }
            },
            AstType::MutPtr(inner) => {
                // MutPtr<T> is the same as Ptr<T> in LLVM - mutability is tracked at language level
                let inner_type = self.to_llvm_type(inner)?;
                match inner_type {
                    Type::Basic(_) => Ok(Type::Basic(self.context.ptr_type(AddressSpace::default()).into())),
                    Type::Struct(_) => Ok(Type::Basic(self.context.ptr_type(AddressSpace::default()).into())),
                    Type::Void => {
                        // For void pointers, use i8* as the LLVM representation
                        Ok(Type::Basic(self.context.ptr_type(AddressSpace::default()).into()))
                    },
                    _ => Err(CompileError::UnsupportedFeature("Unsupported mutable pointer type".to_string(), None)),
                }
            },
            AstType::RawPtr(inner) => {
                // RawPtr<T> is also the same as regular pointers in LLVM - safety is tracked at language level
                let inner_type = self.to_llvm_type(inner)?;
                match inner_type {
                    Type::Basic(_) => Ok(Type::Basic(self.context.ptr_type(AddressSpace::default()).into())),
                    Type::Struct(_) => Ok(Type::Basic(self.context.ptr_type(AddressSpace::default()).into())),
                    Type::Void => {
                        // For void pointers, use i8* as the LLVM representation
                        Ok(Type::Basic(self.context.ptr_type(AddressSpace::default()).into()))
                    },
                    _ => Err(CompileError::UnsupportedFeature("Unsupported raw pointer type".to_string(), None)),
                }
            },
            AstType::Struct { name, fields: _ } => {
                let struct_info = self.struct_types.get(name)
                    .ok_or_else(|| CompileError::TypeError(format!("Undefined struct type: {}", name), None))?;
                Ok(Type::Struct(struct_info.llvm_type))
            },
            AstType::Array(inner) => {
                let inner_type = self.to_llvm_type(inner)?;
                match inner_type {
                    Type::Basic(basic_type) => Ok(Type::Basic(basic_type)), // Dynamic array (pointer)
                    _ => Ok(Type::Basic(self.context.i8_type().array_type(0).into())), // Default to array of bytes
                }
            },
            AstType::FixedArray { element_type, size } => {
                let elem_type = self.to_llvm_type(element_type)?;
                match elem_type {
                    Type::Basic(basic_type) => {
                        // Create an LLVM array type with the specified size
                        let array_type = match basic_type {
                            BasicTypeEnum::IntType(int_type) => int_type.array_type(*size as u32).into(),
                            BasicTypeEnum::FloatType(float_type) => float_type.array_type(*size as u32).into(),
                            BasicTypeEnum::PointerType(ptr_type) => ptr_type.array_type(*size as u32).into(),
                            BasicTypeEnum::StructType(struct_type) => struct_type.array_type(*size as u32).into(),
                            BasicTypeEnum::ArrayType(arr_type) => arr_type.array_type(*size as u32).into(),
                            BasicTypeEnum::VectorType(vec_type) => vec_type.array_type(*size as u32).into(),
                            BasicTypeEnum::ScalableVectorType(_) => {
                                // For now, use a default array of i8
                                self.context.i8_type().array_type(*size as u32).into()
                            }
                        };
                        Ok(Type::Basic(array_type))
                    }
                    _ => Ok(Type::Basic(self.context.i8_type().array_type(*size as u32).into())), // Default to array of bytes
                }
            },
            AstType::Function { args, return_type } => {
                let return_llvm_type = self.to_llvm_type(return_type)?;
                let arg_llvm_types: Result<Vec<BasicTypeEnum<'ctx>>, CompileError> = args.iter().map(|arg| {
                    let arg_type = self.to_llvm_type(arg)?;
                    match arg_type {
                        Type::Basic(basic_type) => Ok(basic_type),
                        _ => Ok(self.context.i64_type().into()), // Default to i64 for complex types
                    }
                }).collect();
                let arg_llvm_types = arg_llvm_types?;
                
                // Convert BasicTypeEnum to BasicMetadataTypeEnum for function signatures
                let arg_metadata_types: Vec<BasicMetadataTypeEnum<'ctx>> = arg_llvm_types.iter().map(|ty| (*ty).into()).collect();
                
                let function_type = match return_llvm_type {
                    Type::Basic(basic_type) => basic_type.fn_type(&arg_metadata_types, false),
                    _ => self.context.i64_type().fn_type(&arg_metadata_types, false),
                };
                Ok(Type::Function(function_type))
            },
            AstType::FunctionPointer { param_types, return_type } => {
                // Function pointers are represented as pointers to functions
                let return_llvm_type = self.to_llvm_type(return_type)?;
                let arg_llvm_types: Result<Vec<BasicTypeEnum<'ctx>>, CompileError> = param_types.iter().map(|arg| {
                    let arg_type = self.to_llvm_type(arg)?;
                    match arg_type {
                        Type::Basic(basic_type) => Ok(basic_type),
                        _ => Ok(self.context.i64_type().into()), // Default to i64 for complex types
                    }
                }).collect();
                let arg_llvm_types = arg_llvm_types?;
                
                // Convert BasicTypeEnum to BasicMetadataTypeEnum for function signatures
                let arg_metadata_types: Vec<BasicMetadataTypeEnum<'ctx>> = arg_llvm_types.iter().map(|ty| (*ty).into()).collect();
                
                let _function_type = match return_llvm_type {
                    Type::Basic(basic_type) => basic_type.fn_type(&arg_metadata_types, false),
                    Type::Void => self.context.void_type().fn_type(&arg_metadata_types, false),
                    _ => self.context.i64_type().fn_type(&arg_metadata_types, false),
                };
                
                // Return a pointer to the function type
                Ok(Type::Basic(self.context.ptr_type(AddressSpace::default()).into()))
            },
            AstType::Enum { name, variants: _ } => {
                // Look up the registered enum type
                if let Some(symbols::Symbol::EnumType(enum_info)) = self.symbols.lookup(name) {
                    Ok(Type::Struct(enum_info.llvm_type))
                } else {
                    // Fallback to a simple tag-only enum if not registered
                    // This should rarely happen as enums should be registered during declaration phase
                    let enum_struct_type = self.context.struct_type(&[
                        self.context.i64_type().into(),  // discriminant/tag only
                    ], false);
                    Ok(Type::Struct(enum_struct_type))
                }
            },
            AstType::Ref(inner) => {
                // Ref<T> is represented as a pointer to T
                let inner_type = self.to_llvm_type(inner)?;
                match inner_type {
                    Type::Basic(basic_type) => Ok(Type::Basic(basic_type)),
                    _ => Ok(Type::Basic(self.context.ptr_type(AddressSpace::default()).into())),
                }
            },
            AstType::Option(inner) => {
                // Option<T> is represented as a pointer to T (null = None, non-null = Some)
                let inner_type = self.to_llvm_type(inner)?;
                match inner_type {
                    Type::Basic(basic_type) => Ok(Type::Basic(basic_type)),
                    _ => Ok(Type::Basic(self.context.ptr_type(AddressSpace::default()).into())),
                }
            },
            AstType::Result { ok_type, err_type } => {
                // Result<T, E> is represented as a tagged union struct
                // Struct layout: { tag: i64, payload: max(sizeof(T), sizeof(E)) }
                let ok_llvm = self.to_llvm_type(ok_type)?;
                let err_llvm = self.to_llvm_type(err_type)?;
                
                // For now, create a struct that can hold either type
                // We'll use i64 for both to ensure consistent size
                let result_struct = self.context.struct_type(&[
                    self.context.i64_type().into(),  // tag (0 = Ok, 1 = Err)
                    self.context.i64_type().into(),  // payload (simplified for now)
                ], false);
                
                Ok(Type::Struct(result_struct))
            },
            AstType::Range { start_type, end_type, inclusive: _ } => {
                // Range is represented as a struct with start and end values
                let _start_type = self.to_llvm_type(start_type)?;
                let _end_type = self.to_llvm_type(end_type)?;
                // For now, just use i64 for both start and end
                let range_struct = self.context.struct_type(&[
                    self.context.i64_type().into(),
                    self.context.i64_type().into(),
                ], false);
                Ok(Type::Struct(range_struct))
            },
            AstType::Vec { element_type, size } => {
                // Vec<T, size> - Fixed-size vector as struct containing array and length
                let elem_llvm_type = self.to_llvm_type(element_type)?;
                match elem_llvm_type {
                    Type::Basic(basic_type) => {
                        // Create struct: { [T; size], usize }
                        let array_type = basic_type.array_type(*size as u32);
                        let len_type = self.context.i64_type(); // Use i64 for length
                        let vec_struct = self.context.struct_type(&[
                            array_type.into(),  // data: [T; size]
                            len_type.into(),    // len: usize (current length)
                        ], false);
                        Ok(Type::Struct(vec_struct))
                    },
                    _ => {
                        // Fallback to array of bytes
                        let array_type = self.context.i8_type().array_type(*size as u32);
                        let len_type = self.context.i64_type();
                        let vec_struct = self.context.struct_type(&[
                            array_type.into(),
                            len_type.into(),
                        ], false);
                        Ok(Type::Struct(vec_struct))
                    }
                }
            },
            AstType::DynVec { element_types, allocator_type: _ } => {
                // DynVec<T> - Dynamic vector as struct containing pointer, length, and capacity
                // For mixed variant types, use a union or tagged union approach
                if element_types.len() == 1 {
                    // Single type DynVec: { ptr, len, capacity }
                    let ptr_type = self.context.ptr_type(AddressSpace::default());
                    let len_type = self.context.i64_type();
                    let cap_type = self.context.i64_type();
                    let dynvec_struct = self.context.struct_type(&[
                        ptr_type.into(),    // data: Ptr<T>
                        len_type.into(),    // len: usize
                        cap_type.into(),    // capacity: usize
                    ], false);
                    Ok(Type::Struct(dynvec_struct))
                } else {
                    // Mixed variant DynVec: { ptr, len, capacity, discriminants }
                    let ptr_type = self.context.ptr_type(AddressSpace::default());
                    let len_type = self.context.i64_type();
                    let cap_type = self.context.i64_type();
                    let discriminant_ptr = self.context.ptr_type(AddressSpace::default()); // Pointer to discriminant array
                    let dynvec_struct = self.context.struct_type(&[
                        ptr_type.into(),           // data: Ptr<union> 
                        len_type.into(),           // len: usize
                        cap_type.into(),           // capacity: usize
                        discriminant_ptr.into(),   // discriminants: Ptr<u8> for variant tracking
                    ], false);
                    Ok(Type::Struct(dynvec_struct))
                }
            },
            AstType::Generic { name, type_args } => {
                // Check if this is actually a user-defined struct type
                if let Some(struct_info) = self.struct_types.get(name) {
                    Ok(Type::Struct(struct_info.llvm_type))
                } else if let Some(symbols::Symbol::EnumType(enum_info)) = self.symbols.lookup(name) {
                    // Check if it's an enum type that was parsed as Generic
                    Ok(Type::Struct(enum_info.llvm_type))
                } else {
                    // After monomorphization, we should not encounter generic types
                    // If we do, it means monomorphization failed to resolve this type
                    eprintln!("DEBUG: Unresolved generic type: name='{}', type_args={:?}", name, type_args);
                    Err(CompileError::InternalError(
                        format!("Unresolved generic type '{}' found after monomorphization. This is a compiler bug.", name),
                        None
                    ))
                }
            },
            AstType::EnumType { name } => {
                // EnumType is used when an enum is referenced as a type constructor
                // Look up the registered enum type
                if let Some(symbols::Symbol::EnumType(enum_info)) = self.symbols.lookup(name) {
                    Ok(Type::Struct(enum_info.llvm_type))
                } else {
                    // Fallback to a default enum structure if not registered
                    let enum_struct_type = self.context.struct_type(&[
                        self.context.i64_type().into(),  // discriminant/tag
                        self.context.i64_type().into(),  // payload (simplified)
                    ], false);
                    Ok(Type::Struct(enum_struct_type))
                }
            },
            AstType::StdModule => {
                // StdModule is a marker type for imported stdlib modules (like math, io)
                // It's represented as an i64 in LLVM (storing module identifier)
                Ok(Type::Basic(self.context.i64_type().into()))
            },
        };
        result
    }
    pub fn expect_basic_type<'a>(&self, t: Type<'a>) -> Result<BasicTypeEnum<'a>, CompileError> {
        match t {
            Type::Basic(ty) => Ok(ty),
            Type::Struct(struct_type) => Ok(struct_type.as_basic_type_enum()),
            _ => Err(CompileError::UnsupportedFeature(
                "Expected basic type, got non-basic type (e.g., function type)".to_string(),
                None,
            )),
        }
    }
} 