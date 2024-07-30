use std::process::Command;
use std::sync::Arc;

use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};

pub type ServerResult<T> = Result<T, Arc<ServerError>>;

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
    format!(
        "{{{}}}",
        vec_tup
            .iter()
            .map(|t| format!("\"{}\": \"{}\"", t.0, t.1))
            .collect::<Vec<String>>()
            .join(", ")
    )
}

/// # Errors
/// Returns an error if any values cannot be found in each mutex
pub fn get_all_json(
    vol_tup: &[(String, String)],
    bri_tup: &[(String, String)],
    bat_tup: &[(String, String)],
    mem_tup: &[(String, String)],
    blu_tup: &[(String, String)],
) -> Result<String, Box<ServerError>> {
    Ok(format!(
        "{{\"volume\": {}, \"brightness\": {}, \"battery\": {}, \"memory\": {}, \"bluetooth\": {}}}",
        get_json_from_tuple(vol_tup),
        get_json_from_tuple(bri_tup),
        get_json_from_tuple(bat_tup),
        get_json_from_tuple(mem_tup),
        get_json_from_tuple(blu_tup),
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
                eprintln!("Retrying function with type {}", std::any::type_name::<F>(),);
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

    let token = socket.lock().await.read(&mut buf).await;

    let n = match token {
        Ok(0) => return Err(Arc::from(ServerError::SocketDisconnect)),
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
    let token = socket.lock().await.write_all(buf).await;
    match token {
        Ok(()) => Ok(()),
        Err(e) => Err(Arc::from(ServerError::SocketWrite { e })),
    }
}

pub async fn get_tup<F, O>(
    get_tuple_func: F,
    error_tx: &mpsc::Sender<Arc<ServerError>>,
) -> Option<O>
where
    F: Fn() -> Result<O, Arc<ServerError>> + std::marker::Send,
    O: std::marker::Send,
{
    match call_and_retry(get_tuple_func) {
        Some(Ok(out)) => Some(out),
        Some(Err(e)) => {
            send_or_print_err(e, error_tx).await;
            None
        }
        None => {
            send_or_print_err(Arc::from(ServerError::RetryError), error_tx).await;
            None
        }
    }
}

pub async fn send_or_print_err(error: Arc<ServerError>, error_tx: &mpsc::Sender<Arc<ServerError>>) {
    if let Err(e) = error_tx.send(error).await {
        eprintln!("Could not send error via channel: {e}");
    }
}
