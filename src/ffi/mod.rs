//! FFI Module - Foreign Function Interface with Builder Pattern
//! Implements the Language Spec v1.1.0 requirements for safe C interop

pub mod builder;
pub mod errors;
pub mod library;
pub mod platform;
pub mod stats;
pub mod type_helpers;
pub mod types;

// Re-export main types for convenient access
pub use builder::{CallbackDefinition, LibBuilder, ValidationRule};
pub use errors::FFIError;
pub use library::Library;
pub use platform::{Platform, PlatformConfig};
pub use stats::CallStatistics;
pub use types::{CallingConvention, FnSignature, FunctionSafety, LoadFlags, TypeMapping, TypeMarshaller};

// Re-export type helpers as a module
pub use type_helpers as types_helpers;

/// FFI builder for safe C interop
pub struct FFI;

impl FFI {
    /// Create a new library builder
    pub fn lib(name: impl Into<String>) -> LibBuilder {
        LibBuilder::new(name.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_builder_pattern() {
        let lib = FFI::lib("sqlite3")
            .path("/usr/lib/libsqlite3.so")
            .function(
                "sqlite3_open",
                FnSignature::new(
                    vec![
                        type_helpers::string(),
                        type_helpers::raw_ptr(type_helpers::raw_ptr(type_helpers::void())),
                    ],
                    type_helpers::i32(),
                )
                .with_safety(FunctionSafety::Unsafe),
            )
            .function(
                "sqlite3_close",
                FnSignature::new(vec![type_helpers::raw_ptr(type_helpers::void())], type_helpers::i32())
                    .with_safety(FunctionSafety::Unsafe),
            )
            .constant("SQLITE_OK", type_helpers::i32())
            .calling_convention(CallingConvention::C)
            .safety_checks(true)
            .build();

        assert!(lib.is_ok());
        let lib = lib.unwrap();
        assert_eq!(lib.name(), "sqlite3");
        assert_eq!(lib.path(), std::path::Path::new("/usr/lib/libsqlite3.so"));
        assert_eq!(lib.functions().len(), 2);
        assert_eq!(lib.constants().len(), 1);
        assert_eq!(lib.calling_convention(), CallingConvention::C);
        assert!(lib.safety_checks());
    }

    #[test]
    fn test_ffi_builder_enhanced() {
        let lib = FFI::lib("custom_lib")
            .path("/tmp/libcustom.so")
            .version("1.2.3")
            .lazy_loading(true)
            .search_path("/custom/path")
            .alias("zen_func", "c_func_impl")
            .struct_def(
                "Point",
                vec![
                    ("x".to_string(), type_helpers::f64()),
                    ("y".to_string(), type_helpers::f64()),
                ],
            )
            .enum_def("Status", vec!["Ok".to_string(), "Error".to_string()])
            .build();

        assert!(lib.is_ok());
        let lib = lib.unwrap();
        assert_eq!(lib.name(), "custom_lib");
        assert!(lib.lazy_loading());
        assert_eq!(lib.version_requirement(), Some("1.2.3"));
        assert_eq!(lib.type_mappings().len(), 2);
    }

    #[test]
    fn test_default_lib_path() {
        let path = LibBuilder::default_lib_path("test");

        #[cfg(target_os = "linux")]
        assert_eq!(path, std::path::PathBuf::from("libtest.so"));

        #[cfg(target_os = "macos")]
        assert_eq!(path, std::path::PathBuf::from("libtest.dylib"));

        #[cfg(target_os = "windows")]
        assert_eq!(path, std::path::PathBuf::from("test.dll"));
    }

    #[test]
    fn test_ffi_builder_advanced_features() {
        let lib = FFI::lib("math")
            .path("/tmp/libmath.so")
            .functions_with_prefix(
                "math",
                vec![
                    ("sin", FnSignature::new(vec![type_helpers::f64()], type_helpers::f64())),
                    ("cos", FnSignature::new(vec![type_helpers::f64()], type_helpers::f64())),
                    ("tan", FnSignature::new(vec![type_helpers::f64()], type_helpers::f64())),
                ],
            )
            .build();

        assert!(lib.is_ok());
        let lib = lib.unwrap();
        assert!(lib.functions().contains_key("math_sin"));
        assert!(lib.functions().contains_key("math_cos"));
        assert!(lib.functions().contains_key("math_tan"));
    }

    #[test]
    fn test_opaque_types() {
        let lib = FFI::lib("opaque_test")
            .path("/tmp/libopaque.so")
            .opaque_type("sqlite3")
            .opaque_type("FILE")
            .build();

        assert!(lib.is_ok());
        let lib = lib.unwrap();
        assert_eq!(lib.type_mappings().len(), 2);
        assert!(lib.type_mappings().contains_key("sqlite3"));
        assert!(lib.type_mappings().contains_key("FILE"));
    }

    #[test]
    fn test_c_declaration_parsing() {
        let lib = FFI::lib("c_test")
            .path("/tmp/libc_test.so")
            .function_from_c_decl("int printf(const char *format)")
            .function_from_c_decl("void exit(int status)")
            .function_from_c_decl("double sqrt(double x)")
            .build();

        assert!(lib.is_ok());
        let lib = lib.unwrap();

        if let Some(printf_sig) = lib.functions().get("printf") {
            assert_eq!(printf_sig.returns, type_helpers::i32());
            assert_eq!(printf_sig.params.len(), 1);
        }

        if let Some(exit_sig) = lib.functions().get("exit") {
            assert_eq!(exit_sig.returns, type_helpers::void());
        }

        if let Some(sqrt_sig) = lib.functions().get("sqrt") {
            assert_eq!(sqrt_sig.returns, type_helpers::f64());
        }
    }

    #[test]
    fn test_metadata_storage() {
        let lib = FFI::lib("metadata_test")
            .path("/tmp/libmeta.so")
            .metadata("version", "1.2.3")
            .metadata("author", "Zen Team")
            .metadata("license", "MIT")
            .build();

        assert!(lib.is_ok());
    }
}
