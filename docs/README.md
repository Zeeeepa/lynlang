# Zen Language Documentation

## Start Here

| Document | Description |
|----------|-------------|
| [OVERVIEW.md](OVERVIEW.md) | Complete language overview, syntax, and features |
| [QUICK_START.md](QUICK_START.md) | Getting started guide with examples |
| [../README.md](../README.md) | Project overview and build instructions |

---

## Reference

| Document | Description |
|----------|-------------|
| [INTRINSICS_REFERENCE.md](INTRINSICS_REFERENCE.md) | Compiler intrinsics documentation |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Compiler pipeline and module structure |
| [LSP_STATUS.md](LSP_STATUS.md) | Language server features and architecture |

---

## Design Documents

For contributors understanding the architecture:

| Document | Description |
|----------|-------------|
| [design/STDLIB_DESIGN.md](design/STDLIB_DESIGN.md) | Standard library architecture and API |
| [design/SAFE_POINTERS_DESIGN.md](design/SAFE_POINTERS_DESIGN.md) | Ptr<T> vs raw pointer rationale |
| [design/SEPARATION_OF_CONCERNS.md](design/SEPARATION_OF_CONCERNS.md) | Compiler layer responsibilities |
| [design/PRIMITIVES_VS_FEATURES.md](design/PRIMITIVES_VS_FEATURES.md) | What belongs in compiler vs stdlib |

---

## Development Status

| Document | Description |
|----------|-------------|
| [ROADMAP_2026-01.md](ROADMAP_2026-01.md) | Current development priorities |
| [PHASE_PLAN.md](PHASE_PLAN.md) | Phase plan from Alpha to Beta |

---

## Internal/Working Documents

These track ongoing refactoring work:

| Document | Description |
|----------|-------------|
| [REFACTOR_SEMA.md](REFACTOR_SEMA.md) | Typechecker integration status |
| [REFACTOR_RESOLVED_TYPES.md](REFACTOR_RESOLVED_TYPES.md) | AST type annotation refactor |

---

## Quick Commands

```bash
# Build
cargo build --release

# Test
cargo test --all

# Run example
./target/release/zen examples/showcase.zen

# Run LSP
./target/release/zen-lsp
```
