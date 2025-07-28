use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::error::DaemonError;

pub const IP_AND_PORT: &str = "127.0.0.1:6969";
pub const BUFFER_SIZE: usize = 1024;

#[derive(Serialize, Deserialize, Debug)]
pub enum DaemonMessage {
    Set { item: DaemonItem, value: String },
    Get { item: DaemonItem },
    Reply { item: DaemonItem, value: String },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum DaemonItem {
    Volume,
}

pub async fn do_daemon() -> Result<(), DaemonError> {
    let listener = TcpListener::bind(IP_AND_PORT).await?;

    loop {
        let (mut stream, _) = listener.accept().await?;

        // Spawn a task which handles this stream
        tokio::spawn(async move {
            let mut buf = [0; BUFFER_SIZE];
            loop {
                let n = match stream.read(&mut buf).await {
                    // Stream closed
                    Ok(0) => return,
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("Failed to read from stream:\n\t{e:?}");
                        return;
                    }
                };

                let message: DaemonMessage = postcard::from_bytes(&buf[..n]).unwrap();
                println!("{message:?}");

                let reply = match message {
                    DaemonMessage::Set { item, value } => {
                        // TODO Set the item
                        DaemonMessage::Reply { item, value }
                    }
                    DaemonMessage::Get { item } => {
                        // TODO Get the value of this item
                        DaemonMessage::Reply {
                            item,
                            value: "This is the value of your gotten thing".to_string(),
                        }
                    }
                    DaemonMessage::Reply { item: _, value: _ } => {
                        // Should not reach this
                        unreachable!()
                    }
                };

                // Send the reply back
                if let Err(e) = stream.write_all(&postcard::to_stdvec(&reply).unwrap()).await {
                    eprintln!("Failed to write to stream:\n\t{e:?}");
                    return;
                }
            }
        });
    }
}

pub async fn send_daemon_messaage(message: DaemonMessage) -> Result<DaemonMessage, DaemonError> {
    // Connect to the daemon
    let mut stream = TcpStream::connect(IP_AND_PORT).await?;

    // Write the serialized message to the daemon
    stream.write_all(&postcard::to_stdvec(&message)?).await?;

    // Get the response from the daemon
    let mut buf = vec![0u8; BUFFER_SIZE];
    let n = stream.read(&mut buf).await?;

    // Serialize reply into DaemonMessage
    Ok(postcard::from_bytes(&buf[..n])?)
}
