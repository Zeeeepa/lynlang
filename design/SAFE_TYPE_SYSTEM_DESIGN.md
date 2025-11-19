# Safe Type System: Eliminate * and & Syntax

## Problem with * and &

The `*` and `&` syntax is inherited from C and is a source of confusion:

```zen
❌ BAD (current Zen):
ptr: *u8              // What is *? Pointer? Dereference? Indirection?
arr: *i32             // Inconsistent with pattern matching
ref: &String          // & is borrow? Reference? Something else?
addr = &value         // Taking address?
val = *ptr            // Dereferencing?

This is confusing. Users don't know:
- Is * a type or an operator?
- Is & a type or an operator?
- Why do we need both?
- How do they compose with generics?
```

## Solution: Explicit Type-Based System

Make everything a first-class type. No special syntax.

```zen
✅ GOOD (proposed):
ptr: Ptr<u8>          // Clear: this is a pointer
ref: Ref<String>      // Clear: this is a reference
addr = ptr_addr(ptr)  // Explicit function call
val = ptr_value(ptr)  // Explicit function call
```

---

## Type System Design

### 1. Ptr<T> - Owned Pointer (Heap Allocation)

```zen
Ptr<T>: enum {
    Some: Ptr_Payload<T>,  // Internal repr of pointer
    None                   // Null
}

// Represents: ownership of dynamically allocated memory
// Semantics: Must be freed when done
// Lifetime: Until explicitly freed
// Pattern: Allocate → Use → Free

// Usage:
str: Ptr<u8> = allocate_string("hello")
str ?
| Some(_) { use_string(str) }
| None { handle_allocation_failure() }
free_string(&str)
```

**Methods**:
- `ptr_new() -> Ptr<T>` - Allocate and wrap
- `ptr_is_some(p: Ptr<T>) -> bool`
- `ptr_is_none(p: Ptr<T>) -> bool`
- `ptr_addr(p: Ptr<T>) -> Option<Ptr_Inner<T>>` - Get inner pointer
- `ptr_value(p: Ptr<T>) -> Option<T>` - Dereference safely
- `ptr_offset(p: Ptr<T>, count: i64) -> Ptr<T>` - Pointer arithmetic
- `ptr_free(p: *Ptr<T>) -> void` - Deallocate

**Mental model**: Like `Box<T>` in Rust (owned heap allocation)

---

### 2. Ref<T> - Borrowed Reference (Stack Borrow)

```zen
Ref<T>: {
    addr: Ptr_Inner<T>,  // Internal pointer representation
    is_valid: bool
}

// Represents: temporary access to memory (owned by someone else)
// Semantics: Cannot be freed (owner frees it)
// Lifetime: Scope where borrowed
// Pattern: Borrow → Use → Return

// Usage:
value: i32 = 42
ref = ref_of(&value)  // Borrow the value
ref ?
| Some(r) { x = ref_value(r) }
| None { handle_invalid_ref() }
// Automatically unborrows when scope ends
```

**Methods**:
- `ref_of(value: *T) -> Ref<T>` - Borrow from address
- `ref_is_valid(r: Ref<T>) -> bool`
- `ref_value(r: Ref<T>) -> Option<T>` - Dereference safely
- `ref_set(r: *Ref<T>, value: T) -> void` - Mutate through reference

**Mental model**: Like `&T` reference in Rust (borrowed borrow)

---

### 3. Ptr_Inner<T> - Raw Internal Pointer (Unsafe)

```zen
Ptr_Inner<T>: {
    // Internal representation of actual pointer
    // Opaque to users - they use Ptr<T> or Ref<T> instead
}

// Never exposed to user code
// Only used internally in Ptr<T> and Ref<T>
// Compiler handles conversion to/from raw machine pointers
```

**Never used directly by user code**. It's an implementation detail.

---

## Type System in Action

### Scenario: String Type

```zen
// BEFORE (with * syntax):
String: {
    data: *u8,
    len: usize,
    capacity: usize,
    allocator: Allocator
}

string_new = (allocator: Allocator) String {
    addr = allocator.allocate(16) as *u8  // ❌ as *u8??
    return String { data: addr, ... }
}

string_at = (s: String, index: usize) Option<u8> {
    ptr = compiler.gep(s.data, index as i64)  // ❌ gep on *u8
    return Option.Some(*(ptr as *u8))  // ❌ double cast
}

// AFTER (all explicit types):
String: {
    data: Ptr<u8>,
    len: usize,
    capacity: usize,
    allocator: Allocator
}

string_new = (allocator: Allocator) String {
    ptr = allocator.allocate_typed<u8>(16)  // ✅ Type-safe
    return String { data: ptr, ... }
}

string_at = (s: String, index: usize) Option<u8> {
    s.data ?
    | Some(_) {
        ptr_offset(s.data, index as i64) ?  // ✅ Works on Ptr<u8>
        | Some(p) { return ptr_value(p) }
        | None { return Option.None }
    }
    | None { return Option.None }
}
```

### Scenario: Generic Vec<T>

```zen
// BEFORE:
Vec<T>: {
    data: *u8,  // ❌ What type is this really? u8? T?
    len: usize,
    capacity: usize
}

vec_push = (v: *Vec<T>, item: T) {
    size = compiler.sizeof(T)
    offset = (v.len * size) as i64
    ptr = compiler.gep(v.data, offset)  // ❌ gep on *u8
    *(ptr as *T) = item  // ❌ unsafe cast
}

// AFTER:
Vec<T>: {
    data: Ptr<T>,  // ✅ Clear: pointer to T
    len: usize,
    capacity: usize
}

vec_push = (v: *Vec<T>, item: T) {
    v.data ?
    | Some(_) {
        offset = (v.len * compiler.sizeof(T)) as i64
        target = ptr_offset(v.data, offset)  // ✅ Type-safe
        target ?
        | Some(t) { ptr_set(t, item) }  // ✅ Type-safe assignment
        | None { /* allocation failed */ }
    }
    | None { /* handle null */ }
}
```

---

## Compiler Primitives Updated

Instead of using `*T` syntax, all primitives work on `Ptr_Inner<T>` (opaque):

```rust
// In compiler.rs - only internal representation
sizeof<T>() -> usize
gep_inner<T>(ptr: Ptr_Inner<T>, offset: i64) -> Ptr_Inner<T>
ptr_to_inner<T>(ptr: Ptr<T>) -> Ptr_Inner<T>
inner_to_ptr<T>(inner: Ptr_Inner<T>) -> Ptr<T>
```

But users never see `Ptr_Inner`. They use:

```zen
// In stdlib - user-facing API
ptr_offset<T>(p: Ptr<T>, count: i64) -> Ptr<T>
ptr_addr<T>(p: Ptr<T>) -> Option<Ptr_Inner<T>>
ptr_value<T>(p: Ptr<T>) -> Option<T>
```

---

## Updated stdlib/core/ptr.zen

```zen
{ compiler } = @std

// ============================================================================
// Ptr<T> - Owned Pointer (Heap Allocation)
// ============================================================================

// Opaque internal representation (users don't touch this)
Ptr_Inner<T>: {
    // Compiler handles this - just a wrapper around machine pointer
}

// Public API: Ptr<T> enum
Ptr<T>: enum {
    Some: Ptr_Inner<T>,
    None
}

// Constructors
ptr_allocate<T>(allocator: Allocator, count: usize) Ptr<T> {
    item_size = compiler.sizeof(T)
    total_size = count * item_size
    
    addr = allocator.allocate(total_size)
    addr ?
    | Some(inner_ptr) {
        return Ptr.Some(inner_ptr)
    }
    | None { return Ptr.None }
}

ptr_none<T>() Ptr<T> {
    return Ptr.None
}

// Checks
ptr_is_some<T>(p: Ptr<T>) bool {
    compiler.discriminant(&p) == 0
}

ptr_is_none<T>(p: Ptr<T>) bool {
    compiler.discriminant(&p) == 1
}

// Safe access
ptr_value<T>(p: Ptr<T>) Option<T> {
    p ?
    | Some(inner) { return Option.Some(*inner) }
    | None { return Option.None }
}

// Pointer arithmetic (stays as Ptr<T>)
ptr_offset<T>(p: Ptr<T>, count: i64) Ptr<T> {
    p ?
    | Some(inner) {
        item_size = compiler.sizeof(T)
        offset = count * item_size
        new_inner = compiler.gep_inner(inner, offset)
        return Ptr.Some(new_inner)
    }
    | None { return Ptr.None }
}

// Get element at index
ptr_at<T>(p: Ptr<T>, index: usize) Option<T> {
    ptr_offset(p, index as i64) ?
    | Some(p_at_index) { return ptr_value(p_at_index) }
    | None { return Option.None }
}

// Deallocation
ptr_free<T>(p: *Ptr<T>, allocator: Allocator, capacity: usize) void {
    p ?
    | Some(inner) {
        item_size = compiler.sizeof(T)
        total_size = capacity * item_size
        allocator.deallocate(inner, total_size)
        p = Ptr.None
    }
    | None { /* no-op */ }
}

// ============================================================================
// Ref<T> - Borrowed Reference (Stack Borrow)
// ============================================================================

Ref<T>: {
    addr: Ptr_Inner<T>,
    is_valid: bool
}

// Create reference from address
ref_from<T>(inner: Ptr_Inner<T>) Ref<T> {
    return Ref<T> {
        addr: inner,
        is_valid: true
    }
}

// Create invalid reference
ref_invalid<T>() Ref<T> {
    return Ref<T> {
        addr: ???,  // Compiler fills with null/invalid
        is_valid: false
    }
}

// Checks
ref_is_valid<T>(r: Ref<T>) bool {
    return r.is_valid
}

// Read value
ref_value<T>(r: Ref<T>) Option<T> {
    r.is_valid ?
    | true { return Option.Some(*r.addr) }
    | false { return Option.None }
}

// Write value (mutable borrow)
ref_set<T>(r: *Ref<T>, value: T) Option<void> {
    r.is_valid ?
    | true {
        *r.addr = value
        return Option.Some(())
    }
    | false { return Option.None }
}
```

---

## User Code - Before vs After

### Before (with * syntax - confusing)

```zen
// What does * mean here?
data: *u8
ptr: *i32
ref: &String

// Is this taking an address? Dereferencing? Creating a pointer?
x = &value
y = *ptr
z = *array[0]

// This is just confusing:
allocate_string = () *u8 {
    return compiler.raw_allocate(16) as *u8
}

use_string = (s: *String) void {
    io.println(s.data)  // What is s.data? It's a *u8, so... what?
}
```

### After (explicit types - clear)

```zen
// Crystal clear what these are:
data: Ptr<u8>
ptr: Ptr<i32>
ref: Ref<String>

// No ambiguity - these are functions:
x = ref_from(addr)
y = ptr_value(ptr)
z = ptr_at(array, 0)

// Clear intent:
allocate_string = (allocator: Allocator) Ptr<u8> {
    return ptr_allocate(allocator, 16)
}

use_string = (s: Ptr<String>) void {
    s ?
    | Some(_) {
        s.data ?
        | Some(_) { io.println(ptr_value(s.data)) }
        | None { io.println("empty") }
    }
    | None { io.println("null") }
}
```

---

## Type Safety Benefits

### No More Confusion

```zen
✅ Ptr<u8> - obviously a pointer to u8
✅ Ptr<i32> - obviously a pointer to i32
✅ Ptr<String> - obviously a pointer to String
✅ Ref<String> - obviously a reference to String

❌ *u8 - pointer? Multiply? Something else?
❌ &String - reference? Address? Borrow? Weird?
```

### Pattern Matching Works

```zen
✅ ptr: Ptr<T> ?
   | Some(p) { use p }
   | None { handle_null }

❌ ptr: *T ?
   | ??? (what are the patterns for a pointer?)
   | ??? (how do we pattern match on *T?)
```

### Generics Work Perfectly

```zen
✅ vec: Vec<Ptr<T>> - Vector of pointers
✅ hash: HashMap<String, Ptr<Value>> - Map to pointers
✅ opt: Option<Ptr<T>> - Optional pointer

❌ vec: Vec<*T> - Vector of... pointer-to-T? Or pointer-to-(Vector of T)?
❌ hash: HashMap<String, *Value> - Confusing
```

### No Syntax Overloading

```zen
✅ ptr_offset, ptr_value, ptr_at - Clear function names
✅ ref_value, ref_set, ref_from - Clear function names
✅ No * used for both declaration and operation

❌ * used for: pointers, dereferencing, multiplication
❌ & used for: references, borrowing, bitwise AND?
❌ Impossible to know without context
```

---

## Refactored Code Examples

### String Type (New Design)

```zen
String: {
    data: Ptr<u8>,
    len: usize,
    capacity: usize,
    allocator: Allocator
}

string_new = (allocator: Allocator) String {
    // Allocate 16 bytes and wrap in Ptr<u8>
    data = ptr_allocate(allocator, 16)
    
    return String {
        data: data,
        len: 0,
        capacity: 16,
        allocator: allocator
    }
}

string_at = (s: String, index: usize) Option<u8> {
    if index >= s.len { return Option.None }
    
    ptr_at(s.data, index)  // Safe, type-checked, no casts
}

string_push = (s: *String, byte: u8) void {
    // Check if we need to grow
    if s.len >= s.capacity {
        new_capacity = s.capacity * 2
        new_data = ptr_allocate(s.allocator, new_capacity)
        
        // Copy old data
        copy_ptr(s.data, new_data, s.len)
        
        // Free old data
        ptr_free(&s.data, s.allocator, s.capacity)
        
        s.data = new_data
        s.capacity = new_capacity
    }
    
    // Append byte at end
    target = ptr_at(s.data, s.len)
    target ?
    | Some(t) { *t = byte }  // Type-safe assignment
    | None { /* error */ }
    
    s.len = s.len + 1
}

string_free = (s: *String) void {
    ptr_free(&s.data, s.allocator, s.capacity)
    s.len = 0
    s.capacity = 0
}
```

### Vec<T> Type (New Design)

```zen
Vec<T>: {
    data: Ptr<T>,
    len: usize,
    capacity: usize,
    allocator: Allocator
}

vec_new = (allocator: Allocator) Vec<T> {
    return Vec<T> {
        data: ptr_none(),
        len: 0,
        capacity: 0,
        allocator: allocator
    }
}

vec_push = (v: *Vec<T>, item: T) void {
    if v.capacity == 0 {
        v.data = ptr_allocate(v.allocator, 1)
        v.capacity = 1
    }
    
    if v.len >= v.capacity {
        new_capacity = v.capacity * 2
        new_data = ptr_allocate(v.allocator, new_capacity)
        
        copy_ptr(v.data, new_data, v.len)
        ptr_free(&v.data, v.allocator, v.capacity)
        
        v.data = new_data
        v.capacity = new_capacity
    }
    
    target = ptr_offset(v.data, v.len as i64)
    target ?
    | Some(t) { *t = item }
    | None { /* error */ }
    
    v.len = v.len + 1
}

vec_get = (v: Vec<T>, index: usize) Option<T> {
    if index >= v.len { return Option.None }
    ptr_at(v.data, index)
}

vec_free = (v: *Vec<T>) void {
    ptr_free(&v.data, v.allocator, v.capacity)
    v.len = 0
    v.capacity = 0
}
```

---

## Migration Path

### Phase 1: Rename Types (Current)
- Rename `*T` usage to `Ptr<T>` in documentation
- Create `Ptr<T>` enum wrapper
- Create `Ref<T>` struct wrapper

### Phase 2: Update Stdlib
- Update String to use `Ptr<u8>`
- Update Vec to use `Ptr<T>`
- Update all memory operations

### Phase 3: Remove * Syntax
- Eventually deprecate `*T` syntax
- Make `Ptr<T>` the canonical way
- Update all examples and docs

### Phase 4: Ref Support
- Implement `Ref<T>` for stack borrowing
- Add borrow checking if needed
- Complete safe reference system

---

## Design Philosophy

This aligns with Zen's core values:

✅ **Explicit**: No magic `*` operator. Everything is a type.
✅ **Safe**: Pattern matching catches null cases.
✅ **Clear**: `Ptr<T>` and `Ref<T>` are obvious.
✅ **Composable**: Works with generics, collections, options.
✅ **Learnable**: New programmers don't need to learn C syntax.

---

## Summary

| Aspect | Before | After |
|--------|--------|-------|
| Syntax | `*T`, `&T`, `as *u8` | `Ptr<T>`, `Ref<T>` |
| Clarity | Confusing (C-style) | Crystal clear |
| Safety | Error-prone casts | Type-checked |
| Pattern matching | Doesn't work | Pattern match on Ptr/Ref |
| Generics | Awkward `*u8` | Clean `Ptr<u8>` |
| Learning curve | Steep (C knowledge needed) | Gentle (explicit types) |
| Code readability | "What is *?" | "Obviously a pointer" |

---

## Conclusion

Eliminating `*` and `&` syntax and using explicit `Ptr<T>` and `Ref<T>` types:

1. **Improves clarity** - No ambiguity about what a type represents
2. **Enhances safety** - Pattern matching catches all null cases
3. **Enables better composition** - Works naturally with generics
4. **Aligns with Zen philosophy** - Explicit > implicit, safe > unsafe
5. **Better for learning** - Doesn't require C knowledge

This is a superior design. Let's implement it.
