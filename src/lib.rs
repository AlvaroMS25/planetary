use std::any::Any;

mod builder;
mod task;
mod condvar;
mod core;
mod defer;
mod handle;
mod hooks;
mod worker;
mod join;
mod macros;

#[cfg(test)]
mod tests;

pub type JoinResult<T> = Result<T, Box<dyn Any + Send + 'static>>;
