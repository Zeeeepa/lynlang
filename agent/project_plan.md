# Lynlang Project Plan

## Project Status
**Current Status**: Making progress on bug fixes and parser improvements  
**Date**: 2025-08-23  
**Test Status**: 1 failing test (test_parse_struct_with_methods)

## Completed Today
✅ Fixed void pointer codegen support
  - Added support for void pointers (*void) in LLVM codegen
  - Map void pointers to i8* in LLVM IR (standard practice)
  - All void pointer tests now pass

✅ Fixed lexer != operator tokenization
  - Added special handling for ! operator to check for != combination
  - != is now correctly tokenized as Operator('!=')
  - All lexer tests pass

## Current Work
🔄 Fixing parser struct with methods test failure
  - Error: "Expected '(' for function parameters"
  - Need to investigate struct method parsing

## Next Tasks (Priority Order)
1. Fix parser struct with methods test failure
2. Implement pattern matching syntax (? operator) in parser
3. Add comptime block parsing support
4. Implement member access (dot operator) parsing
5. Write comprehensive tests for new features

## Project Architecture Overview

### Core Components
- **Lexer** (src/lexer.rs) - Tokenization ✅ Working
- **Parser** (src/parser/) - Modular parsing system 🔄 Needs improvements
- **AST** (src/ast.rs) - Complete type definitions ✅
- **Codegen** (src/codegen/llvm/) - LLVM IR generation 🔄 Mostly working
- **LSP** (src/lsp/) - Language server support 🔄 Basic implementation

### Test Coverage
- Parser tests: 27/28 passing
- Codegen tests: All passing (31 tests)
- Lexer tests: All passing (15 tests)
- FFI tests: All passing (5 tests)

## Critical Features Needing Implementation

### Parser Gaps (High Priority)
- Pattern matching syntax (? operator)
- Comptime block parsing
- Member access parsing (dot operator)
- Method definitions in structs

### Codegen Features (Medium Priority)
- Pattern matching code generation
- Comptime evaluation
- Generic type instantiation
- Trait/behavior system

### Type System (Lower Priority)
- Dedicated type checker module
- Type inference engine
- Generic type support

## Testing Strategy
- 80% implementation time
- 20% testing time
- Focus on end-to-end tests
- Write unit tests for critical paths

## Communication
- Email updates to l.leong1618@gmail.com on major milestones
- Commit and push after every file edit
- Use .agent/ directory for planning and notes

## Build Commands
```bash
# Build
cargo build

# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run lexer tests
cargo test --test lexer

# Run parser tests  
cargo test --test parser

# Run codegen tests
cargo test --test codegen
```

## Recent Commits
- f0cebb5: Fix lexer != operator tokenization
- cb5b668: Fix void pointer codegen support
- 24143e4: Fix struct parsing and update tests
- 5f44932: Fix lexer and parser for loop statements

## Notes
- Language is systems-focused with LLVM backend
- Emphasizes explicit syntax with minimal keywords
- Supports C FFI for library integration
- Compile-time metaprogramming via comptime blocks