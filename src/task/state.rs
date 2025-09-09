use std::sync::atomic::{AtomicU16, Ordering};

/// State representing the state of a task.
pub struct State(AtomicU16);

impl State {
    /// Whether the task is running.
    pub const RUNNING: u16 = 0b0000_0000_0000_0001;
    /// Whether the task finished running.
    pub const FINISHED: u16 = 0b0000_0000_0000_0010;
    /// Whether the task has been aborted and should not run.
    pub const ABORTED: u16 = 0b0000_0000_0000_0100;

    /// Whether the executor is holding the task
    pub const EXECUTOR_ALIVE: u16 = 0b0000_0000_0001_0000;
    /// Whether there is a handle to the task alive
    pub const HANDLE_ALIVE: u16 = 0b0000_0000_0010_0000;

    /// Whether the task has already produced an output.
    pub const OUTPUT_READY: u16 = 0b0000_0001_0000_0000;
    /// Whether the output of the task has been taken.
    pub const OUTPUT_TAKEN: u16 = 0b0000_0010_0000_0000;

    pub fn new() -> Self {
        State(AtomicU16::new(0))
    }

    /// Sets the specified flag bit with the provided value.
    pub fn set(&self, item: u16, value: bool) {
        if value {
            self.0.fetch_or(item, Ordering::AcqRel);
        } else {
            self.0.fetch_and(!item, Ordering::AcqRel);
        }
    }

    /// Checks if the specified flag bit is set.
    pub fn get(&self, item: u16) -> bool {
        self.0.load(Ordering::Acquire) & item != 0
    }

    pub fn load_all(&self) -> u16 {
        self.0.load(Ordering::Acquire)
    }
}
