use pyo3::prelude::*;

#[pymodule]
fn tradoshka_bridge(_m: &Bound<'_, PyModule>) -> PyResult<()> {
    Ok(())
}
