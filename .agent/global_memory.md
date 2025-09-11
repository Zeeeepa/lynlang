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
- Code Generator: ~80% complete (LLVM backend)
- Standard Library: ~80% complete (FFI implemented)
- Self-Hosting: ~35% complete
- LSP Server: ✅ Built successfully
- VSCode Extension: ✅ Configured and ready
- FFI System: ✅ Builder pattern implemented (2025-09-11)
- Test Suite: ✅ Spec compliance tests created (2025-09-11)

## Recent Work (2025-09-11)
- Reviewed LANGUAGE_SPEC.md v1.1.0
- Implemented FFI builder pattern in `/std/ffi.zen`
- Created comprehensive spec compliance test suite
- Built and tested LSP server (compiles successfully)
- Identified 1,440 if violations and 252 match violations
- Standard library is fully spec compliant

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