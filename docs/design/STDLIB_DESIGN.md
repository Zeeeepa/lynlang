# Zen Standard Library Design

## Overview

The Zen stdlib follows a syscall-first architecture. All I/O and synchronization
uses direct Linux syscalls via `compiler.syscall*` intrinsics - **no FFI required**.

| Layer | Location | Purpose |
|-------|----------|---------|
| **Zen Source** | `stdlib/` | User-facing `.zen` files |
| **Compiler Intrinsics** | `compiler.*` | Memory, syscalls, atomics |

---

## Module Structure

```
stdlib/
├── std.zen              # Entry point, re-exports all modules
├── compiler.zen         # Compiler intrinsics re-export
├── core/                # Core types
│   ├── option.zen       # Option<T> enum
│   ├── result.zen       # Result<T, E> enum
│   ├── ptr.zen          # Ptr<T>, MutPtr<T> safe pointers
│   ├── iterator.zen     # Iterator behavior
│   └── propagate.zen    # Error propagation helpers
├── memory/              # Memory management
│   ├── allocator.zen    # Allocator behavior
│   ├── async_allocator.zen  # Async-safe allocator
│   ├── async_pool.zen   # Async memory pool
│   ├── gpa.zen          # General Purpose Allocator
│   └── mmap.zen         # Memory mapping (syscall-based)
├── collections/         # Data structures
│   ├── char.zen         # Character utilities
│   ├── hashmap.zen      # HashMap<K,V> with iterators
│   ├── linkedlist.zen   # LinkedList<T> doubly-linked
│   ├── queue.zen        # Queue<T> circular buffer
│   ├── set.zen          # Set<T> (wraps HashMap)
│   ├── stack.zen        # Stack<T> with iterators
│   ├── string.zen       # Dynamic String type
│   └── vec.zen          # Vec<T> growable array
├── io/                  # I/O (syscall-based)
│   ├── io.zen           # Basic I/O utilities
│   ├── eventfd.zen      # Event file descriptors
│   ├── inotify.zen      # File system notifications
│   ├── signal.zen       # Signal handling
│   ├── timerfd.zen      # Timer file descriptors
│   ├── files/           # File operations
│   │   ├── file.zen     # File operations
│   │   ├── dir.zen      # Directory operations
│   │   ├── copy.zen     # File copy
│   │   ├── fs.zen       # Filesystem utilities
│   │   ├── link.zen     # Hard/soft links
│   │   ├── splice.zen   # Zero-copy data transfer
│   │   └── stat.zen     # File status
│   ├── mux/             # I/O multiplexing
│   │   ├── epoll.zen    # Event polling
│   │   ├── poll.zen     # poll() interface
│   │   └── uring.zen    # io_uring interface
│   └── net/             # Networking
│       ├── pipe.zen     # Pipes
│       ├── socket.zen   # TCP/UDP sockets
│       └── unix_socket.zen  # Unix domain sockets
├── concurrency/         # Concurrency primitives
│   ├── primitives/      # Low-level primitives
│   │   ├── atomic.zen   # Atomic operations
│   │   └── futex.zen    # Low-level futex
│   ├── sync/            # Synchronization (syscall-based)
│   │   ├── barrier.zen  # Thread barrier
│   │   ├── channel.zen  # Thread-safe message queue
│   │   ├── condvar.zen  # Condition variable
│   │   ├── mutex.zen    # Blocking mutex
│   │   ├── once.zen     # One-time initialization
│   │   ├── rwlock.zen   # Read-write lock
│   │   ├── semaphore.zen    # Counting semaphore
│   │   ├── thread.zen   # Thread spawning (clone)
│   │   └── waitgroup.zen    # Wait group
│   ├── async/           # Async runtime
│   │   ├── executor.zen # Task executor
│   │   ├── scheduler.zen    # Task scheduler
│   │   └── task.zen     # Async task
│   └── actor/           # Actor model
│       ├── actor.zen    # Actor base
│       ├── async_actor.zen  # Async actor
│       ├── supervisor.zen   # Actor supervisor
│       └── system.zen   # Actor system
├── sys/                 # System interfaces
│   ├── env.zen          # Environment variables
│   ├── memfd.zen        # Memory file descriptors
│   ├── resource.zen     # Resource limits
│   ├── seccomp.zen      # Seccomp filters
│   ├── syscall.zen      # Syscall constants
│   ├── uname.zen        # System info
│   ├── process/         # Process management
│   │   ├── prctl.zen    # Process control
│   │   ├── process.zen  # Process operations
│   │   └── sched.zen    # Scheduler control
│   └── random/          # Random number generation
│       ├── getrandom.zen    # Kernel random (getrandom)
│       └── prng.zen     # PRNG
├── math.zen             # Math functions
├── time.zen             # Time and sleep (syscall-based)
├── testing.zen          # Test utilities
├── build.zen            # Build configuration
└── ffi.zen              # FFI (for external C libraries only)
```

---

## Design Principles

1. **Syscall-First** - I/O uses `compiler.syscall*`, not libc
2. **No Hidden FFI** - FFI module exists only for loading external C libs
3. **Allocator-Aware** - All heap types take explicit `Allocator`
4. **Safe Pointers** - `Ptr<T>` and `MutPtr<T>` instead of raw pointers
5. **Error Handling** - `Result<T, E>` instead of exceptions
6. **Type Safety** - Generics with compile-time bounds

---

## Compiler Intrinsics

Available via `{ compiler } = @std` or `@std.compiler`.

### Memory
```zen
compiler.raw_allocate(size: usize) i64
compiler.raw_deallocate(ptr: i64, size: usize) void
compiler.memcpy(dst: i64, src: i64, len: usize) void
compiler.memset(ptr: i64, val: u8, len: usize) void
compiler.sizeof<T>() usize
compiler.alignof<T>() usize
```

### Pointers
```zen
compiler.gep(base: i64, offset: i64) i64
compiler.gep_struct(ptr: i64, field: i32) i64
compiler.ptr_to_int(ptr: Ptr<T>) i64
compiler.int_to_ptr(addr: i64) Ptr<T>
compiler.load<T>(ptr: i64) T
compiler.store<T>(ptr: i64, value: T) void
```

### Syscalls (Linux x86-64)
```zen
compiler.syscall0(nr: i64) i64
compiler.syscall1(nr: i64, a1: i64) i64
compiler.syscall2(nr: i64, a1: i64, a2: i64) i64
compiler.syscall3(nr: i64, a1: i64, a2: i64, a3: i64) i64
compiler.syscall4(nr: i64, a1: i64, a2: i64, a3: i64, a4: i64) i64
compiler.syscall5(nr: i64, ...) i64
compiler.syscall6(nr: i64, ...) i64
```

### Atomics
```zen
compiler.atomic_load(ptr: Ptr<u64>) u64
compiler.atomic_store(ptr: Ptr<u64>, val: u64) void
compiler.atomic_add(ptr: Ptr<u64>, val: i64) u64
compiler.atomic_sub(ptr: Ptr<u64>, val: i64) u64
compiler.atomic_cas(ptr: Ptr<u64>, expected: u64, desired: u64) u64
compiler.atomic_xchg(ptr: Ptr<u64>, val: u64) u64
compiler.atomic_fence() void
```

### Byte Swap
```zen
compiler.bswap16(val: u16) u16
compiler.bswap32(val: u32) u32
compiler.bswap64(val: u64) u64
```

---

## Implementation Status

| Module | Status | Notes |
|--------|--------|-------|
| **Core** | ✅ Complete | Option, Result, Ptr, MutPtr, Iterator, propagate |
| **Memory** | ✅ Complete | Allocator, GPA, mmap, async_allocator, async_pool |
| **Collections** | ✅ Complete | Vec, HashMap, Stack, Queue, Set, LinkedList, String, Char |
| **I/O** | ✅ Complete | File, Dir, Socket, Pipe, Epoll, Poll, io_uring, etc. (Linux x86-64) |
| **Concurrency/Sync** | ✅ Complete | Mutex, RwLock, Semaphore, Barrier, Channel, Thread, WaitGroup |
| **Concurrency/Async** | ✅ Complete | Executor, Scheduler, Task |
| **Concurrency/Actor** | ✅ Complete | Actor, AsyncActor, Supervisor, System |
| **Time** | ✅ Complete | Syscall-based clock_gettime, nanosleep |
| **Sys** | ✅ Complete | uname, env, sched, random, seccomp, memfd, resource |
| **FFI** | ⚠️ Basic | load_library/get_symbol work, call_external stub |

### Platform Support
- **Linux x86-64**: Full support via syscalls
- **Darwin/macOS**: Not yet (different syscall numbers)
- **Windows**: Not yet (needs NT API)

---

## Example: Syscall-Based File I/O

```zen
// stdlib/io/file.zen
{ compiler } = @std

SYS_OPEN = 2
SYS_READ = 0
SYS_WRITE = 1
SYS_CLOSE = 3

sys_open = (path_ptr: i64, flags: i32, mode: i32) i64 {
    return compiler.syscall3(SYS_OPEN, path_ptr, flags, mode)
}

sys_read = (fd: i32, buf_ptr: i64, count: usize) i64 {
    return compiler.syscall3(SYS_READ, fd, buf_ptr, count)
}
```

---

## Example: Futex-Based Mutex

```zen
// stdlib/sync/mutex.zen
{ compiler } = @std
{ futex_wait, futex_wake_one } = @std.sync.futex

Mutex.lock = (self: MutPtr<Mutex>) void {
    // Fast path: CAS 0 -> 1
    old = compiler.atomic_cas(&self.val.state.ref() as Ptr<u64>, 0, 1)
    old == 0 ? { return }

    // Slow path: futex wait
    futex_wait(&self.val.state.ref(), 1)
}

Mutex.unlock = (self: MutPtr<Mutex>) void {
    compiler.atomic_store(&self.val.state.ref() as Ptr<u64>, 0)
    futex_wake_one(&self.val.state.ref())
}
```

---

## Adding New Stdlib Modules

1. **Can it use syscalls?** → Use `compiler.syscall*` intrinsics
2. **Needs atomics?** → Use `compiler.atomic_*` intrinsics
3. **Pure computation?** → Write in Zen using existing stdlib
4. **Needs external C lib?** → Use FFI (last resort)

All new modules should follow the header pattern:
```zen
// Zen Standard Library: ModuleName (Syscall-based)
// No FFI - uses compiler.syscall* intrinsics
// Brief description

{ compiler } = @std
// ... imports ...
```
