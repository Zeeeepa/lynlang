# Zen Language Development Plan

## Current Phase: Self-Hosting & Stdlib Development

### Immediate Goals
1. ✅ Fix import syntax (comptime should be for metaprogramming only)
2. 🔄 Develop self-hosted parser in Zen
3. 🔄 Build comprehensive stdlib in Zen  
4. 🔄 Create testing framework
5. 🔄 Implement LSP or syntax checker

### Import Syntax Status
- ✅ Parser handles imports without comptime wrapper
- ✅ Examples updated to show correct syntax
- ✅ Type checker validates import placement
- ✅ LSP includes import syntax checking

### Self-Hosting Progress
Key components to port:
- [ ] Lexer (partial implementation exists)
- [ ] Parser (basic structure exists)
- [ ] Type checker (foundation exists)
- [ ] Code generator
- [ ] Module system

### Development Principles
- Simplicity, elegance, practicality, intelligence
- DRY (Don't Repeat Yourself) & KISS (Keep It Simple, Stupid)
- 80% implementation, 20% testing
- Frequent commits and pushes
- Work best at 40% context window (100K-140K tokens)