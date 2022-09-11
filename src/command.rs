use std::process::Command;
use std::sync::Arc;
use thiserror::Error;

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

// unsafe impl Send for ServerError {}
// unsafe impl Sync for ServerError {}

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
