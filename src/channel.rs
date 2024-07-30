use crate::command::{send_or_print_err, ServerError, ServerResult};

use std::sync::Arc;

use tokio::sync::{mpsc, watch, Mutex};

#[derive(Clone)]
pub enum ServerCommand {
    UpdateAll,
    GetVol { args: Vec<String> },
    UpdateVol,
    GetBri { args: Vec<String> },
    UpdateBri,
    GetBat { args: Vec<String> },
    UpdateBat,
    GetMem { args: Vec<String> },
    UpdateMem,
    GetBlu { args: Vec<String> },
    UpdateBlu,
}

impl std::fmt::Display for ServerCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::UpdateAll => "Update server".to_string(),
                Self::GetVol { args } => format!("Get volume: {args:?}"),
                Self::UpdateVol => "Update volume".to_string(),
                Self::GetBri { args } => format!("Get brightness: {args:?}"),
                Self::UpdateBri => "Update brightness".to_string(),
                Self::GetBat { args } => format!("Get battery: {args:?}"),
                Self::UpdateBat => "Update battery".to_string(),
                Self::GetMem { args } => format!("Get memory: {args:?}"),
                Self::UpdateMem => "Update memory".to_string(),
                Self::GetBlu { args } => format!("Get bluetooth: {args:?}"),
                Self::UpdateBlu => "Update bluetooth".to_string(),
            }
        )
    }
}

/// # Errors
// returns an error if Channel could not send message
pub async fn mpsc_send<T>(channel: mpsc::Sender<T>, message: T) -> Result<(), Arc<ServerError>>
where
    T: std::fmt::Display + Clone + Send,
{
    match channel.send(message.clone()).await {
        Ok(()) => Ok(()),
        Err(_) => Err(Arc::from(ServerError::ChannelSend {
            message: message.to_string(),
        })),
    }
}

/// # Errors
/// Returns error if command cannot be sent across channel
pub async fn send_and_await_response(
    command: ServerCommand,
    server_tx: mpsc::Sender<ServerCommand>,
    server_response_rx: Arc<Mutex<watch::Receiver<ServerResult<String>>>>,
    error_tx: mpsc::Sender<Arc<ServerError>>,
) -> ServerResult<String> {
    if let Err(e) = mpsc_send(server_tx, command.clone()).await {
        send_or_print_err(e, &error_tx).await;
    }

    let mut response_lock = server_response_rx.lock().await;

    if response_lock.changed().await.is_ok() {
        let value = response_lock.borrow().clone()?;

        drop(response_lock);

        Ok(value)
    } else {
        Err(Arc::from(ServerError::ChannelSend {
            message: command.to_string(),
        }))
    }
}
