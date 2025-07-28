use crate::{cli::match_cli, error::DaemonError};

pub mod cli;
pub mod daemon;
pub mod error;

#[tokio::main]
async fn main() -> Result<(), DaemonError> {
    match_cli().await?;

    Ok(())
}
