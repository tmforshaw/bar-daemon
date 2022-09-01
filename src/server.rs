use crate::battery::Battery;
use crate::brightness::Brightness;
use crate::error;
use crate::memory::Memory;
use crate::volume::Volume;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub async fn start() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    tokio::spawn(async move {
        use std::time::Duration;

        loop {
            println!("{}", get_all_json());

            std::thread::sleep(Duration::from_millis(1500));
        }
    });

    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = [0; 1024];

            loop {
                let n = match socket.read(&mut buf).await {
                    // socket closed
                    Ok(n) if n == 0 => return,
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("Failed to read from socket: {e}");
                        return;
                    }
                };

                let message = match String::from_utf8(Vec::from(&buf[0..n])) {
                    Ok(string) => string,
                    Err(e) => error!("Could not parse incoming string: {e}"),
                };

                let args: Vec<&str> = message.split_whitespace().collect();

                let parseable_args = &args[1..args.len()];

                let reply = match args.first() {
                    Some(argument) => match *argument {
                        "volume" | "vol" => Some(Volume::parse_args(parseable_args)),
                        "brightness" | "bri" => Some(Brightness::parse_args(parseable_args)),
                        "battery" | "bat" => Some(Battery::parse_args(parseable_args)),
                        "memory" | "mem" => Some(Memory::parse_args(parseable_args)),
                        "update" => {
                            println!("{}", get_all_json());
                            None
                        }
                        incorrect => Some(format!("'{incorrect}' is not a valid argument")),
                    },
                    None => Some("Please enter an argument to get".to_string()),
                };

                if let Some(r) = reply {
                    if let Some(e) = socket.write_all(r.as_bytes()).await.err() {
                        eprintln!("Could not send to socket: {e}");
                    }
                }
            }
        });
    }
}

fn get_all_json() -> String {
    format!(
        "{{\"volume\": {}, \"brightness\": {}, \"battery\": {}, \"memory\": {}}}",
        Volume::get_json(),
        Brightness::get_json(),
        Battery::get_json(),
        Memory::get_json()
    )
}
