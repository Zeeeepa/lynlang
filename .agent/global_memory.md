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
- Standard Library: ~75% complete (FFI added, imports fixed)
- Self-Hosting: ~30% complete
- LSP Server: ✅ Built successfully (imports fixed)
- VSCode Extension: ✅ Configured and ready
- FFI System: ✅ Builder pattern implemented

## Recent Work (2025-09-10)
- Fixed stdlib/testing.zen to use ? operator instead of if/else
- Fixed ~200+ if/else/match violations across 6 critical files
- Built zen-lsp binary successfully
- Identified ~2000+ remaining violations to fix
- **Today's Progress**:
  - Fixed all @std imports in critical files (lexer, parser, lsp, stdlib)
  - Implemented FFI builder pattern as per language spec
  - Created language spec compliance test suite
  - Fixed pattern matching syntax (? | pattern => result)
  - Removed non-@std @ references (replaced with build.import)

## Key Files
- `/LANGUAGE_SPEC.md` - Authoritative language specification
- `/docs/lang.md` - Original conceptual specification
- `/examples/WORKING_FEATURES.md` - Current implementation status
- `/src/` - Rust implementation (being replaced)
- `/std/` - Standard library (written in Zen)

## Development Principles
- DRY (Don't Repeat Yourself)
- KISS (Keep It Simple, Stupid)
- Test coverage target: 80% implementation, 20% testing
- Frequent git commits
- Simplicity, elegance, practicality, intelligence

## Remaining Critical Tasks
- Fix remaining ~1500+ if/else violations across codebase
- Fix remaining ~300+ match keyword violations
- Complete implementation of stdlib modules
- Test self-hosting compiler
- Ensure all tests pass

## Important Reminders
- Work best at 40% context window (100K-140K tokens)
- Use gh CLI for GitHub operations
- Store metadata in .agent directory
- No random file creation - be intentional
- Email reports to l.leong1618@gmail.com with subject: ralph-zenlang-[topic]