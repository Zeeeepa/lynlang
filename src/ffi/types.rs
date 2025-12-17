// FFI type definitions

use crate::ast::AstType;

/// Function signature for FFI
#[derive(Debug, Clone, PartialEq)]
pub struct FnSignature {
    pub params: Vec<AstType>,
    pub returns: AstType,
    pub variadic: bool,
    pub safety: FunctionSafety,
}

/// Function safety level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionSafety {
    Safe,
    Unsafe,
    Trusted,
}

impl FnSignature {
    pub fn new(params: Vec<AstType>, returns: AstType) -> Self {
        Self {
            params,
            returns,
            variadic: false,
            safety: FunctionSafety::Unsafe,
        }
    }

    pub fn with_variadic(mut self, variadic: bool) -> Self {
        self.variadic = variadic;
        self
    }

    pub fn with_safety(mut self, safety: FunctionSafety) -> Self {
        self.safety = safety;
        self
    }
}

/// Type mapping between Zen and C types
#[derive(Debug, Clone)]
pub struct TypeMapping {
    pub c_type: String,
    pub zen_type: AstType,
    pub marshaller: Option<Arc<TypeMarshaller>>,
}

/// Type marshaller for converting between Zen and C representations
pub struct TypeMarshaller {
    pub to_c: Arc<dyn Fn(&[u8]) -> Vec<u8> + Send + Sync>,
    pub from_c: Arc<dyn Fn(&[u8]) -> Vec<u8> + Send + Sync>,
}

impl std::fmt::Debug for TypeMarshaller {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypeMarshaller")
            .field("to_c", &"<function>")
            .field("from_c", &"<function>")
            .finish()
    }
}

/// Calling convention for FFI functions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallingConvention {
    C,
    Stdcall,
    Fastcall,
    Vectorcall,
    Thiscall,
    System,
}

/// Load flags for library loading behavior
#[derive(Debug, Clone)]
pub struct LoadFlags {
    pub lazy_binding: bool,
    pub global_symbols: bool,
    pub local_symbols: bool,
    pub nodelete: bool,
}

impl Default for LoadFlags {
    fn default() -> Self {
        Self {
            lazy_binding: false,
            global_symbols: false,
            local_symbols: true,
            nodelete: false,
        }
    }
}

use std::sync::Arc;
