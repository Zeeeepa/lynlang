# Zen Import System Documentation

## Overview

The Zen language supports module imports at the module level (top-level scope). Imports allow you to bring in functionality from the standard library or other modules.

## Import Syntax

### Standard Library Imports

Zen provides access to standard library modules through the `@std` namespace:

```zen
// Direct standard library import
core := @std.core
math := @std.math
vec := @std.vec
```

### Build System Imports

Modules can also be imported through the build system:

```zen
// Build system imports
build := @std.build
io := build.import("io")
fs := build.import("fs")
string := build.import("string")
```

## Import Rules

### 1. Module-Level Only

Imports MUST be declared at the module level (top-level scope). They cannot be placed inside:
- Functions
- Structs
- If/else blocks
- Loop bodies
- Comptime blocks

**Valid:**
```zen
// ✓ Module-level import
io := @std.io

main = () i32 {
    io.print("Hello")
    return 0
}
```

**Invalid:**
```zen
main = () i32 {
    // ✗ Import inside function
    io := @std.io
    return 0
}
```

### 2. No Comptime Required

Unlike earlier versions, imports do NOT require `comptime` blocks:

**Current (Correct):**
```zen
// Direct module-level imports
core := @std.core
io := @std.io
```

**Old (Deprecated):**
```zen
// DO NOT use comptime for imports
comptime {
    core := @std.core
    io := @std.io
}
```

### 3. Import Aliases

You can create aliases for imported modules:

```zen
// Create an alias
stdio := build.import("io")
filesystem := build.import("fs")

main = () i32 {
    stdio.print("Using aliased import\n")
    return 0
}
```

## Standard Library Modules

Common standard library modules include:

- `@std.core` - Core language functionality
- `@std.build` - Build system integration
- `@std.math` - Mathematical functions
- `@std.string` - String manipulation
- `@std.vec` - Vector/dynamic array operations
- `@std.io` - Input/output operations
- `@std.fs` - File system operations

## Example Programs

### Hello World with Imports

```zen
core := @std.core
build := @std.build
io := build.import("io")

main = () i32 {
    io.print("Hello, Zen!\n")
    return 0
}
```

### Using Multiple Modules

```zen
core := @std.core
build := @std.build
io := build.import("io")
math := @std.math
string := build.import("string")

main = () i32 {
    // Use math module
    abs_value := math.abs(-42)
    
    // Use io module
    io.print("Absolute value: ")
    io.print_int(abs_value)
    io.print("\n")
    
    return 0
}
```

## Comptime and Meta-programming

`comptime` blocks are reserved for compile-time evaluation and meta-programming, NOT for imports:

```zen
// Imports at module level
math := @std.math

// Comptime for meta-programming
comptime {
    // Compile-time computations
    const_value := compute_at_compile_time()
}
```

## Error Messages

When imports are placed incorrectly, you'll see clear error messages:

```
Error: Import statements are not allowed inside comptime blocks. Move imports to module level.
Error: Module imports should not be inside comptime blocks. Move imports to module level.
```

## Tools

### Syntax Checker

Use the `zen-check` tool to validate your Zen source files:

```bash
./zen-check myfile.zen
./zen-check *.zen
```

This tool will check for syntax errors including incorrect import placement.