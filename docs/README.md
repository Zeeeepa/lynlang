# Zen Language Documentation Index

## Quick Navigation

### Getting Started
- **[../README.md](../README.md)** - Project overview and goals
- **[QUICK_START.md](QUICK_START.md)** - Getting started guide

### Reference Documentation
- **[INTRINSICS_REFERENCE.md](INTRINSICS_REFERENCE.md)** - Compiler primitives documentation

### Architecture & Design
For contributors, read these in order:
1. **[design/ARCHITECTURE.md](design/ARCHITECTURE.md)** - LLVM primitives vs Zen features, decision trees, examples
2. **[design/SEPARATION_OF_CONCERNS.md](design/SEPARATION_OF_CONCERNS.md)** - Compiler primitives vs Zen-defined types

Additional design docs:
- **[design/NEXT_STEPS.md](design/NEXT_STEPS.md)** - Development roadmap
- **[design/STDLIB_DESIGN.md](design/STDLIB_DESIGN.md)** - Standard library architecture & API
- **[design/SAFE_POINTERS_DESIGN.md](design/SAFE_POINTERS_DESIGN.md)** - Ptr<T> vs Ref<T> rationale
- **[design/SAFE_TYPE_SYSTEM_DESIGN.md](design/SAFE_TYPE_SYSTEM_DESIGN.md)** - Type system architecture

### VS Code Extension
- **[../vscode-extension/README.md](../vscode-extension/README.md)** - Extension setup and usage

---

## Code Organization

```
src/
├── ast/                # AST definitions
├── lexer.rs            # Tokenization
├── parser/             # Syntax analysis
├── codegen/            # Code generation
│   └── llvm/           # LLVM backend
│       ├── functions/  # Function codegen
│       └── stdlib_codegen/  # Stdlib codegen helpers
├── stdlib_metadata/    # Intrinsic definitions (Rust)
├── lsp/                # Language server implementation
└── ...

stdlib/                 # Standard library (Zen code)
├── core/               # Core types (Ptr, Option, etc.)
├── memory/             # Memory management (Allocator, GPA)
├── collections/        # Collections (HashMap, Set, etc.)
├── io/                 # I/O operations
├── string.zen          # String type
├── vec.zen             # Vector type
└── ...

tests/                  # Integration tests
├── *.rs                # Rust test files
└── lsp/                # LSP feature tests

docs/                   # All documentation
├── design/             # Architecture and design docs
└── ...
```

---

## Quick Commands

```bash
# Build
cargo build

# Test
cargo test --all
cargo test pattern    # specific tests

# Run example
cargo run --bin zen examples/hello_world.zen
```

---

## Key Files

| File | Purpose |
|------|---------|
| `docs/design/NEXT_STEPS.md` | Development roadmap |
| `docs/design/STDLIB_DESIGN.md` | Stdlib architecture |
| `docs/QUICK_START.md` | Getting started |
| `examples/hello_world.zen` | Working examples |
| `stdlib/string.zen` | String implementation |
| `stdlib/vec.zen` | Vector implementation |
| `src/stdlib_metadata/compiler.rs` | Compiler intrinsics |

---

## Getting Help

1. Check **design/NEXT_STEPS.md** for current priorities
2. Read **design/STDLIB_DESIGN.md** for stdlib details
3. Look at **INTRINSICS_REFERENCE.md** for primitive docs
4. Review **../tests/** for usage examples
5. Check **../examples/** for working code samples
