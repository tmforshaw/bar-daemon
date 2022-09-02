use crate::command::ServerError;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub async fn start(command: &String, args: &Vec<String>) -> Result<(), Arc<ServerError>> {
    // Connect to a peer
    let mut stream = match TcpStream::connect("127.0.0.1:8080").await {
        Ok(stream) => stream,
        Err(e) => return Err(Arc::from(ServerError::SocketConnect { e })),
    };

    if command == "get" {
        if args.is_empty() {
            eprintln!("Nothing to get: please enter arguments");
            Err(Arc::from(ServerError::EmptyArguments))
        } else {
            if let Err(e) = stream.write_all(args.join(" ").as_bytes()).await {
                return Err(Arc::from(ServerError::SocketWrite { e }));
            }

            let mut buf = [0; 1024];

            let n = match stream.read(&mut buf).await {
                Ok(n) if n == 0 => std::process::exit(0x1000),
                Ok(n) => n,
                Err(e) => {
                    eprintln!("Failed to read from socket: {e}");
                    return Err(Arc::from(ServerError::SocketRead { e }));
                }
            };

            let message = match String::from_utf8(Vec::from(&buf[0..n])) {
                Ok(string) => string,
                Err(e) => {
                    eprintln!("Could not parse message to string: {e}");
                    return Err(Arc::from(ServerError::StringConversion {
                        debug_string: format!("{:?}", &buf[0..n]),
                        e,
                    }));
                }
            };

            println!("{message}");

            Ok(())
        }
    } else {
        if let Err(e) = stream.write_all(b"update").await {
            return Err(Arc::from(ServerError::SocketWrite { e }));
        }

        Ok(())
    }
}
