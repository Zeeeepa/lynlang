# Zen Project Map

> **Purpose**: Instant context for each iteration. Read FIRST.
> **Rule**: Update when you create/modify files.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        ZEN SOURCE CODE                          │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│  RUST COMPILER (src/)                                           │
│  ┌─────────┐  ┌─────────┐  ┌───────────┐  ┌─────────────────┐  │
│  │ Lexer   │→ │ Parser  │→ │Typechecker│→ │ LLVM Codegen    │  │
│  └─────────┘  └─────────┘  └───────────┘  └─────────────────┘  │
│       │            │             │               │              │
│       └────────────┴─────────────┴───────────────┘              │
│                           │                                     │
│                    intrinsics.rs                                │
│              (syscalls, atomics, raw memory)                    │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│  ZEN STDLIB (stdlib/)                                           │
│  All language features built on intrinsics                      │
│  ┌──────┐ ┌─────────────┐ ┌──────┐ ┌─────┐ ┌───────┐          │
│  │ core │ │ collections │ │ sync │ │ io  │ │ async │          │
│  └──────┘ └─────────────┘ └──────┘ └─────┘ └───────┘          │
└─────────────────────────────────────────────────────────────────┘
```

---

## RUST COMPILER (src/) — Infrastructure Layer

### Core Pipeline
```
src/
├── main.rs              # CLI entry: run, build, repl commands
├── lib.rs               # Crate root, public API exports
├── compiler.rs          # Orchestrates: parse → typecheck → codegen
├── lexer.rs             # Tokenizer: source → Token stream
├── error.rs             # CompileError types and formatting
├── intrinsics.rs        # INTRINSICS: syscall0-6, atomics, raw memory
├── well_known.rs        # Registry: Option, Result, Vec, HashMap, etc.
├── formatting.rs        # Code formatter
└── stdlib_types.rs      # Built-in type definitions
```

### AST (src/ast/)
```
ast/
├── mod.rs               # AstNode enum, Program struct
├── declarations.rs      # FunctionDecl, StructDecl, EnumDecl, BehaviorDecl
├── expressions.rs       # Expression enum: literals, calls, binary ops
├── statements.rs        # Statement enum: let, return, if, while
├── types.rs             # AstType: primitives, generics, pointers
└── patterns.rs          # Pattern: destructuring, match arms
```

### Parser (src/parser/)
```
parser/
├── mod.rs               # Parser struct, entry point
├── core.rs              # Token consumption, error recovery
├── program.rs           # Top-level program parsing
├── types.rs             # Type annotation parsing
├── functions.rs         # Function declarations
├── structs.rs           # Struct definitions
├── enums.rs             # Enum definitions
├── behaviors.rs         # Behavior (trait) parsing
├── statements.rs        # Statement parsing
├── patterns.rs          # Pattern matching syntax
├── comptime.rs          # Compile-time blocks
├── external.rs          # FFI declarations
└── expressions/
    ├── mod.rs           # Expression dispatcher
    ├── primary.rs       # Identifiers, literals, paths
    ├── operators.rs     # Binary/unary operators, precedence
    ├── calls.rs         # Function/method calls
    ├── collections.rs   # [], {}, array/map literals
    ├── control_flow.rs  # if, match, while as expressions
    ├── blocks.rs        # Block expressions
    ├── patterns.rs      # Pattern expressions
    ├── literals.rs      # Numeric, string, char literals
    └── structs.rs       # Struct instantiation
```

### Type System (src/type_system/)
```
type_system/
├── mod.rs               # Type representation, equality
├── environment.rs       # Generic function/struct registry
├── instantiation.rs     # Generic type instantiation
└── monomorphization.rs  # Generic → concrete expansion
```

### Typechecker (src/typechecker/)
```
typechecker/
├── mod.rs               # TypeChecker struct, main inference
├── types.rs             # Type utilities
├── scope.rs             # Scope management
├── inference.rs         # Type inference engine
├── validation.rs        # Type compatibility checks
├── declaration_checking.rs  # Validate declarations
├── function_checking.rs     # Validate function bodies
├── statement_checking.rs    # Validate statements
├── type_resolution.rs       # Resolve type names
├── behaviors.rs             # Behavior implementation checking
├── intrinsics.rs            # Intrinsic type signatures
├── method_types.rs          # Method resolution
└── self_resolution.rs       # Self type in impls
```

### LLVM Codegen (src/codegen/llvm/)
```
codegen/llvm/
├── mod.rs               # LLVMCompiler struct, entry point
├── types.rs             # AstType → LLVM type conversion
├── symbols.rs           # Symbol table
├── builtins.rs          # Built-in type setup (String, Vec)
├── generics.rs          # Monomorphized generic codegen
├── patterns.rs          # Pattern match codegen
├── control_flow.rs      # If/while/loop codegen
├── binary_ops.rs        # Binary operator codegen
├── literals.rs          # Literal value codegen
├── strings.rs           # String operations
├── structs.rs           # Struct layout/access
├── pointers.rs          # Pointer operations
├── behaviors.rs         # Behavior dispatch
├── functions/
│   ├── mod.rs           # Function dispatcher
│   ├── decl.rs          # Function declarations
│   └── calls.rs         # Call site codegen
├── statements/
│   ├── mod.rs           # Statement dispatcher
│   ├── variables.rs     # Variable declarations
│   ├── control.rs       # Control flow statements
│   └── deferred.rs      # Defer statement handling
├── expressions/
│   ├── mod.rs           # Expression dispatcher
│   ├── literals.rs      # Literal codegen
│   ├── calls.rs         # Call expressions
│   ├── operations.rs    # Operator codegen
│   ├── collections.rs   # Array/map codegen
│   ├── enums.rs         # Enum variant codegen
│   ├── structs.rs       # Struct expr codegen
│   ├── patterns.rs      # Pattern expr codegen
│   ├── control.rs       # Control expr codegen
│   ├── inference.rs     # Expression type inference
│   └── utils.rs         # Codegen utilities
└── stdlib_codegen/
    ├── mod.rs           # Stdlib special handling
    ├── compiler.rs      # Compile-time evaluation
    └── helpers.rs       # Helper codegen
```

### LSP (src/lsp/) — IDE Support
```
lsp/
├── mod.rs               # LSP module root
├── server.rs            # LSP server main loop
├── types.rs             # LSP types
├── document_store.rs    # Open document management
├── analyzer.rs          # Semantic analysis
├── completion.rs        # Autocomplete
├── signature_help.rs    # Function signatures
├── inlay_hints.rs       # Inline type hints
├── semantic_tokens.rs   # Syntax highlighting
├── code_action.rs       # Quick fixes
├── code_lens.rs         # Inline actions
├── rename.rs            # Symbol renaming
├── symbols.rs           # Document/workspace symbols
├── indexing.rs          # Symbol indexing
├── call_hierarchy.rs    # Call tree
├── formatting.rs        # Format provider
├── type_inference.rs    # LSP-specific inference
├── stdlib_resolver.rs   # Stdlib lookup
├── pattern_checking.rs  # Pattern validation
├── utils.rs             # LSP utilities
├── helpers.rs           # Common helpers
├── workspace.rs         # Workspace management
├── compiler_integration.rs  # Compiler hooks
├── symbol_extraction.rs     # Symbol extraction
├── hover/
│   ├── mod.rs           # Hover dispatcher
│   ├── response.rs      # Hover formatting
│   ├── expressions.rs   # Expression hover
│   ├── structs.rs       # Struct hover
│   ├── patterns.rs      # Pattern hover
│   ├── imports.rs       # Import hover
│   ├── builtins.rs      # Builtin hover
│   ├── inference.rs     # Inferred type hover
│   └── format_string.rs # Format string hover
└── navigation/
    ├── mod.rs           # Navigation dispatcher
    ├── definition.rs    # Go to definition
    ├── references.rs    # Find references
    ├── type_definition.rs  # Go to type
    ├── highlight.rs     # Document highlight
    ├── scope.rs         # Scope navigation
    ├── imports.rs       # Import navigation
    ├── ufc.rs           # Uniform function call
    └── utils.rs         # Navigation utils
```

### Module System (src/module_system/)
```
module_system/
├── mod.rs               # Module loading
└── resolver.rs          # Import resolution
```

### Comptime (src/comptime/)
```
comptime/
└── mod.rs               # Compile-time evaluation
```

### Binaries (src/bin/)
```
bin/
├── zen-check.rs         # Type check without codegen
├── zen-format.rs        # Code formatter CLI
└── zen-lsp.rs           # Language server CLI
```

---

## ZEN STDLIB (stdlib/) — Language Features

> All features built using intrinsics. No FFI.

### Core Types (stdlib/core/)
```
core/
├── option.zen           # Option<T>: Some(T) | None
├── result.zen           # Result<T,E>: Ok(T) | Err(E)
├── ptr.zen              # Ptr<T>, MutPtr<T>, RawPtr<T>
├── iterator.zen         # Iterator behavior
└── propagate.zen        # ? operator support
```

### Collections (stdlib/collections/)
```
collections/
├── hashmap.zen          # HashMap<K,V>: open addressing, FNV-1a
├── linkedlist.zen       # LinkedList<T>: doubly-linked
├── stack.zen            # Stack<T>: LIFO, vec-backed
├── queue.zen            # Queue<T>: FIFO, ring buffer
└── set.zen              # Set<T>: wraps HashMap<T,bool>
```

### Memory (stdlib/memory/)
```
memory/
├── allocator.zen        # Allocator behavior interface
├── gpa.zen              # GeneralPurposeAllocator
├── async_allocator.zen  # AsyncAllocator behavior
├── async_pool.zen       # io_uring-based async allocator
└── mmap.zen             # Memory-mapped regions
```

### Sync Primitives (stdlib/sync/)
```
sync/
├── mutex.zen            # Mutex<T>: futex-based
├── rwlock.zen           # RwLock<T>: reader-writer
├── channel.zen          # Channel<T>: bounded MPMC
├── atomic.zen           # Atomic<T>: atomic operations
├── thread.zen           # Thread spawning
├── futex.zen            # Futex primitives
├── semaphore.zen        # Counting semaphore
├── barrier.zen          # Thread barrier
├── condvar.zen          # Condition variable
├── waitgroup.zen        # WaitGroup for goroutine-style sync
└── once.zen             # One-time initialization
```

### Async Runtime (stdlib/async/)
```
async/
├── task.zen             # Task<T>: async task handle
├── executor.zen         # Executor: task runner
├── scheduler.zen        # Scheduler: work stealing
└── pool.zen             # ThreadPool
```

### Actor System (stdlib/actor/)
```
actor/
├── actor.zen            # Actor behavior
├── supervisor.zen       # Supervision trees
└── system.zen           # ActorSystem runtime
```

### I/O (stdlib/io/)
```
io/
├── io.zen               # I/O types and traits
├── file.zen             # File: syscall-based operations
├── socket.zen           # TcpListener, TcpStream, UdpSocket
├── unix_socket.zen      # UnixListener, UnixStream, SocketPair
├── pipe.zen             # Pipe: Unix pipes
├── epoll.zen            # Epoll: event polling
├── poll.zen             # Poll: portable polling
├── uring.zen            # IoUring: io_uring interface
├── eventfd.zen          # EventFd: event notification
├── timerfd.zen          # TimerFd: timer events
├── inotify.zen          # Inotify: file watching
├── signal.zen           # Signal handling
├── splice.zen           # Splice: zero-copy I/O
└── process.zen          # Process spawning
```

### System (stdlib/sys/)
```
sys/
├── syscall.zen          # Raw syscall wrappers
├── env.zen              # Environment variables
├── uname.zen            # System info
├── random.zen           # System random
├── resource.zen         # Resource limits
├── sched.zen            # Scheduling
├── prctl.zen            # Process control
└── memfd.zen            # Memory file descriptors
```

### Top-Level
```
stdlib/
├── std.zen              # Prelude: re-exports common types
├── string.zen           # String: dynamic string
├── vec.zen              # Vec<T>: dynamic array
├── char.zen             # Character utilities
├── error.zen            # Error types
├── time.zen             # Time types
├── random.zen           # Random number generation
├── math/math.zen        # Math functions
├── fs/fs.zen            # Filesystem operations
├── ffi/ffi.zen          # FFI utilities
├── build/build.zen      # Build system
├── compiler/compiler.zen # Compiler API
├── testing/runner.zen   # Test runner
└── time/time.zen        # Extended time
```

---

## Key Type Signatures

```zen
// === Core ===
enum Option<T> { Some(T), None }
enum Result<T, E> { Ok(T), Err(E) }
struct Ptr<T> { raw: i64 }
struct MutPtr<T> { raw: i64 }

// === Collections ===
struct Vec<T> { data: MutPtr<T>, len: u64, cap: u64, allocator: Allocator }
struct String { bytes: Vec<u8> }
struct HashMap<K, V> { entries: Vec<Entry<K,V>>, count: u64, allocator: Allocator }

// === Behaviors ===
Iterator<T>: behavior { next: (mut self) Option<T> }
Allocator: behavior {
    alloc: (mut self, size: u64, align: u64) ?MutPtr<u8>
    free: (mut self, ptr: MutPtr<u8>, size: u64, align: u64)
}

// === Intrinsics (Rust-provided) ===
@builtin.syscall0(nr: i64) -> i64
@builtin.syscall6(nr, a1, a2, a3, a4, a5, a6: i64) -> i64
@builtin.atomic_load(ptr: *T) -> T
@builtin.atomic_store(ptr: *T, val: T)
@builtin.atomic_cas(ptr: *T, expected: T, desired: T) -> bool
@builtin.raw_allocate(size: u64) -> i64
@builtin.memcpy(dst: *u8, src: *u8, len: u64)
```

---

## Test Files (tests/)
```
tests/
├── allocator_compilation.rs  # Allocator codegen tests
├── behavioral_tests.rs       # Behavior system tests
├── codegen_integration.rs    # Full codegen tests
├── lexer_integration.rs      # Lexer tests
├── lexer_tests.rs            # Unit lexer tests
├── parser_integration.rs     # Parser integration
├── parser_tests.rs           # Unit parser tests
├── ptr_ref_tests.rs          # Pointer/reference tests
└── lsp_text_edit.rs          # LSP edit tests
```

---

## Last Updated
- **Iteration**: 4
- **Changes**: Added collection types to WellKnownTypes registry
