use thiserror::Error;

#[derive(Error, Debug)]
pub enum DaemonError {
    #[error("Socket Error:\n\t{0}")]
    SocketError(#[from] tokio::io::Error),

    #[error("Postcard Serialize/Deserialize Error:\n\t{0}")]
    PostcardError(#[from] postcard::Error),

    #[error("Command '{name}' With Args '{args:?}' Could Not Run:\n\t{e}")]
    CommandError { name: String, args: Vec<String>, e: String },

    #[error("Bytes Could Not Convert To String:\n\t{0}")]
    IntegerFromByteString(#[from] std::string::FromUtf8Error),

    #[error("String Could Not Convert To Integer:\n\t{0}")]
    IntegerFromString(#[from] std::num::ParseIntError),

    #[error("String Could Not Convert To Bool:\n\t{0}")]
    BoolFromString(#[from] std::str::ParseBoolError),

    #[error("String Could Not Convert To Float:\n\t{0}")]
    StringToFloatError(#[from] std::num::ParseFloatError),

    #[error("String could not parse enough arguments:\n\t{0}")]
    ParseError(String),

    #[error("Serde JSON Serialization Failed:\n\t{0}")]
    JsonError(#[from] serde_json::Error),

    // TODO
    #[error("Serde JSON Serialization Failed:\n\t{0}")]
    IntErro(#[from] std::num::TryFromIntError),

    #[error("Mutex couldn't be locked")]
    MutexLockError,
}
