use serde::{Deserialize, Serialize};

use crate::{SessionId, WindowId};

/// Messages sent from client to server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Create a new session, optionally with a name and shell command.
    NewSession {
        name: Option<String>,
        command: Option<String>,
        cols: u16,
        rows: u16,
    },

    /// Attach to an existing session.
    Attach {
        session: SessionTarget,
        cols: u16,
        rows: u16,
    },

    /// Detach from the current session.
    Detach,

    /// Send keyboard input to the active pane.
    Input(Vec<u8>),

    /// Resize the client terminal.
    Resize { cols: u16, rows: u16 },

    /// Split the active pane.
    SplitPane {
        horizontal: bool,
    },

    /// Select a pane by direction.
    SelectPane(Direction),

    /// Resize a pane by direction.
    ResizePane {
        direction: Direction,
        amount: u16,
    },

    /// Toggle zoom on the active pane.
    ZoomPane,

    /// Create a new window.
    NewWindow {
        name: Option<String>,
        command: Option<String>,
    },

    /// Close the active pane (or window if last pane).
    ClosePane,

    /// Select a window by index.
    SelectWindow(usize),

    /// Next window.
    NextWindow,

    /// Previous window.
    PrevWindow,

    /// Rename current window.
    RenameWindow(String),

    /// Rename current session.
    RenameSession(String),

    /// List all sessions.
    ListSessions,

    /// Kill a session.
    KillSession(SessionTarget),

    /// Enter copy mode.
    EnterCopyMode,

    /// Copy mode input.
    CopyModeInput(CopyModeAction),

    /// Paste from buffer.
    Paste,

    /// Execute a command string (from : prompt).
    Command(String),

    /// Mouse event.
    MouseEvent {
        kind: MouseEventKind,
        col: u16,
        row: u16,
    },

    /// Ping (keepalive).
    Ping,
}

/// Mouse event kinds.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MouseEventKind {
    /// Left click.
    Click,
    /// Scroll up.
    ScrollUp,
    /// Scroll down.
    ScrollDown,
}

/// Messages sent from server to client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    /// Full screen render output (ANSI escape sequences).
    Output(Vec<u8>),

    /// Session created successfully.
    SessionCreated {
        session_id: SessionId,
        name: String,
    },

    /// Attached to session.
    Attached {
        session_id: SessionId,
        name: String,
    },

    /// Detached from session.
    Detached,

    /// Session list response.
    SessionList(Vec<SessionInfo>),

    /// Error message.
    Error(String),

    /// Pong (keepalive response).
    Pong,

    /// Server is shutting down.
    Shutdown,

    /// Notification message (displayed in status bar).
    Notification(String),
}

/// How to target a session (by name or ID).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionTarget {
    Name(String),
    Id(SessionId),
}

/// Direction for pane navigation/resize.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// Information about a session for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: SessionId,
    pub name: String,
    pub window_count: usize,
    pub pane_count: usize,
    pub created_at: u64,
    pub attached_clients: usize,
}

/// Copy mode actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CopyModeAction {
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    HalfPageUp,
    HalfPageDown,
    Top,
    Bottom,
    StartOfLine,
    EndOfLine,
    StartSelection,
    CopySelection,
    CancelSelection,
    SearchForward(String),
    SearchBackward(String),
    SearchNext,
    SearchPrev,
    Exit,
}

/// Window information for status bar display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: WindowId,
    pub index: usize,
    pub name: String,
    pub active: bool,
    pub pane_count: usize,
}
