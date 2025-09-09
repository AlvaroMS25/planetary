/// Executes the provided block of code if the `tracing` feature is enabled.
macro_rules! tracing_feat {
    ($($tree:tt)*) => {
        #[cfg(feature = "tracing")]
        {
            use tracing::*;
            $($tree)*
        }
    };
}

pub(crate) use tracing_feat;
