use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::error::DaemonError;

pub const IP_AND_PORT: &str = "127.0.0.1:6969";
pub const BUFFER_SIZE: usize = 1024;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DaemonMessage {
    Set { item: DaemonItem, value: String },
    Get { item: DaemonItem },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DaemonReply {
    Value { item: DaemonItem, value: String },
    Error(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
                    DaemonMessage::Set { item, value } => match_set_command(item.clone(), value.clone()).await,
                    DaemonMessage::Get { item } => match_get_command(item.clone()).await,
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

pub async fn send_daemon_messaage(message: DaemonMessage) -> Result<DaemonReply, DaemonError> {
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

pub async fn match_set_command(item: DaemonItem, value: String) -> DaemonReply {
    match item {
        DaemonItem::Volume => println!("Set Volume {value}"),
    }

    // TODO
    DaemonReply::Value { item, value }
}

pub async fn match_get_command(item: DaemonItem) -> DaemonReply {
    match item {
        DaemonItem::Volume => println!("Get Volume"),
    }

    // TODO
    DaemonReply::Value {
        item,
        value: "The value you have gotten".to_string(),
    }
}
