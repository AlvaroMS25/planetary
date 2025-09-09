use std::{marker::PhantomData, pin::Pin, ptr::NonNull, task::{Context, Poll}};

use crate::{task::Header, JoinResult};

pub struct JoinHandle<T> {
    header: NonNull<Header>,
    _marker: PhantomData<T>
}

impl<T> JoinHandle<T> {
    pub(crate) fn new(header: NonNull<Header>) -> Self {
        Self {
            header,
            _marker: PhantomData
        }
    }

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

    pub fn abort(&self) {
        unsafe {
            self.header.as_ref().mark_aborted();
        }
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
