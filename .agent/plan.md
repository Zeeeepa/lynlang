# Zen Language Import System Fix Plan

## Goal
Remove the requirement for `comptime` blocks for imports, allowing top-level imports directly.

## Current State
- Imports currently require `comptime { ... }` blocks
- Parser already handles some module imports at top-level (build.import, @std)
- Need to generalize this to all imports

## Implementation Steps

1. **Parser Changes** (src/parser/statements.rs)
   - Already partially supports top-level imports
   - Need to ensure all import patterns work without comptime
   - Remove comptime requirement validation

2. **Semantic Analysis** 
   - Update validation to accept top-level imports
   - Ensure proper module resolution

3. **Self-Hosted Components**
   - Update self-hosted parser (compiler/parser.zen)
   - Fix self-hosted lexer if needed
   - Enhance stdlib modules

4. **Testing**
   - Update existing tests for new syntax
   - Add comprehensive import tests
   - Ensure backward compatibility where needed

5. **Tooling**
   - Basic LSP or checking tool for validation
   - Linting capabilities

## Priority Order (80% implementation, 20% testing)
1. Fix parser to accept top-level imports (30%)
2. Update semantic analyzer (20%)
3. Fix existing tests (10%)
4. Enhance self-hosted components (20%)
5. Create comprehensive test suite (10%)
6. Basic tooling/LSP (10%)
