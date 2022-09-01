use crate::error;

use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub async fn start(command: &String, args: &Vec<String>) -> Result<(), Box<dyn Error>> {
    // Connect to a peer
    let mut stream = TcpStream::connect("127.0.0.1:8080").await?;

    if command == "get" {
        if args.is_empty() {
            error!("Nothing to get: please enter arguments");
        } else {
            // Write some data.
            stream.write_all(args.join(" ").as_bytes()).await?;

            let mut buf = [0; 1024];

            let n = match stream.read(&mut buf).await {
                Ok(n) if n == 0 => std::process::exit(0x1000),
                Ok(n) => n,
                Err(e) => {
                    eprintln!("failed to read from socket; err = {:?}", e);
                    std::process::exit(0x1000);
                }
            };

            let message = match String::from_utf8(Vec::from(&buf[0..n])) {
                Ok(string) => string,
                Err(e) => {
                    eprintln!("Could not parse message to string: {e}");
                    std::process::exit(0x1000);
                }
            };

            println!("{message}");

            Ok(())
        }
    } else {
        stream.write_all(b"update").await?;

        Ok(())
    }
}
