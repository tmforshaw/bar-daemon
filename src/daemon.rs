use std::path::Path;

use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
};

use crate::{
    battery::{Battery, BatteryItem},
    bluetooth::{Bluetooth, BluetoothItem},
    brightness::{Brightness, BrightnessItem},
    error::DaemonError,
    ram::{Ram, RamItem},
    volume::{Volume, VolumeItem},
};

// pub const IP_AND_PORT: &str = "127.0.0.1:6969";
pub const SOCKET_PATH: &str = "/tmp/bar_daemon.sock";
pub const BUFFER_SIZE: usize = 1024;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DaemonMessage {
    Set { item: DaemonItem, value: String },
    Get { item: DaemonItem },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DaemonReply {
    Value {
        item: DaemonItem,
        value: String,
    },
    Tuples {
        item: DaemonItem,
        tuples: Vec<(String, String)>,
    },
    Error(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DaemonItem {
    Volume(VolumeItem),
    Brightness(BrightnessItem),
    Bluetooth(BluetoothItem),
    Battery(BatteryItem),
    Ram(RamItem),
}

pub async fn do_daemon() -> Result<(), DaemonError> {
    // let listener = TcpListener::bind(IP_AND_PORT).await?;

    // Remove existing socket file
    if Path::new(SOCKET_PATH).exists() {
        std::fs::remove_file(SOCKET_PATH)?;
    }

    // Create new UnixListener at SOCKET_PATH
    let listener = UnixListener::bind(SOCKET_PATH)?;

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

                let reply = match message {
                    DaemonMessage::Set { item, value } => match_set_command(item.clone(), value.clone()).await.unwrap(),
                    DaemonMessage::Get { item } => match_get_command(item.clone()).await.unwrap(),
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
    let mut stream = UnixStream::connect(SOCKET_PATH).await?;

    // Write the serialized message to the daemon
    stream.write_all(&postcard::to_stdvec(&message)?).await?;

    // Get the response from the daemon
    let mut buf = vec![0u8; BUFFER_SIZE];
    let n = stream.read(&mut buf).await?;

    // Serialize reply into DaemonMessage
    Ok(postcard::from_bytes(&buf[..n])?)
}

// // TODO
// pub async fn shutdown_daemon() {
//     let _ = std::fs::remove_file(SOCKET_PATH);
// }

pub async fn match_set_command(item: DaemonItem, value: String) -> Result<DaemonReply, DaemonError> {
    let message = match item.clone() {
        DaemonItem::Volume(volume_item) => Volume::parse_item(item.clone(), volume_item, Some(value))?,
        DaemonItem::Brightness(brightness_item) => Brightness::parse_item(item.clone(), brightness_item, Some(value))?,
        DaemonItem::Bluetooth(bluetooth_item) => Bluetooth::parse_item(item.clone(), bluetooth_item, Some(value))?,
        _ => DaemonReply::Value { item, value },
    };

    Ok(message)
}

pub async fn match_get_command(item: DaemonItem) -> Result<DaemonReply, DaemonError> {
    let message = match item.clone() {
        DaemonItem::Volume(volume_item) => Volume::parse_item(item.clone(), volume_item, None)?,
        DaemonItem::Brightness(brightness_item) => Brightness::parse_item(item.clone(), brightness_item, None)?,
        DaemonItem::Bluetooth(bluetooth_item) => Bluetooth::parse_item(item.clone(), bluetooth_item, None)?,
        DaemonItem::Battery(battery_item) => Battery::parse_item(item.clone(), battery_item)?,
        DaemonItem::Ram(ram_item) => Ram::parse_item(item.clone(), ram_item)?,
    };

    Ok(message)
}
