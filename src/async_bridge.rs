use std::sync::OnceLock;

use pyo3::prelude::*;

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

fn runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("cruspy-async")
            .enable_all()
            .build()
            .expect("cruspy tokio runtime init failed")
    })
}

pub fn future_into_py<F>(py: Python<'_>, fut: F) -> PyResult<Py<PyAny>>
where
    F: std::future::Future<Output = PyResult<Py<PyAny>>> + Send + 'static,
{
    let handle = runtime().handle().clone();
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        handle.spawn(fut).await.expect("cruspy async task join failed")
    })
    .map(|bound| bound.unbind())
}

pub fn init_runtime() {
    let _ = runtime();
}
