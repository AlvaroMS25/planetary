pub trait Runnable {
    type Output: Send + 'static;

    fn run(self) -> Self::Output;
}

impl<T, R> Runnable for T
where
    T: FnOnce() -> R,
    R: Send + 'static,
{
    type Output = R;

    fn run(self) -> Self::Output {
        self()
    }
}
