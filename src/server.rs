use crate::battery;
use crate::brightness;
use crate::error;
use crate::memory;
use crate::volume;

use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

pub async fn start() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    tokio::spawn(async move {
        loop {
            println!("{}", get_all_json());

            std::thread::sleep(Duration::from_millis(1500));
        }
    });

    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = [0; 1024];

            // In a loop, read data from the socket and write the data back.
            loop {
                let n = match socket.read(&mut buf).await {
                    // socket closed
                    Ok(n) if n == 0 => return,
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("failed to read from socket; err = {:?}", e);
                        return;
                    }
                };

                let message = match String::from_utf8(Vec::from(&buf[0..n])) {
                    Ok(string) => string,
                    Err(e) => error!("Could not parse incoming string: {e}"),
                };

                match message.as_str() {
                    "volume" => println!("{}", volume::get_json()),
                    "brightness" => println!("{}", brightness::get_json()),
                    "battery" => println!("{}", battery::get_json()),
                    "memory" => println!("{}", memory::get_json()),
                    "all" => println!("{}", get_all_json()),
                    _ => {}
                };
            }
        });
    }
}

fn get_all_json() -> String {
    format!(
        "{{\"volume\": {}, \"brightness\": {}, \"battery\": {}, \"memory\": {}}}",
        volume::get_json(),
        brightness::get_json(),
        battery::get_json(),
        memory::get_json()
    )
}
