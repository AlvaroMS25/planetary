use std::{cell::UnsafeCell, sync::Arc};

use crossbeam_deque::Worker;

use crate::{core::Core, defer, hooks::Hooks, macros::tracing_feat, task::TypeErasedTask};

thread_local! {
    static WORKER: UnsafeCell<Option<*const WorkerCore>> = UnsafeCell::new(None);
}

pub struct WorkerCore {
    core: Core,
    pub queue: Worker<TypeErasedTask>,
    id: usize
}

impl WorkerCore {
    pub fn new(core: Core, id: usize) -> Self {
        let queue = Worker::new_fifo();

        Self {
            core,
            queue,
            id
        }
    }
}

impl Drop for WorkerCore {
    fn drop(&mut self) {
        // Remove the worker from the core
        self.core.remove_worker(self.id);
    }
}

pub fn run_worker(core: WorkerCore, initial_task: Option<TypeErasedTask>) {
    tracing_feat!(info!("Worker {} started", core.id));

    WORKER.with(|worker| {
        unsafe {
            *worker.get() = Some(&core);
        }
    });
    defer!(|| {
        WORKER.with(|worker| {
            unsafe {
                *worker.get() = None;
            }
        });

        core.core.hooks.call_on_stop_fn();
        core.core.remove_worker(core.id);

        tracing_feat!(info!("Worker {} stopped", core.id));
    });

    crate::handle::sealed::set_handle(crate::handle::Planetary {
        inner: core.core.clone()
    });

    core.core.hooks.call_on_start_fn();

    if let Some(task) = initial_task {
        execute_task_inner(&core.core.hooks, task);
    }

    loop {
        if core.core.should_stop() {
            return;
        }

        // try execute a task, if we cant sleep for timeout at max and die
        if !try_execute_task(&core) {
            if core.core.park() {
                return; // die, defer macro will do its magic here
            }
        }
    }
}

/// Tries to execute a task, and returns whether it was executed successfully or not
fn try_execute_task(core: &WorkerCore) -> bool {
    if let Some(task) = core.queue.pop() {
        execute_task_inner(&core.core.hooks, task);
        return true;
    }

    // try stealing a task from another worker
    if let Some(task) = core.core.try_steal(core.id) {
        execute_task_inner(&core.core.hooks, task);
        true
    } else {
        false
    }
}

fn execute_task_inner(hooks: &Hooks, task: TypeErasedTask) {
    hooks.call_before_work_fn();
    task.run();
    hooks.call_after_work_fn();
}

/// Yields execution to the current worker for a single task,
/// panics if called outside a threadpool worker
pub fn yield_now() {
    let core = unsafe {
        let opt = WORKER.with(|c| &*c.get());

        opt.expect("yield_now must be called from within a worker context");

        &*opt.unwrap()
    };

    try_execute_task(core);
}

pub(crate) fn try_get_worker() -> Option<&'static WorkerCore> {
    unsafe {
        let ptr = WORKER.with(|w| {
            w.get()
        });

        (&*ptr).map(|core| &*core)
    }
}
