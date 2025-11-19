# Tests Directory Cleanup

## Summary

The `/tests` directory has been cleaned up to remove all generated artifacts (.c, .ll, .o files and test binaries).

## Changes Made

1. **Removed tracked test artifacts**:
   - `tests/test_string_types_new` (ELF binary)
   - `tests/test_string_types_new.ll` (LLVM IR)
   - `tests/test_c_strtod.c` (C test file)
   - `tests/test_hashmap_basic_debug.c` (C test file)

2. **Updated `.gitignore`**:
   - Explicitly preserve intentional test source files (`.zen` files)
   - Ensure generated artifacts remain ignored (.c, .ll, .o, binaries)

## Artifact Location

When running tests:

- **Without `-o` flag** (normal test execution):
  - The compiler uses JIT execution
  - No files are written to disk
  - This is the mode used by `python3 scripts/run_tests.py`

- **With `-o` flag** (explicit compilation):
  - Generated files go to `target/` directory
  - If no directory is specified in output name, automatically prefixed with `target/`
  - Example: `zen myfile.zen -o myapp` creates `target/myapp`, `target/myapp.ll`, `target/myapp.o`

## Test Execution

The standard test suite:
```bash
make test
# or
python3 scripts/run_tests.py
```

This runs Zen files directly without the `-o` flag, so no artifacts are generated. All test outputs are transient (JIT execution only).

## Why This Matters

- Keeps the repository clean
- Tests are reproducible without accumulated build artifacts
- Generated files don't pollute version control
- Separates source code from build artifacts
