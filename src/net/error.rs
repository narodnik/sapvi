use std::fmt;

/// Returns the relevant network error if a program fails.
pub type NetResult<T> = std::result::Result<T, NetError>;

/// Defines a set of common network errors. Used for error handling.
#[derive(Debug, Copy, Clone)]
pub enum NetError {
    OperationFailed,
    ConnectFailed,
    ConnectTimeout,
    ChannelStopped,
    ChannelTimeout,
    ServiceStopped,
}

impl std::error::Error for NetError {}

impl fmt::Display for NetError {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        match *self {
            NetError::OperationFailed => f.write_str("Operation failed"),
            NetError::ConnectFailed => f.write_str("Connection failed"),
            NetError::ConnectTimeout => f.write_str("Connection timed out"),
            NetError::ChannelStopped => f.write_str("Channel stopped"),
            NetError::ChannelTimeout => f.write_str("Channel timed out"),
            NetError::ServiceStopped => f.write_str("Service stopped"),
        }
    }
}
