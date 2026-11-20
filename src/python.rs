use crate::{Error, Light};
use pyo3::{
    exceptions::{PyRuntimeError, PyValueError},
    prelude::*,
    types::{PyBytes, PyModule},
};
use std::{sync::Mutex, time::Duration};

fn hardware_error(error: Error) -> PyErr {
    PyRuntimeError::new_err((error.kind(), error.to_string()))
}

#[pyclass(name = "Light", module = "rs485_light_sdk")]
struct PyLight {
    light: Mutex<Option<Light>>,
}

#[pymethods]
impl PyLight {
    #[new]
    #[pyo3(signature = (port, station="FF", baud=9600, timeout_ms=750))]
    fn open(
        py: Python<'_>,
        port: &str,
        station: &str,
        baud: u32,
        timeout_ms: u64,
    ) -> PyResult<Self> {
        let station = station.parse().map_err(hardware_error)?;
        let light = py
            .detach(|| Light::open(port, baud, station, Duration::from_millis(timeout_ms)))
            .map_err(hardware_error)?;
        Ok(Self {
            light: Mutex::new(Some(light)),
        })
    }

    fn verify_connection(&self, py: Python<'_>) -> PyResult<u16> {
        py.detach(|| self.operate(Light::verify_connection))
    }

    fn apply(&self, py: Python<'_>, scene: &str) -> PyResult<()> {
        let scene = scene.parse().map_err(hardware_error)?;
        py.detach(|| self.operate(|light| light.apply(scene)))
    }

    #[pyo3(signature = (scene, hold_ms=750))]
    fn preview(&self, py: Python<'_>, scene: &str, hold_ms: u64) -> PyResult<()> {
        let scene = scene.parse().map_err(hardware_error)?;
        py.detach(|| self.operate(|light| light.preview(scene, Duration::from_millis(hold_ms))))
    }

    fn close(&self) -> PyResult<()> {
        *self.light.try_lock().map_err(lock_error)? = None;
        Ok(())
    }
}

impl PyLight {
    fn operate<T>(&self, callback: impl FnOnce(&mut Light) -> Result<T, Error>) -> PyResult<T> {
        let mut guard = self.light.try_lock().map_err(lock_error)?;
        let light = guard
            .as_mut()
            .ok_or_else(|| PyRuntimeError::new_err(("closed", "serial connection is closed")))?;
        callback(light).map_err(hardware_error)
    }
}

fn lock_error(error: std::sync::TryLockError<std::sync::MutexGuard<'_, Option<Light>>>) -> PyErr {
    let kind = match error {
        std::sync::TryLockError::WouldBlock => "busy",
        std::sync::TryLockError::Poisoned(_) => "connection_unusable",
    };
    PyRuntimeError::new_err((kind, error.to_string()))
}

#[pyfunction]
fn list_ports(py: Python<'_>) -> PyResult<Vec<(String, Option<String>)>> {
    let ports = py
        .detach(crate::available_ports)
        .map_err(hardware_error_open)?;
    Ok(ports
        .into_iter()
        .map(|port| {
            let description = match port.port_type {
                crate::SerialPortType::UsbPort(info) => info.product.or(info.manufacturer),
                _ => None,
            };
            (port.port_name, description)
        })
        .collect())
}

fn hardware_error_open(error: serialport::Error) -> PyErr {
    PyRuntimeError::new_err(("enumeration_failed", error.to_string()))
}

#[pyfunction]
fn crc16(data: &[u8]) -> u16 {
    crate::protocol::crc16(data)
}

#[pyfunction]
fn preset_frame(py: Python<'_>, station: u8, code: u8) -> Bound<'_, PyBytes> {
    PyBytes::new(py, &crate::protocol::preset_frame(station, code))
}

#[pyfunction]
fn verify_echo(echo: &[u8], station: u8, code: u8) -> PyResult<()> {
    crate::protocol::verify_echo(echo, station, code)
        .map_err(|error| PyValueError::new_err(error.description()))
}

#[pymodule]
fn rs485_light_sdk(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyLight>()?;
    module.add_function(wrap_pyfunction!(list_ports, module)?)?;
    module.add_function(wrap_pyfunction!(crc16, module)?)?;
    module.add_function(wrap_pyfunction!(preset_frame, module)?)?;
    module.add_function(wrap_pyfunction!(verify_echo, module)?)?;
    Ok(())
}
