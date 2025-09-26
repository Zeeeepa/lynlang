# Zen Language: Allocator & Collection Analysis

## Date: 2025-09-26

## Current State Analysis

### 1. **Critical Issue: NO ALLOCATOR SUPPORT IN COLLECTIONS**

#### HashMap Issues:
- **SPEC VIOLATION**: HashMap should take allocator as per LANGUAGE_SPEC.zen
- Current: `HashMap<K, V>` has `allocator: Allocator` field but doesn't properly use it
- Uses undefined `Array` type for buckets instead of allocator-based dynamic array
- Line 15: `buckets: Array<Option<Ptr<HashBucket<K, V>>>>` - Array is not defined!
- Line 25, 35: `Array.new()` calls but Array doesn't exist as a stdlib type

#### Vec Issues:
- **Fixed-size Vec<T, N>** doesn't need allocator (stack allocated) ✓
- **DynVec<T>** DOES take allocator properly ✓
  - Has `allocator: Allocator` field
  - Uses it for `allocate<T>()` and `free()` calls
  - Proper implementation per spec

#### Array Type Confusion:
- `Array` used in HashMap but NOT defined in stdlib
- Compiler has internal `AstType::Array` support
- No stdlib implementation of Array type
- Creates compilation errors when HashMap tries to use it

### 2. **Nested Generics Status**

#### Working:
- Simple nested generics work perfectly:
  - `Option<Option<T>>` ✓
  - `Result<Option<T>, E>` ✓
  - `Option<Result<T, E>>` ✓
  - `Result<Option<Result<T,E>>,E>` (triple nesting!) ✓

#### Not Working:
- Collections with nested generics have issues:
  - `HashMap<K, Option<V>>` - syntax errors
  - `DynVec<Result<T, E>>` - syntax errors  
  - Need to use dot notation not colon notation

### 3. **Array vs Vec vs DynVec Analysis**

Current types:
1. **Vec<T, N>** - Fixed-size, stack-allocated, no allocator needed
2. **DynVec<T>** - Dynamic, heap-allocated, uses allocator
3. **Array** - Referenced but not implemented, causing bugs!

**THE PROBLEM**: We have references to `Array` type that doesn't exist!

## Recommendations

### Immediate Actions Needed:

1. **Fix HashMap to use DynVec instead of Array**:
   ```zen
   HashMap<K, V> = {
       buckets: DynVec<Option<Ptr<HashBucket<K, V>>>>  // Not Array!
       size: usize
       capacity: usize
       allocator: Allocator
   }
   ```

2. **Remove all Array references** or implement Array properly:
   - Option A: Replace Array with DynVec everywhere
   - Option B: Implement Array as alias to DynVec
   - Option C: Implement Array as distinct type (not recommended - redundant)

3. **Fix allocator passing**:
   - HashMap already takes allocator but uses undefined Array
   - Need to use DynVec with that allocator

4. **Test nested generics with collections after fixes**

### Design Decision Needed:

**Do we need Array at all?**
- Vec<T, N> handles fixed-size arrays
- DynVec<T> handles dynamic arrays  
- Array seems redundant

**Recommendation**: Remove Array entirely, use:
- `Vec<T, N>` for stack-allocated fixed arrays
- `DynVec<T>` for heap-allocated dynamic arrays with allocators

This gives us allocator control (sync vs async) and clear memory semantics.

## Action Plan

1. Update HashMap to use DynVec instead of non-existent Array
2. Ensure DynVec is properly imported in hashmap.zen
3. Update HashMap.new() to initialize DynVec with allocator
4. Test HashMap with allocators
5. Test nested generics with collections
6. Remove any other Array references
7. Update documentation to clarify Vec vs DynVec usage

## Impact

This will:
- Enable NO-GC goal (allocators control memory)
- Fix HashMap compilation errors  
- Enable proper async/sync behavior via allocators
- Simplify type system (no redundant Array type)
- Improve nested generics support