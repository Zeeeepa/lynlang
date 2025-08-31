# Zen Language Development TODO List

## Completed Tasks ✅
1. ✅ Fix import syntax - remove comptime wrapper for imports
2. ✅ Update stdlib files using old comptime import syntax  
3. ✅ Update example files using old comptime import syntax
4. ✅ Verify core stdlib modules are implemented in Zen
5. ✅ Create comprehensive test suite (run_tests.sh)
6. ✅ Set up automated testing script
7. ✅ Verify self-hosting components (lexer, parser documented)
8. ✅ Create advanced linter/syntax checker (zen-lint.sh)
9. ✅ Document self-hosting status

## In Progress 🔄

## Pending Tasks 📋

### Next Priority: Memory Management
10. 📋 Implement malloc/free integration
    - Add proper external function declarations
    - Integrate with LLVM runtime
    - Test memory allocation for Vec and HashMap

### Next Priority: Bootstrap Process  
11. 📋 Define bootstrap sequence
    - Compile small programs with both compilers
    - Compare output between Rust and self-hosted
    - Ensure compatibility

### Documentation (Priority: Low)
9. 📋 Document testing process in .agent directory
   - Testing conventions
   - How to add new tests
   - CI/CD process

## Notes
- Import syntax has been updated: no more `comptime { imports }` wrapper needed
- Stdlib modules are already well implemented
- Self-hosting components exist in stdlib (lexer, parser, ast, type_checker, codegen)
- Focus should be on testing and validation