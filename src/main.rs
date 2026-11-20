use rs485_light_sdk::{available_ports, Light, Scene};
use std::{env, error::Error, process::ExitCode, time::Duration};

const USAGE: &str = "rs485-light ports
rs485-light check <port> [station-hex] [baud]
rs485-light preview <port> <scene> [station-hex] [baud]
rs485-light off <port> [station-hex] [baud]

Default station FF, baud 9600. Preview holds for 750 ms then sends all-off.
Scenes: <red|yellow|green|white|cyan|purple|blue|alarm>-<steady|slow|fast>,
        buzzer-on, buzzer-off, off";

fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    match args {
        [] => println!("{USAGE}"),
        [arg] if arg == "--help" || arg == "-h" => println!("{USAGE}"),
        [arg] if arg == "ports" => {
            for port in available_ports()? {
                println!("{}\t{:?}", port.port_name, port.port_type);
            }
        }
        [command, port, rest @ ..] if matches!(command.as_str(), "check" | "preview" | "off") => {
            let (scene, connection) = if command == "preview" {
                let (scene, connection) = rest.split_first().ok_or(USAGE)?;
                (Some(scene.parse::<Scene>()?), connection)
            } else {
                (None, rest)
            };
            if connection.len() > 2 {
                return Err(USAGE.into());
            }
            let station = connection
                .first()
                .map(String::as_str)
                .unwrap_or("FF")
                .parse()?;
            let baud = connection
                .get(1)
                .map(String::as_str)
                .unwrap_or("9600")
                .parse()?;
            let mut light = Light::open(port, baud, station, Duration::from_millis(750))?;
            if let Some(scene) = scene {
                light.preview(scene, Duration::from_millis(750))?;
                println!("Scene and all-off acknowledged");
            } else if command == "off" {
                light.apply(Scene::AllOff)?;
                println!("All-off acknowledged");
            } else {
                let raw = light.verify_connection()?;
                println!("Station {station} responded, register 00FF = {raw:04X}");
            }
        }
        _ => return Err(USAGE.into()),
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<_> = env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
