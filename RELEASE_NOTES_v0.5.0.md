# Zenlang v0.5.0 Release Notes

## Overview
This release marks a significant milestone for Zenlang with production-ready Language Server Protocol (LSP) support and comprehensive demonstration suite showcasing all language features.

## Major Enhancements

### 1. Production-Ready LSP Server

#### Position Tracking Fix
- **Issue**: LSP was incorrectly highlighting error positions due to column indexing mismatch
- **Solution**: Fixed lexer to use 0-based column indexing consistently throughout the LSP
- **Impact**: Error highlighting now accurately shows the exact location of syntax errors

#### Enhanced Go-to-Definition
- Complete symbol table tracking for all declarations (functions, structs, enums, behaviors)
- Local variable resolution within function scopes
- Function parameter tracking with accurate position mapping
- Support for navigating to:
  - Function definitions
  - Struct/enum declarations
  - Variable declarations (both mutable and immutable)
  - Behavior definitions and implementations

#### Rich Hover Information
- Comprehensive type information display
- Function signatures with parameter types and return types
- Struct field listings with types
- Enum variant information with payloads
- Contextual documentation and usage examples
- Special handling for:
  - Standard library modules (@std.*)
  - Keywords with language spec references
  - Variables with inferred vs explicit types

#### Find References
- Context-aware reference finding across entire codebase
- Whole-word matching to avoid false positives
- Support for renamed symbols
- Accurate range calculation for multi-character operators

#### Document Symbols
- Hierarchical symbol navigation
- Nested symbol support (e.g., struct fields, enum variants)
- Accurate range calculation for symbol definitions
- Symbol categorization by type (function, struct, enum, behavior, etc.)

### 2. Comprehensive Example Suite

Located in `examples/full_demo/`, demonstrating:

#### main.zen
- Complete showcase of all language features
- Pattern matching with the `?` operator
- Memory management with smart pointers
- Behaviors (trait system)
- Comptime evaluation
- Colorless async
- UFCS (Uniform Function Call Syntax)
- Error handling without exceptions
- String interpolation
- Loop patterns

#### Supporting Examples
- `patterns.zen` - Advanced pattern matching techniques
- `async_demo.zen` - Async/await and concurrency patterns
- `ffi_demo.zen` - Foreign Function Interface integration
- `build.zen` - Build system usage
- `self_hosting_demo.zen` - Self-hosting compiler features
- `lib.zen` - Mathematical library with generics
- `builder_demo.zen` - Builder pattern implementation

### 3. Testing Improvements
- Added comprehensive LSP enhancement tests
- Fixed test module imports
- Validated position tracking with dedicated test program
- All 24 lib tests passing
- Integration tests for LSP features

## Technical Details

### LSP Position Tracking
The lexer now correctly tracks positions with:
- Line numbers: 1-indexed (matching editor conventions)
- Column numbers: 0-indexed (matching LSP specification)
- Character offsets: Absolute position in the file
- Proper handling of multi-byte UTF-8 characters

### Symbol Table Implementation
- Efficient symbol lookup with HashMap storage
- Separate tracking for different symbol types
- Reference tracking for each symbol
- Definition range tracking for accurate navigation

### Error Recovery
- Context-aware error messages
- Suggestions for fixing common mistakes
- References to LANGUAGE_SPEC.md for syntax rules
- Visual indicators showing exact error location

## Breaking Changes
None - This release maintains full backward compatibility.

## Migration Guide
No migration required. Simply update to v0.5.0 to get enhanced LSP features.

## Known Issues
- LSP hover information for deeply nested generic types could be more detailed
- Go-to-definition for standard library sources not yet available (planned for v0.6.0)

## Future Work (v0.6.0)
- Package registry launch
- WebAssembly target completion
- IDE plugins for VSCode, Neovim, IntelliJ
- Standard library expansion (networking, cryptography)
- Debugger integration with LLDB/GDB

## Contributors
Thanks to all contributors who made this release possible through testing, feedback, and code contributions.

## Installation
```bash
git clone https://github.com/lantos1618/zenlang
cd zenlang
cargo build --release
./target/release/zen-lsp  # Start LSP server
```

## Testing
```bash
cargo test  # Run all tests
./target/release/zen examples/full_demo/main.zen  # Run comprehensive demo
```

---
*Zenlang - A modern systems language with radical simplicity*