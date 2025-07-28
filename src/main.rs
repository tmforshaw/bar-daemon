use crate::{cli::match_cli, error::DaemonError};

pub mod cli;
pub mod command;
pub mod daemon;
pub mod error;
pub mod volume;

// pub const ICON_EXT: &str = "-symbolic.svg";
pub const ICON_EXT: &str = "";

pub const NOTIFICATION_ID: u32 = 42069;
pub const NOTIFICATION_TIMEOUT: u32 = 1000;

#[tokio::main]
async fn main() -> Result<(), DaemonError> {
    match_cli().await?;

    Ok(())
}
