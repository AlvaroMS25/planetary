use std::any::Any;

use crate::{handle::Planetary, join::JoinHandle, task::Runnable};

pub mod builder;
pub mod task;
mod condvar;
mod core;
mod defer;
pub mod handle;
mod hooks;
mod worker;
pub mod join;
mod macros;

#[cfg(test)]
mod tests;

pub type JoinResult<T> = Result<T, Box<dyn Any + Send + 'static>>;

pub fn spawn<F: Runnable>(fun: F) -> JoinHandle<F::Output> {
    Planetary::current().spawn(fun)
}
