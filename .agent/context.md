# Zen Language - Core Architecture

## Compiler Stack
- **Frontend**: Lexer → Parser → AST (`src/ast/`, `src/parser/`)
- **Middle**: Type checker with generics, pattern matching (`src/typechecker/`)
- **Backend**: LLVM codegen via inkwell (`src/codegen/llvm/`)
- **LSP**: Full language server (`src/lsp/enhanced_server.rs` - 6.6K lines)

## Key Features Working
- Type system: primitives, structs, enums, generics, Result/Option
- Pattern matching with exhaustiveness checking
- Collections: DynVec, Array (HashMap/HashSet partial)
- Error handling: .raise() propagation
- Comptime introspection: @comptime, @typeof, @fields
- Standard library: io, collections, allocators
- 194 tests passing (100% of enabled tests)

## Build
```bash
cargo build --release
./target/release/zen file.zen
```
