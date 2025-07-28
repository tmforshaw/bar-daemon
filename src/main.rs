use crate::{cli::match_cli, error::DaemonError};

pub mod cli;
pub mod command;
pub mod daemon;
pub mod error;
pub mod volume;

pub const ICON_EXT: &str = "-symbolic.svg";

#[tokio::main]
async fn main() -> Result<(), DaemonError> {
    match_cli().await?;

    Ok(())
}
