use std::{task::Waker, thread::Thread};

#[derive(Default)]
pub enum Parker {
    Waker(Waker),
    Thread(Thread),
    #[default]
    None
}


impl Parker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_waker(&mut self, waker: Waker) {
        *self = Parker::Waker(waker);
    }

    pub fn set_thread(&mut self, thread: Thread) {
        *self = Parker::Thread(thread);
    }

    pub fn wake(self) {
        match self {
            Parker::Waker(waker) => {
                waker.wake();
            }
            Parker::Thread(thread) => {
                thread.unpark();
            }
            Parker::None => {}
        }
    }

    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }
}
