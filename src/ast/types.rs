//! Type representations in the AST

use std::fmt;

/// Resolve String type from stdlib - returns the struct type definition
/// String is defined in stdlib/string.zen as:
/// struct String {
///     data: Ptr<u8>
///     len: u64
///     capacity: u64
///     allocator: Allocator
/// }
pub fn resolve_string_struct_type() -> AstType {
    AstType::Struct {
        name: "String".to_string(),
        fields: vec![
            ("data".to_string(), AstType::Ptr(Box::new(AstType::U8))),
            ("len".to_string(), AstType::U64),
            ("capacity".to_string(), AstType::U64),
            ("allocator".to_string(), AstType::Generic {
                name: "Allocator".to_string(),
                type_args: vec![],
            }),
        ],
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AstType {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    Usize,
    F32,
    F64,
    Bool,
    StaticLiteral, // Internal: Compiler-known string literals (LLVM use only)
    StaticString,  // User-facing: Static strings (compile-time, immutable, no allocator)
    // String is defined in stdlib/string.zen as a struct, not a compiler primitive
    // Use resolve_string_struct_type() helper to get the struct type
    Void,
    // New pointer types as per spec
    Ptr(Box<AstType>),    // Immutable pointer
    MutPtr(Box<AstType>), // Mutable pointer
    RawPtr(Box<AstType>), // Raw pointer for FFI/unsafe
    Array(Box<AstType>),
    // Vec<T, size> - Fixed-size vector with compile-time size
    Vec {
        element_type: Box<AstType>,
        size: usize, // Always has compile-time size
    },
    // DynVec<T> or DynVec<T1, T2, ...> - Dynamic vector with allocator
    DynVec {
        element_types: Vec<AstType>, // Multiple types for mixed variant vectors
        allocator_type: Option<Box<AstType>>, // Optional allocator type
    },
    FixedArray {
        element_type: Box<AstType>,
        size: usize,
    },
    Function {
        args: Vec<AstType>,
        return_type: Box<AstType>,
    },
    FunctionPointer {
        param_types: Vec<AstType>,
        return_type: Box<AstType>,
    },
    Struct {
        name: String,
        fields: Vec<(String, AstType)>,
    },
    #[allow(dead_code)]
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
    },
    // Enhanced type system support
    Ref(Box<AstType>), // Managed reference
    // NOTE: Option<T> and Result<T, E> are now defined in stdlib/core/option.zen and stdlib/core/result.zen
    // They should be referenced as Generic { name: "Option", type_args: [T] } or Generic { name: "Result", type_args: [T, E] }
    Range {
        start_type: Box<AstType>,
        end_type: Box<AstType>,
        inclusive: bool,
    }, // Range types for .. and ..=
    // For generic types (future)
    Generic {
        name: String,
        type_args: Vec<AstType>,
    },
    // For enum type references (e.g., when MyOption is used as an identifier)
    #[allow(dead_code)]
    EnumType {
        name: String,
    },
    // For stdlib module references (e.g., math, io when imported)
    StdModule,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumVariant {
    pub name: String,
    pub payload: Option<AstType>, // Some(type) for variants with data, None for unit variants
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeParameter {
    pub name: String,
    pub constraints: Vec<TraitConstraint>, // Trait bounds like T: Geometric + Serializable
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitConstraint {
    pub trait_name: String,
}

// Display implementation for generating clean type names
impl fmt::Display for AstType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AstType::I8 => write!(f, "i8"),
            AstType::I16 => write!(f, "i16"),
            AstType::I32 => write!(f, "i32"),
            AstType::I64 => write!(f, "i64"),
            AstType::U8 => write!(f, "u8"),
            AstType::U16 => write!(f, "u16"),
            AstType::U32 => write!(f, "u32"),
            AstType::U64 => write!(f, "u64"),
            AstType::Usize => write!(f, "usize"),
            AstType::F32 => write!(f, "f32"),
            AstType::F64 => write!(f, "f64"),
            AstType::Bool => write!(f, "bool"),
            AstType::StaticLiteral => write!(f, "StaticString"), // Display as StaticString to users
            AstType::StaticString => write!(f, "StaticString"),
            // String is now a struct type from stdlib, resolved via Struct variant
            AstType::Void => write!(f, "void"),
            AstType::Ptr(inner) => write!(f, "Ptr<{}>", inner),
            AstType::MutPtr(inner) => write!(f, "MutPtr<{}>", inner),
            AstType::RawPtr(inner) => write!(f, "RawPtr<{}>", inner),
            AstType::Array(inner) => write!(f, "Array<{}>", inner),
            AstType::Vec { element_type, size } => write!(f, "Vec<{}, {}>", element_type, size),
            AstType::DynVec { element_types, .. } => {
                write!(f, "DynVec<")?;
                for (i, t) in element_types.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", t)?;
                }
                write!(f, ">")
            }
            AstType::FixedArray { element_type, size } => {
                write!(f, "[{}; {}]", element_type, size)
            }
            AstType::Function { args, return_type } => {
                write!(f, "(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ") {}", return_type)
            }
            AstType::FunctionPointer { param_types, return_type } => {
                write!(f, "fn(")?;
                for (i, param) in param_types.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, ") {}", return_type)
            }
            AstType::Struct { name, .. } => write!(f, "{}", name),
            AstType::Enum { name, .. } => write!(f, "{}", name),
            AstType::Ref(inner) => write!(f, "Ref<{}>", inner),
            // Option and Result are now Generic types - handled above
            AstType::Range { start_type, inclusive, .. } => {
                if *inclusive {
                    write!(f, "Range<{}..={}>", start_type, start_type)
                } else {
                    write!(f, "Range<{}..{}>", start_type, start_type)
                }
            }
            AstType::Generic { name, type_args } => {
                write!(f, "{}", name)?;
                if !type_args.is_empty() {
                    write!(f, "<")?;
                    for (i, arg) in type_args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            }
            AstType::EnumType { name } => write!(f, "{}", name),
            AstType::StdModule => write!(f, "std"),
        }
    }
}
