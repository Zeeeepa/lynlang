# LSP Test Suite

## Overview

This directory contains tests for the Zen Language Server Protocol (LSP) implementation.

## Current State

- **61 test files** - Many duplicates and outdated tests
- **Cleanup needed** - Consolidate into unified test suite
- **Modern features** - Most LSP 3.17 features implemented

## Quick Start

### Run Unified Test Suite
```bash
python3 tests/lsp/test_unified_lsp.py
```

### Run Hello World Tests
```bash
python3 tests/lsp/test_hello_world.py
```

### Cleanup Old Tests (backup first!)
```bash
cd tests/lsp
./cleanup_tests.sh
```

## Test Files

### Keep These (Modern, Comprehensive)
- `test_unified_lsp.py` - **Main unified test suite** ⭐
- `test_hello_world.py` - Hello world specific tests
- `test_comprehensive_automated.py` - Comprehensive automated tests
- `test_all_lsp_features.py` - All features test

### Remove These (Duplicates, Debug, Outdated)
See `cleanup_tests.sh` for full list of files to remove.

## Test Coverage

### ✅ Tested Features
- Hover (variables, functions, structs, format strings)
- Completion (struct fields, methods)
- Go to Definition
- Signature Help
- Document Symbols

### 🔧 Needs Testing
- Cross-file navigation
- Workspace symbols
- Code actions
- Semantic tokens
- Inlay hints
- Rename
- Formatting
- Call hierarchy
- References
- Type definition

## LSP Features Checklist

See `LSP_FEATURES_CHECKLIST.md` for complete feature list and status.

## Running Tests

### Prerequisites
```bash
# Build LSP first
cargo build --release
```

### Run All Tests
```bash
# Unified suite (recommended)
python3 tests/lsp/test_unified_lsp.py

# Hello world specific
python3 tests/lsp/test_hello_world.py
```

### Test Individual Features
The unified test suite can be extended with more test methods.

## Adding New Tests

1. Add test method to `UnifiedLSPSuite` class
2. Register in `run_all_tests()` method
3. Follow existing test patterns
4. Update checklist when feature is tested

## Test Structure

```python
def test_feature_name(self, result: TestResult):
    """Test description"""
    # Setup
    uri = f"file://{os.getcwd()}/test_file.zen"
    self.client.did_open(uri, content)
    
    # Execute
    req_id = self.client.send_request("textDocument/feature", {...})
    response = self.client.wait_for_response(req_id)
    
    # Assert
    if response and "result" in response:
        result.passed = True
        result.message = "Success message"
    else:
        result.message = "Failure message"
```

## TODO

See main project TODO list for LSP testing and fixes.

## Notes

- Tests use LSP protocol directly (no client library)
- Tests are synchronous (wait for responses)
- Test files are created temporarily and cleaned up
- All tests should be deterministic and repeatable

