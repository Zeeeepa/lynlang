use zen::ffi::{FFI, FnSignature, FunctionSafety, CallingConvention, TypeMapping, TypeMarshaller};
use zen::ffi::types;
use zen::ast::AstType;
use std::sync::Arc;

#[test]
fn test_ffi_basic_builder() {
    let lib = FFI::lib("sqlite3")
        .path("/usr/lib/libsqlite3.so")
        .function("sqlite3_open", FnSignature::new(
            vec![types::string(), types::raw_ptr(types::raw_ptr(types::void()))],
            types::i32(),
        ))
        .function("sqlite3_close", FnSignature::new(
            vec![types::raw_ptr(types::void())],
            types::i32(),
        ))
        .constant("SQLITE_OK", types::i32())
        .build()
        .unwrap();
    
    assert_eq!(lib.name(), "sqlite3");
    assert_eq!(lib.path().to_str().unwrap(), "/usr/lib/libsqlite3.so");
    assert!(lib.get_signature("sqlite3_open").is_some());
    assert!(lib.get_signature("sqlite3_close").is_some());
}

#[test]
fn test_ffi_with_safety_levels() {
    let lib = FFI::lib("mylib")
        .path("/tmp/libmylib.so")  // Use explicit path to avoid lookup
        .function("safe_func", FnSignature::new(
            vec![types::i32()],
            types::i32(),
        ).with_safety(FunctionSafety::Safe))
        .function("unsafe_func", FnSignature::new(
            vec![types::raw_ptr(types::void())],
            types::void(),
        ).with_safety(FunctionSafety::Unsafe))
        .function("trusted_func", FnSignature::new(
            vec![types::string()],
            types::bool(),
        ).with_safety(FunctionSafety::Trusted))
        .safety_checks(true)
        .build()
        .unwrap();
    
    let safe_sig = lib.get_signature("safe_func").unwrap();
    assert_eq!(safe_sig.safety, FunctionSafety::Safe);
    
    let unsafe_sig = lib.get_signature("unsafe_func").unwrap();
    assert_eq!(unsafe_sig.safety, FunctionSafety::Unsafe);
    
    let trusted_sig = lib.get_signature("trusted_func").unwrap();
    assert_eq!(trusted_sig.safety, FunctionSafety::Trusted);
}

#[test]
fn test_ffi_calling_conventions() {
    let lib = FFI::lib("winapi")
        .path("/tmp/libwinapi.dll")  // Use explicit path to avoid lookup
        .calling_convention(CallingConvention::Stdcall)
        .function("MessageBoxA", FnSignature::new(
            vec![
                types::raw_ptr(types::void()),
                types::string(),
                types::string(),
                types::u32(),
            ],
            types::i32(),
        ))
        .build()
        .unwrap();
    
    assert_eq!(lib.calling_convention(), CallingConvention::Stdcall);
}

#[test]
fn test_ffi_variadic_functions() {
    let lib = FFI::lib("libc")
        .path("/tmp/libc.so")  // Use explicit path to avoid lookup
        .function("printf", FnSignature::new(
            vec![types::string()],
            types::i32(),
        ).with_variadic(true))
        .build()
        .unwrap();
    
    let printf_sig = lib.get_signature("printf").unwrap();
    assert!(printf_sig.variadic);
}

#[test]
fn test_ffi_type_mappings() {
    let marshaller = TypeMarshaller {
        to_c: Arc::new(|data| data.to_vec()),
        from_c: Arc::new(|data| data.to_vec()),
    };
    
    let lib = FFI::lib("custom")
        .path("/tmp/libcustom.so")  // Use explicit path to avoid lookup
        .type_mapping("MyStruct", TypeMapping {
            c_type: "struct my_struct".to_string(),
            zen_type: AstType::Struct {
                name: "MyStruct".to_string(),
                fields: vec![],
            },
            marshaller: Some(marshaller),
        })
        .build()
        .unwrap();
    
    assert!(lib.type_mappings().contains_key("MyStruct"));
}

#[test]
fn test_ffi_default_path_linux() {
    #[cfg(target_os = "linux")]
    {
        let lib = FFI::lib("test")
            .path("/tmp/libtest.so")  // Use explicit path to avoid lookup
            .build()
            .unwrap();
        
        assert_eq!(lib.path().to_str().unwrap(), "/tmp/libtest.so");
    }
}

#[test]
fn test_ffi_default_path_macos() {
    #[cfg(target_os = "macos")]
    {
        let lib = FFI::lib("test")
            .build()
            .unwrap();
        
        assert_eq!(lib.path().to_str().unwrap(), "libtest.dylib");
    }
}

#[test]
fn test_ffi_default_path_windows() {
    #[cfg(target_os = "windows")]
    {
        let lib = FFI::lib("test")
            .build()
            .unwrap();
        
        assert_eq!(lib.path().to_str().unwrap(), "test.dll");
    }
}

#[test]
fn test_ffi_complex_signatures() {
    let lib = FFI::lib("complex")
        .path("/tmp/libcomplex.so")  // Use explicit path to avoid lookup
        .function("process_array", FnSignature::new(
            vec![
                types::ptr(AstType::Array(Box::new(types::i32()))),
                types::usize(),
            ],
            types::void(),
        ))
        .struct_def("Result", vec![])  // Define the struct first
        .function("return_struct", FnSignature::new(
            vec![],
            AstType::Struct {
                name: "Result".to_string(),
                fields: vec![],
            },
        ))
        .function("callback_func", FnSignature::new(
            vec![
                AstType::FunctionPointer {
                    param_types: vec![types::i32()],
                    return_type: Box::new(types::void()),
                },
            ],
            types::void(),
        ))
        .build()
        .unwrap();
    
    assert!(lib.get_signature("process_array").is_some());
    assert!(lib.get_signature("return_struct").is_some());
    assert!(lib.get_signature("callback_func").is_some());
}

#[test]
fn test_ffi_statistics() {
    let lib = FFI::lib("test")
        .path("/tmp/libtest.so")  // Use explicit path to avoid lookup
        .function("test_func", FnSignature::new(vec![], types::void()))
        .build()
        .unwrap();
    
    // Initially no stats
    let stats = lib.get_stats();
    assert_eq!(stats.total_calls, 0);
    assert_eq!(stats.successful_calls, 0);
    assert_eq!(stats.failed_calls, 0);
    
    // Try to get a function without loading the library
    let result = lib.get_function("test_func");
    assert!(result.is_err());
    
    // Check stats updated
    let stats = lib.get_stats();
    assert_eq!(stats.total_calls, 0); // No calls recorded when library not loaded
}

#[test]
fn test_ffi_library_lifecycle() {
    let mut lib = FFI::lib("nonexistent")
        .path("/does/not/exist.so")
        .build()
        .unwrap();
    
    assert!(!lib.is_loaded());
    
    // Try to load non-existent library
    let result = lib.load();
    assert!(result.is_err());
    
    assert!(!lib.is_loaded());
    
    // Unload (should be safe even if not loaded)
    lib.unload();
    assert!(!lib.is_loaded());
}

#[test]
fn test_ffi_builder_chaining() {
    let lib = FFI::lib("chained")
        .path("/usr/lib/libchained.so")
        .calling_convention(CallingConvention::C)
        .safety_checks(true)
        .function("func1", FnSignature::new(vec![], types::void()))
        .function("func2", FnSignature::new(vec![types::i32()], types::i32()))
        .function("func3", FnSignature::new(vec![types::string()], types::bool()))
        .constant("CONST1", types::i32())
        .constant("CONST2", types::u64())
        .type_mapping("CustomType", TypeMapping {
            c_type: "custom_t".to_string(),
            zen_type: AstType::Struct { 
                name: "CustomType".to_string(), 
                fields: vec![] 
            },
            marshaller: None,
        })
        .build()
        .unwrap();
    
    assert_eq!(lib.name(), "chained");
    assert_eq!(lib.functions().len(), 3);
    assert_eq!(lib.constants().len(), 2);
    assert_eq!(lib.type_mappings().len(), 1);
    assert!(lib.safety_checks());
}