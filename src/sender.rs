use crate::error;

use std::error::Error;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

pub async fn start(args: &Vec<String>) -> Result<(), Box<dyn Error>> {
    // Connect to a peer
    let mut stream = TcpStream::connect("127.0.0.1:8080").await?;

    if args.is_empty() {
        error!("No input to send");
    } else {
        // Write some data.
        stream.write_all(args.join(" ").as_bytes()).await?;

        Ok(())
    }
}
