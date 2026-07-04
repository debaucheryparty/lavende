use pyo3::prelude::*;

mod player;

use player::PyPlayer;

#[pymodule]
fn _lavende(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPlayer>()?;
    m.add_function(wrap_pyfunction!(load, m)?)?;
    Ok(())
}

#[pyfunction]
fn load<'py>(py: Python<'py>, identifier: String) -> PyResult<Bound<'py, PyAny>> {
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        lavende_core::load(identifier)
            .await
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))
    })
}
