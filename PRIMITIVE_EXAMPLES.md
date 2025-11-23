# LLVM Primitives vs Features: Concrete Examples from Codebase

**This document shows real code examples from the Zen compiler and stdlib.**

---

## Example 1: raw_allocate (LLVM Primitive)

### Declaration: `src/stdlib_metadata/compiler.rs`
```rust
functions.insert(
    "raw_allocate".to_string(),
    StdFunction {
        name: "raw_allocate".to_string(),
        params: vec![("size".to_string(), AstType::Usize)],
        return_type: AstType::Ptr(Box::new(AstType::U8)),
        is_builtin: true,  // ← Marks as LLVM primitive
    },
);
```

### Implementation: `src/codegen/llvm/stdlib_codegen/compiler.rs`
```rust
pub fn compile_raw_allocate<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    let size = compiler.compile_expression(&args[0])?;
    let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
    
    // Get or declare malloc from libc
    let malloc_fn = compiler.module.get_function("malloc").unwrap_or_else(|| {
        let i64_type = compiler.context.i64_type();
        let fn_type = ptr_type.fn_type(&[i64_type.into()], false);
        compiler.module.add_function("malloc", fn_type, Some(Linkage::External))
    });
    
    // Generate LLVM IR to call malloc
    let ptr = compiler.builder.build_call(malloc_fn, &[size_i64.into()], "allocated_ptr")?
        .try_as_basic_value()
        .left()
        .ok_or_else(|| CompileError::InternalError(...))?;
    
    Ok(ptr)
}
```

**Why LLVM Primitive?**
- Must call C `malloc()` function
- Requires external libc linkage
- Direct memory allocation cannot be implemented in Zen

**Usage in Zen** (from stdlib):
```zen
// stdlib/memory/gpa.zen - GPA wraps raw_allocate for safety
allocate = (alloc: Allocator, size: usize) Option<*u8> {
    compiler.raw_allocate(size)
}
```

---

## Example 2: GPA.allocate (Zen Feature wrapping Primitive)

### Implementation: `stdlib/memory/gpa.zen`
```zen
GPA: {
    allocate: (self, size: usize) Option<*u8> {
        compiler.raw_allocate(size)  // Calls LLVM primitive
    },
    deallocate: (self, ptr: *u8, size: usize) void {
        compiler.raw_deallocate(ptr, size)  // Calls LLVM primitive
    },
}
```

**Why Zen Feature?**
- No special code generation needed
- Simple wrapper around primitives
- Can be 100% implemented in Zen
- Provides type-safe interface

**Usage**:
```zen
allocator = gpa.default_gpa()
ptr = allocator.allocate(100)  // Safe, returns Option<*u8>
```

---

## Example 3: Ptr<T> (Zen Feature using Primitives)

### Declaration: `stdlib/core/ptr.zen`
```zen
Ptr<T>: Some: i64, None

// Core operations:
ptr_allocate = (alloc: Allocator, count: usize) Ptr<T> {
    size = compiler.sizeof(T) * count  // Uses LLVM primitive: sizeof
    alloc.allocate(size) ?
        | Some(raw_ptr) { Ptr.Some(raw_ptr as i64) }
        | None { Ptr.None }
}

ptr_value = (p: Ptr<T>) Option<T> {
    p ?
        | Some(addr) { 
            ptr_addr = compiler.gep(addr as *u8, 0)  // Uses LLVM primitive: gep
            // Load value (pseudo-code)
            Some(load ptr_addr)
        }
        | None { None }
}
```

**Why Zen Feature?**
- Built on top of primitives (sizeof, gep)
- Type-safe pointer wrapper
- All logic can be in Zen
- No special code generation needed

**Usage**:
```zen
// Safe: compiler enforces pattern matching
ptr_val = ptr.ptr_value(my_ptr)
ptr_val ?
    | Some(val) { io.println("${val}") }
    | None { io.println("Null pointer") }
```

---

## Example 4: String (Zen Feature using Primitives)

### Declaration: `stdlib/string.zen`
```zen
String: {
    data: Ptr<u8>,        // Pointer to bytes
    len: usize,           // Current length
    capacity: usize,      // Allocated capacity
    allocator: Allocator,
}

string_new = (alloc: Allocator) String {
    capacity = 16
    data = ptr.ptr_allocate(alloc, capacity)  // Uses Ptr<T> feature
    return String { data: data, len: 0, capacity: capacity, allocator: alloc }
}

string_push = (str: *String, byte: u8) {
    // Grow if needed
    if str.len >= str.capacity {
        string_grow(str)
    }
    
    // Write byte using gep (LLVM primitive)
    offset = compiler.gep(str.data.addr as *u8, str.len)
    store_byte(offset, byte)
    str.len = str.len + 1
}

string_grow = (str: *String) {
    new_capacity = str.capacity * 2
    new_data = ptr.ptr_allocate(str.allocator, new_capacity)
    
    // Copy old data
    (0..str.len).loop((i) {
        byte = string_at(str, i)
        new_ptr = compiler.gep(new_data.addr as *u8, i)
        store_byte(new_ptr, byte)
    })
    
    // Free old data
    ptr.ptr_free(&str.data, str.allocator, str.capacity)
    str.data = new_data
    str.capacity = new_capacity
}
```

**Why Zen Feature?**
- Buffer management is pure algorithm
- Uses primitives (gep, sizeof) for mechanics
- Can be fully implemented in Zen
- No special code generation

---

## Example 5: Vec<T> (Zen Feature with Generics)

### Declaration: `stdlib/vec.zen`
```zen
Vec<T>: {
    data: Ptr<T>,
    len: usize,
    capacity: usize,
    allocator: Allocator,
}

vec_new = (alloc: Allocator) Vec<T> {
    Vec { 
        data: ptr.ptr_none(), 
        len: 0, 
        capacity: 0, 
        allocator: alloc 
    }
}

vec_push = (vec: *Vec<T>, elem: T) {
    if vec.len >= vec.capacity {
        vec_grow(vec)
    }
    
    // Calculate offset using sizeof (LLVM primitive)
    offset = vec.len * compiler.sizeof(T)
    addr = compiler.gep(vec.data.addr as *u8, offset)
    store_value(addr, elem)
    vec.len = vec.len + 1
}

vec_get = (vec: Vec<T>, index: usize) Option<T> {
    if index >= vec.len { return None }
    
    // Calculate offset
    offset = index * compiler.sizeof(T)
    addr = compiler.gep(vec.data.addr as *u8, offset)
    load_value(addr)
}
```

**Why Zen Feature?**
- Generic container logic
- Uses sizeof for type-aware offsets
- All implemented in Zen
- Same pattern as String, just generic

---

## Example 6: io.println (LLVM Primitive)

### Declaration: `src/stdlib_metadata/io.rs`
```rust
functions.insert(
    "println".to_string(),
    StdFunction {
        name: "println".to_string(),
        params: vec![("message".to_string(), string_type())],
        return_type: AstType::Void,
        is_builtin: true,
    },
);
```

### Implementation: `src/codegen/llvm/stdlib_codegen/io.rs`
```rust
pub fn compile_io_println<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // Compile the string argument
    let arg_value = compiler.compile_expression(&args[0])?;
    
    // Generate LLVM IR to call printf with newline
    let printf_fn = compiler.module.get_function("printf").unwrap_or_else(|| {
        let i32_type = compiler.context.i32_type();
        let i8_ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
        let fn_type = i32_type.fn_type(&[i8_ptr_type.into()], true);
        compiler.module.add_function("printf", fn_type, None)
    });
    
    let format_str = compiler.builder.build_global_string_ptr("%s\n", "fmt")?;
    
    let _call = compiler.builder.build_call(
        printf_fn,
        &[format_str.into(), arg_value.into()],
        "printf_call"
    )?;
    
    Ok(compiler.context.i32_type().const_zero().into())
}
```

**Why LLVM Primitive?**
- Must call C `printf()` function
- System call for output
- Cannot be implemented in Zen
- Requires libc linkage

**Could We Wrap It in Zen?** 
```zen
// stdlib/io.zen - optional wrapper for consistency
io_println = (msg: string) {
    compiler.builtin_println(msg)  // No, this IS the primitive
}
```

Not practical - it's already the low-level operation.

---

## Example 7: sizeof (LLVM Primitive)

### Declaration: `src/stdlib_metadata/compiler.rs`
```rust
// sizeof is special - it's a generic function
// Handled specially in type checking and codegen
```

### Implementation: `src/codegen/llvm/stdlib_codegen/compiler.rs`
```rust
pub fn compile_sizeof<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    type_arg: &AstType,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // Get the LLVM type
    let llvm_type = compiler.get_llvm_type(type_arg)?;
    
    // Get size in bytes from target data layout
    let size = compiler.module.get_data_layout().get_type_alloc_size(&llvm_type);
    
    // Return as usize constant
    Ok(compiler.context.i64_type().const_int(size, false).into())
}
```

**Why LLVM Primitive?**
- Must read LLVM type layout information
- Compile-time type size calculation
- Cannot be done in Zen
- Used by generics (Vec, String, Ptr)

**Usage in Zen**:
```zen
size = compiler.sizeof(i32)     // Returns 4
size = compiler.sizeof(String)  // Returns layout size of String struct
```

---

## Example 8: gep (LLVM Primitive)

### Declaration: `src/stdlib_metadata/compiler.rs`
```rust
functions.insert(
    "gep".to_string(),
    StdFunction {
        name: "gep".to_string(),
        params: vec![
            ("base".to_string(), AstType::Ptr(Box::new(AstType::U8))),
            ("offset".to_string(), AstType::I64),
        ],
        return_type: AstType::Ptr(Box::new(AstType::U8)),
        is_builtin: true,
    },
);
```

### Implementation: `src/codegen/llvm/stdlib_codegen/compiler.rs`
```rust
pub fn compile_gep<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    let base_ptr = compiler.compile_expression(&args[0])?.into_pointer_value();
    let offset = compiler.compile_expression(&args[1])?.into_int_value();
    
    // Generate LLVM GEP instruction
    let result = compiler.builder.build_gep(
        compiler.context.i8_type(),  // GEP on i8 for byte offsets
        base_ptr,
        &[offset],
        "gep_result"
    )?;
    
    Ok(result.into())
}
```

**Why LLVM Primitive?**
- Direct LLVM GEP instruction
- Pointer arithmetic at LLVM level
- Cannot be expressed in Zen
- Used by all pointer-based data structures

**Usage in Zen**:
```zen
// Used internally by String, Vec, Ptr
offset_ptr = compiler.gep(base_ptr, byte_offset)
```

---

## Comparison Table

| Feature | Location | Language | LLVM Direct? | Can Be Zen? | Notes |
|---------|----------|----------|--------------|------------|-------|
| raw_allocate | src/stdlib_metadata/compiler.rs | Rust | Yes (malloc) | No | Needs libc |
| gep | src/stdlib_metadata/compiler.rs | Rust | Yes (GEP inst) | No | Direct LLVM |
| sizeof | src/stdlib_metadata/compiler.rs | Rust | Yes (layout) | No | Type info |
| io.println | src/stdlib_metadata/io.rs | Rust | Yes (printf) | No | Needs libc |
| GPA | stdlib/memory/gpa.zen | Zen | No | Yes | Wraps primitives |
| Ptr<T> | stdlib/core/ptr.zen | Zen | No | Yes | Wraps gep, sizeof |
| String | stdlib/string.zen | Zen | No | Yes | Uses gep, sizeof |
| Vec<T> | stdlib/vec.zen | Zen | No | Yes | Uses gep, sizeof |
| Option<T> | stdlib/core/option.zen | Zen | No | Yes | Pure enum |

---

## Key Takeaways

1. **LLVM Primitives** (in Rust):
   - Call C functions (malloc, printf, sin, etc.)
   - Use LLVM instructions (gep, sizeof, etc.)
   - Cannot be implemented any other way
   - Minimized for self-hosting

2. **Zen Features** (in stdlib/*.zen):
   - Built on primitives + existing types
   - Type-safe wrappers
   - Algorithm implementations
   - Fully portable Zen code

3. **The Separation**:
   - Rust codegen: Only what MUST be in Rust
   - Zen stdlib: Everything else
   - Each layer tested independently

---

**Want to add a feature?** Check if it needs an LLVM primitive using the decision tree in `PRIMITIVES_VS_FEATURES.md`.
