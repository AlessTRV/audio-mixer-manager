//! Error type shared by the whole crate.

/// Errors returned by the audio layer.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A COM or Win32 call failed.
    #[error("Windows API error: {0}")]
    Windows(#[from] windows::core::Error),
    /// The running Windows build does not expose the required interface.
    #[error("not supported on this Windows build: {0}")]
    Unsupported(&'static str),
    /// The endpoint id does not match any device known to the system.
    #[error("audio device not found: {0}")]
    DeviceNotFound(String),
}

/// Result alias used across the crate.
pub type Result<T> = std::result::Result<T, Error>;
