# Zen Language Roadmap

**Status**: Late Alpha | **Updated**: January 2026

---

## What Works

- Zero-keyword syntax with pattern matching (`?`)
- Type system: structs, enums, generics, Option<T>, Result<T,E>
- UFC (Uniform Function Call)
- Zig-style allocators (GPA, Arena, Pool)
- Collections: Vec, String, HashMap, Stack, Queue, Set
- Safe pointers: Ptr<T>, MutPtr<T>, RawPtr<T>
- Syscall-based I/O (Linux x86-64)
- Full LSP support
- 25+ compiler intrinsics

---

## Current Priorities

### 1. Module System Fixes
Generic type resolution across module boundaries has issues:
- Monomorphization of imported generic types fails
- Type inference for behavior implementations incomplete

This blocks runtime testing of stdlib from .zen files.

### 2. Iterator Combinators
Need first-class closures for:
```zen
vec.iter().map(fn).filter(fn).collect()
```

Currently only specialized methods work (sum, product, min, max).

### 3. Cross-Platform Support
- [ ] macOS syscalls
- [ ] Windows syscalls

### 4. Architecture Cleanup
- Remove duplicate type inference from codegen (~1000 LOC)
- Split large modules (codegen/ 11K LOC, lsp/ 12K LOC)

---

## Self-Hosting Path

1. Intrinsics only in Rust compiler (mostly done)
2. Stdlib in Zen (done)
3. Lexer/Parser in Zen
4. Typechecker in Zen
5. Codegen in Zen (LLVM bindings or custom backend)

---

## Commands

```bash
cargo build --release          # Build compiler
cargo test --all               # Run tests
./target/release/zen FILE      # Run .zen file
./target/release/zen-lsp       # Start LSP
```
