//! Wire primitives for host applications that already manage their own transport.

pub const PRESET_REGISTER: u16 = 0x00c2;
const WRITE_SINGLE: u8 = 0x06;

pub fn crc16(bytes: &[u8]) -> u16 {
    bytes.iter().fold(0xffff, |crc, byte| {
        (0..8).fold(crc ^ u16::from(*byte), |crc, _| {
            if crc & 1 == 1 {
                (crc >> 1) ^ 0xa001
            } else {
                crc >> 1
            }
        })
    })
}

/// Encodes the supplied wire bytes. Validate station and scene policy in the caller.
pub fn preset_frame(station: u8, code: u8) -> [u8; 8] {
    super::frame(station, WRITE_SINGLE, PRESET_REGISTER, u16::from(code))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EchoMismatch {
    TooShort,
    TooLong,
    Crc,
    Station,
    Function,
    Address,
    Value,
}

impl std::fmt::Display for EchoMismatch {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.description())
    }
}

impl std::error::Error for EchoMismatch {}

impl EchoMismatch {
    pub const fn description(self) -> &'static str {
        match self {
            Self::TooShort => "write echo is shorter than eight bytes",
            Self::TooLong => "write echo is longer than eight bytes",
            Self::Crc => "write echo CRC mismatch",
            Self::Station => "write echo station mismatch",
            Self::Function => "write echo function mismatch",
            Self::Address => "write echo register mismatch",
            Self::Value => "write echo value mismatch",
        }
    }
}

/// Validates all eight bytes, including the high byte of the 16-bit register value.
pub fn verify_echo(echo: &[u8], station: u8, code: u8) -> Result<(), EchoMismatch> {
    let frame: &[u8; 8] = echo.try_into().map_err(|_| {
        if echo.len() < 8 {
            EchoMismatch::TooShort
        } else {
            EchoMismatch::TooLong
        }
    })?;
    super::validate_crc(frame).map_err(|_| EchoMismatch::Crc)?;
    if frame[0] != station {
        return Err(EchoMismatch::Station);
    }
    if frame[1] != WRITE_SINGLE {
        return Err(EchoMismatch::Function);
    }
    if u16::from_be_bytes([frame[2], frame[3]]) != PRESET_REGISTER {
        return Err(EchoMismatch::Address);
    }
    if u16::from_be_bytes([frame[4], frame[5]]) != u16::from(code) {
        return Err(EchoMismatch::Value);
    }
    Ok(())
}
