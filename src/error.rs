use thiserror::Error;

#[derive(Error, Debug)]
pub enum DaemonError {
    #[error("Tcp Error:\n\t{0}")]
    TcpError(#[from] tokio::io::Error),

    #[error("Postcard Serialize/Deserialize Error:\n\t{0}")]
    PostcardError(#[from] postcard::Error),
}
