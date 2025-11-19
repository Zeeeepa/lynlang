# Quick Start: Using Allocators in Zen

**Last Updated**: 2025-01-27  
**Status**: Ready to use  

## Overview

The Zen allocator interface provides flexible memory management through the `@std.memory` module.

## Basic Usage

### Get the Default Allocator

```zen
{ memory } = @std

fn main() {
    let alloc = memory.default_gpa()
    // Use alloc for all memory operations
}
```

### Allocate Memory

```zen
// Raw allocation (returns *u8)
let ptr = alloc.allocate(256)

// Type-safe single value allocation
let int_ptr = memory.gpa.allocate_one<i32>(alloc)

// Type-safe array allocation
let array = memory.gpa.allocate_array<i32>(alloc, 100)  // 100 i32 values
```

### Deallocate Memory

```zen
// Raw deallocation
alloc.deallocate(ptr, 256)

// Type-safe single value
memory.gpa.deallocate_one<i32>(alloc, int_ptr)

// Type-safe array
memory.gpa.deallocate_array<i32>(alloc, array, 100)
```

### Reallocate Memory

```zen
// Raw reallocation
let new_ptr = alloc.reallocate(ptr, old_size, new_size)

// Type-safe array reallocation
let new_array = memory.gpa.reallocate_array<i32>(alloc, array, old_count, new_count)
```

## Common Patterns

### Manual Memory Management

```zen
fn allocate_buffer(alloc: memory.GPA, size: usize) -> *u8 {
    if (size == 0) {
        return null // Or use error handling
    }
    return alloc.allocate(size)
}

fn cleanup(alloc: memory.GPA, ptr: *u8, size: usize) -> void {
    if (ptr != null) {
        alloc.deallocate(ptr, size)
    }
}
```

### Typed Array Handling

```zen
fn process_integers(alloc: memory.GPA, count: usize) -> void {
    let arr = memory.gpa.allocate_array<i32>(alloc, count)
    if (arr == null) {
        // Handle allocation failure
        return
    }
    
    // Use arr...
    
    memory.gpa.deallocate_array<i32>(alloc, arr, count)
}
```

### Zero-Initialized Memory

```zen
fn allocate_zeroed(alloc: memory.GPA, size: usize) -> *u8 {
    let ptr = alloc.allocate(size)
    if (ptr != null) {
        memory.gpa.memzero(ptr, size)
    }
    return ptr
}
```

### Memory Copying

```zen
fn copy_data(alloc: memory.GPA, src: *u8, size: usize) -> *u8 {
    let dst = alloc.allocate(size)
    if (dst != null) {
        memory.gpa.memcpy(dst, src, size)
    }
    return dst
}
```

## Module Structure

```
@std.memory
├── allocator.zen
│   ├── Allocator trait
│   ├── AllocatorConstraints
│   ├── AllocError enum
│   ├── AllocResult<T> type
│   └── Specialized allocators
│       ├── ArenaAllocator
│       ├── PoolAllocator
│       ├── ThreadsafeAllocator
│       └── StatsAllocator
│
└── gpa.zen
    ├── GPA allocator struct
    ├── GPA.new()
    ├── GPA.allocate()
    ├── GPA.deallocate()
    ├── GPA.reallocate()
    ├── default_gpa()
    ├── Typed helpers
    │   ├── allocate_one<T>()
    │   ├── deallocate_one<T>()
    │   ├── allocate_array<T>()
    │   ├── deallocate_array<T>()
    │   └── reallocate_array<T>()
    └── Memory utilities
        ├── memzero()
        └── memcpy()
```

## Allocator Methods

### Allocator Trait

```zen
Allocator = {
    /// Allocate size bytes
    allocate: (size: usize) -> *u8,
    
    /// Free previously allocated memory
    deallocate: (ptr: *u8, size: usize) -> void,
    
    /// Resize existing allocation
    reallocate: (ptr: *u8, old_size: usize, new_size: usize) -> *u8,
}
```

### Safety Features

✅ **Null-Safe Deallocation**: Deallocating null is safe (no-op)  
✅ **Overflow Protection**: Array allocation checks for overflow  
✅ **Zero-Size Handling**: Zero-size allocation returns null  
✅ **Type Safety**: Generics ensure correct size calculations  

## Error Handling

Currently, allocators return null on failure. In future, AllocResult will provide errors:

```zen
// Future API (not yet implemented)
AllocError = {
    OutOfMemory,
    InvalidPointer,
    InvalidSize,
    Alignment,
    Corruption,
}

AllocResult<T> = Result<T, AllocError>
```

## Performance Tips

1. **Prefer allocate_array** over manual size calculation
2. **Use reallocate_array** instead of allocate+copy+deallocate
3. **Zero memory only when needed** (memzero has overhead)
4. **Consider arena allocator** for temporary allocations
5. **Profile allocation patterns** before optimizing

## Examples

### String with Allocator (Future)

```zen
// Not yet implemented, but planned:
let str = String.new(alloc)  // Create string with custom allocator
str.push_str("Hello")
// String uses allocator internally for buffer management
```

### Vec with Allocator (Future)

```zen
// Not yet implemented, but planned:
let vec = Vec.new<i32>(alloc)  // Create vector with custom allocator
vec.push(42)
// Vec uses allocator for dynamic resizing
```

### Custom Allocator (Future)

```zen
// Not yet implemented, example for future use:
impl ArenaAllocator {
    fn allocate(size: usize) -> *u8 { ... }
    fn deallocate(ptr: *u8, size: usize) -> void { ... }
    fn reset() -> void { ... }  // Reset entire arena
}

let arena = ArenaAllocator.new(1024)
let ptr = arena.allocate(256)
// Use ptr...
arena.reset()  // Free all at once
```

## What's Next

### Already Available
- ✅ GPA allocator with full functionality
- ✅ Typed allocation helpers
- ✅ Memory utilities (memzero, memcpy)
- ✅ Allocator interface definition

### Coming Soon
- 🔄 Collection integration (String, Vec, HashMap with allocators)
- 🔄 Arena allocator implementation
- 🔄 Pool allocator implementation
- 🔄 Statistics tracking allocator

### Future
- 📋 Thread-safe allocators
- 📋 Custom allocator examples
- 📋 Memory profiling tools
- 📋 Allocation pooling

## Resources

- See `INTRINSICS_REFERENCE.md` for compiler primitives
- See `TASK_18_COMPLETION.md` for implementation details
- See `stdlib/memory/allocator.zen` for trait definition
- See `stdlib/memory/gpa.zen` for GPA implementation

---

**Questions?** Check the comprehensive documentation in `TASK_18_COMPLETION.md`
