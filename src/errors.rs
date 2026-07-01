use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KnxError {
    #[error("Invalid address: {0}. Incorrect format.")]
    InvalidKnxAddressException(String),

    #[error("The object does not contain valid parameters to encode the dpt")]
    InvalidParametersForDpt,

    #[error("This DPT is not available for encoding or decoding, or it does not exist.")]
    DPTNotFound,

    #[error("Connection timeout")]
    Timeout,

    #[error("IO error: {0}")]
    Io(String),

    #[error("Connection closed by peer")]
    ConnectionClosed,

    #[error("Protocol error: {0}")]
    Protocol(String),
}

impl From<std::io::Error> for KnxError {
    fn from(err: std::io::Error) -> Self {
        KnxError::Io(err.to_string())
    }
}
