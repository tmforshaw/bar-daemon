use crate::command::ServerError;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub async fn start(args: &[String]) -> Result<(), Arc<ServerError>> {
    let mut stream = match TcpStream::connect("127.0.0.1:8080").await {
        Ok(stream) => stream,
        Err(e) => return Err(Arc::from(ServerError::SocketConnect { e })),
    };

    if args.is_empty() {
        Err(Arc::from(ServerError::EmptyArguments))
    } else {
        if let Err(e) = stream.write_all(args.join(" ").as_bytes()).await {
            return Err(Arc::from(ServerError::SocketWrite { e }));
        }

        let mut buf = [0; 1024];

        let n = match stream.read(&mut buf).await {
            Ok(n) if n == 0 => std::process::exit(0x1000),
            Ok(n) => n,
            Err(e) => return Err(Arc::from(ServerError::SocketRead { e })),
        };

        let message = match String::from_utf8(Vec::from(&buf[0..n])) {
            Ok(string) => string,
            Err(e) => {
                return Err(Arc::from(ServerError::StringConversion {
                    debug_string: format!("{:?}", &buf[0..n]),
                    e,
                }))
            }
        };

        println!("{message}");

        Ok(())
    }
}
