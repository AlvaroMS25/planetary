use std::{marker::PhantomData, pin::Pin, ptr::NonNull, task::{Context, Poll}};

use crate::{task::{state::State, Header}, JoinResult};

/// Handle used to wait for a task's output.
/// 
/// If the handle is not required, please call [`JoinHandle::detach`]
#[must_use = "If unnecessary, please drop it or call .abort()"]
pub struct JoinHandle<T> {
    header: NonNull<Header>,
    _marker: PhantomData<T>
}

impl<T> JoinHandle<T> {
    pub(crate) fn new(header: NonNull<Header>) -> Self {
        unsafe {
            let header = header.as_ref();
            header.state.set(State::HANDLE_ALIVE, true);
        };

        Self {
            header,
            _marker: PhantomData
        }
    }

    /// Waits for the underlying task to dinish and returns the output.
    pub fn join(mut self) -> JoinResult<T> {
        if let Some(output) = self.try_join() {
            return output;
        }

        loop {
            {
                let thread = std::thread::current();
                let header = unsafe { self.header.as_ref() };
                header.parker()
                    .lock()
                    .unwrap_or_else(|t| t.into_inner())
                    .set_thread(thread);
            }
    
            std::thread::park();
            
            let Some(data) = self.try_join() else {
                continue;
            };

            break data;
        }
    }

    /// Marks the underlying task as aborted, telling the workers to don't run it
    /// if they haven't already.
    pub fn abort(&self) {
        Header::abort(self.header);
    }

    pub fn is_aborted(&self) -> bool {
        unsafe {
            self.header.as_ref().state_snapshot().get(State::ABORTED)
        }
    }

    /// Checks whether the task is finished
    pub fn is_finished(&self) -> bool {
        unsafe {
            self.header.as_ref().state_snapshot().get(State::FINISHED)
        }
    }

    /// Detaches the handle from the underlying task
    pub fn detach(self) {
        drop(self);
    }

    fn try_join(&mut self) -> Option<JoinResult<T>> {
        let mut res = None;
        // SAFETY: This method can only be called from join, so we have ownership
        // of the handle, and this function is only called by handles
        unsafe {
            Header::try_get_output(self.header, &mut res as *mut _ as *mut ());
        }
        res
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = JoinResult<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        if let Some(out) = this.try_join() {
            return Poll::Ready(out);
        }

        unsafe {
            let this = this.header.as_ref();
            this.parker()
                .lock()
                .unwrap_or_else(|l| l.into_inner())
                .set_waker(cx.waker().clone());
        }

        Poll::Pending
    }
}

impl<T> Drop for JoinHandle<T> {
    fn drop(&mut self) {
        unsafe {
            self.header.as_ref().state.set(State::HANDLE_ALIVE, false);
        }
        
        Header::try_dealloc(self.header);
    }
}
