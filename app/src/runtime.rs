use std::future::Future;
use std::sync::LazyLock;

use tokio::runtime::Runtime;

static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("stashden-io")
        .build()
        .expect("starting the network runtime")
});

pub fn spawn<F>(work: F) -> impl Future<Output = F::Output> + Send + 'static
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let handle = RUNTIME.spawn(work);
    async move {
        match handle.await {
            Ok(output) => output,
            Err(error) => std::panic::resume_unwind(error.into_panic()),
        }
    }
}

#[cfg(test)]
pub fn block_on<F: Future>(work: F) -> F::Output {
    RUNTIME.block_on(work)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawned_work_runs_on_the_tokio_runtime_from_a_foreign_executor() {
        let answer = on_another_runtime(spawn(async {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            42
        }));
        assert_eq!(answer, 42);
    }

    fn on_another_runtime<F>(work: F) -> F::Output
    where
        F: Future + Send,
        F::Output: Send,
    {
        std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let local = tokio::runtime::Builder::new_current_thread()
                        .build()
                        .unwrap();
                    local.block_on(work)
                })
                .join()
                .unwrap()
        })
    }
}
