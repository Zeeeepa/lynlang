# Zen Language Development Todos

## Priority Order (Estimated)

### ✅ Completed
1. [x] Fix comptime import blocks - All imports now at module level

### 🚧 In Progress
2. [ ] **Bootstrap Compiler Foundation** (40% complete)
   - [x] Created bootstrap_compiler.zen with full pipeline structure
   - [ ] Integrate with existing lexer/parser/type_checker
   - [ ] Implement LLVM backend integration
   - [ ] Test compilation of simple programs

### 📋 Planned

3. [ ] **Self-Hosted Stdlib Modules** (20% ready)
   - [ ] Complete core module (memory, types, runtime)
   - [ ] Complete io module (print, read, file operations)
   - [ ] Complete fs module (file system operations)
   - [ ] Complete string module (string manipulation)
   - [ ] Complete vec module (vector/array operations)

4. [ ] **Comprehensive Test Suite**
   - [ ] Unit tests for lexer
   - [ ] Unit tests for parser
   - [ ] Unit tests for type checker
   - [ ] Integration tests for full pipeline
   - [ ] Self-hosting validation tests

5. [ ] **LSP Implementation**
   - [x] Basic LSP structure (zen-lsp-basic.zen)
   - [ ] Integrate with actual parser for diagnostics
   - [ ] Add completion support
   - [ ] Add hover support
   - [ ] Add go-to-definition

6. [ ] **Self-Hosting Compilation**
   - [ ] Stage 1: Compile stdlib with Rust compiler
   - [ ] Stage 2: Compile compiler with Stage 1 output
   - [ ] Stage 3: Compile compiler with itself
   - [ ] Validation: Binary comparison

## Working Principles
- 80% implementation, 20% testing
- Frequent git commits
- DRY & KISS principles
- Work at 40% context window (100K-140K tokens)
- Use .agent directory for meta information