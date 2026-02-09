use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::windows::named_pipe::NamedPipeServer;
use tokio::sync::Mutex;
use tracing::{debug, error, info};
use wtmux_common::ipc::{create_server, create_server_instance, recv_message, send_message};
use wtmux_common::protocol::{SessionInfo, SessionTarget};
use wtmux_common::{ClientId, ClientMessage, ServerMessage, SessionId};
use wtmux_config::Config;

use crate::copymode::CopyMode;
use crate::pastebuffer::PasteBuffer;
use crate::renderer::Renderer;
use crate::session::Session;

/// Server-wide state accessible by the command executor.
pub struct ServerState {
    pub sessions: HashMap<SessionId, Session>,
    pub config: Config,
    pub paste_buffer: PasteBuffer,
}

impl ServerState {
    /// Get the active session for the first attached client (simplified).
    pub fn active_session(&self) -> Option<&Session> {
        self.sessions.values().next()
    }

    pub fn active_session_mut(&mut self) -> Option<&mut Session> {
        self.sessions.values_mut().next()
    }
}

struct ConnectedClient {
    session_id: Option<SessionId>,
    cols: u16,
    rows: u16,
    copy_mode: Option<CopyMode>,
}

/// Shared inner state protected by a mutex for concurrent client access.
struct ServerInner {
    state: ServerState,
    clients: HashMap<ClientId, ConnectedClient>,
}

pub struct Server {
    pipe_name: String,
    inner: Arc<Mutex<ServerInner>>,
}

impl Server {
    pub fn new(pipe_name: &str) -> Result<Self> {
        let config = Config::load().unwrap_or_else(|_| Config::default_config());

        Ok(Server {
            pipe_name: pipe_name.to_string(),
            inner: Arc::new(Mutex::new(ServerInner {
                state: ServerState {
                    sessions: HashMap::new(),
                    config,
                    paste_buffer: PasteBuffer::new(50),
                },
                clients: HashMap::new(),
            })),
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Server starting, waiting for connections...");

        // Create first pipe instance
        let server = create_server(&self.pipe_name)?;
        self.accept_and_serve(server).await
    }

    async fn accept_and_serve(&self, mut pipe: NamedPipeServer) -> Result<()> {
        loop {
            // Wait for a client to connect
            pipe.connect().await?;
            info!("Client connected");

            let client_id = ClientId::new();
            {
                let mut inner = self.inner.lock().await;
                inner.clients.insert(
                    client_id,
                    ConnectedClient {
                        session_id: None,
                        cols: 80,
                        rows: 24,
                        copy_mode: None,
                    },
                );
            }

            // Create next pipe instance for future clients BEFORE spawning handler
            let next_pipe = create_server_instance(&self.pipe_name)?;

            // Spawn client handler as independent task
            let inner = Arc::clone(&self.inner);
            tokio::spawn(async move {
                handle_client(inner, client_id, pipe).await;
            });

            pipe = next_pipe;
        }
    }
}

/// Handle a single client connection. Runs as an independent tokio task.
async fn handle_client(
    inner: Arc<Mutex<ServerInner>>,
    client_id: ClientId,
    mut pipe: NamedPipeServer,
) {
    loop {
        // Read next message from client (no lock held during I/O)
        let msg: Result<ClientMessage> = recv_message(&mut pipe).await;

        match msg {
            Ok(client_msg) => {
                // Lock inner state, process the message
                let mut guard = inner.lock().await;
                let response = guard.process_message(client_id, client_msg).await;

                match response {
                    Some(ServerMessage::Detached) => {
                        drop(guard); // release lock before I/O
                        let _ = send_message(&mut pipe, &ServerMessage::Detached).await;
                        break;
                    }
                    Some(msg) => {
                        drop(guard);
                        if let Err(e) = send_message(&mut pipe, &msg).await {
                            error!("Failed to send message: {}", e);
                            break;
                        }
                    }
                    None => {
                        // Send updated screen after state change
                        let output = guard.render_for_client(client_id);
                        drop(guard);
                        if let Some(output) = output {
                            if let Err(e) =
                                send_message(&mut pipe, &ServerMessage::Output(output)).await
                            {
                                error!("Failed to send output: {}", e);
                                break;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                debug!("Client read error: {}", e);
                break;
            }
        }
    }

    // Clean up client on disconnect
    let mut guard = inner.lock().await;
    guard.clients.remove(&client_id);
    info!("Client disconnected: {}", client_id);
}

impl ServerInner {
    async fn process_message(
        &mut self,
        client_id: ClientId,
        msg: ClientMessage,
    ) -> Option<ServerMessage> {
        match msg {
            ClientMessage::NewSession {
                name,
                command,
                cols,
                rows,
            } => {
                let session_name =
                    name.unwrap_or_else(|| format!("{}", self.state.sessions.len()));
                let shell = command.unwrap_or_else(|| {
                    self.state.config.options.default_shell.clone()
                });

                match Session::new(session_name.clone(), &shell, cols, rows) {
                    Ok(session) => {
                        let session_id = session.id;
                        self.state.sessions.insert(session_id, session);

                        if let Some(client) = self.clients.get_mut(&client_id) {
                            client.session_id = Some(session_id);
                            client.cols = cols;
                            client.rows = rows;
                        }

                        info!("Session created: {} ({})", session_name, session_id);
                        Some(ServerMessage::SessionCreated {
                            session_id,
                            name: session_name,
                        })
                    }
                    Err(e) => Some(ServerMessage::Error(format!(
                        "Failed to create session: {}",
                        e
                    ))),
                }
            }

            ClientMessage::Attach {
                session,
                cols,
                rows,
            } => {
                let session_id = match &session {
                    SessionTarget::Name(name) => self
                        .state
                        .sessions
                        .iter()
                        .find(|(_, s)| s.name == *name)
                        .map(|(id, _)| *id),
                    SessionTarget::Id(id) => {
                        if self.state.sessions.contains_key(id) {
                            Some(*id)
                        } else {
                            None
                        }
                    }
                };

                match session_id {
                    Some(id) => {
                        if let Some(client) = self.clients.get_mut(&client_id) {
                            client.session_id = Some(id);
                            client.cols = cols;
                            client.rows = rows;
                        }

                        if let Some(session) = self.state.sessions.get_mut(&id) {
                            let _ = session.resize(cols, rows);
                            let name = session.name.clone();
                            info!("Client attached to session: {}", name);
                            Some(ServerMessage::Attached {
                                session_id: id,
                                name,
                            })
                        } else {
                            Some(ServerMessage::Error("Session not found".to_string()))
                        }
                    }
                    None => Some(ServerMessage::Error("Session not found".to_string())),
                }
            }

            ClientMessage::Detach => Some(ServerMessage::Detached),

            ClientMessage::Input(data) => {
                if let Some(client) = self.clients.get(&client_id) {
                    if let Some(session_id) = client.session_id {
                        if let Some(session) = self.state.sessions.get_mut(&session_id) {
                            let pane_id = session.active_pane_id();
                            if let Some(pane) = session
                                .active_window_mut()
                                .panes
                                .get_mut(&pane_id)
                            {
                                if let Err(e) = pane.write_input(&data).await {
                                    error!("PTY write failed: {}", e);
                                }

                                // Read any available output with a timeout
                                let mut buf = vec![0u8; 8192];
                                loop {
                                    match tokio::time::timeout(
                                        std::time::Duration::from_millis(50),
                                        pane.pty.read(&mut buf),
                                    )
                                    .await
                                    {
                                        Ok(Ok(n)) if n > 0 => {
                                            pane.terminal.process_bytes(&buf[..n]);
                                        }
                                        _ => break,
                                    }
                                }
                            }
                        }
                    }
                }
                None // Will trigger a render
            }

            ClientMessage::Resize { cols, rows } => {
                if let Some(client) = self.clients.get_mut(&client_id) {
                    client.cols = cols;
                    client.rows = rows;
                    if let Some(session_id) = client.session_id {
                        if let Some(session) = self.state.sessions.get_mut(&session_id) {
                            let _ = session.resize(cols, rows);
                        }
                    }
                }
                None
            }

            ClientMessage::ResizePane { direction, amount } => {
                if let Some(session_id) = self.get_client_session(client_id) {
                    if let Some(session) = self.state.sessions.get_mut(&session_id) {
                        let _ = session.active_window_mut().resize_pane_direction(direction, amount);
                    }
                }
                None
            }

            ClientMessage::SplitPane { horizontal } => {
                let shell = self.state.config.options.default_shell.clone();
                if let Some(session_id) = self.get_client_session(client_id) {
                    if let Some(session) = self.state.sessions.get_mut(&session_id) {
                        match session
                            .active_window_mut()
                            .split_pane(&shell, horizontal)
                        {
                            Ok(_) => {}
                            Err(e) => {
                                return Some(ServerMessage::Error(format!(
                                    "Split failed: {}",
                                    e
                                )));
                            }
                        }
                    }
                }
                None
            }

            ClientMessage::SelectPane(direction) => {
                if let Some(session_id) = self.get_client_session(client_id) {
                    if let Some(session) = self.state.sessions.get_mut(&session_id) {
                        session
                            .active_window_mut()
                            .select_pane_direction(direction);
                    }
                }
                None
            }

            ClientMessage::ZoomPane => {
                if let Some(session_id) = self.get_client_session(client_id) {
                    if let Some(session) = self.state.sessions.get_mut(&session_id) {
                        session.active_window_mut().toggle_zoom();
                    }
                }
                None
            }

            ClientMessage::NewWindow { name, command } => {
                let shell = command.unwrap_or_else(|| {
                    self.state.config.options.default_shell.clone()
                });
                if let Some(session_id) = self.get_client_session(client_id) {
                    if let Some(client) = self.clients.get(&client_id) {
                        if let Some(session) = self.state.sessions.get_mut(&session_id) {
                            let _ =
                                session.new_window(name, &shell, client.cols, client.rows);
                        }
                    }
                }
                None
            }

            ClientMessage::ClosePane => {
                if let Some(session_id) = self.get_client_session(client_id) {
                    if let Some(session) = self.state.sessions.get_mut(&session_id) {
                        let pane_id = session.active_pane_id();
                        let window_empty =
                            session.active_window_mut().close_pane(pane_id);
                        if window_empty {
                            let win_id = session.active_window().id;
                            let session_empty = session.close_window(win_id);
                            if session_empty {
                                self.state.sessions.remove(&session_id);
                                return Some(ServerMessage::Detached);
                            }
                        }
                    }
                }
                None
            }

            ClientMessage::SelectWindow(idx) => {
                if let Some(session_id) = self.get_client_session(client_id) {
                    if let Some(session) = self.state.sessions.get_mut(&session_id) {
                        session.select_window(idx);
                    }
                }
                None
            }

            ClientMessage::NextWindow => {
                if let Some(session_id) = self.get_client_session(client_id) {
                    if let Some(session) = self.state.sessions.get_mut(&session_id) {
                        session.next_window();
                    }
                }
                None
            }

            ClientMessage::PrevWindow => {
                if let Some(session_id) = self.get_client_session(client_id) {
                    if let Some(session) = self.state.sessions.get_mut(&session_id) {
                        session.prev_window();
                    }
                }
                None
            }

            ClientMessage::RenameWindow(name) => {
                if let Some(session_id) = self.get_client_session(client_id) {
                    if let Some(session) = self.state.sessions.get_mut(&session_id) {
                        session.active_window_mut().name = name;
                    }
                }
                None
            }

            ClientMessage::RenameSession(name) => {
                if let Some(session_id) = self.get_client_session(client_id) {
                    if let Some(session) = self.state.sessions.get_mut(&session_id) {
                        session.name = name;
                    }
                }
                None
            }

            ClientMessage::ListSessions => {
                let sessions: Vec<SessionInfo> = self
                    .state
                    .sessions
                    .values()
                    .map(|s| {
                        let attached = self
                            .clients
                            .values()
                            .filter(|c| c.session_id == Some(s.id))
                            .count();
                        SessionInfo {
                            id: s.id,
                            name: s.name.clone(),
                            window_count: s.windows.len(),
                            pane_count: s.pane_count(),
                            created_at: s.created_at,
                            attached_clients: attached,
                        }
                    })
                    .collect();
                Some(ServerMessage::SessionList(sessions))
            }

            ClientMessage::KillSession(target) => {
                let session_id = match &target {
                    SessionTarget::Name(name) => self
                        .state
                        .sessions
                        .iter()
                        .find(|(_, s)| s.name == *name)
                        .map(|(id, _)| *id),
                    SessionTarget::Id(id) => Some(*id),
                };

                if let Some(id) = session_id {
                    self.state.sessions.remove(&id);
                    // Detach any clients on this session
                    for client in self.clients.values_mut() {
                        if client.session_id == Some(id) {
                            client.session_id = None;
                        }
                    }
                    Some(ServerMessage::Notification(
                        "Session killed".to_string(),
                    ))
                } else {
                    Some(ServerMessage::Error("Session not found".to_string()))
                }
            }

            ClientMessage::EnterCopyMode => {
                if let Some(client) = self.clients.get_mut(&client_id) {
                    if let Some(session_id) = client.session_id {
                        if let Some(session) = self.state.sessions.get(&session_id) {
                            let (cx, cy) = {
                                let pane_id = session.active_pane_id();
                                if let Some(pane) = session.active_window().panes.get(&pane_id)
                                {
                                    pane.terminal.cursor_pos()
                                } else {
                                    (0, 0)
                                }
                            };
                            client.copy_mode = Some(CopyMode::new(cx, cy));
                        }
                    }
                }
                None
            }

            ClientMessage::CopyModeInput(action) => {
                if let Some(client) = self.clients.get_mut(&client_id) {
                    if let Some(ref mut copy_mode) = client.copy_mode {
                        if let Some(session_id) = client.session_id {
                            if let Some(session) = self.state.sessions.get(&session_id) {
                                let pane_id = session.active_pane_id();
                                if let Some(pane) =
                                    session.active_window().panes.get(&pane_id)
                                {
                                    if let Some(text) =
                                        copy_mode.handle_action(&action, &pane.terminal)
                                    {
                                        self.state.paste_buffer.push(text);
                                    }
                                }
                            }
                        }
                        if !copy_mode.active {
                            client.copy_mode = None;
                        }
                    }
                }
                None
            }

            ClientMessage::Paste => {
                if let Some(text) = self.state.paste_buffer.top() {
                    let text = text.to_string();
                    if let Some(session_id) = self.get_client_session(client_id) {
                        if let Some(session) = self.state.sessions.get_mut(&session_id) {
                            let pane_id = session.active_pane_id();
                            if let Some(pane) =
                                session.active_window_mut().panes.get_mut(&pane_id)
                            {
                                let _ = pane.write_input(text.as_bytes()).await;
                            }
                        }
                    }
                }
                None
            }

            ClientMessage::Command(cmd) => {
                match crate::command_executor::execute_command(&mut self.state, &cmd) {
                    Ok(Some(result)) => {
                        if result == "__detach__" {
                            Some(ServerMessage::Detached)
                        } else if result.starts_with("__") {
                            // Internal commands handled separately
                            Some(ServerMessage::Notification(result))
                        } else {
                            Some(ServerMessage::Notification(result))
                        }
                    }
                    Ok(None) => None,
                    Err(e) => Some(ServerMessage::Error(format!("Command error: {}", e))),
                }
            }

            ClientMessage::MouseEvent { kind, col, row } => {
                use wtmux_common::protocol::MouseEventKind;

                if !self.state.config.options.mouse {
                    return None;
                }

                if let Some(session_id) = self.get_client_session(client_id) {
                    if let Some(session) = self.state.sessions.get_mut(&session_id) {
                        let window = session.active_window_mut();

                        match kind {
                            MouseEventKind::Click => {
                                // Find which pane was clicked
                                let geometries = window.pane_geometries();
                                for (pane_id, rect) in &geometries {
                                    if col >= rect.x
                                        && col < rect.x + rect.width
                                        && row >= rect.y
                                        && row < rect.y + rect.height
                                    {
                                        if *pane_id != window.active_pane {
                                            window.last_active_pane = Some(window.active_pane);
                                            window.active_pane = *pane_id;
                                        }
                                        break;
                                    }
                                }
                            }
                            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                                // Scroll the active pane's copy mode, or send scroll keys
                                let pane_id = window.active_pane;
                                if let Some(client) = self.clients.get_mut(&client_id) {
                                    if let Some(ref mut copy_mode) = client.copy_mode {
                                        match kind {
                                            MouseEventKind::ScrollUp => {
                                                copy_mode.scroll_offset += 3;
                                            }
                                            MouseEventKind::ScrollDown => {
                                                copy_mode.scroll_offset =
                                                    copy_mode.scroll_offset.saturating_sub(3);
                                            }
                                            _ => {}
                                        }
                                    } else {
                                        // Not in copy mode: enter copy mode on scroll up
                                        if matches!(kind, MouseEventKind::ScrollUp) {
                                            if let Some(session) =
                                                self.state.sessions.get(&session_id)
                                            {
                                                let (cx, cy) = {
                                                    if let Some(pane) = session
                                                        .active_window()
                                                        .panes
                                                        .get(&pane_id)
                                                    {
                                                        pane.terminal.cursor_pos()
                                                    } else {
                                                        (0, 0)
                                                    }
                                                };
                                                let mut cm =
                                                    crate::copymode::CopyMode::new(cx, cy);
                                                cm.scroll_offset = 3;
                                                client.copy_mode = Some(cm);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                None
            }

            ClientMessage::Ping => Some(ServerMessage::Pong),
        }
    }

    fn get_client_session(&self, client_id: ClientId) -> Option<SessionId> {
        self.clients.get(&client_id)?.session_id
    }

    fn render_for_client(&self, client_id: ClientId) -> Option<Vec<u8>> {
        let client = self.clients.get(&client_id)?;
        let session_id = client.session_id?;
        let session = self.state.sessions.get(&session_id)?;

        let renderer = Renderer::new(client.cols, client.rows);
        let mut output = renderer.render(session);

        // Add copy mode overlay if active
        if let Some(ref copy_mode) = client.copy_mode {
            output.extend_from_slice(&copy_mode.render_indicator());
        }

        Some(output)
    }
}
