# Safe Pointers Design: Ptr<T> and Ref<T> Wrappers

## Problem with null_ptr()

Raw `null_ptr()` is unsafe:
```zen
ptr = compiler.null_ptr()  // Returns *u8
if ptr == null ? { ... }   // Silent crashes possible
```

Better: Type-safe wrappers that force explicit handling.

---

## Solution: Ptr<T> and Ref<T> Types

### Option 1: Ptr<T> as Enum (Recommended)

```zen
{ compiler } = @std

// Safe pointer wrapper
Ptr<T>: enum {
    Some: *T,
    None
}

// Methods
ptr_is_some = (p: Ptr<T>) bool {
    compiler.discriminant(&p) == 0
}

ptr_is_none = (p: Ptr<T>) bool {
    compiler.discriminant(&p) == 1
}

ptr_value = (p: Ptr<T>) *T {
    // Panic if None
    p ?
    | Some(addr) { return addr }
    | None { /* panic */ }
}

ptr_addr = (p: Ptr<T>) Option<*T> {
    // Safe version - returns Option
    p ?
    | Some(addr) { return Option.Some(addr) }
    | None { return Option.None }
}

ptr_none = () Ptr<T> {
    return Ptr.None
}

ptr_some = (addr: *T) Ptr<T> {
    return Ptr.Some(addr)
}
```

**Usage**:
```zen
ptr: Ptr<i32>

// Pattern match (forced)
ptr ?
| Some(addr) {
    value = *addr
    io.println("Pointer has value: ${value}")
}
| None {
    io.println("Pointer is null")
}

// Or safe access
maybe_addr = ptr_addr(ptr)
maybe_addr ?
| Some(addr) { /* use addr */ }
| None { /* handle null */ }

// Chaining
ptr_is_some(ptr) ?
| true {
    addr = ptr_value(ptr)
    // Use addr safely
}
| false { /* handle None */ }
```

### Option 2: Ref<T> as Struct (Alternative)

```zen
{ compiler } = @std

// Reference wrapper with validation
Ref<T>: {
    addr: *T,
    is_valid: bool
}

ref_new = (addr: *T) Ref<T> {
    // Check if addr is null
    is_null = addr == compiler.null_ptr() as *T
    return Ref<T> {
        addr: addr,
        is_valid: !is_null
    }
}

ref_none = () Ref<T> {
    return Ref<T> {
        addr: compiler.null_ptr() as *T,
        is_valid: false
    }
}

ref_is_some = (r: Ref<T>) bool {
    return r.is_valid
}

ref_value = (r: Ref<T>) Option<T> {
    r.is_valid ?
    | true { return Option.Some(*r.addr) }
    | false { return Option.None }
}

ref_addr = (r: Ref<T>) Option<*T> {
    r.is_valid ?
    | true { return Option.Some(r.addr) }
    | false { return Option.None }
}
```

**Usage**:
```zen
ref: Ref<i32>

// Safe access
ref_value(ref) ?
| Some(value) { io.println("Value: ${value}") }
| None { io.println("Reference is null") }
```

---

## Recommended: Use Ptr<T> Enum (Option 1)

**Why Ptr<T> is better**:
1. ✅ **Type-safe**: Must pattern match
2. ✅ **Consistent**: Same pattern as Option<T>
3. ✅ **Efficient**: No extra fields, just discriminant + pointer
4. ✅ **Composable**: Works with option_map, etc.
5. ✅ **Familiar**: Users know how to use Option

**Why Ref<T> is weaker**:
1. ❌ **Redundant**: is_valid field + null check
2. ❌ **Indirect**: Extra dereferencing
3. ❌ **Inconsistent**: Different pattern than Option<T>

---

## Updated Compiler Primitives

Instead of `null_ptr()`:

### ✅ KEEP: Memory Primitives
```rust
raw_allocate(size: usize) -> *u8
raw_deallocate(ptr: *u8, size: usize) -> void
raw_reallocate(ptr: *u8, old, new) -> *u8
```

### ✅ KEEP: Pointer Operations
```rust
gep(base: *u8, offset: i64) -> *u8
gep_struct(ptr: *u8, field: i32) -> *u8
raw_ptr_cast(ptr: *u8) -> *u8
raw_ptr_offset(ptr: *u8, offset: i64) -> *u8  // deprecated
```

### ✅ ADD: Type Introspection
```rust
sizeof<T>() -> usize
// null_ptr() → DELETE, use Ptr<T> instead
```

### ✅ KEEP: Enum Operations
```rust
discriminant(val: *u8) -> i32
set_discriminant(ptr: *u8, tag: i32) -> void
get_payload(val: *u8) -> *u8
set_payload(ptr: *u8, payload: *u8) -> void
```

**Total**: 12 primitives (not 15)
- Memory (3)
- Pointers (4) ← removed raw_ptr_offset (use gep), removed null_ptr (use Ptr<T>)
- Types (1) ← just sizeof
- Enums (4)
- FFI (4 placeholders) ← lower priority

---

## Ptr<T> Implementation Details

### In stdlib/core/ptr.zen

```zen
{ compiler } = @std

Ptr<T>: enum {
    Some: *T,
    None
}

// Construction
ptr_some = (addr: *T) Ptr<T> {
    return Ptr.Some(addr)
}

ptr_none = () Ptr<T> {
    return Ptr.None
}

ptr_from_alloc = (allocator: Allocator, count: usize) Ptr<T> {
    item_size = compiler.sizeof(T)
    total_size = count * item_size
    addr = allocator.gpa_allocate(allocator, total_size) as *T
    
    // Safety check - could also use Option here
    addr == (0 as *T) ?
    | true { return Ptr.None }
    | false { return Ptr.Some(addr) }
}

// Checks
ptr_is_some = (p: Ptr<T>) bool {
    compiler.discriminant(&p) == 0
}

ptr_is_none = (p: Ptr<T>) bool {
    compiler.discriminant(&p) == 1
}

// Safe access
ptr_addr = (p: Ptr<T>) Option<*T> {
    p ?
    | Some(addr) { return Option.Some(addr) }
    | None { return Option.None }
}

ptr_value = (p: Ptr<T>) Option<T> {
    p ?
    | Some(addr) { return Option.Some(*addr) }
    | None { return Option.None }
}

// Unsafe unwrap (panics on None)
ptr_unwrap = (p: Ptr<T>) *T {
    p ?
    | Some(addr) { return addr }
    | None { 
        // Panic
        loop(() { true ? { break } | false {} })
        return 0 as *T  // unreachable
    }
}

// Transformations
ptr_map = (p: Ptr<T>, f: (*T) -> U) Ptr<U> {
    p ?
    | Some(addr) {
        result = f(addr)
        return Ptr.Some(result as *U)
    }
    | None { return Ptr.None }
}

// Offset
ptr_offset = (p: Ptr<T>, count: i64) Ptr<T> {
    p ?
    | Some(addr) {
        item_size = compiler.sizeof(T)
        offset = count * item_size
        new_addr = compiler.gep(addr as *u8, offset) as *T
        return Ptr.Some(new_addr)
    }
    | None { return Ptr.None }
}

// Deallocation
ptr_free = (p: *Ptr<T>, allocator: Allocator, count: usize) void {
    p ?
    | Some(addr) {
        item_size = compiler.sizeof(T)
        total_size = count * item_size
        allocator.gpa_deallocate(allocator, addr as *u8, total_size)
        p = Ptr.None
    }
    | None { /* no-op */ }
}
```

### Usage Examples

**Before** (with null_ptr):
```zen
ptr = compiler.null_ptr()
if ptr == null ? { ... }  // Unsafe, easy to miss
```

**After** (with Ptr<T>):
```zen
ptr: Ptr<i32> = ptr_none()

// Must handle both cases
ptr ?
| Some(addr) {
    value = *addr
    io.println("Value: ${value}")
}
| None {
    io.println("Pointer is null, must handle this")
}

// Or use helpers
ptr_value(ptr) ?
| Some(val) { io.println("Value: ${val}") }
| None { io.println("No value") }

// Allocate safely
allocator = gpa.default_gpa()
ptr = ptr_from_alloc(allocator, 10)  // Returns Ptr<i32>
ptr ?
| Some(addr) { /* use the allocated block */ }
| None { io.println("Allocation failed") }
```

---

## String Type Using Ptr<T>

### Better String Definition

```zen
{ compiler } = @std
{ gpa } = @std.memory
{ ptr } = @std.core

String: {
    data: Ptr<u8>,         // ← Type-safe pointer!
    len: usize,
    capacity: usize,
    allocator: gpa.Allocator
}

string_new = (allocator: gpa.Allocator) String {
    initial_capacity = 16
    addr = allocator.gpa_allocate(allocator, initial_capacity) as *u8
    
    data = addr == (0 as *u8) ?
    | true { ptr.ptr_none() }
    | false { ptr.ptr_some(addr) }
    
    return String {
        data: data,
        len: 0,
        capacity: initial_capacity,
        allocator: allocator
    }
}

string_push = (s: *String, char: u8) Option<void> {
    // Handle case where data is None
    s.data ?
    | None {
        // Try to allocate
        addr = s.allocator.gpa_allocate(s.allocator, 16) as *u8
        s.data = addr == (0 as *u8) ?
        | true { return Option.None }  // Allocation failed
        | false { ptr.ptr_some(addr) }
        s.capacity = 16
    }
    | Some(_) { /* continue */ }
    
    // Now we know data is Some
    if s.len >= s.capacity {
        new_capacity = s.capacity * 2
        new_addr = s.allocator.gpa_reallocate(
            s.allocator,
            ptr.ptr_unwrap(s.data) as *u8,
            s.capacity,
            new_capacity
        ) as *u8
        
        s.data = new_addr == (0 as *u8) ?
        | true { return Option.None }
        | false { ptr.ptr_some(new_addr) }
        
        s.capacity = new_capacity
    }
    
    // Safely write character
    base_addr = ptr.ptr_unwrap(s.data)
    char_ptr = compiler.gep(base_addr as *u8, s.len as i64)
    *(char_ptr as *u8) = char
    s.len = s.len + 1
    
    return Option.Some(())
}

string_at = (s: String, index: usize) Option<u8> {
    if index >= s.len {
        return Option.None
    }
    
    s.data ?
    | Some(base_addr) {
        char_ptr = compiler.gep(base_addr as *u8, index as i64)
        return Option.Some(*(char_ptr as *u8))
    }
    | None { return Option.None }
}

string_free = (s: *String) void {
    s.data ?
    | Some(addr) {
        s.allocator.gpa_deallocate(s.allocator, addr as *u8, s.capacity)
    }
    | None { /* no-op */ }
    
    s.data = ptr.ptr_none()
    s.len = 0
    s.capacity = 0
}
```

**Benefits**:
- ✅ No silent null pointer bugs
- ✅ Type-safe: compiler forces you to handle None case
- ✅ Explicit: allocation failures visible in return type
- ✅ Composable: works with Option<T>, pattern matching

---

## Vec<T> Using Ptr<T>

```zen
{ compiler } = @std
{ gpa } = @std.memory
{ ptr } = @std.core
{ option } = @std.core

Vec<T>: {
    data: Ptr<T>,
    len: usize,
    capacity: usize,
    allocator: gpa.Allocator
}

vec_new = (allocator: gpa.Allocator) Vec<T> {
    return Vec<T> {
        data: ptr.ptr_none(),
        len: 0,
        capacity: 0,
        allocator: allocator
    }
}

vec_push = (v: *Vec<T>, item: T) Option<void> {
    item_size = compiler.sizeof(T)
    
    // Allocate if needed
    if v.len == 0 && v.capacity == 0 {
        addr = v.allocator.gpa_allocate(v.allocator, item_size) as *T
        v.data = addr == (0 as *T) ?
        | true { return Option.None }
        | false { ptr.ptr_some(addr) }
        v.capacity = 1
    }
    
    // Reallocate if full
    if v.len >= v.capacity {
        new_capacity = v.capacity * 2
        
        old_addr = ptr.ptr_unwrap(v.data) as *u8
        old_size = v.capacity * item_size
        new_size = new_capacity * item_size
        
        new_addr = v.allocator.gpa_reallocate(
            v.allocator,
            old_addr,
            old_size,
            new_size
        ) as *T
        
        v.data = new_addr == (0 as *T) ?
        | true { return Option.None }
        | false { ptr.ptr_some(new_addr) }
        
        v.capacity = new_capacity
    }
    
    // Write item
    base = ptr.ptr_unwrap(v.data)
    offset = (v.len * item_size) as i64
    item_ptr = compiler.gep(base as *u8, offset) as *T
    *item_ptr = item
    v.len = v.len + 1
    
    return Option.Some(())
}

vec_get = (v: Vec<T>, index: usize) Option<T> {
    if index >= v.len {
        return Option.None
    }
    
    v.data ?
    | Some(base) {
        item_size = compiler.sizeof(T)
        offset = (index * item_size) as i64
        item_ptr = compiler.gep(base as *u8, offset) as *T
        return Option.Some(*item_ptr)
    }
    | None { return Option.None }
}

vec_free = (v: *Vec<T>) void {
    v.data ?
    | Some(addr) {
        old_size = v.capacity * compiler.sizeof(T)
        v.allocator.gpa_deallocate(v.allocator, addr as *u8, old_size)
    }
    | None { /* no-op */ }
    
    v.data = ptr.ptr_none()
    v.len = 0
    v.capacity = 0
}
```

---

## Comparison: Old vs New

### Old Approach (Unsafe)
```zen
ptr = compiler.null_ptr()
if ptr == null ? { ... }
value = *ptr  // Silent crash if null
```

### New Approach (Safe)
```zen
ptr: Ptr<T> = ptr_none()

ptr ?
| Some(addr) {
    value = *addr  // Safe - ptr is valid
}
| None {
    // Must handle null case explicitly
}

// Or with helpers
ptr_value(ptr) ?
| Some(value) { /* use value safely */ }
| None { /* handle null */ }
```

---

## Updated Compiler Primitives (Final)

```
MEMORY (3):
  ✅ raw_allocate(size: usize) -> *u8
  ✅ raw_deallocate(ptr: *u8, size) -> void
  ✅ raw_reallocate(ptr, old, new) -> *u8

POINTERS (4):
  ✅ gep(base: *u8, offset: i64) -> *u8
  ✅ gep_struct(ptr: *u8, field: i32) -> *u8
  ✅ raw_ptr_cast(ptr: *u8) -> *u8
  ⏳ raw_ptr_offset (deprecated, use gep)

TYPES (1):
  ✅ sizeof<T>() -> usize

ENUMS (4):
  ✅ discriminant(val: *u8) -> i32
  ✅ set_discriminant(ptr, tag: i32) -> void
  ✅ get_payload(val: *u8) -> *u8
  ✅ set_payload(ptr, payload: *u8) -> void

FFI (4 placeholders):
  ✅ inline_c(code: StaticString) -> void
  ✅ load_library(path) -> *u8
  ✅ get_symbol(lib, name) -> *u8
  ✅ unload_library(lib) -> void

TOTAL: 12 core primitives + 4 FFI placeholders
```

---

## Migration Path (Revised)

### Step 1: Add sizeof() Intrinsic
```rust
// src/stdlib/compiler.rs
functions.insert("sizeof".to_string(), ...)
```

### Step 2: Define Ptr<T> in Stdlib
```zen
// stdlib/core/ptr.zen
Ptr<T>: enum { Some: *T, None }
ptr_some, ptr_none, ptr_is_some, ptr_is_none, ptr_value, ptr_addr
```

### Step 3: Redefine String Using Ptr<T>
```zen
// stdlib/string.zen
String: {
    data: Ptr<u8>,  // Type-safe!
    len: usize,
    capacity: usize,
    allocator: Allocator
}
```

### Step 4: Redefine Vec<T> Using Ptr<T>
```zen
// stdlib/vec.zen
Vec<T>: {
    data: Ptr<T>,  // Type-safe!
    len: usize,
    capacity: usize,
    allocator: Allocator
}
```

### Step 5: Everything Else
- Option<T>, Result<T,E>
- HashMap, Set, Queue
- etc.

---

## Summary

**Don't add `null_ptr()` intrinsic.**

Instead:
1. **Add only `sizeof<T>()` intrinsic** (needed for generics)
2. **Define `Ptr<T>` as enum in stdlib/core/ptr.zen** (type-safe wrapper)
3. **Use `Ptr<T>` everywhere instead of raw nulls**

This way:
- ✅ No silent null pointer bugs
- ✅ Compiler forces explicit null handling
- ✅ Type-safe, consistent with Zen philosophy
- ✅ Works naturally with pattern matching
- ✅ Better error messages (None vs Some(addr))

Perfect alignment with eliminating the "billion dollar mistake".
