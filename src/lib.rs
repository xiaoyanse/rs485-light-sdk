//! Modbus RTU control for signal lights with the C2 preset register.

use std::{
    fmt,
    io::{self, Read, Write},
    str::FromStr,
    thread,
    time::{Duration, Instant},
};

pub use serialport::{available_ports, SerialPortInfo, SerialPortType};

#[cfg(feature = "python")]
mod python;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Station(u8);

impl Station {
    pub const FACTORY: Self = Self(0xff);

    pub fn new(value: u8) -> Result<Self, Error> {
        match value {
            1..=247 | 255 => Ok(Self(value)),
            _ => Err(Error::InvalidStation),
        }
    }

    pub const fn value(self) -> u8 {
        self.0
    }
}

impl FromStr for Station {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let input = input.trim();
        if !(1..=2).contains(&input.len()) || !input.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(Error::InvalidStation);
        }
        Self::new(u8::from_str_radix(input, 16).map_err(|_| Error::InvalidStation)?)
    }
}

impl fmt::Display for Station {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02X}", self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Color {
    Red,
    Yellow,
    Green,
    White,
    Cyan,
    Purple,
    Blue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Pattern {
    Steady,
    Slow,
    Fast,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Scene {
    Light(Color, Pattern),
    RedWithBuzzer(Pattern),
    BuzzerOn,
    BuzzerOff,
    AllOff,
}

impl Scene {
    pub const fn command(self) -> u8 {
        match self {
            Self::Light(color, pattern) => {
                (match color {
                    Color::Red => 1,
                    Color::Yellow => 2,
                    Color::Green => 3,
                    Color::White => 5,
                    Color::Cyan => 6,
                    Color::Purple => 7,
                    Color::Blue => 8,
                }) | pattern.command_base()
            }
            Self::RedWithBuzzer(pattern) => pattern.command_base() | 4,
            Self::BuzzerOn => 0x40,
            Self::BuzzerOff => 0x41,
            Self::AllOff => 0x61,
        }
    }
}

impl FromStr for Scene {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "off" => return Ok(Self::AllOff),
            "buzzer-on" => return Ok(Self::BuzzerOn),
            "buzzer-off" => return Ok(Self::BuzzerOff),
            _ => {}
        }
        let (color, pattern) = input.split_once('-').ok_or(Error::InvalidScene)?;
        let pattern = match pattern {
            "steady" => Pattern::Steady,
            "slow" => Pattern::Slow,
            "fast" => Pattern::Fast,
            _ => return Err(Error::InvalidScene),
        };
        let color = match color {
            "red" => Color::Red,
            "yellow" => Color::Yellow,
            "green" => Color::Green,
            "white" => Color::White,
            "cyan" => Color::Cyan,
            "purple" => Color::Purple,
            "blue" => Color::Blue,
            "alarm" => return Ok(Self::RedWithBuzzer(pattern)),
            _ => return Err(Error::InvalidScene),
        };
        Ok(Self::Light(color, pattern))
    }
}

impl Pattern {
    const fn command_base(self) -> u8 {
        match self {
            Self::Steady => 0x10,
            Self::Slow => 0x20,
            Self::Fast => 0x30,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("expected a hexadecimal station in 01–F7 or FF")]
    InvalidStation,
    #[error("unknown scene")]
    InvalidScene,
    #[error("baud rate must be positive and timeout must be between 1 ms and 30 s")]
    InvalidConfig,
    #[error("preview hold must be greater than zero and at most five seconds")]
    InvalidHold,
    #[error("could not open serial port: {0}")]
    Open(#[source] serialport::Error),
    #[error("could not prepare serial transaction: {0}")]
    Prepare(#[source] serialport::Error),
    #[error("serial request or response failed; a write may already have taken effect: {0}")]
    Exchange(#[source] io::Error),
    #[error("no complete response before deadline; a write may already have taken effect")]
    Timeout,
    #[error("invalid response: {0}")]
    Response(&'static str),
    #[error("device rejected request with Modbus exception 0x{0:02X}")]
    DeviceException(u8),
    #[error("preview reset could not be confirmed: {reset}; initial command: {apply:?}")]
    ResetUnconfirmed {
        apply: Option<Box<Error>>,
        reset: Box<Error>,
    },
}

impl Error {
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::InvalidStation => "invalid_station",
            Self::InvalidScene => "invalid_scene",
            Self::InvalidConfig => "invalid_config",
            Self::InvalidHold => "invalid_hold",
            Self::Open(_) => "open_failed",
            Self::Prepare(_) => "prepare_failed",
            Self::Exchange(_) => "exchange_failed",
            Self::Timeout => "timeout",
            Self::Response(_) => "invalid_response",
            Self::DeviceException(_) => "device_exception",
            Self::ResetUnconfirmed { .. } => "reset_unconfirmed",
        }
    }
}

/// Holds one exclusive 8N1 serial connection. Mutable operations serialize requests.
pub struct Light {
    port: Box<dyn serialport::SerialPort>,
    station: Station,
    timeout: Duration,
}

impl Light {
    pub fn open(port: &str, baud: u32, station: Station, timeout: Duration) -> Result<Self, Error> {
        if baud == 0 || !(Duration::from_millis(1)..=Duration::from_secs(30)).contains(&timeout) {
            return Err(Error::InvalidConfig);
        }
        let port = serialport::new(port, baud)
            .data_bits(serialport::DataBits::Eight)
            .parity(serialport::Parity::None)
            .stop_bits(serialport::StopBits::One)
            .flow_control(serialport::FlowControl::None)
            .timeout(timeout)
            .open()
            .map_err(Error::Open)?;
        Ok(Self {
            port,
            station,
            timeout,
        })
    }

    /// Reads one word from register 00FF. The raw word is firmware-dependent.
    pub fn verify_connection(&mut self) -> Result<u16, Error> {
        let request = frame(self.station, 0x03, 0x00ff, 1);
        let response = self.exchange(&request, 7)?;
        station_word(&response)
    }

    /// Sends a persistent scene command and verifies its full echo.
    pub fn apply(&mut self, scene: Scene) -> Result<(), Error> {
        let request = frame(self.station, 0x06, 0x00c2, u16::from(scene.command()));
        let response = self.exchange(&request, 8)?;
        verify_echo(&request, &response)
    }

    /// Bench operation. Attempts all-off even when the first acknowledgement is uncertain.
    pub fn preview(&mut self, scene: Scene, hold: Duration) -> Result<(), Error> {
        preview_with(scene, hold, |scene| self.apply(scene), thread::sleep)
    }

    fn exchange(&mut self, request: &[u8; 8], response_len: usize) -> Result<Vec<u8>, Error> {
        let start = Instant::now();
        self.port
            .set_timeout(self.timeout)
            .map_err(Error::Prepare)?;
        self.port
            .clear(serialport::ClearBuffer::Input)
            .map_err(Error::Prepare)?;
        self.port.write_all(request).map_err(Error::Exchange)?;
        let deadline = start + self.timeout;
        let mut response = [0; 8];
        self.read_until(&mut response[..2], deadline)?;
        if response[0] != request[0] {
            return Err(Error::Response("station mismatch"));
        }
        let length = if response[1] == (request[1] | 0x80) {
            5
        } else if response[1] == request[1] {
            response_len
        } else {
            return Err(Error::Response("function mismatch"));
        };
        self.read_until(&mut response[2..length], deadline)?;
        validate_crc(&response[..length])?;
        if length == 5 {
            return Err(Error::DeviceException(response[2]));
        }
        Ok(response[..length].to_vec())
    }

    fn read_until(&mut self, mut bytes: &mut [u8], deadline: Instant) -> Result<(), Error> {
        while !bytes.is_empty() {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining < Duration::from_millis(1) {
                return Err(Error::Timeout);
            }
            self.port.set_timeout(remaining).map_err(Error::Prepare)?;
            match self.port.read(bytes) {
                Ok(0) => return Err(Error::Timeout),
                Ok(count) => bytes = &mut bytes[count..],
                Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Err(err) if err.kind() == io::ErrorKind::TimedOut => return Err(Error::Timeout),
                Err(err) => return Err(Error::Exchange(err)),
            }
        }
        Ok(())
    }
}

fn preview_with(
    scene: Scene,
    hold: Duration,
    mut apply: impl FnMut(Scene) -> Result<(), Error>,
    wait: impl FnOnce(Duration),
) -> Result<(), Error> {
    if hold.is_zero() || hold > Duration::from_secs(5) {
        return Err(Error::InvalidHold);
    }
    let applied = apply(scene);
    if applied.is_ok() {
        wait(hold);
    }
    match apply(Scene::AllOff) {
        Ok(()) => applied,
        Err(reset) => Err(Error::ResetUnconfirmed {
            apply: applied.err().map(Box::new),
            reset: Box::new(reset),
        }),
    }
}

fn frame(station: Station, function: u8, register: u16, value: u16) -> [u8; 8] {
    let [rh, rl] = register.to_be_bytes();
    let [vh, vl] = value.to_be_bytes();
    let mut bytes = [station.value(), function, rh, rl, vh, vl, 0, 0];
    let crc = crc16(&bytes[..6]);
    bytes[6..].copy_from_slice(&crc.to_le_bytes());
    bytes
}

fn crc16(bytes: &[u8]) -> u16 {
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

fn validate_crc(bytes: &[u8]) -> Result<(), Error> {
    let (body, crc) = bytes.split_at(bytes.len() - 2);
    if crc16(body).to_le_bytes() != crc {
        return Err(Error::Response("CRC mismatch"));
    }
    Ok(())
}

fn station_word(response: &[u8]) -> Result<u16, Error> {
    if let [_, _, 2, hi, lo, _, _] = response {
        Ok(u16::from_be_bytes([*hi, *lo]))
    } else {
        Err(Error::Response("expected one register"))
    }
}

fn verify_echo(request: &[u8; 8], response: &[u8]) -> Result<(), Error> {
    if response != request {
        return Err(Error::Response("write echo did not match request"));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
