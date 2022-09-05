#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]

mod battery;
mod brightness;
pub mod command;
mod memory;
mod sender;
mod server;
mod volume;

use command::ServerError;

pub static IP_AND_PORT: &str = "127.0.0.69:6969";

#[tokio::main]
async fn main() -> Result<(), std::sync::Arc<ServerError>> {
    let args = std::env::args().collect::<Vec<String>>();

    match args.get(1) {
        Some(argument) => match argument.as_str() {
            "get" | "update" => sender::start(args.split_at(1).1).await,
            "daemon" => server::start().await,
            incorrect => Err(std::sync::Arc::from(ServerError::IncorrectArgument {
                incorrect: incorrect.to_string(),
                valid: vec!["get", "update", "daemon"]
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect(),
            })),
        },
        None => server::start().await,
    }?;

    Ok(())
}
