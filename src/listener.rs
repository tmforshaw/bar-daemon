use crate::command::{call_and_retry_async, socket_read, socket_write, ServerError};
use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::sync::Mutex;

pub async fn start() -> Result<(), Arc<ServerError>> {
    let stream =
        match call_and_retry_async(|| async { TcpStream::connect(crate::IP_AND_PORT).await }).await
        {
            Some(Ok(stream)) => Arc::from(Mutex::new(stream)),
            Some(Err(e)) => return Err(Arc::from(ServerError::AddressInUse { e })),
            None => return Err(Arc::from(ServerError::RetryError)),
        };

    socket_write(stream.clone(), b"listen").await?;

    loop {
        let message = socket_read(stream.clone()).await?;
        println!("{message}");
    }
}
