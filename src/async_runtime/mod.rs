// Colorless Async Runtime for Zen
// Implements allocator-based async execution per Language Spec v1.1.0

use std::sync::Arc;
use std::pin::Pin;
use std::future::Future;
use std::task::{Context as TaskContext, Poll, Waker};
use std::collections::VecDeque;
use std::sync::Mutex;
use crate::ast::AstType;

/// Allocator trait that includes execution context
/// This enables colorless async - same functions work sync or async
#[derive(Clone)]
pub struct Allocator {
    // Memory operations
    pub alloc_fn: Arc<dyn Fn(usize) -> *mut u8 + Send + Sync>,
    pub free_fn: Arc<dyn Fn(*mut u8) + Send + Sync>,
    
    // Execution mode
    pub is_async: bool,
    
    // Async operations (no-op for sync)
    pub suspend_fn: Option<Arc<dyn Fn() -> Option<Continuation> + Send + Sync>>,
    pub resume_fn: Option<Arc<dyn Fn(Continuation) + Send + Sync>>,
    
    // Runtime context
    runtime: Option<Arc<Runtime>>,
}

impl Allocator {
    /// Create a synchronous allocator
    pub fn sync() -> Self {
        Self {
            alloc_fn: Arc::new(|size| {
                let layout = std::alloc::Layout::from_size_align(size, 8).unwrap();
                unsafe { std::alloc::alloc(layout) }
            }),
            free_fn: Arc::new(|ptr| {
                let layout = std::alloc::Layout::from_size_align(1, 8).unwrap();
                unsafe { std::alloc::dealloc(ptr, layout) }
            }),
            is_async: false,
            suspend_fn: None,
            resume_fn: None,
            runtime: None,
        }
    }
    
    /// Create an asynchronous allocator with runtime
    pub fn async_with_runtime(runtime: Arc<Runtime>) -> Self {
        let runtime_clone = runtime.clone();
        let runtime_clone2 = runtime.clone();
        
        Self {
            alloc_fn: Arc::new(|size| {
                let layout = std::alloc::Layout::from_size_align(size, 8).unwrap();
                unsafe { std::alloc::alloc(layout) }
            }),
            free_fn: Arc::new(|ptr| {
                let layout = std::alloc::Layout::from_size_align(1, 8).unwrap();
                unsafe { std::alloc::dealloc(ptr, layout) }
            }),
            is_async: true,
            suspend_fn: Some(Arc::new(move || {
                runtime_clone.suspend()
            })),
            resume_fn: Some(Arc::new(move |cont| {
                runtime_clone2.resume(cont);
            })),
            runtime: Some(runtime),
        }
    }
    
    /// Allocate memory
    pub fn alloc<T>(&self, size: usize) -> *mut T {
        (self.alloc_fn)(size) as *mut T
    }
    
    /// Free memory
    pub fn free<T>(&self, ptr: *mut T) {
        (self.free_fn)(ptr as *mut u8)
    }
    
    /// Suspend execution (async only)
    pub fn suspend(&self) -> Option<Continuation> {
        if let Some(suspend) = &self.suspend_fn {
            suspend()
        } else {
            None
        }
    }
    
    /// Resume execution (async only)
    pub fn resume(&self, cont: Continuation) {
        if let Some(resume) = &self.resume_fn {
            resume(cont)
        }
    }
}

/// Continuation for suspended async operations
#[derive(Debug, Clone)]
pub struct Continuation {
    pub id: usize,
    pub waker: Option<Waker>,
    pub state: ContinuationState,
}

#[derive(Debug, Clone)]
pub enum ContinuationState {
    Ready(Vec<u8>),
    Pending,
    Completed(Vec<u8>),
}

/// Async runtime for managing colorless async execution
pub struct Runtime {
    tasks: Arc<Mutex<VecDeque<Task>>>,
    next_task_id: Arc<Mutex<usize>>,
}

impl Runtime {
    /// Initialize a new runtime
    pub fn init() -> Arc<Self> {
        Arc::new(Self {
            tasks: Arc::new(Mutex::new(VecDeque::new())),
            next_task_id: Arc::new(Mutex::new(0)),
        })
    }
    
    /// Suspend current task and return continuation
    pub fn suspend(&self) -> Option<Continuation> {
        let mut id_lock = self.next_task_id.lock().unwrap();
        let id = *id_lock;
        *id_lock += 1;
        
        Some(Continuation {
            id,
            waker: None,
            state: ContinuationState::Pending,
        })
    }
    
    /// Resume a continuation
    pub fn resume(&self, cont: Continuation) {
        let task = Task {
            id: cont.id,
            continuation: cont,
            future: None,
        };
        
        let mut tasks = self.tasks.lock().unwrap();
        tasks.push_back(task);
    }
    
    /// Run the event loop
    pub fn run(&self) {
        loop {
            let task = {
                let mut tasks = self.tasks.lock().unwrap();
                tasks.pop_front()
            };
            
            match task {
                Some(mut task) => {
                    // Process task
                    match task.continuation.state {
                        ContinuationState::Pending => {
                            // Re-queue pending task
                            let mut tasks = self.tasks.lock().unwrap();
                            tasks.push_back(task);
                        }
                        ContinuationState::Ready(_) => {
                            // Execute ready task
                            task.continuation.state = ContinuationState::Completed(vec![]);
                        }
                        ContinuationState::Completed(_) => {
                            // Task done, don't re-queue
                        }
                    }
                }
                None => {
                    // No tasks, check if we should exit
                    let tasks = self.tasks.lock().unwrap();
                    if tasks.is_empty() {
                        break;
                    }
                    // Otherwise yield to system
                    std::thread::yield_now();
                }
            }
        }
    }
    
    /// Spawn a new async task
    pub fn spawn<F>(&self, future: F) -> usize
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let mut id_lock = self.next_task_id.lock().unwrap();
        let id = *id_lock;
        *id_lock += 1;
        
        let task = Task {
            id,
            continuation: Continuation {
                id,
                waker: None,
                state: ContinuationState::Pending,
            },
            future: Some(Box::pin(future)),
        };
        
        let mut tasks = self.tasks.lock().unwrap();
        tasks.push_back(task);
        
        id
    }
}

/// Task managed by the runtime
struct Task {
    id: usize,
    continuation: Continuation,
    future: Option<Pin<Box<dyn Future<Output = ()> + Send>>>,
}

/// Example async file read implementation
pub async fn read_file_async(path: &str, alloc: &Allocator) -> Result<Vec<u8>, std::io::Error> {
    if alloc.is_async {
        // Async path using tokio or similar
        tokio::fs::read(path).await
    } else {
        // Sync path
        std::fs::read(path)
    }
}

/// Sync allocator for deterministic execution
pub struct SyncAllocator {
    allocated: Arc<Mutex<Vec<usize>>>, // Store as usize to avoid Send/Sync issues
}

impl SyncAllocator {
    pub fn new() -> Self {
        Self {
            allocated: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    pub fn into_allocator(self) -> Allocator {
        let allocated = self.allocated.clone();
        let allocated_free = self.allocated.clone();
        
        Allocator {
            alloc_fn: Arc::new(move |size| {
                let layout = std::alloc::Layout::from_size_align(size, 8).unwrap();
                let ptr = unsafe { std::alloc::alloc(layout) };
                allocated.lock().unwrap().push(ptr as usize);
                ptr
            }),
            free_fn: Arc::new(move |ptr| {
                let layout = std::alloc::Layout::from_size_align(1, 8).unwrap();
                unsafe { std::alloc::dealloc(ptr, layout) };
                allocated_free.lock().unwrap().retain(|&p| p != ptr as usize);
            }),
            is_async: false,
            suspend_fn: None,
            resume_fn: None,
            runtime: None,
        }
    }
}

/// Async allocator with runtime integration
pub struct AsyncAllocator {
    runtime: Arc<Runtime>,
    allocated: Arc<Mutex<Vec<usize>>>, // Store as usize to avoid Send/Sync issues
}

impl AsyncAllocator {
    pub fn with_runtime(runtime: Arc<Runtime>) -> Self {
        Self {
            runtime,
            allocated: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    pub fn into_allocator(self) -> Allocator {
        Allocator::async_with_runtime(self.runtime)
    }
}

/// Helper to determine execution mode from allocator
pub fn is_async_context(alloc: &Allocator) -> bool {
    alloc.is_async
}

/// Execute with appropriate allocator based on context
pub fn execute_with_allocator<T, F>(alloc: &Allocator, f: F) -> T
where
    F: FnOnce(&Allocator) -> T,
{
    f(alloc)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sync_allocator() {
        let alloc = Allocator::sync();
        assert!(!alloc.is_async);
        
        let ptr = alloc.alloc::<u32>(4);
        assert!(!ptr.is_null());
        alloc.free(ptr);
    }
    
    #[test]
    fn test_async_allocator() {
        let runtime = Runtime::init();
        let alloc = Allocator::async_with_runtime(runtime);
        assert!(alloc.is_async);
        
        let cont = alloc.suspend();
        assert!(cont.is_some());
    }
    
    #[test]
    fn test_colorless_function() {
        // Same function works with both sync and async allocators
        fn process_data(data: &[u8], alloc: &Allocator) -> Vec<u8> {
            if alloc.is_async {
                // Could suspend here for async operations
                if let Some(_cont) = alloc.suspend() {
                    // In real implementation, would resume after I/O
                }
            }
            
            // Process data
            data.to_vec()
        }
        
        // Test with sync allocator
        let sync_alloc = Allocator::sync();
        let result = process_data(b"test", &sync_alloc);
        assert_eq!(result, b"test");
        
        // Test with async allocator
        let runtime = Runtime::init();
        let async_alloc = Allocator::async_with_runtime(runtime);
        let result = process_data(b"test", &async_alloc);
        assert_eq!(result, b"test");
    }
}