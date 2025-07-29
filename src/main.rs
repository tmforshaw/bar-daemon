use crate::{battery::Battery, cli::match_cli, error::DaemonError};

pub mod battery;
pub mod bluetooth;
pub mod brightness;
pub mod cli;
pub mod command;
pub mod daemon;
pub mod error;
pub mod volume;

pub const ICON_END: &str = "-symbolic";
pub const ICON_EXT: &str = "-symbolic.svg";

pub const NOTIFICATION_ID: u32 = 42069;
pub const NOTIFICATION_TIMEOUT: u32 = 1000;

// TODO Battery and Memory

#[tokio::main]
async fn main() -> Result<(), DaemonError> {
    Battery::get()?;

    match_cli().await?;

    Ok(())
}
