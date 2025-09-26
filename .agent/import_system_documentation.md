# Zen Language Import System Documentation

## Overview
The Zen import system provides a flexible way to organize code into modules and share functionality across files. The system supports both file-based modules and built-in standard library modules.

## Import Syntax

### Basic Import
```zen
// Import a module with an alias
{ Option, Result } = @std
```

### Module Path Import
```zen
// Import from a file-based module
{ HashMap, HashSet } = @collections
```

### Standard Library Import
```zen
// Built-in @std module (no file required)
{ get_default_allocator, Option, Result } = @std
```

## How Imports Work

### 1. Module Resolution Process

When you write an import like `{ Option, Result } = @std`, the compiler:

1. **Parses the import statement** - The parser recognizes the destructuring pattern and creates a `ModuleImport` declaration
2. **Checks for built-in modules** - If the path starts with `@std` or `std.`, it's treated as a built-in module
3. **Searches for file modules** - For other imports, searches in:
   - Current directory
   - `./lib` directory  
   - `./modules` directory
   - `$ZEN_HOME/std` and `$ZEN_HOME/lib` (if ZEN_HOME is set)

### 2. File Path Resolution

Module paths are converted to file paths:
- `@collections` → `collections.zen`
- `@utils.string` → `utils/string.zen`
- `@mylib` → `mylib.zen` or `mylib/mod.zen`

### 3. Module Loading

The ModuleSystem (`src/module_system/mod.rs`):
- Maintains a cache of loaded modules to avoid duplicate parsing
- Recursively loads imported modules
- Merges all declarations from imported modules into the main program

### 4. Symbol Resolution

The ModuleResolver (`src/module_system/resolver.rs`):
- Tracks exported symbols from each module
- Resolves qualified names (e.g., `io.println`)
- Validates that imported symbols are exported (public)
- Rewrites qualified names for code generation

### 5. Export Rules

Symbols are automatically exported based on visibility:
- **Public (exported)**: Functions, structs, enums, and type aliases not starting with `_`
- **Private (not exported)**: Any symbol starting with `_`

Example:
```zen
// Public - automatically exported
print = (msg: string) void { ... }
HashMap = struct { ... }

// Private - not exported
_internal_helper = () void { ... }
```

## Built-in Modules

### @std Module
The `@std` module is special - it's built into the compiler and provides core types:
- `Option<T>` - Optional values
- `Result<T,E>` - Error handling
- `get_default_allocator()` - System memory allocator
- Basic types and utilities

These are implemented in the LLVM codegen layer rather than as Zen source files.

## Module System Architecture

```
┌─────────────┐
│   Parser    │ ─── Recognizes import syntax
└──────┬──────┘     Creates ModuleImport declarations
       │
       ▼
┌─────────────┐
│  Compiler   │ ─── process_imports() method
└──────┬──────┘     Orchestrates module loading
       │
       ▼
┌─────────────┐
│ModuleSystem │ ─── Loads and caches modules
└──────┬──────┘     Manages search paths
       │
       ▼
┌─────────────┐
│  Resolver   │ ─── Resolves qualified names
└──────┬──────┘     Validates exports
       │
       ▼
┌─────────────┐
│   Codegen   │ ─── Generates LLVM IR
└─────────────┘     Links all modules
```

## Import Processing Steps

1. **Parse Phase**: Import statements are parsed into AST nodes
2. **Module Loading**: Files are loaded and parsed recursively
3. **Symbol Collection**: Public symbols are identified and tracked
4. **Resolution**: Qualified names are resolved and validated
5. **Merge**: All module declarations are merged into one program
6. **Codegen**: LLVM IR is generated with all modules linked

## Example Import Flow

```zen
// main.zen
{ HashMap } = @collections
{ print } = @utils

main = () i32 {
    map := HashMap.new(get_default_allocator())
    print("Hello")
    return 0
}
```

1. Parser creates two `ModuleImport` declarations
2. Compiler loads `collections.zen` and `utils.zen`
3. Extracts `HashMap` from collections, `print` from utils
4. Merges all declarations into single program
5. Generates LLVM IR with all functions available

## Best Practices

1. **Use destructuring imports** to be explicit about dependencies
2. **Prefix private functions with `_`** to prevent exports
3. **Organize related functions into modules** for better code organization
4. **Use @std for core types** like Option, Result, and allocators

## Technical Implementation Details

- Import resolution happens at compile-time, not runtime
- All imports are resolved before type checking
- Circular imports are not currently supported
- Module paths are case-sensitive
- The system uses a HashMap for O(1) symbol lookups