use crate::command::{call_and_retry_async, ServerError};
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub async fn start() -> Result<(), Arc<ServerError>> {
    let mut stream =
        match call_and_retry_async(|| async { TcpStream::connect(crate::IP_AND_PORT).await }).await
        {
            Some(Ok(stream)) => stream,
            Some(Err(e)) => return Err(Arc::from(ServerError::AddressInUse { e })),
            None => return Err(Arc::from(ServerError::RetryError)),
        };

    if let Err(e) = stream.write_all(b"listen").await {
        return Err(Arc::from(ServerError::SocketWrite { e }));
    }

    loop {
        let mut buf = [0; 1024];

        let n = match stream.read(&mut buf).await {
            Ok(n) if n == 0 => return Ok(()),
            Ok(n) => n,
            Err(e) => {
                return Err(Arc::from(ServerError::SocketRead { e }));
            }
        };

        let message = match String::from_utf8(Vec::from(&buf[0..n])) {
            Ok(string) => string,
            Err(e) => {
                return Err(Arc::from(ServerError::StringConversion {
                    debug_string: format!("{:?}", &buf[0..n]),
                    e,
                }));
            }
        };

        println!("{message}");
    }
}
