use crate::ast::AstType;

/// Helper functions for working with types
impl AstType {
    /// Check if this type is numeric (integer or float)
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            AstType::I8
                | AstType::I16
                | AstType::I32
                | AstType::I64
                | AstType::U8
                | AstType::U16
                | AstType::U32
                | AstType::U64
                | AstType::Usize
                | AstType::F32
                | AstType::F64
        )
    }

    /// Check if this type is an integer type
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            AstType::I8
                | AstType::I16
                | AstType::I32
                | AstType::I64
                | AstType::U8
                | AstType::U16
                | AstType::U32
                | AstType::U64
                | AstType::Usize
        )
    }

    /// Check if this type is a floating-point type
    pub fn is_float(&self) -> bool {
        matches!(self, AstType::F32 | AstType::F64)
    }

    /// Check if this type is a signed integer
    #[allow(dead_code)]
    pub fn is_signed_integer(&self) -> bool {
        matches!(
            self,
            AstType::I8 | AstType::I16 | AstType::I32 | AstType::I64
        )
    }

    /// Check if this type is an unsigned integer
    pub fn is_unsigned_integer(&self) -> bool {
        matches!(
            self,
            AstType::U8 | AstType::U16 | AstType::U32 | AstType::U64 | AstType::Usize
        )
    }

    /// Get the size of the type in bits
    pub fn bit_size(&self) -> Option<usize> {
        match self {
            AstType::I8 | AstType::U8 => Some(8),
            AstType::I16 | AstType::U16 => Some(16),
            AstType::I32 | AstType::U32 | AstType::F32 => Some(32),
            AstType::I64 | AstType::U64 | AstType::F64 | AstType::Usize => Some(64), // usize is 64-bit on 64-bit platforms
            AstType::Bool => Some(1),
            _ => None,
        }
    }

    /// Get a default value for initialization
    #[allow(dead_code)]
    pub fn default_value(&self) -> String {
        match self {
            AstType::I8 | AstType::I16 | AstType::I32 | AstType::I64 => "0".to_string(),
            AstType::U8 | AstType::U16 | AstType::U32 | AstType::U64 | AstType::Usize => "0".to_string(),
            AstType::F32 | AstType::F64 => "0.0".to_string(),
            AstType::Bool => "false".to_string(),
            AstType::Struct { name, .. } if name == "String" => "\"\"".to_string(),
            AstType::Ptr(_) | AstType::MutPtr(_) | AstType::RawPtr(_) => "null".to_string(),
            _ => "null".to_string(),
        }
    }

    /// Check if this type is a pointer type
    #[allow(dead_code)]
    pub fn is_pointer(&self) -> bool {
        matches!(
            self,
            AstType::Ptr(_) | AstType::MutPtr(_) | AstType::RawPtr(_)
        )
    }

    /// Get the pointed-to type if this is a pointer type
    #[allow(dead_code)]
    pub fn pointee_type(&self) -> Option<&AstType> {
        match self {
            AstType::Ptr(inner) | AstType::MutPtr(inner) | AstType::RawPtr(inner) => Some(inner),
            _ => None,
        }
    }

    /// Check if this is a mutable pointer type
    #[allow(dead_code)]
    pub fn is_mutable_pointer(&self) -> bool {
        matches!(self, AstType::MutPtr(_))
    }

    /// Check if this is a raw/unsafe pointer type
    #[allow(dead_code)]
    pub fn is_raw_pointer(&self) -> bool {
        matches!(self, AstType::RawPtr(_))
    }

    /// Convert between pointer types while preserving the pointee type
    #[allow(dead_code)]
    pub fn as_pointer_type(&self, pointer_kind: PointerKind) -> Option<AstType> {
        self.pointee_type().map(|inner| match pointer_kind {
            PointerKind::Immutable => AstType::Ptr(Box::new(inner.clone())),
            PointerKind::Mutable => AstType::MutPtr(Box::new(inner.clone())),
            PointerKind::Raw => AstType::RawPtr(Box::new(inner.clone())),
        })
    }
}

/// Enum for different pointer kinds
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum PointerKind {
    Immutable, // Ptr<T>
    Mutable,   // MutPtr<T>
    Raw,       // RawPtr<T>
}
