//! Type representations in the AST

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
    String,
    Void,
    // New pointer types as per spec
    Ptr(Box<AstType>),       // Immutable pointer
    MutPtr(Box<AstType>),    // Mutable pointer  
    RawPtr(Box<AstType>),    // Raw pointer for FFI/unsafe
    Array(Box<AstType>),
    // Vec<T, size> - Fixed-size vector with compile-time size
    Vec {                    
        element_type: Box<AstType>,
        size: usize,         // Always has compile-time size
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
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
    },
    // Enhanced type system support
    Ref(Box<AstType>), // Managed reference
    // TODO: These should be removed once stdlib is updated to use Generic types
    Option(Box<AstType>), // Option<T> - legacy, use Generic instead
    Result {
        ok_type: Box<AstType>,
        err_type: Box<AstType>,
    }, // Result<T, E> - legacy, use Generic instead
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