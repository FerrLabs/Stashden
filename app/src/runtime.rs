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

pub async fn blocking<T, E>(work: impl FnOnce() -> Result<T, E> + Send + 'static) -> Result<T, E>
where
    T: Send + 'static,
    E: From<String> + Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|error| E::from(error.to_string()))?
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

    #[test]
    fn blocking_work_hands_back_its_result_and_turns_a_panic_into_an_error() {
        assert_eq!(block_on(blocking(|| Ok::<_, String>(7))), Ok(7));
        assert_eq!(
            block_on(blocking(|| Err::<(), _>("refused".to_owned()))),
            Err("refused".to_owned())
        );
        let panicked = block_on(blocking::<(), String>(|| panic!("boom")));
        assert!(panicked.is_err_and(|error| error.contains("panicked")));
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
