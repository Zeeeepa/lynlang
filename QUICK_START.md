# Zen Language - Quick Start Guide

**Updated**: 2025-11-19 | **Status**: ✅ Ready to use

---

## What You Need to Know Right Now

### Zen has type-safe pointers with no null crashes

```zen
// ✅ Safe: Type system prevents crashes
ptr: Ptr<i32> = ptr.ptr_none()
ptr ?
| Some(addr) { value = *addr }
| None { io.println("No value") }

// ❌ Unsafe (not allowed): Direct null dereference impossible
value = *ptr  // Compiler won't allow - pattern match first!
```

### Strings auto-grow with type safety

```zen
{ string } = @std
{ gpa } = @std.memory

allocator = gpa.default_gpa()
s = string.string_new(allocator)

string.string_push(&s, 72)  // 'H'
string.string_push(&s, 105) // 'i'

string.string_free(&s)
```

### Vectors work with any type

```zen
{ vec } = @std
{ gpa } = @std.memory

allocator = gpa.default_gpa()
numbers = vec.vec_new(allocator)

vec.vec_push(&numbers, 42)
vec.vec_push(&numbers, 99)

vec.vec_get(numbers, 0) ?
| Some(n) { io.println("${n}") }
| None { io.println("Empty") }

vec.vec_free(&numbers)
```

---

## Files You Should Read (In Order)

1. **NEXT_STEPS.md** - What's done, what's pending
2. **DOCUMENTATION_INDEX.md** - Navigation hub
3. **design/SAFE_POINTERS_DESIGN.md** - Why Ptr<T> is better than raw pointers
4. **INTRINSICS_REFERENCE.md** - Compiler primitives you can use

---

## Build & Test

```bash
# Build project
cargo build

# Run all tests
cargo test --all

# Expected result: 87 tests passing ✅
```

---

## Key Types

### Ptr<T> - Owned pointer (must free)
```zen
p = ptr.ptr_allocate(allocator, 10)
ptr.ptr_value(p) ?                     // Safe deref
| Some(val) { io.println("${val}") }
| None { io.println("Null") }
ptr.ptr_free(&p, allocator, 10)        // Cleanup
```

### Ref<T> - Borrowed reference (stack lifetime)
```zen
r = ref.ref_from(some_address)
ref.ref_value(r) ?                     // Safe read
| Some(val) { io.println("${val}") }
| None { io.println("Invalid") }
// No cleanup needed - stack allocated
```

### String - Dynamic text
```zen
s = string.string_new(allocator)
string.string_push(&s, byte)
string.string_len(s) // Current length
string.string_free(&s)
```

### Vec<T> - Generic array
```zen
v = vec.vec_new(allocator)
vec.vec_push(&v, element)
vec.vec_get(v, 0) ?                    // Safe index
| Some(elem) { /* use */ }
| None { /* empty */ }
vec.vec_free(&v)
```

---

## Design Philosophy

### ✅ What Zen does right
- **Type-safe pointers** - Compiler prevents null crashes
- **No garbage collector** - Manual memory with allocators
- **Explicit allocations** - Know where memory comes from
- **Pattern matching** - Force null handling
- **Generic types** - Compile-time specialization

### ⚠️ What you must do
- **Free memory** - Explicit deallocation required
- **Pattern match** - Handle Some/None cases
- **Validate pointers** - Check ref.is_valid before use
- **Use allocators** - All allocations go through them

---

## Common Patterns

### Safe allocation & deallocation
```zen
allocator = gpa.default_gpa()
ptr = ptr.ptr_allocate(allocator, 100)
ptr ?
| Some(addr) {
    // Use the pointer
    ptr.ptr_free(&ptr, allocator, 100)
}
| None {
    io.println("Allocation failed")
}
```

### Building a collection
```zen
v = vec.vec_new(allocator)
vec.vec_reserve(&v, 100)  // Pre-allocate

i = 0
loop(() {
    if i >= 10 { break }
    vec.vec_push(&v, i)
    i = i + 1
})

io.println("Length: ${vec.vec_len(v)}")
vec.vec_free(&v)
```

### Safe iteration (current limitation)
```zen
// Loop through vec manually (no iterator yet)
v = vec.vec_with_capacity(allocator, 3)
vec.vec_push(&v, 10)
vec.vec_push(&v, 20)
vec.vec_push(&v, 30)

i = 0
loop(() {
    if i >= vec.vec_len(v) { break }
    
    vec.vec_get(v, i) ?
    | Some(val) { io.println("${val}") }
    | None {}
    
    i = i + 1
})

vec.vec_free(&v)
```

---

## Intrinsics You Can Use

```zen
{ compiler } = @std

// Memory
addr = compiler.raw_allocate(1024)
compiler.raw_deallocate(addr, 1024)
new_addr = compiler.raw_reallocate(addr, 1024, 2048)

// Pointers
ptr = compiler.gep(base, 8)              // +8 bytes
field = compiler.gep_struct(ptr, 2)      // field 2
casted = compiler.raw_ptr_cast(ptr)      // reinterpret

// Types
size = compiler.sizeof(i32)              // 4 bytes

// Enums
tag = compiler.discriminant(&my_enum)
compiler.set_discriminant(&my_enum, 1)
payload = compiler.get_payload(&my_enum)
compiler.set_payload(&my_enum, data)
```

---

## What's NOT Implemented Yet

- [ ] Iterators for Vec/String
- [ ] HashMap, Set, Queue, Stack
- [ ] String formatting/methods
- [ ] FFI (C interop)
- [ ] Inline assembly
- [ ] Advanced allocators

---

## Troubleshooting

### "Error: cannot dereference null pointer"
**Solution**: Pattern match first to verify it's Some
```zen
ptr ?
| Some(addr) { value = *addr }  // Now it's safe
| None { /* handle error */ }
```

### "Allocation failed"
**Solution**: Check the Option result
```zen
ptr.ptr_allocate(alloc, size) ?
| Some(addr) { /* use pointer */ }
| None { io.println("OOM") }
```

### "Expected pattern match but got..."
**Solution**: Ptr/Option require pattern matching, no escape
```zen
// ✅ Correct
option ?
| Some(x) { io.println(x) }
| None {}

// ❌ Wrong
if option == Some(x) { }  // Won't compile
```

---

## Next Reading

After this, read these in order:
1. `design/SAFE_POINTERS_DESIGN.md` - Deep dive on Ptr vs Ref
2. `INTRINSICS_REFERENCE.md` - All compiler primitives
3. `design/STDLIB_WORK_BREAKDOWN.md` - Implementation details
4. Look at `stdlib/` folder for actual code

---

## Real Example: Count Characters

```zen
{ string } = @std
{ gpa } = @std.memory

main = () i32 {
    allocator = gpa.default_gpa()
    text = string.string_new(allocator)
    
    // Add text
    string.string_push(&text, 72)  // H
    string.string_push(&text, 105) // i
    string.string_push(&text, 33)  // !
    
    // Count
    len = string.string_len(text)
    io.println("Characters: ${len}")  // 3
    
    // Cleanup
    string.string_free(&text)
    return 0
}
```

---

## Real Example: Sum a Vector

```zen
{ vec } = @std
{ gpa } = @std.memory

sum_vector = (v: vec.Vec<i32>) i32 {
    result = 0
    i = 0
    
    loop(() {
        if i >= vec.vec_len(v) { break }
        
        vec.vec_get(v, i) ?
        | Some(n) {
            result = result + n
        }
        | None {}
        
        i = i + 1
    })
    
    return result
}

main = () i32 {
    allocator = gpa.default_gpa()
    numbers = vec.vec_new(allocator)
    
    vec.vec_push(&numbers, 10)
    vec.vec_push(&numbers, 20)
    vec.vec_push(&numbers, 30)
    
    total = sum_vector(numbers)
    io.println("Sum: ${total}")  // 60
    
    vec.vec_free(&numbers)
    return 0
}
```

---

## Commands You'll Use

```bash
# Development
cargo build              # Compile
cargo test --all         # Run tests
cargo test pattern       # Run specific tests
cargo run                # Run compiled binary

# Debugging
cargo build --verbose    # See compilation details
cargo test -- --nocapture --test-threads=1  # See output

# Release
cargo build --release    # Optimized build
```

---

## Key Reminder

**Zen eliminates null pointer crashes by design.**

Every pointer is either:
- `Ptr.Some(addr)` - Valid, safe to use
- `Ptr.None` - Null, must be handled explicitly

You can't accidentally crash on null. The compiler won't allow it.

---

**Status**: 🟢 Ready  
**Tests**: 87/87 passing  
**Next**: Read NEXT_STEPS.md
