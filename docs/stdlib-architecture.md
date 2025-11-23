# Standard Library Architecture Analysis

## Executive Summary

The Zen language compiler has **three distinct stdlib-related folders** that serve different purposes but have overlapping responsibilities, leading to confusion and potential duplication:

1. **`stdlib/`** - Zen source files (`.zen` files) - User-facing standard library
2. **`src/stdlib_metadata/`** - Rust module registry/metadata - Type checking and module resolution
3. **`src/codegen/llvm/stdlib/`** - LLVM codegen implementations - Runtime code generation

## Folder Purposes and Responsibilities

### 1. `stdlib/` - Zen Source Standard Library

**Location**: `/home/ubuntu/zenlang/stdlib/`

**Purpose**: Contains the actual Zen language standard library source files that users import via `@std.*` syntax.

**Contents**:
- Zen source files (`.zen`) organized by module
- Modules: `core/`, `io/`, `math/`, `fs/`, `net/`, `collections/`, `memory/`, `compiler/`, `testing/`, `ffi/`
- Top-level files: `std.zen`, `string.zen`, `vec.zen`, `error.zen`, `random.zen`, `time.zen`

**Key Files**:
- `stdlib/std.zen` - Main entry point that re-exports all modules
- `stdlib/core/ptr.zen` - Safe pointer implementation
- `stdlib/core/option.zen` - Option type
- `stdlib/core/result.zen` - Result type
- `stdlib/io/io.zen` - IO operations (print, println, read_line)
- `stdlib/math/math.zen` - Math functions (sin, cos, sqrt, etc.)

**Usage**:
- Loaded by `ModuleSystem` (`src/module_system/mod.rs`) when resolving `@std.*` imports
- Indexed by LSP (`src/lsp/indexing.rs`, `src/lsp/stdlib_resolver.rs`) for IDE features
- Parsed as regular Zen code during compilation

**Status**: **Active** - This is the primary user-facing stdlib that should grow over time.

---

### 2. `src/stdlib_metadata/` - Rust Module Registry

**Location**: `/home/ubuntu/zenlang/src/stdlib_metadata/`

**Purpose**: Provides Rust-side metadata and type information for stdlib modules. Used primarily for:
- Type checking (`src/typechecker/stdlib.rs`)
- Module resolution (`src/stdlib_metadata/mod.rs`)
- Function signature registration

**Contents**:
- `mod.rs` - `StdNamespace`, `StdModule`, `StdModuleTrait`, `StdFunction` definitions
- `core.rs` - `CoreModule` (assert, panic, size_of, align_of, type_name, min, max, abs, clamp)
- `math.rs` - `MathModule` (sin, cos, tan, sqrt, pow, log, exp, floor, ceil, round, min, max, abs)
- `io.rs` - `IOModule` (print, println, eprint, eprintln, read_line, read_input)
- `fs.rs` - `FsModule` (read_file, write_file, exists, remove_file, create_dir, etc.)
- `compiler.rs` - `CompilerModule` (compiler intrinsics metadata)
- `vec.rs` - `VecModule` (vec operations metadata)
- `build.rs` - `BuildModule` (build system functions)
- `net.rs` - Network module declarations
- `result.rs` - Result/Option type helpers

**Key Structures**:
```rust
pub struct StdNamespace {
    modules: HashMap<String, StdModule>,
}

pub struct StdFunction {
    pub name: String,
    pub params: Vec<(String, AstType)>,
    pub return_type: AstType,
    pub is_builtin: bool,
}
```

**Usage**:
- Imported in `src/lib.rs` as `pub mod stdlib;`
- Used by `src/typechecker/mod.rs` via `use crate::stdlib::StdNamespace;`
- Functions registered in `src/typechecker/stdlib.rs` for type checking
- Referenced in `src/stdlib_metadata/mod.rs` for `@std.*` resolution

**Status**: **Active but Redundant** - Provides metadata that could be derived from Zen files.

---

### 3. `src/codegen/llvm/stdlib/` - LLVM Codegen Implementations

**Location**: `/home/ubuntu/zenlang/src/codegen/llvm/stdlib/`

**Purpose**: Contains the actual LLVM IR code generation functions for stdlib operations. These are the runtime implementations that get compiled into the final binary.

**Contents**:
- `mod.rs` - Delegation functions that route to specific modules
- `core.rs` - `compile_core_assert()`, `compile_core_panic()`
- `math.rs` - `compile_math_function()` (dispatches to libm functions)
- `io.rs` - `compile_io_print()`, `compile_io_println()`, `compile_io_print_int()`, `compile_io_print_float()`
- `fs.rs` - `compile_fs_read_file()`, `compile_fs_write_file()`, `compile_fs_exists()`, `compile_fs_remove_file()`, `compile_fs_create_dir()`
- `compiler.rs` - All compiler intrinsics: `compile_raw_allocate()`, `compile_raw_deallocate()`, `compile_gep()`, `compile_discriminant()`, etc.
- `collections.rs` - `compile_hashmap_new()`, `compile_hashset_new()`, `compile_dynvec_new()`
- `helpers.rs` - Result/Error helpers

**Usage**:
- Imported in `src/codegen/llvm/functions/mod.rs` as `pub mod stdlib;`
- Called from `src/codegen/llvm/functions/calls.rs` when compiling function calls
- Routes `@std.module.function` calls to appropriate LLVM codegen functions

**Status**: **Active and Necessary** - These are the actual implementations that must remain in Rust.

---

## Import Dependency Graph

```
User Code (@std.io.println)
    ↓
Parser (src/parser/statements.rs)
    ↓ Parses @std.* syntax
    ↓
Module System (src/module_system/mod.rs)
    ↓ Resolves to stdlib/io/io.zen
    ↓
Type Checker (src/typechecker/mod.rs)
    ↓ Uses src/stdlib_metadata/ for type info
    ↓ Checks against StdNamespace
    ↓
Code Generator (src/codegen/llvm/functions/calls.rs)
    ↓ Routes to src/codegen/llvm/stdlib/io.rs
    ↓ Calls compile_io_println()
    ↓
LLVM IR Generation
```

**Key Import Points**:

1. **`src/lib.rs`**: `pub mod stdlib_metadata;` - Exposes `src/stdlib_metadata/` module
2. **`src/typechecker/mod.rs`**: `use crate::stdlib::StdNamespace;` - Uses metadata for type checking
3. **`src/codegen/llvm/functions/mod.rs`**: `pub mod stdlib;` - Exposes codegen functions
4. **`src/codegen/llvm/functions/calls.rs`**: `use super::stdlib;` - Routes calls to codegen
5. **`src/module_system/mod.rs`**: Loads `stdlib/*.zen` files from filesystem
6. **`src/lsp/stdlib_resolver.rs`**: Resolves `@std.*` paths to `stdlib/` files

---

## Duplication Analysis

### Function Name Overlap

#### Math Functions
| Function | `stdlib/math/math.zen` | `src/stdlib_metadata/math.rs` | `src/codegen/llvm/stdlib/math.rs` |
|----------|------------------------|----------------------|---------------------------------------------|
| `sin`    | ✅ Declared            | ✅ Registered        | ✅ Implemented (`compile_math_function`)     |
| `cos`    | ✅ Declared            | ✅ Registered        | ✅ Implemented                               |
| `tan`    | ✅ Declared            | ✅ Registered        | ✅ Implemented                               |
| `sqrt`   | ✅ Declared            | ✅ Registered        | ✅ Implemented                               |
| `pow`    | ✅ Declared            | ✅ Registered        | ✅ Implemented                               |
| `log`    | ✅ Declared            | ✅ Registered        | ✅ Implemented                               |
| `exp`    | ✅ Declared            | ✅ Registered        | ✅ Implemented                               |
| `floor`  | ✅ Declared            | ✅ Registered        | ✅ Implemented                               |
| `ceil`   | ✅ Declared            | ✅ Registered        | ✅ Implemented                               |
| `round`  | ✅ Declared            | ✅ Registered        | ✅ Implemented                               |
| `min`    | ✅ Declared            | ✅ Registered        | ✅ Implemented                               |
| `max`    | ✅ Declared            | ✅ Registered        | ✅ Implemented                               |

**Analysis**: All three layers have math functions. This is **intentional** - Zen files declare, Rust metadata registers for type checking, codegen implements.

#### IO Functions
| Function    | `stdlib/io/io.zen` | `src/stdlib_metadata/io.rs` | `src/codegen/llvm/stdlib/io.rs` |
|-------------|-------------------|-------------------|-------------------------------------------|
| `print`     | ✅ Declared       | ✅ Registered     | ✅ Implemented (`compile_io_print`)        |
| `println`   | ✅ Declared       | ✅ Registered     | ✅ Implemented (`compile_io_println`)      |
| `eprint`    | ✅ Declared       | ✅ Registered     | ❌ Not implemented                         |
| `eprintln`  | ✅ Declared       | ✅ Registered     | ❌ Not implemented                         |
| `read_line` | ✅ Declared       | ✅ Registered     | ❌ Not implemented                         |
| `read_input`| ✅ Declared       | ✅ Registered     | ❌ Not implemented                         |

**Analysis**: Some IO functions are declared but not fully implemented in codegen. This is **incomplete implementation**, not duplication.

#### Core Functions
| Function    | `stdlib/core/*.zen` | `src/stdlib_metadata/core.rs` | `src/codegen/llvm/stdlib/core.rs` |
|-------------|---------------------|---------------------|---------------------------------------------|
| `assert`    | ❌ Not in Zen       | ✅ Registered       | ✅ Implemented (`compile_core_assert`)      |
| `panic`     | ❌ Not in Zen       | ✅ Registered       | ✅ Implemented (`compile_core_panic`)       |
| `size_of`   | ❌ Not in Zen       | ✅ Registered       | ❌ Not in codegen (compile-time)            |
| `align_of`  | ❌ Not in Zen       | ✅ Registered       | ❌ Not in codegen (compile-time)            |

**Analysis**: Core functions like `assert` and `panic` are **only in Rust layers**, not in Zen files. This is intentional - they're compiler intrinsics.

#### Compiler Intrinsics
| Function          | `stdlib/compiler/` | `src/stdlib_metadata/compiler.rs` | `src/codegen/llvm/stdlib/compiler.rs` |
|-------------------|---------------------|-------------------------|-------------------------------------------------|
| `raw_allocate`    | ✅ Used in Zen      | ✅ Registered           | ✅ Implemented                                   |
| `raw_deallocate`  | ✅ Used in Zen      | ✅ Registered           | ✅ Implemented                                   |
| `gep`             | ✅ Used in Zen      | ✅ Registered           | ✅ Implemented                                   |
| `discriminant`    | ✅ Used in Zen      | ✅ Registered           | ✅ Implemented                                   |
| `inline_c`        | ❌ Not in Zen       | ✅ Registered           | ✅ Implemented (placeholder)                    |

**Analysis**: Compiler intrinsics are declared in Rust metadata and implemented in codegen. Zen files **use** them but don't declare them (they're compiler magic).

#### FS Functions
| Function      | `stdlib/fs/fs.zen` | `src/stdlib_metadata/fs.rs` | `src/codegen/llvm/stdlib/fs.rs` |
|---------------|-------------------|-------------------|-------------------------------------------|
| `read_file`   | ✅ Declared       | ✅ Registered     | ✅ Implemented                            |
| `write_file`  | ✅ Declared       | ✅ Registered     | ✅ Implemented                            |
| `exists`      | ✅ Declared       | ✅ Registered     | ✅ Implemented                            |
| `remove_file` | ✅ Declared       | ✅ Registered     | ✅ Implemented                            |
| `create_dir`  | ✅ Declared       | ✅ Registered     | ✅ Implemented                            |

**Analysis**: FS functions are fully implemented across all three layers. This is **intentional** - complete implementation.

---

## Current Architecture Issues

### 1. **Naming Confusion**
All three folders are named "stdlib" which makes it unclear which one to modify or where to look for functionality.

### 2. **Metadata Duplication**
`src/stdlib_metadata/` contains function signatures that could be parsed from `stdlib/*.zen` files instead of being manually maintained in Rust.

### 3. **Incomplete Implementation**
Some functions are declared in Zen files and registered in Rust metadata but not implemented in codegen (e.g., `eprint`, `eprintln`, `read_line`).

### 4. **Missing Zen Declarations**
Some functions exist only in Rust layers (`assert`, `panic`, `size_of`, `align_of`) and aren't available as Zen source files.

### 5. **Unclear Boundaries**
It's not always clear which functions should be:
- Pure Zen implementations (e.g., `Option`, `Result`, `Ptr`)
- Rust codegen implementations (e.g., `print`, `math.sin`)
- Compiler intrinsics (e.g., `raw_allocate`, `gep`)

---

## Recommended Consolidation Strategy

### Phase 1: Rename for Clarity (COMPLETED)

**Implementation**: Descriptive Names

1. **Kept**: `stdlib/` - User-facing Zen source files
2. **Renamed**: `src/stdlib/` → `src/stdlib_metadata/` ✅
   - Makes it clear this is metadata/registry for type checking
3. **Renamed**: `src/codegen/llvm/functions/stdlib/` → `src/codegen/llvm/stdlib/` ✅
   - Moved up one level for clarity (stdlib codegen is at same level as functions)
   - Makes it clear this is codegen-specific implementations

**Status**: ✅ Completed - All imports updated and code compiles successfully.

### Phase 2: Eliminate Metadata Duplication (Medium-term)

**Goal**: Parse function signatures from Zen files instead of maintaining them in Rust.

**Approach**:
1. Parse `stdlib/*.zen` files during compiler initialization
2. Extract function signatures automatically
3. Use parsed signatures for type checking instead of `src/stdlib_metadata/*.rs` registries
4. Keep `src/stdlib_metadata/` only for functions that can't be expressed in Zen (compiler intrinsics)

**Benefits**:
- Single source of truth (Zen files)
- No manual sync between Zen declarations and Rust metadata
- Easier to add new stdlib functions

**Challenges**:
- Requires robust Zen AST parsing
- Need to handle built-in functions that aren't in Zen files

### Phase 3: Complete Implementation (Short-term)

**Goal**: Implement all declared functions.

**Actions**:
1. Implement missing IO functions: `eprint`, `eprintln`, `read_line`, `read_input`
2. Add Zen declarations for compiler intrinsics: `assert`, `panic`, `size_of`, `align_of`
3. Document which functions are compiler intrinsics vs regular functions

### Phase 4: Documentation Cleanup (Immediate)

**Actions**:
1. Update `ARCHITECTURE.md` to reference this document
2. Add comments in code explaining the three-layer architecture
3. Remove outdated design docs that contradict current structure
4. Add README in each folder explaining its purpose

---

## Detailed Function Inventory

### Functions in All Three Layers (Complete Implementation)
- Math: `sin`, `cos`, `tan`, `sqrt`, `pow`, `log`, `exp`, `floor`, `ceil`, `round`, `min`, `max`
- IO: `print`, `println`
- FS: `read_file`, `write_file`, `exists`, `remove_file`, `create_dir`
- Compiler: `raw_allocate`, `raw_deallocate`, `raw_reallocate`, `gep`, `gep_struct`, `discriminant`, `get_payload`, `set_payload`, `load`, `store`, `ptr_to_int`, `int_to_ptr`

### Functions in Two Layers (Incomplete)
- IO: `eprint`, `eprintln`, `read_line`, `read_input` (declared + registered, not implemented)

### Functions Only in Rust Layers (Compiler Intrinsics)
- Core: `assert`, `panic`, `size_of`, `align_of`, `type_name`
- Compiler: `inline_c`, `call_external`, `load_library`, `get_symbol`, `unload_library`, `null_ptr`

### Functions Only in Zen Files (Pure Zen Implementation)
- Core: `Option`, `Result`, `Ptr` types and their methods
- Memory: `GPA`, `Allocator` types
- Collections: `HashMap`, `HashSet`, `DynVec` types
- String: `String` type and methods
- Vec: `Vec`, `Array` types and methods

---

## Migration Path

### Step 1: Rename Folders (Low Risk)
1. ✅ Rename `src/stdlib/` → `src/stdlib_metadata/` (COMPLETED)
2. Update all imports in codebase
3. ✅ Rename `src/codegen/llvm/functions/stdlib/` → `src/codegen/llvm/stdlib/` (COMPLETED)
4. Update all imports in codebase
5. Test compilation

### Step 2: Document Boundaries (No Code Changes)
1. Add README files explaining each folder's purpose
2. Document which functions belong in which layer
3. Create decision tree for adding new stdlib functions

### Step 3: Eliminate Duplication (Medium Risk)
1. Implement signature parsing from Zen files
2. Gradually migrate from Rust registries to parsed signatures
3. Keep Rust registries only for compiler intrinsics
4. Test thoroughly

### Step 4: Complete Implementation (Low Risk)
1. Implement missing IO functions
2. Add Zen declarations for compiler intrinsics
3. Ensure all declared functions are implemented

---

## Conclusion

The three stdlib folders serve distinct purposes but have confusing names and some duplication. The recommended approach is:

1. **Rename for clarity** - Make purpose of each folder obvious
2. **Eliminate metadata duplication** - Parse signatures from Zen files
3. **Complete implementations** - Ensure all declared functions work
4. **Document boundaries** - Clear guidelines for what goes where

The architecture is fundamentally sound - Zen files for user-facing code, Rust metadata for type checking, Rust codegen for implementations. The main issue is naming and some incomplete implementations.

