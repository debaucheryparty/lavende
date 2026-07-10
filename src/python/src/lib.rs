use pyo3::prelude::*;

mod player;

use player::PyPlayer;

#[pymodule]
fn _lavende(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPlayer>()?;
    m.add_function(wrap_pyfunction!(load, m)?)?;
    m.add_function(wrap_pyfunction!(set_config_path, m)?)?;
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (path=None))]
fn set_config_path(path: Option<String>) {
    lavende_core::set_config_path(path);
}

#[pyfunction]
fn load<'py>(py: Python<'py>, identifier: String) -> PyResult<Bound<'py, PyAny>> {
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        lavende_core::load(identifier)
            .await
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))
    })
}
