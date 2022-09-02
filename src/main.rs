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

#[tokio::main]
async fn main() -> Result<(), std::sync::Arc<ServerError>> {
    let mut args = std::env::args().collect::<Vec<String>>();

    // No arguments entered
    let result = if args.len() <= 1 {
        // Attempt to open the server
        server::start().await
    } else {
        match args[1].as_str() {
            "get" | "update" => {
                // Allow sending of messages
                sender::start(&args[1].clone(), &args.split_off(2)).await
            }
            "daemon" => {
                // Attempt to open the server
                server::start().await
            }
            incorrect => Err(std::sync::Arc::from(ServerError::IncorrectArgument {
                incorrect: incorrect.to_string(),
                valid: vec!["get", "update", "daemon"]
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect(),
            })),
        }
    };

    if let Err(e) = result {
        eprintln!("{e}");
    }

    Ok(())
}
