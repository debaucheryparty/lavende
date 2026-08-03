use pyo3::prelude::*;

mod player;

use player::PyPlayer;

#[pymodule]
fn _lavende(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPlayer>()?;
    m.add_function(wrap_pyfunction!(set_config_path, m)?)?;
    m.add_function(wrap_pyfunction!(load, m)?)?;
    m.add_function(wrap_pyfunction!(load_lyrics, m)?)?;
    m.add_function(wrap_pyfunction!(load_lyrics_by_search, m)?)?;
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (path=None))]
fn set_config_path(path: Option<String>) {
    lavende::set_config_path(path);
}

#[pyfunction]
fn load<'py>(py: Python<'py>, identifier: String) -> PyResult<Bound<'py, PyAny>> {
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        let result = lavende::load(identifier)
            .await
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))?;
        serde_json::to_string(&result)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    })
}

#[pyfunction]
#[pyo3(signature = (encoded_track, skip_track_source=false))]
fn load_lyrics<'py>(
    py: Python<'py>,
    encoded_track: String,
    skip_track_source: bool,
) -> PyResult<Bound<'py, PyAny>> {
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        lavende::load_lyrics(encoded_track, skip_track_source)
            .await
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))
    })
}

#[pyfunction]
fn load_lyrics_by_search<'py>(
    py: Python<'py>,
    title: String,
    artist: String,
) -> PyResult<Bound<'py, PyAny>> {
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        lavende::load_lyrics_by_search(title, artist)
            .await
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))
    })
}
