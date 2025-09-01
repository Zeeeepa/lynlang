# Zen Language Import Syntax Fix and Self-Hosting Plan

## Current Sprint: Fix Import Syntax
Move imports outside of comptime blocks to be regular top-level statements.

### Phase 1: Parser Changes (Current)
1. Update parser to handle top-level imports
2. Remove requirement for comptime wrapper around imports
3. Ensure backwards compatibility during transition

### Phase 2: Semantic Analysis
1. Update semantic analyzer to process top-level imports
2. Ensure proper module resolution
3. Update type checking for imported symbols

### Phase 3: Update Codebase
1. Fix all test files
2. Update stdlib modules
3. Update examples and documentation

### Phase 4: Self-Hosting Validation
1. Run self-hosting tests
2. Bootstrap compiler with itself
3. Validate all features work correctly

## Long-term Goals
- Complete self-hosting
- Stdlib written in Zen
- Comprehensive test suite
- LSP support for development