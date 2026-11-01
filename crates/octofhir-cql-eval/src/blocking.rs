//! Running async provider calls from the synchronous evaluator.

use std::future::Future;
use tokio::runtime::{Builder, Handle, RuntimeFlavor};

/// Drive `future` to completion from synchronous code.
///
/// - inside a multi-threaded Tokio runtime: `block_in_place` + `Handle::block_on`;
/// - inside a current-thread runtime (which cannot be blocked): on a scoped
///   helper thread with its own runtime;
/// - outside any runtime: on a temporary current-thread runtime.
///
/// Returns `None` only if a runtime cannot be created or the helper thread panics.
pub fn block_on<F>(future: F) -> Option<F::Output>
where
    F: Future + Send,
    F::Output: Send,
{
    match Handle::try_current() {
        Ok(handle) if handle.runtime_flavor() == RuntimeFlavor::MultiThread => {
            Some(tokio::task::block_in_place(|| handle.block_on(future)))
        }
        Ok(_) => std::thread::scope(|scope| {
            scope
                .spawn(|| run_on_new_runtime(future))
                .join()
                .ok()
                .flatten()
        }),
        Err(_) => run_on_new_runtime(future),
    }
}

fn run_on_new_runtime<F: Future>(future: F) -> Option<F::Output> {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()
        .map(|runtime| runtime.block_on(future))
}

#[cfg(test)]
mod tests {
    use super::block_on;

    #[test]
    fn outside_a_runtime() {
        assert_eq!(block_on(async { 1 + 1 }), Some(2));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn inside_current_thread_runtime() {
        assert_eq!(block_on(async { 2 }), Some(2));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn inside_multi_thread_runtime() {
        assert_eq!(block_on(async { 3 }), Some(3));
    }
}
