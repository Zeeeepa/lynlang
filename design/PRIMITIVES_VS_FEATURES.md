# LLVM Primitives vs Zen Features - Quick Reference

**Use this to know where to implement something.**

---

## Quick Decision: Primitive or Feature?

| Question | Answer | → |
|----------|--------|---|
| Does it call malloc/free/libc? | YES | LLVM Primitive |
| Does it need LLVM GEP/sizeof? | YES | LLVM Primitive |
| Can it be implemented with existing primitives/types? | YES | Zen Feature |
| Is it a data structure (Vec, String, Ptr)? | YES | Zen Feature |
| Is it a wrapper around a primitive? | YES | Zen Feature |
| Is it purely a type definition (enum, struct)? | YES | Zen Feature |

---

## The 13 LLVM Primitives (Can't Be Moved to Zen)

Located: `src/stdlib_metadata/compiler.rs` (declarations) + `src/codegen/llvm/stdlib_codegen/compiler.rs` (implementation)

### Memory (3)
```zen
raw_allocate(size: usize) -> *u8
raw_deallocate(ptr: *u8, size: usize) -> void
raw_reallocate(ptr: *u8, old_size, new_size) -> *u8
```

### Pointers (3)
```zen
gep(ptr: *u8, offset: i64) -> *u8
gep_struct(ptr: *u8, field_index: i32) -> *u8
raw_ptr_cast(ptr: *u8) -> *u8
```

### Types (1)
```zen
sizeof<T>() -> usize
```

### Enums (4)
```zen
discriminant(enum_ptr: *u8) -> i32
set_discriminant(enum_ptr: *u8, value: i32) -> void
get_payload(enum_ptr: *u8) -> *u8
set_payload(enum_ptr: *u8, data: *u8) -> void
```

### External Functions (2, location varies)
```zen
io.println(msg: string) -> void  // src/stdlib_metadata/io.rs
inline_c(code: string) -> void   // src/stdlib_metadata/compiler.rs
```

---

## Common Zen Features (Can Be/Are in Zen)

Located: `stdlib/` directory

| Feature | File | Uses Primitives | Status |
|---------|------|-----------------|--------|
| Option<T> | stdlib/core/option.zen | None (pure type) | ✅ Done |
| Result<T,E> | stdlib/core/result.zen | None (pure type) | ✅ Done |
| Ptr<T> | stdlib/core/ptr.zen | gep, sizeof | ✅ Done |
| Allocator | stdlib/memory/allocator.zen | None (trait) | ✅ Done |
| GPA | stdlib/memory/gpa.zen | raw_allocate, raw_deallocate | ✅ Done |
| String | stdlib/string.zen | gep, sizeof, GPA | ✅ Done |
| Vec<T> | stdlib/vec.zen | gep, sizeof, GPA | ✅ Done |
| HashMap | stdlib/collections/hashmap.zen | gep, sizeof, GPA | ⏳ WIP |
| String.concat | stdlib/string.zen | existing string ops | ✅ Done |
| Vec.iter | stdlib/vec.zen | Range + loop | ✅ Done |

---

## Where to Add a New Feature

### Scenario 1: Adding `String.split(separator: string)`

**Question**: Can I implement this using existing String operations?  
**Answer**: YES

**Implementation**:
1. Add function to `stdlib/string.zen`
2. Uses `string_at()`, `string_push()`, etc.
3. No LLVM primitive needed

**Files to change**:
- `stdlib/string.zen` - Add `string_split` function

**Example**:
```zen
string_split = (s: String, sep: String, alloc: Allocator) Vec<String> {
    result = vec.vec_new(alloc)
    current = string_new(alloc)
    // Loop through bytes, split when separator found
    return result
}
```

---

### Scenario 2: Adding `Vector.capacity_grow_to(new_capacity)`

**Question**: Can I implement this using existing Vec operations?  
**Answer**: YES

**Implementation**:
1. Add function to `stdlib/vec.zen`
2. Uses `raw_allocate`, `gep` (already wrapped)
3. No new LLVM primitive needed

**Files to change**:
- `stdlib/vec.zen` - Add `vec_capacity_grow_to` function

**Example**:
```zen
vec_capacity_grow_to = (vec: *Vec<T>, new_capacity: usize, alloc: Allocator) {
    if new_capacity <= vec.capacity { return }
    
    // Allocate new buffer
    new_size = new_capacity * compiler.sizeof(T)
    new_ptr = alloc.allocate(new_size)
    
    // Copy old data (using gep)
    (0..vec.len).loop((i) {
        old_addr = compiler.gep(vec.data, i * compiler.sizeof(T))
        new_addr = compiler.gep(new_ptr, i * compiler.sizeof(T))
        // Copy byte-by-byte (or bulk copy)
    })
    
    // Free old and update
    alloc.deallocate(vec.data, vec.capacity * compiler.sizeof(T))
    vec.data = new_ptr
    vec.capacity = new_capacity
}
```

---

### Scenario 3: Adding `Random.next() -> i32`

**Question**: Does this need to call C?  
**Answer**: YES (random number generation needs OS entropy)

**Implementation**:
1. Declare in `src/stdlib_metadata/math.rs`
2. Implement in `src/codegen/llvm/stdlib_codegen/math.rs`
3. Optionally wrap in `stdlib/math/random.zen`

**Files to change**:
- `src/stdlib_metadata/math.rs` - Add function declaration
- `src/codegen/llvm/stdlib_codegen/math.rs` - Add code generation
- `stdlib/math/random.zen` - Optional Zen wrapper

**Example** (Rust):
```rust
// src/stdlib_metadata/math.rs
functions.insert("next".to_string(), StdFunction {
    name: "next".to_string(),
    params: vec![],
    return_type: AstType::I32,
    is_builtin: true,
});

// src/codegen/llvm/stdlib_codegen/math.rs
pub fn compile_random_next<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    _args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // Call getrand() or similar
    let rand_fn = compiler.module.get_function("rand")
        .unwrap_or_else(|| {
            let fn_type = compiler.context.i32_type().fn_type(&[], false);
            compiler.module.add_function("rand", fn_type, Some(Linkage::External))
        });
    
    let result = compiler.builder.build_call(rand_fn, &[], "random")?
        .try_as_basic_value().left().unwrap();
    
    Ok(result)
}
```

---

### Scenario 4: Adding `File.exists(path: string) -> bool`

**Question**: Does this need system calls?  
**Answer**: YES (stat/access system call)

**Implementation**:
1. Declare in `src/stdlib_metadata/fs.rs`
2. Implement in `src/codegen/llvm/stdlib_codegen/fs.rs`
3. Optionally wrap in `stdlib/fs.zen`

**Files to change**:
- `src/stdlib_metadata/fs.rs` - Add function declaration
- `src/codegen/llvm/stdlib_codegen/fs.rs` - Add code generation  
- `stdlib/fs.zen` - Optional Zen wrapper

---

### Scenario 5: Adding `Option.map<U>(f: (T) -> U) -> Option<U>`

**Question**: Can I implement this in Zen using pattern matching?  
**Answer**: YES

**Implementation**:
1. Add function to `stdlib/core/option.zen`
2. Uses pattern matching on Option
3. No primitives needed

**Files to change**:
- `stdlib/core/option.zen` - Add `option_map` function

**Example**:
```zen
option_map = (opt: Option<T>, f: (T) -> U) Option<U> {
    opt ?
        | Some(val) { Some(f(val)) }
        | None { None }
}
```

---

## Checklist for Adding a New Primitive

If you determine something MUST be an LLVM primitive:

- [ ] Add declaration in `src/stdlib_metadata/my_module.rs` with `is_builtin: true`
- [ ] Add code generation in `src/codegen/llvm/stdlib_codegen/my_module.rs`
- [ ] Register function call routing in `src/codegen/llvm/stdlib_codegen/mod.rs`
- [ ] Add tests in `tests/`
- [ ] Document in `INTRINSICS_REFERENCE.md`
- [ ] Add example usage in `examples/`
- [ ] Consider: Can this be wrapped in Zen for safety?

---

## Common Mistakes

❌ **Mistake 1**: Implementing in Zen when it needs libc
```zen
file_read = (path: string) string {
    // ERROR: Can't call C functions from Zen
    fopen(path)  // Won't work
}
```

✅ **Fix**: Make it an LLVM primitive, optionally wrap in Zen

---

❌ **Mistake 2**: Making an LLVM primitive when Zen would work
```rust
// src/stdlib_metadata/string.rs - WRONG
pub fn compile_string_reverse(...) {
    // Complex LLVM IR generation
    // When it could be in Zen!
}
```

✅ **Fix**: Implement in `stdlib/string.zen`
```zen
string_reverse = (s: String, alloc: Allocator) String {
    // Zen loop and operations
}
```

---

❌ **Mistake 3**: Mixing concerns (primitives exposed in Zen)
```zen
main = () {
    // Accessing raw primitives directly
    alloc = compiler.raw_allocate(100)  // Should use GPA instead
    compiler.raw_deallocate(alloc, 100)
}
```

✅ **Fix**: Use safe wrappers
```zen
main = () {
    alloc = gpa.default_gpa()
    ptr = alloc.allocate(100)  // Safe wrapper
}
```

---

## Quick Reference Table

| What | Where | Language | Pub? |
|------|-------|----------|------|
| Type definitions | `stdlib/*/type.zen` | Zen | Yes |
| Safe wrappers | `stdlib/*/safe.zen` | Zen | Yes |
| LLVM primitives | `src/stdlib_metadata/*.rs` | Rust | No (internal) |
| Codegen impl | `src/codegen/llvm/stdlib_codegen/*.rs` | Rust | No (internal) |
| Tests | `tests/` | Rust or Zen | N/A |
| Examples | `examples/` | Zen | Yes |

---

## When in Doubt

1. **Can it be 100% implemented in existing Zen code?** → Put it in `stdlib/`
2. **Does it need `malloc`, `free`, or C functions?** → LLVM primitive
3. **Is it a data structure?** → Zen (using primitives)
4. **Is it a system call (file, network, random)?** → LLVM primitive

---

**Remember**: The fewer LLVM primitives, the closer to self-hosting.
