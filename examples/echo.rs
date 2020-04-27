use std::env;
use std::path::Path;
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use serialport::posix::TTYPort;
use serialport::{SerialPortSettings, FlowControl, Parity, DataBits, StopBits};
use rs485::SerialRs485;
use hex_slice::AsHex;

const NAME: &'static str = env!("CARGO_PKG_NAME");
const DEFAULT_SERIALPORT: &'static str  = "/dev/ttyO4";

fn help() {
    println!("usage: ./{} [serial-port]\n for example : ./{} {}\n \
    Serial port settings: 115200 8N1", NAME, NAME, DEFAULT_SERIALPORT);
    std::process::exit(0);
}

fn main() {
    println!("RS 485 Echo Example");
    let args: Vec<String> = env::args().collect();

    let mut port_name = String::new();
    match args.len() {
        2 => {
            port_name = args[1].to_string();
        },
        _ => { help(); }
    };

    let mut settings: SerialPortSettings = Default::default();
    settings.baud_rate = 115200;
    settings.flow_control = FlowControl::None;
    settings.parity = Parity::None;
    settings.data_bits = DataBits::Eight;
    settings.stop_bits = StopBits::One;

    let path = Path::new(&port_name);

    let mut port;
    match TTYPort::open(&path, &settings)
    {
        Ok(r) => {
            port = r;
            println!("Successfully Opened : {}", port_name);
        },
        Err(_) => {
            println!("Error. Failed connect to open serial port : {}", port_name.to_string());
            std::process::exit(1);
        }
    }

    let fd = port.as_raw_fd();
    println!("fd : {}", fd);

    let mut rs485ctl = SerialRs485::default();

    rs485ctl.set_rts_on_send(false);
    rs485ctl.set_rts_after_send(true);

    rs485ctl.set_on_fd(fd).unwrap();

    println!("Press CTRL+C to exit");

    let mut serial_data = [0 as u8; 32];

    loop
    {
        match port.read(&mut serial_data)
        {
            Ok(size) => {
                println!("Read bytes : {:02X} text : {:?}", serial_data[..size].as_hex(), String::from_utf8_lossy(&serial_data[..size]));
                if size != 0 {
                    let wr = port.write(&serial_data[..size]).expect("Failed to write to serial port");
                    println!("Write result : {:?}", wr);
                }
            },
            Err(e) => {
                if e.kind() == std::io::ErrorKind::TimedOut { continue; }
                println!("Error. Failed read. serial-port : {}. Error : {}", port_name.to_string(), e);
            },
        };
    }
}

