use crate::{Error, Light, Station};
use std::{
    ffi::{c_char, CStr},
    panic::{catch_unwind, AssertUnwindSafe},
    time::Duration,
};

type Status = i32;
const OK: Status = 0;
const INVALID_ARGUMENT: Status = 1;
const OPEN_FAILED: Status = 2;
const IO_FAILED: Status = 3;
const TIMEOUT: Status = 4;
const BAD_RESPONSE: Status = 5;
const DEVICE_EXCEPTION: Status = 6;
const RESET_UNCONFIRMED: Status = 7;
const BUFFER_TOO_SMALL: Status = 8;
const INTERNAL: Status = 9;

fn ffi(operation: impl FnOnce() -> Result<(), Status>) -> Status {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(())) => OK,
        Ok(Err(status)) => status,
        Err(_) => INTERNAL,
    }
}

fn status(error: Error) -> Status {
    match error {
        Error::InvalidStation | Error::InvalidScene | Error::InvalidConfig | Error::InvalidHold => {
            INVALID_ARGUMENT
        }
        Error::Open(_) => OPEN_FAILED,
        Error::Prepare(_) | Error::Exchange(_) => IO_FAILED,
        Error::Timeout => TIMEOUT,
        Error::Response(_) => BAD_RESPONSE,
        Error::DeviceException(_) => DEVICE_EXCEPTION,
        Error::ResetUnconfirmed { .. } => RESET_UNCONFIRMED,
    }
}

unsafe fn text<'a>(value: *const c_char) -> Result<&'a str, Status> {
    if value.is_null() {
        return Err(INVALID_ARGUMENT);
    }
    // 调用方保证字符串在本次调用期间有效且以 NUL 结尾。
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .map_err(|_| INVALID_ARGUMENT)
}

#[no_mangle]
pub extern "C" fn rs485_light_abi_version() -> u32 {
    1
}

#[no_mangle]
pub extern "C" fn rs485_light_status_message(code: Status) -> *const c_char {
    match code {
        OK => c"success",
        INVALID_ARGUMENT => c"invalid argument",
        OPEN_FAILED => c"serial open failed; inspect port availability and access",
        IO_FAILED => c"serial I/O failed; write may already have taken effect",
        TIMEOUT => c"response timeout; write may already have taken effect",
        BAD_RESPONSE => c"invalid or mismatched response; write may already have taken effect",
        DEVICE_EXCEPTION => c"device rejected request",
        RESET_UNCONFIRMED => c"preview all-off was not confirmed",
        BUFFER_TOO_SMALL => c"output buffer too small",
        INTERNAL => c"internal failure",
        _ => c"unknown status",
    }
    .as_ptr()
}

/// # Safety
/// `required` must be writable and any supplied buffer must be valid for `capacity` bytes.
#[no_mangle]
pub unsafe extern "C" fn rs485_light_list_ports(
    buffer: *mut c_char,
    capacity: usize,
    required: *mut usize,
) -> Status {
    ffi(|| {
        if required.is_null() || (buffer.is_null() && capacity != 0) {
            return Err(INVALID_ARGUMENT);
        }
        let names = crate::available_ports()
            .map_err(|_| IO_FAILED)?
            .into_iter()
            .map(|port| port.port_name)
            .collect::<Vec<_>>()
            .join("\n");
        let needed = names.len() + 1;
        unsafe { required.write(needed) };
        if buffer.is_null() {
            return Ok(());
        }
        if capacity < needed {
            return Err(BUFFER_TOO_SMALL);
        }
        unsafe {
            std::ptr::copy_nonoverlapping(names.as_ptr(), buffer.cast::<u8>(), names.len());
            buffer.add(names.len()).write(0);
        }
        Ok(())
    })
}

/// # Safety
/// `hex` must be a valid C string and `value` must be writable.
#[no_mangle]
pub unsafe extern "C" fn rs485_light_parse_station(hex: *const c_char, value: *mut u8) -> Status {
    ffi(|| {
        if value.is_null() {
            return Err(INVALID_ARGUMENT);
        }
        let station: Station = unsafe { text(hex) }?.parse().map_err(status)?;
        unsafe { value.write(station.value()) };
        Ok(())
    })
}

/// # Safety
/// `port` must be a valid C string and `handle` must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn rs485_light_open(
    port: *const c_char,
    station: u8,
    baud: u32,
    timeout_ms: u32,
    handle: *mut *mut Light,
) -> Status {
    ffi(|| {
        if handle.is_null() {
            return Err(INVALID_ARGUMENT);
        }
        unsafe { handle.write(std::ptr::null_mut()) };
        let port = unsafe { text(port) }?;
        let station = Station::new(station).map_err(status)?;
        let light = Light::open(
            port,
            baud,
            station,
            Duration::from_millis(timeout_ms.into()),
        )
        .map_err(status)?;
        unsafe { handle.write(Box::into_raw(Box::new(light))) };
        Ok(())
    })
}

/// # Safety
/// Non-null handles must be live, returned by open, and used exclusively by this call.
/// `word` must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn rs485_light_check(handle: *mut Light, word: *mut u16) -> Status {
    ffi(|| {
        if word.is_null() {
            return Err(INVALID_ARGUMENT);
        }
        let light = unsafe { handle.as_mut() }.ok_or(INVALID_ARGUMENT)?;
        let raw = light.verify_connection().map_err(status)?;
        unsafe { word.write(raw) };
        Ok(())
    })
}

/// # Safety
/// Non-null handles must be live and used exclusively by this call. `scene` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn rs485_light_apply(handle: *mut Light, scene: *const c_char) -> Status {
    ffi(|| {
        let light = unsafe { handle.as_mut() }.ok_or(INVALID_ARGUMENT)?;
        let scene = unsafe { text(scene) }?.parse().map_err(status)?;
        light.apply(scene).map_err(status)
    })
}

/// # Safety
/// Non-null handles must be live and used exclusively by this call. `scene` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn rs485_light_preview(
    handle: *mut Light,
    scene: *const c_char,
    hold_ms: u32,
) -> Status {
    ffi(|| {
        let light = unsafe { handle.as_mut() }.ok_or(INVALID_ARGUMENT)?;
        let scene = unsafe { text(scene) }?.parse().map_err(status)?;
        light
            .preview(scene, Duration::from_millis(hold_ms.into()))
            .map_err(status)
    })
}

/// # Safety
/// Non-null handles must be live, returned by open, and used exclusively by this call.
/// The caller must discard the handle after closing.
#[no_mangle]
pub unsafe extern "C" fn rs485_light_close(handle: *mut Light) -> Status {
    ffi(|| {
        if !handle.is_null() {
            unsafe { drop(Box::from_raw(handle)) };
        }
        Ok(())
    })
}
