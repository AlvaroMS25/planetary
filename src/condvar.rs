use std::{sync::{Condvar, Mutex, WaitTimeoutResult}, time::Duration};

/// Condvar used to make threads wait for a condition to be met
pub struct Cv {
    mutex: Mutex<()>,
    condvar: Condvar,
}

impl Cv {
    pub fn new() -> Self {
        Self {
            mutex: Mutex::new(()),
            condvar: Condvar::new(),
        }
    }

    /// Wait on the condvar, returns if the condvar timed out
    pub fn wait_timeout(&self, timeout: Duration) -> bool {
        let _guard = self.mutex.lock().unwrap();
        match self.condvar.wait_timeout(_guard, timeout) {
            Err(e) => e.into_inner().1.timed_out(),
            Ok((_, res)) => res.timed_out()
        }
    }

    /// Notify a single thread waiting on the condvar
    pub fn notify_one(&self) {
        self.condvar.notify_one();
    }

    /// Notify all threads waiting on the condvar
    pub fn notify_all(&self) {
        self.condvar.notify_all();
    }
}
