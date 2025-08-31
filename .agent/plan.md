# Zenlang Development Plan

## Immediate Goals (Priority 1)
1. **Fix Import System** - Remove comptime requirement for imports
   - Move imports to module level
   - Update parser to handle module-level imports
   - Ensure imports work without comptime blocks

2. **Update Examples** - Ensure all examples use new syntax
   - Update hello.zen, fibonacci.zen, etc.
   - Test each example after changes

## Short-term Goals (Priority 2)
3. **Stdlib in Zen** - Implement core stdlib modules
   - io module (print, input)
   - math module (basic operations)
   - string module (manipulation)
   - memory module (allocation)

4. **Testing Framework** - Create comprehensive tests
   - Unit tests for parser
   - Integration tests for compiler
   - Example program tests

## Medium-term Goals (Priority 3)
5. **LSP/Validation** - Implement checking system
   - Basic syntax checking
   - Type checking
   - Import resolution

6. **Self-hosting** - Port compiler to Zen
   - Start with lexer
   - Then parser
   - Finally code generation

## Principles
- Simplicity and elegance
- Practical solutions
- Frequent commits
- 80% implementation, 20% testing
- DRY & KISS