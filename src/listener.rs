use std::{collections::HashMap, path::Path, sync::Arc, time::Duration};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    sync::Mutex,
};
use uuid::Uuid;

use crate::{
    daemon::{get_all_tuples, DaemonMessage, SOCKET_PATH},
    error::DaemonError,
};

pub struct Client {
    pub id: Uuid,
    pub stream: UnixStream,
}

/// # Errors
/// Returns an error if ``SOCKET_PATH`` cannot be found
/// Returns an error if ``UnixListener`` cannot be bound
/// Returns an error if ``DaemonMessage`` could not be created from bytes
/// Returns an error if socket cannot be read
/// Returns an error if socket could not be wrote to
pub async fn listen() -> Result<(), DaemonError> {
    if !Path::new(SOCKET_PATH).exists() {
        eprintln!("Socket not found. Is the daemon running?");
        return Ok(());
    }

    let mut stream = UnixStream::connect(SOCKET_PATH).await?;

    // Tell the daemon that this client wants to listen
    stream.write_all(&postcard::to_stdvec(&DaemonMessage::Listen)?).await?;

    // Read the lines which the client sends
    let reader = BufReader::new(stream);
    let mut lines = reader.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        println!("{line}");
    }

    Ok(())
}

pub type SharedClients = Arc<Mutex<HashMap<Uuid, Client>>>;

/// # Errors
/// Returns an error if ``SOCKET_PATH`` cannot be found
/// Returns an error if ``UnixListener`` cannot be bound
/// Returns an error if ``DaemonMessage`` could not be created from bytes
/// Returns an error if socket cannot be read
/// Returns an error if socket could not be wrote to
pub async fn handle_clients(clients: SharedClients) -> Result<(), DaemonError> {
    loop {
        let clients_empty = clients.lock().await.is_empty();

        if !clients_empty {
            let mut to_remove = vec![];

            // Get the tuples for all values
            let tuples = get_all_tuples().await?;

            // Convert tuples nested hashmap
            let mut json_map: HashMap<String, HashMap<String, String>> = HashMap::new();

            for (group, pairs) in tuples {
                let inner_map = pairs.into_iter().collect::<HashMap<_, _>>();

                json_map.insert(group, inner_map);
            }

            let json = serde_json::to_string(&json_map)? + "\n";

            // Broadcast to each client
            for (id, client) in clients.lock().await.iter_mut() {
                if let Err(e) = client.stream.try_write(json.as_bytes()) {
                    eprintln!("Write failed for {id}: {e}");
                    to_remove.push(*id);
                }
            }

            // Remove dead clients
            for id in to_remove {
                clients.lock().await.remove(&id);
                println!("Client {id} removed");
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
