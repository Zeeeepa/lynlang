# Zen Language Development Plan

## Current Goal
Fix import system to work without comptime blocks and continue self-hosting efforts.

## Key Changes Needed

### 1. Import System Reform
**Current State:**
```zen
comptime {
    core := @std.core
    build := @std.build
    io := build.import("io")
}
```

**Target State:**
```zen
core := @std.core
build := @std.build
io := build.import("io")

main = () i32 {
    io.print("Hello, Zen!\n");
    return 0
}
```

### 2. Implementation Steps
1. **Parser Changes**: Allow top-level import statements
2. **AST Updates**: Add import nodes at module level
3. **Module Resolution**: Handle imports during compilation not comptime
4. **Stdlib in Zen**: Rewrite core modules in Zen itself
5. **Testing**: Comprehensive test suite
6. **Tooling**: Basic LSP or checker

## Architecture Notes
- Comptime should be for metaprogramming only
- Imports are compile-time resolution but not comptime evaluation
- Module system should be separate from comptime system

## Progress Tracking
See todos.md for current task status