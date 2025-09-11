# Zenlang Project Global Memory

## Project Overview
- **Language**: Zenlang (zen) - A modern systems programming language
- **Philosophy**: Clarity, explicitness, minimal syntax, errors as values
- **Key Feature**: NO if/else keywords - uses `?` operator exclusively
- **Status**: ~80% implemented, moving toward self-hosting

## Critical Design Decisions
1. **Pattern Matching Only**: All conditionals use `?` operator
2. **No Exceptions**: All errors are values (Result/Option types)
3. **Explicit Memory**: No hidden allocations
4. **UFCS**: Uniform Function Call Syntax for method-like calls
5. **Single Loop Keyword**: Only `loop` for all iteration

## Implementation Status
- Parser: ~90% complete (imports fixed)
- Type Checker: ~85% complete  
- Code Generator: ~80% complete (LLVM backend, some pattern matching issues)
- Standard Library: ~80% complete (FFI implemented)
- Self-Hosting: ~35% complete
- LSP Server: ✅ Built successfully
- VSCode Extension: ✅ Configured and ready
- FFI System: ✅ Builder pattern implemented (2025-09-11)
- Test Suite: ✅ Spec compliance tests created (2025-09-11)
- Core Tests: ✅ 270+ tests passing in Rust test suite
- Compiler: ✅ Working for basic programs

## Recent Work (2025-09-11)
- Reviewed LANGUAGE_SPEC.md v1.1.0
- Implemented FFI builder pattern in `/std/ffi.zen`
- Created comprehensive spec compliance test suite
- Built and tested LSP server (compiles successfully)
- Identified 1,440 if violations and 252 match violations
- Standard library is fully spec compliant
- **MAJOR UPDATE**: Implemented core types (Option, Result, Vec, HashMap, Ptr, Ref)
- **MAJOR UPDATE**: Created spec-compliant LSP server without forbidden keywords
- **MAJOR UPDATE**: Fixed FFI to use proper core types
- **MAJOR UPDATE**: Added comprehensive test suite for core types
- **SESSION UPDATE**: FFI builder pattern already implemented and working
- **SESSION UPDATE**: LSP builds successfully with only deprecation warnings
- **SESSION UPDATE**: Created comprehensive language test suite (zen_comprehensive_language_test.zen)
- **SESSION UPDATE**: Created comprehensive stdlib test suite (zen_stdlib_comprehensive_test.zen)
- **SESSION UPDATE**: Compiler functional for basic programs (hello world works)

## Current Session Status (2025-09-11 - Latest)
- ✅ Rust compiler builds successfully (42MB binary)
- ✅ All 13 library tests passing
- ✅ 270+ integration tests passing
- ✅ FFI tests all passing (7 tests)
- ✅ Hello World example runs successfully
- ✅ LSP server built (18MB binary)
- ✅ Created .agent directory for metadata
- 🔄 Some Zen test files have syntax issues
- 🔄 FFI Zen implementation needs refinement

## Key Files
- `/LANGUAGE_SPEC.md` - Authoritative language specification v1.1.0
- `/std/ffi.zen` - FFI builder pattern implementation
- `/tests/spec_compliance_test.zen` - Comprehensive test suite
- `/src/` - Rust implementation (being replaced)
- `/std/` - Standard library (written in Zen, spec compliant)

## Outstanding Violations (as of 2025-09-11)
- 1,440 `if` keyword violations
- 252 `match` keyword violations
- Total: 1,692 spec violations to fix
- Mostly in examples and older code

## Development Principles
- DRY (Don't Repeat Yourself)
- KISS (Keep It Simple, Stupid)
- Test coverage target: 80% implementation, 20% testing
- Frequent git commits
- Simplicity, elegance, practicality, intelligence

## Critical Next Steps
1. Fix all 1,692 spec violations
2. Complete self-hosting compiler
3. Get zen runtime working
4. Run spec compliance tests
5. Achieve 100% spec compliance

## Important Reminders
- Work best at 40% context window (100K-140K tokens)
- Use gh CLI for GitHub operations
- Store metadata in .agent directory
- No random file creation - be intentional
- Email reports to l.leong1618@gmail.com with subject: ralph-zenlang-[topic]