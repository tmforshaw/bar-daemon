use std::{collections::HashMap, path::Path, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
    sync::{mpsc, Mutex},
};
use uuid::Uuid;

use crate::{
    battery::{Battery, BatteryItem},
    bluetooth::{Bluetooth, BluetoothItem},
    brightness::{Brightness, BrightnessItem},
    error::DaemonError,
    fan_profile::{FanProfile, FanProfileItem},
    listener::{handle_clients, Client, ClientMessage, SharedClients},
    ram::{Ram, RamItem},
    tuples::get_all_tuples,
    volume::{Volume, VolumeItem},
    POLLING_RATE,
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
    FanProfile(FanProfileItem),
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

    // Enable back and forth communication from each socket handler and the client handler
    let (clients_tx, mut clients_rx) = mpsc::unbounded_channel::<ClientMessage>();

    // Remember listener clients to broadcast to
    let clients: SharedClients = Arc::new(Mutex::new(HashMap::new()));

    // Spawn a task which handles listener clients
    let clients_clone = clients.clone();
    tokio::spawn(async move { handle_clients(clients_clone, &mut clients_rx).await });

    // Create a task which polls the state of certain values
    let clients_clone = clients.clone();
    let clients_tx_clone = clients_tx.clone();
    tokio::spawn(async move {
        loop {
            let clients_empty = clients_clone.lock().await.is_empty();

            // Only poll the values when there are listener clients
            if !clients_empty {
                clients_tx_clone
                    .send(ClientMessage::UpdateBattery)
                    .unwrap_or_else(|e| eprintln!("{}", Into::<DaemonError>::into(e)));

                clients_tx_clone
                    .send(ClientMessage::UpdateRam)
                    .unwrap_or_else(|e| eprintln!("{}", Into::<DaemonError>::into(e)));
            }

            // Set the polling rate
            tokio::time::sleep(tokio::time::Duration::from_millis(POLLING_RATE)).await;
        }
    });

    // Handle sockets
    loop {
        let (stream, _) = listener.accept().await?;

        let clients_tx_clone = clients_tx.clone();

        // Spawn a task which handles this socket
        let clients_clone = clients.clone();
        tokio::spawn(async move { handle_socket(stream, clients_clone, clients_tx_clone).await });
    }
}

/// # Errors
/// Returns an error if socket cannot be read
/// Returns an error if ``DaemonMessage`` could not be created from bytes
/// Returns an error if requested value cannot be found or parsed
/// Returns an error if socket could not be wrote to
pub async fn handle_socket(
    mut stream: UnixStream,
    clients: SharedClients,
    clients_tx: mpsc::UnboundedSender<ClientMessage>,
) -> Result<(), DaemonError> {
    let mut buf = [0; BUFFER_SIZE];
    loop {
        let n = match stream.read(&mut buf).await? {
            // Stream closed
            0 => break,
            n => n,
        };

        let message: DaemonMessage = postcard::from_bytes(&buf[..n])?;

        let reply = match message {
            DaemonMessage::Set { item, value } => {
                // Broadcast which value has been updated
                clients_tx.send(match item {
                    DaemonItem::Volume(_) => ClientMessage::UpdateVolume,
                    DaemonItem::Brightness(_) => ClientMessage::UpdateBrightness,
                    DaemonItem::Bluetooth(_) => ClientMessage::UpdateBluetooth,
                    DaemonItem::Battery(_) => ClientMessage::UpdateBattery,
                    DaemonItem::Ram(_) => ClientMessage::UpdateRam,
                    DaemonItem::FanProfile(_) => ClientMessage::UpdateFanProfile,
                    DaemonItem::All => ClientMessage::UpdateAll,
                })?;

                match_set_command(item.clone(), value.clone())?
            }
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

/// # Errors
/// Returns an error if the requested value could not be parsed
pub fn match_set_command(item: DaemonItem, value: String) -> Result<DaemonReply, DaemonError> {
    let message = match item.clone() {
        DaemonItem::Volume(volume_item) => Volume::parse_item(item, &volume_item, Some(value))?,
        DaemonItem::Brightness(brightness_item) => Brightness::parse_item(item, &brightness_item, Some(value))?,
        DaemonItem::Bluetooth(bluetooth_item) => Bluetooth::parse_item(item, &bluetooth_item, Some(value))?,
        DaemonItem::FanProfile(fan_profile_item) => FanProfile::parse_item(item, &fan_profile_item, Some(value))?,
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
        DaemonItem::FanProfile(fan_profile_item) => FanProfile::parse_item(item.clone(), &fan_profile_item, None)?,
        DaemonItem::All => DaemonReply::AllTuples {
            tuples: get_all_tuples().await?,
        },
    };

    Ok(message)
}
