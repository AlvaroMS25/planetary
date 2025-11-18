use std::{cell::UnsafeCell, collections::HashSet, ops::Deref, sync::{atomic::{AtomicUsize, Ordering}, Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard}, thread::JoinHandle, time::Duration};

use crossbeam_deque::{Injector, Steal, Stealer};

use crate::{builder::PlanetaryBuilder, condvar::Cv, hooks::Hooks, macros::tracing_feat, task::TypeErasedTask, worker::{self, WorkerCore}};

#[derive(Clone)]
pub struct Core(Arc<CoreInner>);

/// Core shared amongst all worker threads
pub struct CoreInner {
    /// Global injection queue, will be used when spawning task outside
    /// a worker thread.
    injector: Injector<TypeErasedTask>,
    /// Condvar used by worker threads to park themselves until a task is made available
    condvar: Cv,
    /// Thread information for each worker thread
    threads: RwLock<Vec<ThreadInfo>>,
    /// Occupied thread ids
    used_ids: Mutex<HashSet<usize>>,
    /// Hooks to be called on threadpool events
    pub hooks: Hooks,
    /// Whether to stop the thread pool and all the workers
    stop: UnsafeCell<bool>,
    /// Timeout for worker threads to be alive while not executing any task.
    timeout: Duration,

    // - Thread state counters -
    /// Number of threads that are currently idle, waiting for work while parked
    idle: AtomicUsize,
    /// Number of threads that are currently working
    working: AtomicUsize,

    /// Optional stack size to use when spawning threads
    stack_size: Option<usize>,
    /// Maximum number of threads that can be spawned
    max_threads: usize,
    /// Conditional variable used when shutting down the threadpool
    shutdown_cv: Cv,
}

unsafe impl Send for CoreInner {}
unsafe impl Sync for CoreInner {}

impl Core {
    pub fn new(builder: PlanetaryBuilder) -> Self {
        Self(Arc::new(CoreInner {
            injector: Injector::new(),
            condvar: Cv::new(),
            threads: RwLock::new(Vec::new()),
            used_ids: Mutex::new(HashSet::new()),
            hooks: builder.hooks,
            stop: UnsafeCell::new(false),
            timeout: builder.timeout,
            idle: AtomicUsize::new(0),
            working: AtomicUsize::new(0),
            stack_size: builder.stack_size,
            max_threads: builder.max_threads,
            shutdown_cv: Cv::new()
        }))
    }

    pub fn spawn_task(&self, task: TypeErasedTask) {
        if !self.should_spawn_thread() {
            if let Some(worker) = worker::try_get_worker() {
                tracing_feat!(trace!("Pushing task into current worker"));
                worker.queue.push(task);
                return;
            }

            tracing_feat!(trace!("Task spawned, injecting into global injector"));

            self.injector.push(task);
            self.condvar.notify_one(); // wake if a thread is parked
            return;
        }

        tracing_feat!(trace!("Task spawned, spawning new thread"));
        self.spawn_thread_with(Some(task));
    }

    #[allow(mismatched_lifetime_syntaxes)]
    fn lock_threads(&self) -> RwLockWriteGuard<Vec<ThreadInfo>> {
        if self.threads.is_poisoned() {
            self.threads.clear_poison();
        }

        self.threads.write()
            .unwrap_or_else(|s| s.into_inner())
    }

    #[allow(mismatched_lifetime_syntaxes)]
    fn lock_threads_read(&self) -> RwLockReadGuard<Vec<ThreadInfo>> {
        if self.threads.is_poisoned() {
            self.threads.clear_poison();
        }
        
        self.threads.read()
            .unwrap_or_else(|s| s.into_inner())
    }

    pub fn spawn_thread_with(&self, task: Option<TypeErasedTask>) {
        let mut lock = self.lock_threads();
        let mut ids = self.used_ids.lock().unwrap_or_else(|s| s.into_inner());
        assert!(lock.len() < self.max_threads);
        assert!(ids.len() < self.max_threads);

        let id = loop {
            let id = fastrand::usize(0..self.max_threads);

            if ids.insert(id) {
                break id;
            }
        };

        let worker = WorkerCore::new(self.clone(), id);
        let stealer = worker.queue.stealer();
        self.working.fetch_add(1, Ordering::SeqCst);

        let mut thread_builder = std::thread::Builder::new()
            .name(self.hooks.call_name_fn());

        if let Some(stack_size) = self.stack_size {
            thread_builder = thread_builder.stack_size(stack_size);
        }

        let handle = thread_builder
            .spawn(move || {
                crate::worker::run_worker(worker, task);
            })
            .unwrap_or_else(|_| panic!("Failed to spawn thread"));

        tracing_feat!(trace!("Adding thread {id} to threads"));
        lock.push(ThreadInfo {
            queue: stealer,
            handle,
            id
        });
        tracing_feat!(trace!("Thread id {id} added"));
    }

    /// Checks whether a thread should be spawned, there are idle
    /// threads that can be used to execute the task or we already
    /// spawned the maximum number of threads.
    pub fn should_spawn_thread(&self) -> bool {
        let threads = self.lock_threads();
        let idle = self.idle.load(Ordering::SeqCst);

        // If there are idle threads, we can use them to execute the task
        if idle > 0 {
            return false;
        }

        // If we already spawned the maximum number of threads, we can't spawn more
        if threads.len() >= self.max_threads {
            return false;
        }

        // We can spawn a new thread
        true
    }

    pub fn enter_idle(&self) {
        self.idle.fetch_add(1, Ordering::SeqCst);
    }

    pub fn leave_idle(&self) {
        self.idle.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn enter_working(&self) {
        self.working.fetch_add(1, Ordering::SeqCst);
    }

    pub fn leave_working(&self) {
        self.working.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn remove_worker(&self, id: usize) {
        let mut threads = self.lock_threads();
        threads.retain(|t| t.id != id);
        self.shutdown_cv.notify_all();
    }

    /// Tries taking a task from the injector, if it fails, it will try
    /// to steal it from a worker queue.
    pub fn try_steal(&self, worker_id: usize) -> Option<TypeErasedTask> {
        tracing_feat!(trace!("Worker {worker_id} trying to steal a task"));

        if let Steal::Success(task) = self.injector.steal() {
            tracing_feat!(trace!("Worker {worker_id} took a task from the global injector"));
            return Some(task);
        }

        let threads = self.lock_threads_read();
        let len = threads.len(); // - 1; the one stealing doesnt count, but the range is non inclusive, so not -1

        (0..len)
            .find_map(|_| {
                let target = fastrand::usize(0..len);

                let target_worker = unsafe { threads.get_unchecked(target) };

                if target_worker.id == worker_id {
                    return None;
                }

                // SAFETY:
                // We are in bounds due to how we got the index, so this cant be UB
                let steal = target_worker
                    .queue
                    .steal();

                match steal {
                    Steal::Empty | Steal::Retry => {
                        tracing_feat!(trace!("Worker {worker_id} failed to steal a task from worker {}", target_worker.id));
                        None
                    },
                    Steal::Success(task) => {
                        tracing_feat!(trace!("Worker {worker_id} stole a task from worker {}", target_worker.id));
                        Some(task)
                    }
                }
            })
    }

    pub fn should_stop(&self) -> bool {
        unsafe {
            std::ptr::read_volatile(self.stop.get())
        }
    }

    pub fn set_stop(&self, stop: bool) {
        unsafe {
            std::ptr::write_volatile(self.stop.get(), stop);
        }
    }

    pub fn wait_stop(&self) {
        let all_stopped = || {
            self.threads.read().unwrap().len() == 0
        };

        while !all_stopped() {
            self.shutdown_cv.wait_no_timeout();
        }
    }

    /// Parks the caller thread until a task is made available or it exceeds
    /// its timeout lifespan. Returns whether the park has timed out
    pub fn park(&self) -> bool {
        self.leave_working();
        self.enter_idle();
        self.hooks.call_on_park_fn();
        let res = self.condvar.wait_timeout(self.timeout);
        self.hooks.call_on_unpark_fn();

        if !res {
            // only enter working state if we got a wakeup for a task
            self.enter_working();
        }

        // leave idle either way, if we didnt get work, we will just kill the worker
        self.leave_idle();
        
        res
    }
}

impl Deref for Core {
    type Target = CoreInner;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

struct ThreadInfo {
    /// The thread stealer that will be used to steal tasks from its local queue
    queue: Stealer<TypeErasedTask>,
    /// The thread handle
    #[allow(unused)]
    handle: JoinHandle<()>,
    /// Thread id
    id: usize
}


