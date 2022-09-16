#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]

mod battery;
mod bluetooth;
mod brightness;
pub mod channel;
pub mod command;
mod listener;
mod memory;
mod sender;
mod server;
mod volume;

use command::ServerError;

pub static IP_AND_PORT: &str = "127.0.0.69:6969";
pub static RETRY_AMOUNT: u32 = 5;
pub static RETRY_TIMEOUT: u64 = 2000;

// pub static ICON_THEME_PATH: &str = "/usr/share/icons/Tela-purple-dark/symbolic";
pub static ICON_EXT: &str = "-symbolic";

#[tokio::main]
async fn main() -> Result<(), std::sync::Arc<ServerError>> {
    let args = std::env::args().collect::<Vec<String>>();

    match args.get(1) {
        Some(argument) => match argument.as_str() {
            "get" | "update" => sender::start(args.split_at(1).1).await,
            "listen" => listener::start().await,
            "daemon" => server::start().await,
            incorrect => Err(std::sync::Arc::from(ServerError::IncorrectArgument {
                incorrect: incorrect.to_string(),
                valid: vec!["get", "update", "listen", "daemon"]
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect(),
            })),
        },
        None => server::start().await,
    }?;

    Ok(())
}
