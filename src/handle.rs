use std::sync::Arc;

use crate::{core::Core, join::JoinHandle, task::{Runnable, Task}, worker};

pub(crate) mod sealed {
    use std::cell::RefCell;

    use crate::handle::Planetary;

    thread_local! {
        static HANDLE: RefCell<Option<Planetary>> = RefCell::new(None);
    }

    pub fn get_handle() -> Planetary {
        try_get_handle().expect("Cannot get handle outside of the context of a threadpool")
    }

    pub fn try_get_handle() -> Option<Planetary> {
        HANDLE.with(|h| h.borrow().clone())
    }

    pub fn set_handle(new_handle: Planetary) -> Option<Planetary> {
        HANDLE.with(|h| h.borrow_mut().replace(new_handle))
    }

    pub fn remove_handle() {
        HANDLE.with(|h| *h.borrow_mut() = None);
    }
}

/// The handle to an instance of a planetary threadpool,
/// can be used to interact with it and spawn tasks.
#[derive(Clone)]
pub struct Planetary {
    pub(crate) inner: Core
}

impl Planetary {
    pub fn builder() -> crate::builder::PlanetaryBuilder {
        crate::builder::PlanetaryBuilder::new()
    }

    pub fn spawn<F: Runnable>(&self, runnable: F) -> JoinHandle<F::Output> {
        let task = Task::new(runnable).erase();
        let header = task.header;
        self.inner.spawn_task(task);

        JoinHandle::new(header)
    }

    pub fn current() -> Self {
        sealed::get_handle()
    }

    pub fn try_current() -> Option<Self> {
        sealed::try_get_handle()
    }

    pub fn shutdown(self) {
        sealed::remove_handle();
        self.inner.set_stop(true);
        self.inner.wait_stop();
    }
}
