#[doc(hidden)]
pub struct Defer<F: FnOnce()> {
    inner: Option<F>
}

impl<T: FnOnce()> Defer<T> {
    #[doc(hidden)]
    pub fn new(f: T) -> Self {
        Self { inner: Some(f) }
    }
}

impl<T: FnOnce()> Drop for Defer<T> {
    fn drop(&mut self) {
        if let Some(f) = self.inner.take() {
            f();
        }
    }
}

#[macro_export]
macro_rules! defer {
    (|| $($tree:tt)*) => {
        let _defer = $crate::defer::Defer::new(|| $($tree)*);
    };
    (move || $($tree:tt)*) => {
        let _defer = $crate::defer::Defer::new(move || $($tree)*);
    };
    ($fun: ident, $($args:expr),*) => {
        let _defer = $crate::defer::Defer::new(move || $fun($($args),*));
    };
    ($fun: ident) => {
        let _defer = $crate::defer::Defer::new(move || $fun());
    }
}
