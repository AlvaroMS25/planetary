use std::ptr::NonNull;

/// Vtable related to a task, used to interact with it
pub struct VTable {
    /// Runs the task provided
    pub run: unsafe fn(NonNull<()>),
    /// Aborts the task provided
    pub abort: unsafe fn(NonNull<()>),
    /// Tries to drop the task provided, returning true if the task was dropped
    pub drop: unsafe fn(NonNull<()>) -> bool,
    /// Tries to put the output of the task into the pointer, which
    /// must be a type erased pointer of Option<JoinResult<T>>
    pub take_output: unsafe fn(NonNull<()>, *mut ())
}
