# Failed Attempts & Lessons Learned

## Things That Didn't Work (Don't Repeat These)

### 1. Direct Payload Extraction Without Null Checks
**Problem**: Extracting enum payloads before checking if they exist caused segfaults
**Error**: Segmentation fault when matching Option<None> after Some(x)
**Solution**: Added null pointer checks and proper control flow branching with PHI nodes

### 2. Type Checking Block-Scoped Variables Too Strictly  
**Problem**: Overly strict type checking prevented valid block-scoped variable usage
**Error**: "Variable already declared" errors in valid pattern matching
**Solution**: Temporarily disabled strict type checking for complex cases

### 3. Using i64 as Default Integer Type
**Problem**: Defaulting to i64 for all integers caused type mismatches
**Error**: LLVM type mismatch errors in binary operations
**Solution**: Changed to i32 as default, then try i64 if needed

### 4. Unconditional Payload Extraction in Pattern Matching
**Problem**: Pattern matching extracted payloads before checking discriminants
**Error**: Null pointer dereferences and segfaults
**Solution**: Defer payload extraction until after discriminant check passes

### 5. Mixed String/Integer Enum Payloads Without Type Tags
**Problem**: Storing both strings (pointers) and integers (wrapped in pointers) in same field
**Error**: String payloads misinterpreted as integers, causing crashes
**Status**: UNFIXED - Needs runtime type information or separate fields

### 6. Return Statements for Result<T,E> Functions
**Problem**: Return statements expect simple types but Result<T,E> is LLVM struct {i64, ptr}
**Error**: Type mismatch in return statements  
**Status**: UNFIXED - Major architectural issue affecting 11 tests

### 7. Generic Type Instantiation for Nested Types
**Problem**: Complex nested generics like Result<Option<T>,E> don't instantiate correctly
**Error**: Type inference failures and incorrect payload extraction
**Status**: PARTIALLY FIXED - Basic types work, complex nesting doesn't

### 8. Using -> for Pattern Matching Syntax
**Problem**: Old syntax used -> which isn't in LANGUAGE_SPEC.zen
**Error**: Parser errors
**Solution**: Replaced with | syntax throughout codebase

### 9. Using extern Instead of inline.c()
**Problem**: Old FFI syntax used extern keyword
**Error**: Syntax violations of LANGUAGE_SPEC.zen
**Solution**: Replaced all extern with inline.c(...) 

### 10. Test Files in Root Directory
**Problem**: Test files cluttered root directory
**Error**: Poor project organization
**Solution**: Moved all test files to tests/ folder with proper naming

## Patterns to Avoid:
- Don't assume payload exists without checking
- Don't default to i64 for integers - use i32 first
- Don't mix return types without proper LLVM struct handling
- Don't put test files in root directory
- Don't use deprecated syntax (extern, ->, export)
- Don't extract payloads before discriminant checks