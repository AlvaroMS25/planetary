use std::sync::Arc;

use crate::{core::Core, join::JoinHandle, task::{Runnable, Task}, worker};

/// The handle to an instance of a planetary threadpool,
/// can be used to interact with it and spawn tasks.
pub struct Planetary {
    inner: Arc<Core>
}

impl Planetary {
    pub fn spawn<F: Runnable>(&self, runnable: F) -> JoinHandle<F::Output> {
        let task = Task::new(runnable).erase();
        let header = task.header;
        let worker = worker::try_get_worker();
        if self.inner.should_spawn_thread() || worker.is_none() {
            self.inner.spawn_task(task);
        } else {
            worker.unwrap().queue.push(task);
        }

        JoinHandle::new(header)
    }
}
