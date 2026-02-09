use thiserror::Error;

#[derive(Error, Debug)]
pub enum WtmuxError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("ConPTY error: {0}")]
    ConPty(String),

    #[error("IPC error: {0}")]
    Ipc(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Window not found: {0}")]
    WindowNotFound(String),

    #[error("Pane not found: {0}")]
    PaneNotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Win32 error: code {0}")]
    Win32(u32),

    #[error("{0}")]
    Other(String),
}
