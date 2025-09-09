use std::time::Duration;

use crate::hooks::Hooks;

/// Builder for a `Planetary` instance.
pub struct PlanetaryBuilder {
    /// Hooks to be executed from the threadpool.
    pub(crate) hooks: Hooks,
    /// Maximum number of threads that can be spawned.
    pub(crate) max_threads: usize,
    /// Stack size for the threads.
    pub(crate) stack_size: Option<usize>,
    /// Timeout for the worker threads while not doing any work.
    pub(crate) timeout: Duration,
}

impl PlanetaryBuilder {
    pub fn new() -> Self {
        Self {
            hooks: Hooks::new(),
            max_threads: num_cpus::get(),
            stack_size: None,
            timeout: Duration::from_secs(15),
        }
    }

    /// Sets the maximum number of threads.
    pub fn max_threads(&mut self, threads: usize) -> &mut Self {
        self.max_threads = threads;
        self
    }

    /// Sets the stack size for worker threads.
    pub fn stack_size(&mut self, size: usize) -> &mut Self {
        self.stack_size = Some(size);
        self
    }

    /// Sets the timeout for worker threads without work.
    pub fn timeout(&mut self, timeout: Duration) -> &mut Self {
        self.timeout = timeout;
        self
    }

    /// Sets the hooks to be executed from the threadpool.
    pub fn with_hooks(&mut self, fun: impl FnOnce(&mut Hooks)) -> &mut Self {
        fun(&mut self.hooks);

        self
    }
}

