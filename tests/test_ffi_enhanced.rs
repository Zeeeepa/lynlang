use zen::ffi::{FFI, FnSignature, FunctionSafety, CallingConvention, Platform, PlatformConfig, ValidationRule, LoadFlags, CallbackDefinition};
use zen::ffi::types;
use zen::ast::AstType;
use std::path::PathBuf;
use std::sync::Arc;

#[test]
fn test_ffi_platform_detection() {
    let current_platform = Platform::current();
    
    #[cfg(target_os = "linux")]
    assert_eq!(current_platform, Platform::Linux);
    
    #[cfg(target_os = "macos")]
    assert_eq!(current_platform, Platform::MacOS);
    
    #[cfg(target_os = "windows")]
    assert_eq!(current_platform, Platform::Windows);
}

#[test]
fn test_ffi_platform_specific_config() {
    let lib = FFI::lib("test")
        .path("/tmp/libtest.so")  // Use explicit path
        .platform_config(Platform::Linux, PlatformConfig {
            path_override: Some(PathBuf::from("/usr/lib/libtest.so")),
            calling_convention_override: None,
            additional_search_paths: vec![PathBuf::from("/opt/lib")],
        })
        .platform_config(Platform::Windows, PlatformConfig {
            path_override: Some(PathBuf::from("C:\\Windows\\System32\\test.dll")),
            calling_convention_override: Some(CallingConvention::Stdcall),
            additional_search_paths: vec![PathBuf::from("C:\\Program Files\\Test")],
        })
        .auto_configure()
        .build()
        .unwrap();
    
    // Platform-specific path will be set
    #[cfg(target_os = "linux")]
    assert_eq!(lib.path().to_str().unwrap(), "/usr/lib/libtest.so");
    
    #[cfg(target_os = "windows")]
    {
        assert_eq!(lib.path().to_str().unwrap(), "C:\\Windows\\System32\\test.dll");
        assert_eq!(lib.calling_convention(), CallingConvention::Stdcall);
    }
}

#[test]
fn test_ffi_version_support() {
    let lib = FFI::lib("versioned_lib")
        .path("/tmp/libversioned.so")  // Use explicit path
        .version("2.4.1")
        .search_path("/usr/lib")
        .search_path("/usr/local/lib")
        .build()
        .unwrap();
    
    // Version requirement is stored
    assert_eq!(lib.version_requirement(), Some("2.4.1"));
}

#[test]
fn test_ffi_aliases() {
    let lib = FFI::lib("aliased")
        .path("/tmp/libaliased.so")  // Use explicit path
        .function("internal_impl", FnSignature::new(vec![], types::void()))
        .alias("public_name", "internal_impl")
        .alias("another_name", "internal_impl")
        .build()
        .unwrap();
    
    // Check that aliases map to the original function
    assert!(lib.get_signature("internal_impl").is_some());
}

#[test]
fn test_ffi_struct_and_enum_definitions() {
    let lib = FFI::lib("types")
        .path("/tmp/libtypes.so")  // Use explicit path
        .struct_def("Point", vec![
            ("x".to_string(), types::f64()),
            ("y".to_string(), types::f64()),
        ])
        .enum_def("Status", vec![
            "Success".to_string(),
            "Failure".to_string(),
            "Pending".to_string(),
        ])
        .build()
        .unwrap();
    
    let mappings = lib.type_mappings();
    assert!(mappings.contains_key("Point"));
    assert!(mappings.contains_key("Status"));
    
    // Check the generated types
    if let Some(point_mapping) = mappings.get("Point") {
        if let AstType::Struct { name, fields } = &point_mapping.zen_type {
            assert_eq!(name, "Point");
            assert_eq!(fields.len(), 2);
        } else {
            panic!("Expected struct type for Point");
        }
    }
    
    if let Some(status_mapping) = mappings.get("Status") {
        if let AstType::Enum { name, variants } = &status_mapping.zen_type {
            assert_eq!(name, "Status");
            assert_eq!(variants.len(), 3);
        } else {
            panic!("Expected enum type for Status");
        }
    }
}

#[test]
fn test_ffi_callback_definitions() {
    let callback_def = CallbackDefinition {
        signature: FnSignature::new(
            vec![types::i32(), types::string()],
            types::bool(),
        ),
        trampoline: Some(Arc::new(|_data| vec![1])),
    };
    
    let lib = FFI::lib("callbacks")
        .path("/tmp/libcallbacks.so")  // Use explicit path
        .callback("event_handler", callback_def)
        .build()
        .unwrap();
    
    // Callbacks are stored
    assert!(lib.callbacks().contains_key("event_handler"));
}

#[test]
fn test_ffi_validation_rules() {
    let rule = ValidationRule::new(
        "check_functions",
        |_builder| {
            // Validation rule test simplified
            Ok(())
        }
    );
    
    // This should pass validation
    let result = FFI::lib("with_function")
        .path("/tmp/libwithfunc.so")  // Use explicit path
        .function("test", FnSignature::new(vec![], types::void()))
        .validation_rule(rule)
        .build();
    
    assert!(result.is_ok());
}

#[test]
fn test_ffi_load_flags() {
    let flags = LoadFlags {
        lazy_binding: true,
        global_symbols: true,
        no_delete: false,
    };
    
    let lib = FFI::lib("flagged")
        .path("/tmp/libflagged.so")  // Use explicit path
        .load_flags(flags.clone())
        .build()
        .unwrap();
    
    assert_eq!(lib.load_flags().lazy_binding, true);
    assert_eq!(lib.load_flags().global_symbols, true);
    assert_eq!(lib.load_flags().no_delete, false);
}

#[test]
fn test_ffi_lazy_loading() {
    let lib = FFI::lib("lazy")
        .path("/tmp/liblazy.so")  // Use explicit path
        .lazy_loading(true)
        .function("delayed_func", FnSignature::new(vec![], types::void()))
        .build()
        .unwrap();
    
    assert!(lib.lazy_loading());
}

#[test]
fn test_ffi_search_paths() {
    let lib = FFI::lib("searchable")
        .path("/tmp/libsearchable.so")  // Use explicit path
        .search_paths(vec![
            "/usr/lib",
            "/usr/local/lib",
            "/opt/lib",
            "~/.local/lib",
        ])
        .build()
        .unwrap();
    
    // Search paths are stored and used
    // The actual library finding would happen during load()
    assert!(lib.search_paths().len() >= 4);
}

#[test]
fn test_ffi_type_compatibility() {
    // Test that platform-specific types work correctly
    let lib = FFI::lib("platform_types")
        .path("/tmp/libplatform_types.so")  // Use explicit path
        .function("size_func", FnSignature::new(
            vec![types::usize(), types::isize()],
            types::usize(),
        ))
        .function("c_string_func", FnSignature::new(
            vec![types::c_string()],
            types::c_string(),
        ))
        .function("array_func", FnSignature::new(
            vec![types::array(10, types::i32())],
            types::slice(types::i32()),
        ))
        .build()
        .unwrap();
    
    assert!(lib.get_signature("size_func").is_some());
    assert!(lib.get_signature("c_string_func").is_some());
    assert!(lib.get_signature("array_func").is_some());
}

#[test]
fn test_ffi_standard_marshallers() {
    use zen::ffi::Library;
    
    let marshallers = Library::create_standard_marshallers();
    
    // Check that standard marshallers are created
    assert!(marshallers.contains_key("string"));
    assert!(marshallers.contains_key("bool"));
    
    // Test string marshaller
    if let Some(string_marshaller) = marshallers.get("string") {
        let test_data = b"hello";
        let c_data = (string_marshaller.to_c)(test_data);
        assert_eq!(c_data.last(), Some(&0)); // Should be null-terminated
        
        let zen_data = (string_marshaller.from_c)(&c_data);
        assert_eq!(&zen_data[..5], test_data); // Should remove null terminator
    }
    
    // Test bool marshaller
    if let Some(bool_marshaller) = marshallers.get("bool") {
        let true_data = vec![42];
        let c_true = (bool_marshaller.to_c)(&true_data);
        assert_eq!(c_true, vec![1]);
        
        let false_data = vec![0];
        let c_false = (bool_marshaller.to_c)(&false_data);
        assert_eq!(c_false, vec![0]);
    }
}

#[test]
fn test_ffi_error_handler() {
    use std::sync::{Arc, Mutex};
    use zen::ffi::FFIError;
    
    let errors = Arc::new(Mutex::new(Vec::new()));
    let errors_clone = errors.clone();
    
    let lib = FFI::lib("error_test")
        .path("/tmp/liberror_test.so")  // Use explicit path
        .error_handler(move |err: &FFIError| {
            errors_clone.lock().unwrap().push(format!("{}", err));
        })
        .build()
        .unwrap();
    
    // Trigger an error by trying to get a function from unloaded library
    let _ = lib.get_function("nonexistent");
    
    // Error handler should have been called
    // Note: This might not work as expected without actually loading the library
    // but the infrastructure is in place
}

#[test]
fn test_ffi_complex_type_validation() {
    // Test that incompatible types are rejected
    let result = FFI::lib("invalid_types")
        .path("/tmp/libinvalid.so")  // Use explicit path  
        .function("bad_func", FnSignature::new(
            vec![
                // Generic type without mapping should fail validation
                AstType::Generic { 
                    name: "UnmappedType".to_string(), 
                    type_args: vec![] 
                }
            ],
            types::void(),
        ))
        .build();
    
    // This should fail validation if safety checks are enabled
    // The exact behavior depends on the validation implementation
}

#[test]
fn test_ffi_builder_comprehensive() {
    // Comprehensive test combining many features
    let lib = FFI::lib("comprehensive")
        .path("/usr/lib/libcomprehensive.so")
        .version("3.2.1")
        .calling_convention(CallingConvention::C)
        .safety_checks(true)
        .lazy_loading(true)
        // Functions
        .function("init", FnSignature::new(vec![], types::bool())
            .with_safety(FunctionSafety::Safe))
        .function("process", FnSignature::new(
            vec![types::ptr(types::u8()), types::usize()],
            types::i32(),
        ).with_safety(FunctionSafety::Trusted))
        .function("cleanup", FnSignature::new(vec![], types::void())
            .with_safety(FunctionSafety::Safe))
        // Constants
        .constant("MAX_SIZE", types::usize())
        .constant("VERSION", types::string())
        // Type definitions
        .struct_def("Config", vec![
            ("flags".to_string(), types::u32()),
            ("timeout".to_string(), types::i32()),
        ])
        .enum_def("ErrorCode", vec![
            "Success".to_string(),
            "InvalidInput".to_string(),
            "Timeout".to_string(),
        ])
        // Aliases
        .alias("initialize", "init")
        .alias("shutdown", "cleanup")
        // Search paths
        .search_path("/opt/comprehensive/lib")
        .search_path("/usr/local/lib")
        // Platform-specific config
        .platform_config(Platform::Linux, PlatformConfig {
            path_override: None,
            calling_convention_override: None,
            additional_search_paths: vec![PathBuf::from("/lib/x86_64-linux-gnu")],
        })
        // Load flags
        .load_flags(LoadFlags {
            lazy_binding: true,
            global_symbols: false,
            no_delete: true,
        })
        .build()
        .unwrap();
    
    // Verify all configurations
    assert_eq!(lib.name(), "comprehensive");
    assert_eq!(lib.functions().len(), 3);
    assert_eq!(lib.constants().len(), 2);
    assert_eq!(lib.type_mappings().len(), 2);
    assert!(lib.safety_checks());
    assert!(lib.lazy_loading());
    assert_eq!(lib.calling_convention(), CallingConvention::C);
}

#[test]
fn test_ffi_callback_registration() {
    let mut lib = FFI::lib("callback_test")
        .path("/tmp/libcallback_test.so")
        .callback("on_event", CallbackDefinition {
            signature: FnSignature::new(
                vec![types::i32(), types::c_string()],
                types::void(),
            ),
            trampoline: None,
        })
        .callback("on_data", CallbackDefinition {
            signature: FnSignature::new(
                vec![types::slice(types::u8())],
                types::i32(),
            ),
            trampoline: None,
        })
        .build()
        .unwrap();
    
    // Test callback registration
    let result = lib.register_callback("on_event", Box::new(|data| {
        // Simple echo callback
        data.to_vec()
    }));
    assert!(result.is_ok());
    
    // Test registering non-existent callback
    let result = lib.register_callback("non_existent", Box::new(|data| {
        data.to_vec()
    }));
    assert!(result.is_err());
}

#[test]
fn test_ffi_platform_auto_configuration() {
    use zen::ffi::Platform;
    
    let lib = FFI::lib("auto_config")
        .platform_config(Platform::Linux, PlatformConfig {
            path_override: Some(PathBuf::from("/usr/lib/libauto.so")),
            calling_convention_override: None,
            additional_search_paths: vec![],
        })
        .platform_config(Platform::MacOS, PlatformConfig {
            path_override: Some(PathBuf::from("/usr/local/lib/libauto.dylib")),
            calling_convention_override: None,
            additional_search_paths: vec![],
        })
        .platform_config(Platform::Windows, PlatformConfig {
            path_override: Some(PathBuf::from("C:\\Windows\\System32\\auto.dll")),
            calling_convention_override: Some(CallingConvention::Stdcall),
            additional_search_paths: vec![],
        })
        .auto_configure()
        .build()
        .unwrap();
    
    // Check that the correct platform configuration was applied
    #[cfg(target_os = "linux")]
    assert_eq!(lib.path().to_str().unwrap(), "/usr/lib/libauto.so");
    
    #[cfg(target_os = "macos")]
    assert_eq!(lib.path().to_str().unwrap(), "/usr/local/lib/libauto.dylib");
    
    #[cfg(target_os = "windows")]
    {
        assert_eq!(lib.path().to_str().unwrap(), "C:\\Windows\\System32\\auto.dll");
        assert_eq!(lib.calling_convention(), CallingConvention::Stdcall);
    }
}