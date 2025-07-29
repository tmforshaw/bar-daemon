use std::{collections::HashMap, path::Path, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
    sync::Mutex,
};
use uuid::Uuid;

use crate::{
    battery::{Battery, BatteryItem},
    bluetooth::{Bluetooth, BluetoothItem},
    brightness::{Brightness, BrightnessItem},
    error::DaemonError,
    listener::{handle_clients, Client, SharedClients},
    ram::{Ram, RamItem},
    volume::{Volume, VolumeItem},
};

pub const SOCKET_PATH: &str = "/tmp/bar_daemon.sock";
pub const BUFFER_SIZE: usize = 1024;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DaemonMessage {
    Set { item: DaemonItem, value: String },
    Get { item: DaemonItem },
    Listen,
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
    AllTuples {
        tuples: Vec<(String, Vec<(String, String)>)>,
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
    All,
}

/// # Errors
/// Returns an error if ``SOCKET_PATH`` cannot be found
/// Returns an error if ``UnixListener`` cannot be bound
/// Returns an error if socket cannot be accepted
pub async fn do_daemon() -> Result<(), DaemonError> {
    // Remove existing socket file
    if Path::new(SOCKET_PATH).exists() {
        std::fs::remove_file(SOCKET_PATH)?;
    }

    // Create new UnixListener at SOCKET_PATH
    let listener = UnixListener::bind(SOCKET_PATH)?;

    // Remember listener clients to broadcast to
    let clients: SharedClients = Arc::new(Mutex::new(HashMap::new()));

    // Spawn a task which handles listener clients
    let clients_clone = clients.clone();
    tokio::spawn(async move { handle_clients(clients_clone).await });

    // Handle sockets
    loop {
        let (stream, _) = listener.accept().await?;

        // Spawn a task which handles this socket
        let clients_clone = clients.clone();
        tokio::spawn(async move { handle_socket(stream, clients_clone).await });
    }
}

/// # Errors
/// Returns an error if socket cannot be read
/// Returns an error if ``DaemonMessage`` could not be created from bytes
/// Returns an error if requested value cannot be found or parsed
/// Returns an error if socket could not be wrote to
pub async fn handle_socket(mut stream: UnixStream, clients: SharedClients) -> Result<(), DaemonError> {
    let mut buf = [0; BUFFER_SIZE];
    loop {
        let n = match stream.read(&mut buf).await? {
            // Stream closed
            0 => break,
            n => n,
        };

        let message: DaemonMessage = postcard::from_bytes(&buf[..n])?;

        let reply = match message {
            DaemonMessage::Set { item, value } => match_set_command(item.clone(), value.clone())?,
            DaemonMessage::Get { item } => match_get_command(item.clone()).await?,
            DaemonMessage::Listen => {
                // Add the client writer and their uuid to clients
                let client_id = Uuid::new_v4();
                clients.lock().await.insert(client_id, Client { id: client_id, stream });

                break;
            }
        };

        // Send the reply back
        stream.write_all(&postcard::to_stdvec(&reply)?).await?;
    }

    Ok(())
}

/// # Errors
/// Returns an error if ``SOCKET_PATH`` cannot be found
/// Returns an error if socket cannot be read
/// Returns an error if socket could not be wrote to
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

/// # Errors
/// Returns an error if the requested value could not be parsed
pub fn match_set_command(item: DaemonItem, value: String) -> Result<DaemonReply, DaemonError> {
    let message = match item.clone() {
        DaemonItem::Volume(volume_item) => Volume::parse_item(item, &volume_item, Some(value))?,
        DaemonItem::Brightness(brightness_item) => Brightness::parse_item(item, &brightness_item, Some(value))?,
        DaemonItem::Bluetooth(bluetooth_item) => Bluetooth::parse_item(item, &bluetooth_item, Some(value))?,
        _ => DaemonReply::Value { item, value },
    };

    Ok(message)
}

/// # Errors
/// Returns an error if the requested value could not be parsed
pub async fn match_get_command(item: DaemonItem) -> Result<DaemonReply, DaemonError> {
    let message = match item.clone() {
        DaemonItem::Volume(volume_item) => Volume::parse_item(item.clone(), &volume_item, None)?,
        DaemonItem::Brightness(brightness_item) => Brightness::parse_item(item.clone(), &brightness_item, None)?,
        DaemonItem::Bluetooth(bluetooth_item) => Bluetooth::parse_item(item.clone(), &bluetooth_item, None)?,
        DaemonItem::Battery(battery_item) => Battery::parse_item(item.clone(), &battery_item)?,
        DaemonItem::Ram(ram_item) => Ram::parse_item(item.clone(), &ram_item)?,
        DaemonItem::All => DaemonReply::AllTuples {
            tuples: get_all_tuples().await?,
        },
    };

    Ok(message)
}

/// # Errors
/// Returns an error if the requested value could not be parsed
pub async fn get_all_tuples() -> Result<Vec<(String, Vec<(String, String)>)>, DaemonError> {
    Ok(vec![
        ("volume".to_string(), Volume::get_tuples()?),
        ("brightness".to_string(), Brightness::get_tuples()?),
        ("bluetooth".to_string(), Bluetooth::get_tuples()?),
        ("battery".to_string(), Battery::get_tuples()?),
        ("ram".to_string(), Ram::get_tuples()?),
    ])
}
