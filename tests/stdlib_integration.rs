use zen::stdlib::{StdNamespace, StdModuleTrait};

#[test]
fn test_stdlib_modules_registered() {
    let namespace = StdNamespace::new();
    
    // Test core modules are registered
    assert!(namespace.get_module("core").is_some());
    assert!(namespace.get_module("build").is_some());
    assert!(namespace.get_module("io").is_some());
    assert!(namespace.get_module("math").is_some());
    assert!(namespace.get_module("string").is_some());
    assert!(namespace.get_module("vec").is_some());
    assert!(namespace.get_module("fs").is_some());
}

#[test]
fn test_core_module_functions() {
    let namespace = StdNamespace::new();
    
    if let Some(module) = namespace.get_module("core") {
        // Test intrinsics
        assert!(matches!(module, zen::stdlib::StdModule::Core(_)));
        
        // We can't directly test functions without exposing more internals,
        // but we can ensure the module exists and is the right type
    } else {
        panic!("Core module not found");
    }
}

#[test]
fn test_std_namespace_resolution() {
    use zen::ast::Expression;
    
    // Test that std module names resolve correctly
    assert!(StdNamespace::resolve_std_access("core").is_some());
    assert!(StdNamespace::resolve_std_access("build").is_some());
    assert!(StdNamespace::resolve_std_access("io").is_some());
    assert!(StdNamespace::resolve_std_access("math").is_some());
    assert!(StdNamespace::resolve_std_access("string").is_some());
    assert!(StdNamespace::resolve_std_access("vec").is_some());
    assert!(StdNamespace::resolve_std_access("fs").is_some());
    
    // Test that invalid module names don't resolve
    assert!(StdNamespace::resolve_std_access("invalid_module").is_none());
}

#[test]
fn test_build_module_exists() {
    let namespace = StdNamespace::new();
    
    // Test that build module was successfully created
    assert!(namespace.get_module("build").is_some());
    
    if let Some(zen::stdlib::StdModule::Build(build_module)) = namespace.get_module("build") {
        // Test that build module has correct name
        assert_eq!(build_module.name(), "build");
        
        // Test that key functions exist
        assert!(build_module.get_function("import").is_some());
        assert!(build_module.get_function("compile_file").is_some());
        assert!(build_module.get_function("default_config").is_some());
        
        // Test that key types exist
        assert!(build_module.get_type("BuildConfig").is_some());
        assert!(build_module.get_type("BuildResult").is_some());
    } else {
        panic!("Build module not found or wrong type");
    }
}

#[test]
fn test_math_module_functions() {
    let namespace = StdNamespace::new();
    
    if let Some(zen::stdlib::StdModule::Math(math_module)) = namespace.get_module("math") {
        // Test common math functions exist
        assert!(math_module.get_function("abs").is_some());
        assert!(math_module.get_function("sqrt").is_some());
        assert!(math_module.get_function("sin").is_some());
        assert!(math_module.get_function("cos").is_some());
        assert!(math_module.get_function("min").is_some());
        assert!(math_module.get_function("max").is_some());
    } else {
        panic!("Math module not found or wrong type");
    }
}

#[test]
fn test_string_module_functions() {
    let namespace = StdNamespace::new();
    
    if let Some(zen::stdlib::StdModule::String(string_module)) = namespace.get_module("string") {
        // Test string manipulation functions exist
        assert!(string_module.get_function("len").is_some());
        assert!(string_module.get_function("concat").is_some());
        assert!(string_module.get_function("substring").is_some());
        assert!(string_module.get_function("split").is_some());
        assert!(string_module.get_function("trim").is_some());
    } else {
        panic!("String module not found or wrong type");
    }
}

#[test]
fn test_vec_module_functions() {
    let namespace = StdNamespace::new();
    
    if let Some(zen::stdlib::StdModule::Vec(vec_module)) = namespace.get_module("vec") {
        // Test vector operations exist
        assert!(vec_module.get_function("new").is_some());
        assert!(vec_module.get_function("push").is_some());
        assert!(vec_module.get_function("pop").is_some());
        assert!(vec_module.get_function("len").is_some());
        assert!(vec_module.get_function("get").is_some());
    } else {
        panic!("Vec module not found or wrong type");
    }
}

#[test]
fn test_fs_module_functions() {
    let namespace = StdNamespace::new();
    
    if let Some(zen::stdlib::StdModule::Fs(fs_module)) = namespace.get_module("fs") {
        // Test filesystem operations exist
        assert!(fs_module.get_function("read_file").is_some());
        assert!(fs_module.get_function("write_file").is_some());
        assert!(fs_module.get_function("exists").is_some());
        assert!(fs_module.get_function("is_file").is_some());
        assert!(fs_module.get_function("create_dir").is_some());
    } else {
        panic!("Fs module not found or wrong type");
    }
}