use std::process::Command;
use std::sync::Arc;

use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Failed to convert '{debug_string}' to string: {e}")]
    StringConversion {
        debug_string: String,
        e: std::string::FromUtf8Error,
    },
    #[error("Failed to run '{command_name} {}': {e}", args.join(" "))]
    Command {
        command_name: String,
        args: Vec<String>,
        e: std::io::Error,
    },
    #[error("Incorrect argument '{incorrect}': enter [{}]", valid.iter()
        .map(|a| {
            let mut string = String::from("'");

            string.push_str(a);
            string.push('\'');

            string
        })
        .collect::<Vec<String>>()
        .join(", "))]
    IncorrectArgument {
        incorrect: String,
        valid: Vec<String>,
    },
    #[error("Failed to read from socket: {e}")]
    SocketRead { e: std::io::Error },
    #[error("Failed to write to socket: {e}")]
    SocketWrite { e: std::io::Error },
    #[error("Please enter some arguments")]
    EmptyArguments,
    #[error("Could not parse '{debug_string}' to {ty}")]
    StringParse {
        debug_string: String,
        ty: String,
        e: Arc<dyn std::error::Error + Send + Sync>,
    },
    #[error("Could not find '{looking_for}' part of output: '{output}'")]
    NotInOutput { looking_for: String, output: String },
    #[error("Socket address already in use: {e}")]
    AddressInUse { e: std::io::Error },
    #[error("Failed to join socket to loop thread: {e}")]
    SocketJoin { e: tokio::task::JoinError },
    #[error("Failed to connect to socket: {e}")]
    SocketConnect { e: std::io::Error },
    #[error("Socket disconnected")]
    SocketDisconnect,
    #[error("Unknown value '{incorrect}' found for '{object}'")]
    UnknownValue { incorrect: String, object: String },
    #[error("Could not retry command correctly")]
    RetryError,
    #[error("Could not send '{message}' across channel")]
    ChannelSend { message: String },
}

/// # Errors
/// Returns an error if the command fails to run
/// Returns an error if the command's output can't be parsed into a string
pub fn run(command_name: &str, args: &[&str]) -> Result<String, ServerError> {
    match Command::new(command_name).args(args).output() {
        Ok(out) => match String::from_utf8(out.clone().stdout) {
            Ok(out_string) => Ok(out_string.trim().to_string()),
            Err(e) => Err(ServerError::StringConversion {
                debug_string: format!("{out:?}"),
                e,
            }),
        },
        Err(e) => Err(ServerError::Command {
            command_name: command_name.to_string(),
            args: args.iter().map(std::string::ToString::to_string).collect(),
            e,
        }),
    }
}

fn get_json_from_tuple(vec_tup: &[(String, String)]) -> String {
    let joined_string = vec_tup
        .iter()
        .map(|t| format!("\"{}\": \"{}\"", t.0, t.1))
        .collect::<Vec<String>>()
        .join(", ");

    format!("{{{}}}", joined_string)
}

/// # Errors
/// Returns an error if any values cannot be found in each mutex
pub async fn get_all_json(
    vol_mutex: Arc<Mutex<Vec<(String, String)>>>,
    bri_mutex: Arc<Mutex<Vec<(String, String)>>>,
    bat_mutex: Arc<Mutex<Vec<(String, String)>>>,
    mem_mutex: Arc<Mutex<Vec<(String, String)>>>,
    blu_mutex: Arc<Mutex<Vec<(String, String)>>>,
) -> Result<String, Box<ServerError>> {
    let volume_tup = vol_mutex.lock().await.clone();
    let brightness_tup = bri_mutex.lock().await.clone();
    let battery_tup = bat_mutex.lock().await.clone();
    let memory_tup = mem_mutex.lock().await.clone();
    let bluetooth_tup = blu_mutex.lock().await.clone();

    Ok(format!(
        "{{\"volume\": {}, \"brightness\": {}, \"battery\": {}, \"memory\": {}, \"bluetooth\": {}}}",
        get_json_from_tuple(&volume_tup),
        get_json_from_tuple(&brightness_tup),
        get_json_from_tuple(&battery_tup),
        get_json_from_tuple(&memory_tup),
        get_json_from_tuple(&bluetooth_tup),
    ))
}

pub fn call_and_retry<O, E>(func: impl Fn() -> Result<O, E>) -> Option<Result<O, E>>
where
    E: std::error::Error,
{
    let mut output = None;

    for count in 0..=crate::RETRY_AMOUNT {
        let match_output = match func() {
            Ok(output) => Ok(output),
            Err(_) if count < crate::RETRY_AMOUNT => {
                eprintln!("Retrying function with type {}", std::any::type_name::<O>(),);
                std::thread::sleep(std::time::Duration::from_millis(crate::RETRY_TIMEOUT));

                continue;
            }
            Err(e) => Err(e),
        };

        output = Some(match_output);
        break;
    }

    output
}

pub async fn call_and_retry_async<O, E, F, Fut>(func: F) -> Option<Result<O, E>>
where
    O: Send + Sync,
    E: std::error::Error + Send + Sync,
    F: Fn() -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<O, E>> + Send + Sync,
{
    let mut output = None;

    for count in 0..=crate::RETRY_AMOUNT {
        let match_output = match func().await {
            Ok(output) => Ok(output),
            Err(_) if count < crate::RETRY_AMOUNT => {
                eprintln!("Retrying function with type {}", std::any::type_name::<O>(),);
                std::thread::sleep(std::time::Duration::from_millis(crate::RETRY_TIMEOUT));

                continue;
            }
            Err(e) => Err(e),
        };

        output = Some(match_output);
        break;
    }

    output
}

/// # Errors
/// Returns error when socket disconnects
/// Returns error when socket cannot be read
/// Returns error when string cannot be created from value
pub async fn socket_read(socket: Arc<Mutex<TcpStream>>) -> Result<String, Arc<ServerError>> {
    let mut buf = [0; 1024];

    let n = match socket.lock().await.read(&mut buf).await {
        Ok(n) if n == 0 => return Err(Arc::from(ServerError::SocketDisconnect)),
        Ok(n) => n,
        Err(e) => return Err(Arc::from(ServerError::SocketRead { e })),
    };

    match String::from_utf8(Vec::from(&buf[0..n])) {
        Ok(string) => Ok(string),
        Err(e) => Err(Arc::from(ServerError::StringConversion {
            debug_string: format!("{:?}", &buf[0..n]),
            e,
        })),
    }
}

/// # Errors
/// Returns error when socket cannot be written to
pub async fn socket_write(
    socket: Arc<Mutex<TcpStream>>,
    buf: &[u8],
) -> Result<(), Arc<ServerError>> {
    match socket.lock().await.write_all(buf).await {
        Ok(_) => Ok(()),
        Err(e) => Err(Arc::from(ServerError::SocketWrite { e })),
    }
}

/// # Errors
// returns an error if Channel could not send message
pub async fn mpsc_send<T>(channel: mpsc::Sender<T>, message: T) -> Result<(), Arc<ServerError>>
where
    T: std::fmt::Display + Clone + Send,
{
    let msg = message.to_string();

    match channel.send(message).await {
        Ok(_) => Ok(()),
        Err(_) => Err(Arc::from(ServerError::ChannelSend { message: msg })),
    }
}
