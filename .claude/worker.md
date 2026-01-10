# Zen Work Streams

## Active Stream: 2 (Actor Framework)
## Last Updated: 2026-01-10

### Stream 1 (Allocators) - COMPLETE (pending intrinsics):
- ✅ AsyncAllocator behavior interface (`stdlib/memory/async_allocator.zen`)
- ✅ AsyncPool allocator with io_uring (`stdlib/memory/async_pool.zen`)
- ✅ Task/Coroutine system (`stdlib/async/task.zen`)
- ✅ Scheduler with task queues (`stdlib/async/scheduler.zen`)
- ⚠️ Blocked: `context_switch` needs compiler intrinsic (assembly)
- ⚠️ Blocked: TLS for `CURRENT_TASK` (needs FS segment register)

### Stream 2 (Actors) - IN PROGRESS:
- ✅ Actor type with ActorBehavior (`stdlib/actor/actor.zen`)
- ✅ Supervisor with restart strategies (`stdlib/actor/supervisor.zen`)
- ✅ ActorSystem with registry (`stdlib/actor/system.zen`)
- ✅ AsyncActor for scheduler integration (`stdlib/actor/async_actor.zen`)
- ✅ Thread-spawned actors via `spawn_threaded`

### Anti-Slop Passes (Iterations 9-10):
- ✅ Created `stdlib/sys/syscall.zen` - single source for all syscall numbers
- ✅ Migrated 10+ files to use centralized syscall constants
- ✅ Fixed bug in async_pool.zen (allocate not incrementing offset)
- ✅ Extracted alloc_pending_op helper in async_pool.zen
- ✅ Removed unused imports/constants in scheduler.zen

---

## Stream 1: Allocator-Driven Concurrency (PRIORITY)

**Goal:** Make `GPA` (sync) vs `AsyncPool` (async) first-class - no function coloring

**From LANGUAGE_SPEC.zen lines 260-270:**
```zen
// Multisync function - sync or async based on allocator!
fetch_game_data = (url: string, alloc: Allocator) Result<Data, Error> {
    client = HttpClient(alloc)  // Behavior determined by allocator
    response = client.get(url)  // This blocks or doesn't based on allocator!
    ...
}
```

### Tasks:
- [ ] **1.1 AsyncAllocator Behavior** - Define async allocator interface
  - AsyncPool.init() creates non-blocking allocator
  - Integrates with io_uring/epoll for async I/O
  - Uses cooperative scheduling (no OS threads for async)

- [ ] **1.2 Allocator Detection at Call Sites**
  - When function receives Allocator, detect if async
  - Async allocators suspend/resume using coroutine-like mechanism
  - Sync allocators block (current behavior)

- [ ] **1.3 Await Points**
  - Await is implicit at I/O boundaries
  - Allocator determines whether operation suspends or blocks
  - No `async`/`await` keywords - allocator decides

- [ ] **1.4 Runtime Scheduler**
  - AsyncPool needs work-stealing scheduler
  - Integration with system scheduling (sched_yield, CPU affinity)
  - Futex-based parking/waking

### Key Files:
- `stdlib/memory/allocator.zen` - Base Allocator behavior
- `stdlib/memory/gpa.zen` - Sync allocator (done)
- `stdlib/memory/async_pool.zen` - NEW: Async allocator
- `stdlib/sync/scheduler.zen` - NEW: Work-stealing scheduler

---

## Stream 2: Actor Framework (Hollywood-style)

**Goal:** First-class actors like Go's Hollywood framework

**From LANGUAGE_SPEC.zen lines 273-286:**
```zen
create_fibonacci = () Actor {
    return Actor((receiver) {
        a ::= 0
        b ::= 1
        loop(() {
            receiver.send(a)
            temp = a + b
            a = b
            b = temp
        })
    })
}
```

### Requires:
- Stream 1 complete (AsyncAllocator)
- System scheduling integration

### Tasks:
- [ ] **2.1 Actor Type**
  - Actor is a lightweight process with mailbox
  - Spawned onto scheduler (not OS thread)
  - Has receive/send semantics

- [ ] **2.2 Mailbox**
  - Lock-free bounded queue (extends Channel)
  - Supports selective receive
  - Timeout support

- [ ] **2.3 Supervision**
  - Parent-child actor relationships
  - Failure propagation strategies (restart, stop, escalate)
  - Hollywood-style "one for one" / "all for one"

- [ ] **2.4 Actor Registry**
  - Named actors
  - Location transparency (local vs remote)

### Key Files:
- `stdlib/actor/actor.zen` - NEW
- `stdlib/actor/mailbox.zen` - NEW
- `stdlib/actor/supervisor.zen` - NEW
- `stdlib/actor/registry.zen` - NEW

---

## Stream 3: LSP Architecture (Compiler-Driven)

**Goal:** LSP relies on compiler/parser, not hardcoded solutions

### Current Problems:
- Hardcoded type checks (e.g., `if name == "Option"`)
- Duplicate type inference in codegen (1,023 LOC)
- LSP doesn't use typechecker properly

### Tasks:
- [ ] **3.1 Typed AST**
  - Add `resolved_type: Option<Type>` to Expression nodes
  - Typechecker fills in types during check pass
  - LSP reads from typed AST

- [ ] **3.2 Remove Codegen Inference**
  - Delete `codegen/llvm/expressions/inference.rs` (1,023 LOC)
  - Codegen trusts typechecker output

- [ ] **3.3 Symbol Table for LSP**
  - Single source of truth for symbols
  - Updated by parser, refined by typechecker
  - LSP queries symbol table directly

- [ ] **3.4 Incremental Compilation**
  - File-level dependency tracking
  - Incremental typechecking for LSP responsiveness

### Key Files:
- `src/typechecker/mod.rs` - Add type annotations to AST
- `src/codegen/llvm/expressions/inference.rs` - DELETE
- `src/lsp/` - Refactor to use symbol table

---

## Stream 4: Intrinsics Purity

**Goal:** Rust provides ONLY intrinsics, not language features

### Principle:
> "If it can be written in Zen using intrinsics, it should be."

### Current Violations (AUDITED):

**codegen/llvm/generics.rs:**
- `if name == "Array"` - layout hardcoded
- `if name == "Vec"` - layout hardcoded
- `if name == "HashMap"` - layout hardcoded
- `if name == "HashSet"` - layout hardcoded

**codegen/llvm/statements/variables.rs:**
- `if type_name == "Array"` - allocation hardcoded
- `if type_name == "HashMap"` - allocation hardcoded
- `if type_name == "HashSet"` - allocation hardcoded
- `if type_name == "DynVec"` - allocation hardcoded

**codegen/llvm/types.rs:**
- `if name == "Array"` - type mapping
- `if name == "HashMap"` - type mapping
- `if name == "HashSet"` - type mapping
- `if name == "String"` - type mapping (should be struct)
- `if name == "DynVec"` - type mapping

**codegen/llvm/expressions/inference.rs (1,023 LOC - DUPLICATE!):**
- `if name == "Range.new"` - special casing
- `if name == "Result.Ok"` - enum constructor
- `if name == "Result.Err"` - enum constructor
- `if name == "Option.Some"` - enum constructor
- `if name == "Option.None"` - enum constructor
- `if name == "HashMap"` - method return types
- `if name == "HashSet"` - method return types
- `if name == "Array"` - element access

**codegen/llvm/behaviors.rs:**
- `if name == "Range"` - special dispatch

**Total: ~50 hardcoded type checks in codegen**

### Tasks:
- [ ] **4.1 Audit Remaining Hardcoded Features** ✅ DONE (see above)

- [ ] **4.2 Move to Stdlib**
  - Anything not an LLVM primitive -> Zen stdlib
  - Add new intrinsics ONLY if needed for LLVM

- [ ] **4.3 Document Intrinsic Boundary**
  - Update `docs/INTRINSICS_BOUNDARY.md`
  - Clear definition: what IS an intrinsic

### Allowed Intrinsics:
```
Memory:     raw_allocate, raw_deallocate, memcpy, memset, memcmp
Pointers:   gep, gep_struct, ptr_to_int, int_to_ptr, load<T>, store<T>
Types:      sizeof<T>, alignof<T>
Syscalls:   syscall0..syscall6
Atomics:    atomic_load, atomic_store, atomic_cas, atomic_add, atomic_sub
Bit ops:    bswap16/32/64, ctlz, cttz, ctpop
```

### NOT Intrinsics (should be Zen):
- String methods
- Collection methods
- Iterator operations
- Pattern matching dispatch

---

## Stream 5: Runtime Threading

**Goal:** Proper thread spawning using clone() syscall

### Current State:
- `stdlib/sync/thread.zen` uses clone() syscall (good!)
- Missing proper stack management
- No thread-local storage

### Tasks:
- [ ] **5.1 Stack Allocation**
  - mmap for thread stacks
  - Guard pages for stack overflow detection

- [ ] **5.2 Thread-Local Storage**
  - FS segment register on x86-64
  - Per-thread allocator support

- [ ] **5.3 Thread Pool**
  - Fixed pool for CPU-bound work
  - Work-stealing for load balancing

---

## Dependencies

```
Stream 1 (Allocators) ──┬──> Stream 2 (Actors)
                        │
                        └──> Stream 5 (Threading)

Stream 3 (LSP) ─────────> Stream 4 (Intrinsics)
```

---

## Current Status

| Stream | Status | Blocking |
|--------|--------|----------|
| 1. Allocators | COMPLETE* | context_switch intrinsic |
| 2. Actors | IN PROGRESS | - |
| 3. LSP | NOT STARTED | - |
| 4. Intrinsics | AUDITED | - |
| 5. Threading | BASIC | - |

*Allocators complete at stdlib level, blocked on compiler intrinsics for full functionality

---

## Ralph Loop Assignments

When starting a ralph loop, set `Active Stream` at top of this file.

### Suggested Order:
1. **Stream 4** (Intrinsics Purity) - Foundation work
2. **Stream 3** (LSP) - Improves dev experience
3. **Stream 1** (Allocators) - Core feature
4. **Stream 2** (Actors) - Depends on 1
5. **Stream 5** (Threading) - Depends on 1
