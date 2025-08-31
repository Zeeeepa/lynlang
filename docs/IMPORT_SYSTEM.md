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

### 2. Imports Are NOT Comptime

**IMPORTANT:** Imports are compile-time module resolution, not comptime evaluation. They should NEVER be placed in `comptime` blocks.

**Correct - Module-level imports:**
```zen
// Direct module-level imports (NOT in comptime)
core := @std.core
io := @std.io
```

**INCORRECT - Will cause compilation error:**
```zen
// ✗ NEVER put imports in comptime blocks
comptime {
    core := @std.core  // ERROR: Imports not allowed in comptime
    io := @std.io      // ERROR: Imports not allowed in comptime
}
```

`comptime` is for meta-programming and compile-time computation, NOT for imports.

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

## Comptime vs Imports

### Key Distinction

- **Imports**: Module resolution that happens at compile time (but not in comptime blocks)
- **Comptime**: Meta-programming and compile-time code execution

### Comptime Use Cases

`comptime` blocks are for:
- Compile-time computation
- Code generation
- Type-level programming
- Constant evaluation

**NOT for imports!**

Example of proper comptime usage:
```zen
// Correct comptime usage - computation, not imports
comptime {
    BUFFER_SIZE := 1024 * 4  // Compile-time constant
    MAX_USERS := 100
}

// Imports are OUTSIDE comptime
io := @std.io
```

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