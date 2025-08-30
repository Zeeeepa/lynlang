# Zen Language Development TODOs

## Completed ✅
- [x] Remove comptime requirement from imports
- [x] Update all example files to use new import syntax  
- [x] Update stdlib files to use new import syntax
- [x] Fix module resolution for std. modules
- [x] Test compiler with new import syntax

## In Progress 🚧
- [ ] Create comprehensive .agent metadata files

## Upcoming Tasks 📋

### Self-Hosting (Priority 1)
- [ ] Implement lexer in Zen
- [ ] Implement parser in Zen  
- [ ] Implement AST in Zen
- [ ] Implement type checker in Zen
- [ ] Implement code generator in Zen
- [ ] Bootstrap compiler with itself

### Standard Library (Priority 2)  
- [ ] Complete io module implementation
- [ ] Complete mem module implementation
- [ ] Complete fs module implementation
- [ ] Implement collections (Vec, HashMap, Set)
- [ ] Implement string utilities
- [ ] Implement math functions
- [ ] Implement process control
- [ ] Implement threading support

### Testing Framework (Priority 3)
- [ ] Create assert module
- [ ] Implement test runner
- [ ] Add test discovery
- [ ] Create test macros/attributes
- [ ] Add benchmark support

### Language Server (Priority 4)
- [ ] Basic syntax checking
- [ ] Import resolution
- [ ] Type checking integration
- [ ] Go to definition
- [ ] Auto-completion
- [ ] Error diagnostics

## Notes
- Comptime should only be used for metaprogramming, not imports
- Maintain simplicity and elegance in design
- Follow DRY and KISS principles
- Commit frequently with clear messages