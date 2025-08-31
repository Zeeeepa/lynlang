# Zenlang Import System & Self-Hosting Plan

## Current Sprint: Remove Comptime Import Requirements

### Objective
Transform all imports from comptime-wrapped syntax to clean module-level syntax, following the pattern:
```zen
// OLD (to remove)
comptime {
    core := @std.core
    io := build.import("io")
}

// NEW (target)
core := @std.core
io := @std.io
```

## Phases

### Phase 1: Import System Cleanup (Current)
1. ✅ Parser already supports new syntax
2. 🚧 Clean up remaining examples with old syntax
3. 🚧 Re-enable typechecker validation
4. 🚧 Comprehensive testing

### Phase 2: Self-Hosting Enhancement
1. Complete self-hosted lexer integration
2. Enhance self-hosted parser
3. Implement self-hosted code generator
4. Bootstrap compiler with itself

### Phase 3: Developer Experience
1. LSP import validation
2. Import auto-completion
3. Module dependency analysis
4. Import optimization

## Technical Details

### Parser Status
- Location: `src/parser/statements.rs:14-120`
- Feature: Lookahead for `identifier := @std.module`
- Status: ✅ Working

### Module System
- Location: `src/module_system/mod.rs`
- Features: Module resolution, stdlib handling
- Status: ✅ Working

### Validation
- Location: `src/typechecker/validation.rs:160-181`
- Status: ⚠️ Disabled (needs re-enabling)

### Test Coverage
- Import rejection tests: ✅
- No-comptime import tests: ✅
- Integration tests: 🚧 Need updates

## Success Metrics
1. Zero comptime-wrapped imports in codebase
2. All tests passing
3. Self-hosted compiler can compile itself
4. LSP provides import validation

## Git Strategy
- Frequent commits (every significant change)
- Clear commit messages
- Test before commit
- Merge to main when stable

## Time Allocation
- 80% Implementation
- 20% Testing & Validation