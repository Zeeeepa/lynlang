use zen::lexer::Lexer;
use zen::parser::Parser;
use zen::ast::{Declaration, Statement, Expression};

#[test]
fn test_colorless_async_function_with_allocator() {
    let code = r#"
        read_file = (path: string, alloc: Ptr<Allocator>) Result<Slice<u8>, Error> {
            fd := fs.open(path) ? | .Ok -> f => f
                                  | .Err -> e => return .Err(e)
            defer fs.close(fd)
            
            buffer := alloc.value.alloc([4096, u8])
            defer alloc.value.free(buffer)
            
            bytes_read := alloc.value.is_async ?
                | true => {
                    cont := alloc.value.suspend()
                    io.read_async(fd, buffer, cont)
                }
                | false => {
                    io.read_sync(fd, buffer)
                }
            
            .Ok(buffer.slice(0, bytes_read))
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
    match &program.declarations[0] {
        Declaration::Function(func) => {
            assert_eq!(func.name, "read_file");
            assert_eq!(func.params.len(), 2);
            assert_eq!(func.params[1].0, "alloc");
        }
        _ => panic!("Expected function declaration"),
    }
}

#[test]
fn test_allocator_trait_definition() {
    let code = r#"
        Allocator = {
            alloc: <T>(size: usize) Ptr<T>,
            free: <T>(ptr: Ptr<T>) void,
            is_async: bool,
            suspend: () Option<Continuation>,
            resume: (cont: Continuation) void,
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
}

#[test]
fn test_sync_and_async_execution() {
    let code = r#"
        main = () void {
            // Synchronous execution
            sync_alloc := Ptr::new(SyncAllocator::new())
            data := read_file("config.zen", sync_alloc)
            
            // Asynchronous execution
            async_alloc := Ptr::new(AsyncAllocator::with_runtime(Runtime::init()))
            data := read_file("config.zen", async_alloc)
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
}

#[test]
fn test_channel_definition() {
    let code = r#"
        Channel<T> = {
            send: (msg: T) void,
            receive: () T,
            try_receive: () Option<T>,
            close: () void,
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
}

#[test]
fn test_actor_pattern() {
    let code = r#"
        Actor<State, Msg> = {
            state:: State,
            mailbox: Channel<Msg>,
            
            run: (self: Ptr<Actor<State, Msg>>) void {
                loop {
                    msg := self.value.mailbox.receive()
                    self.value.handle_message(msg)
                }
            },
            
            handle_message: (self: Ptr<Actor<State, Msg>>, msg: Msg) void,
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
}

#[test]
fn test_thread_spawn() {
    let code = r#"
        worker = () void {
            Thread.spawn(() void {
                loop {
                    task := get_task()
                    process(task)
                }
            })
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
}

#[test]
fn test_atomic_operations() {
    let code = r#"
        counter_example = () void {
            counter := Atomic<u64>::new(0)
            old := counter.fetch_add(1, .SeqCst)
            value := counter.load(.Acquire)
            counter.store(100, .Release)
            result := counter.compare_exchange(100, 200, .AcqRel)
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
}

#[test]
fn test_async_with_defer() {
    let code = r#"
        async_operation = (alloc: Ptr<Allocator>) Result<void, Error> {
            resource := acquire_resource(alloc)
            defer release_resource(resource)
            
            alloc.value.is_async ?
                | true => {
                    cont := alloc.value.suspend()
                    async_work(resource, cont)
                }
                | false => {
                    sync_work(resource)
                }
            
            .Ok(void)
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
}

#[test]
fn test_continuation_type() {
    let code = r#"
        Continuation = {
            id: usize,
            state: Ptr<void>,
            resume_point: usize,
        }
        
        suspend_point = (alloc: Ptr<Allocator>) Continuation {
            alloc.value.suspend() ?? Continuation{ 
                id: 0, 
                state: Ptr::null(), 
                resume_point: 0 
            }
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 2);
}

#[test]
fn test_runtime_initialization() {
    let code = r#"
        Runtime = {
            init: () Runtime,
            spawn: <T>(task: () T) Handle<T>,
            block_on: <T>(future: Future<T>) T,
            shutdown: () void,
        }
        
        main = () void {
            runtime := Runtime::init()
            defer runtime.shutdown()
            
            handle := runtime.spawn(() void {
                do_work()
            })
            
            runtime.block_on(handle)
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 2);
}

#[test]
fn test_async_patterns_no_coloring() {
    // Test that the same function works with both sync and async allocators
    // without any async/await keywords
    let code = r#"
        http_request = (url: string, alloc: Ptr<Allocator>) Result<Response, Error> {
            conn := connect(url, alloc)
            defer conn.close()
            
            request := build_request(url)
            send_result := conn.send(request, alloc)
            
            send_result ? 
                | .Ok -> _ => conn.receive(alloc)
                | .Err -> e => .Err(e)
        }
        
        // Can be called with either sync or async allocator
        test_both = () void {
            sync_result := http_request("example.com", Ptr::new(SyncAllocator::new()))
            async_result := http_request("example.com", Ptr::new(AsyncAllocator::new()))
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 2);
}