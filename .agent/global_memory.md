# Zen Language Global Memory

## Project Overview
Zen is a systems programming language focusing on simplicity, elegance, and practicality. Currently working towards self-hosting.

## Key Design Decisions

### Import System
- **OLD**: Imports required `comptime { }` blocks
- **NEW**: Direct imports at module level without comptime
- Syntax: `module := @std.module` or `module := build.import("module")`
- @std modules and std. modules are built-in, handled by compiler

### Declaration System  
- `:=` for immutable bindings (constants)
- `::=` for mutable bindings (variables)
- `=` for type definitions and functions

### Conditional System
- No `if/else` keywords
- Universal `?` operator for all conditionals
- Pattern matching with `| pattern => expression`

### Module Resolution
- Built-in modules: @std.* and std.*
- User modules: resolved from search paths
- Module paths converted to file paths (dots to slashes)

## Technical Details

### Compiler Architecture
- Written in Rust (transitioning to self-hosted)
- LLVM backend for code generation
- Module system with import resolution
- Type checking and inference

### File Structure
- `/src` - Rust compiler source
- `/stdlib` - Standard library (Zen)
- `/examples` - Example programs
- `/tests` - Test suite
- `/.agent` - Development metadata

### Build Commands
- `cargo build --release` - Build compiler
- `./target/release/zen file.zen` - Compile Zen file
- Output: LLVM IR (can be compiled to native)

## Current State
- Basic language features working
- Import system refactored (no comptime needed) ✅
- Standard library structure in place
- Working towards self-hosting
- All parser import tests passing (5/5)
- Examples updated with new import syntax
- Fixed struct syntax in self-hosting files (removed 'struct' keyword)
- Documentation updated to reflect new import syntax (no comptime wrapper)

## Next Steps
1. Implement core compiler components in Zen
2. Expand standard library
3. Create testing framework
4. Develop language server