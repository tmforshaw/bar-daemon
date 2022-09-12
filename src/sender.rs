use crate::command::{socket_read, socket_write, ServerError};
use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::sync::Mutex;

pub async fn start(args: &[String]) -> Result<(), Arc<ServerError>> {
    let stream = match TcpStream::connect(crate::IP_AND_PORT).await {
        Ok(stream) => Arc::from(Mutex::new(stream)),
        Err(e) => return Err(Arc::from(ServerError::SocketConnect { e })),
    };

    if args.is_empty() {
        Err(Arc::from(ServerError::EmptyArguments))
    } else {
        socket_write(stream.clone(), args.join(" ").as_bytes()).await?;

        let message = socket_read(stream.clone()).await?;

        println!("{message}");

        Ok(())
    }
}
