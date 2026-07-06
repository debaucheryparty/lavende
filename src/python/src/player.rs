use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde_json::Value;
use std::sync::Arc;

#[pyclass(name = "Player")]
pub struct PyPlayer {
    inner: Arc<lavende_core::Player>,
}

#[pymethods]
impl PyPlayer {
    #[new]
    fn new(guild_id: String) -> Self {
        PyPlayer {
            inner: Arc::new(lavende_core::Player::new(guild_id)),
        }
    }

    fn play<'py>(
        &self,
        py: Python<'py>,
        user_id: String,
        channel_id: String,
        session_id: String,
        token: String,
        endpoint: String,
        url: String,
        callback: PyObject,
    ) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let cb = move |_event_type: &str, payload: Value| {
                Python::with_gil(|py| {
                    if let Ok(json_str) = serde_json::to_string(&payload) {
                        let _ = callback.call1(py, (py.None(), json_str));
                    }
                });
            };

            inner
                .play(user_id, channel_id, session_id, token, endpoint, url, cb)
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))?;

            Ok(())
        })
    }

    fn pause<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            inner.pause().await;
            Ok(())
        })
    }

    fn resume<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            inner.resume().await;
            Ok(())
        })
    }

    fn stop<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            inner.stop().await;
            Ok(())
        })
    }

    fn seek<'py>(&self, py: Python<'py>, position_ms: i64) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            inner.seek(position_ms).await;
            Ok(())
        })
    }

    fn set_volume<'py>(&self, py: Python<'py>, volume: f64) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            inner.set_volume(volume).await;
            Ok(())
        })
    }

    fn get_position(&self) -> i64 {
        self.inner.get_position()
    }

    fn is_paused(&self) -> bool {
        self.inner.is_paused()
    }

    fn set_filters<'py>(
        &self,
        py: Python<'py>,
        filters_json: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            inner
                .set_filters(filters_json)
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))?;
            Ok(())
        })
    }

    fn __repr__(&self) -> String {
        format!("Player()")
    }
}
