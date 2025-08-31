# Zen Language Import System Refactor Plan

## Goal
Fix imports to work at module level instead of inside comptime blocks, enabling self-hosting.

## Current State
- Imports currently work inside comptime blocks: `comptime { io := @std.io }`
- Need to move to module-level: `io := @std.io`
- Parser, semantic analyzer, and code generation need updates

## Implementation Order (80% coding, 20% testing)

### Phase 1: Core Import System Fix (Priority 1)
1. Update parser to handle module-level imports
2. Update semantic analyzer for new import syntax  
3. Update code generation for imports
4. Create basic tests

### Phase 2: Self-Hosting Foundation (Priority 2)
1. Port lexer to Zen
2. Port parser to Zen
3. Port type checker to Zen
4. Create bootstrap compiler

### Phase 3: Standard Library (Priority 3)
1. Core modules (io, mem, string)
2. Collections (vec, hashmap, list)
3. Testing framework
4. Build system

### Phase 4: Tooling (Priority 4)
1. Improve zen-check syntax checker
2. Basic LSP functionality
3. Package manager basics

## Key Files to Modify
- compiler/parser.zen - Update import parsing
- compiler/lexer.zen - Ensure proper tokenization
- compiler/type_checker.zen - Type checking for imports
- compiler/codegen.zen - Code generation for imports

## Testing Strategy
- Unit tests for each component
- Integration tests for full compilation
- Bootstrap test to ensure self-hosting works