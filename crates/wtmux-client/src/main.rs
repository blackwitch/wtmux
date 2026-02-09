mod input_handler;

use anyhow::Result;
use clap::{Parser, Subcommand};
use crossterm::terminal::{self, ClearType};
use crossterm::{cursor, execute};
use std::io::{self, Write};
use std::os::windows::process::CommandExt;
use tokio::io::AsyncReadExt;
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;
use wtmux_common::ipc::{connect_client, recv_message, send_message};
use wtmux_common::protocol::SessionTarget;
use wtmux_common::{pipe_name, ClientMessage, ServerMessage};

use input_handler::InputHandler;

#[derive(Parser)]
#[command(name = "wtmux", about = "Windows terminal multiplexer")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new session
    #[command(name = "new-session", alias = "new")]
    NewSession {
        /// Session name
        #[arg(short = 's', long)]
        name: Option<String>,

        /// Shell command to run
        #[arg(short, long)]
        command: Option<String>,
    },

    /// Attach to an existing session
    #[command(name = "attach-session", alias = "attach", alias = "a")]
    Attach {
        /// Target session (name or ID)
        #[arg(short = 't', long)]
        target: Option<String>,
    },

    /// List sessions
    #[command(name = "list-sessions", alias = "ls")]
    ListSessions,

    /// Kill a session
    #[command(name = "kill-session")]
    KillSession {
        /// Target session
        #[arg(short = 't', long)]
        target: String,
    },

    /// Kill the server
    #[command(name = "kill-server")]
    KillServer,

    /// Start the server (usually done automatically)
    #[command(name = "start-server")]
    StartServer,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let pipe = pipe_name();

    match cli.command {
        None | Some(Commands::NewSession { .. }) => {
            let (name, command) = match &cli.command {
                Some(Commands::NewSession { name, command }) => {
                    (name.clone(), command.clone())
                }
                _ => (None, None),
            };

            let (cols, rows) = terminal::size()?;
            let mut client = ensure_server_and_connect(&pipe).await?;

            // Send new-session request
            send_message(
                &mut client,
                &ClientMessage::NewSession {
                    name,
                    command,
                    cols,
                    rows,
                },
            )
            .await?;

            // Wait for session created response
            let response: ServerMessage = recv_message(&mut client).await?;
            match response {
                ServerMessage::SessionCreated { session_id, name } => {
                    info!("Session created: {} ({})", name, session_id);
                    run_interactive(client).await?;
                }
                ServerMessage::Error(e) => {
                    eprintln!("Error: {}", e);
                }
                _ => {
                    eprintln!("Unexpected response from server");
                }
            }
        }

        Some(Commands::Attach { target }) => {
            let (cols, rows) = terminal::size()?;
            let mut client = connect_client(&pipe).await?;

            let session_target = match target {
                Some(t) => SessionTarget::Name(t),
                None => SessionTarget::Name("0".to_string()),
            };

            send_message(
                &mut client,
                &ClientMessage::Attach {
                    session: session_target,
                    cols,
                    rows,
                },
            )
            .await?;

            let response: ServerMessage = recv_message(&mut client).await?;
            match response {
                ServerMessage::Attached { session_id, name } => {
                    info!("Attached to session: {} ({})", name, session_id);
                    run_interactive(client).await?;
                }
                ServerMessage::Error(e) => {
                    eprintln!("Error: {}", e);
                }
                _ => {
                    eprintln!("Unexpected response from server");
                }
            }
        }

        Some(Commands::ListSessions) => {
            let mut client = connect_client(&pipe).await?;
            send_message(&mut client, &ClientMessage::ListSessions).await?;

            let response: ServerMessage = recv_message(&mut client).await?;
            match response {
                ServerMessage::SessionList(sessions) => {
                    if sessions.is_empty() {
                        println!("No sessions.");
                    } else {
                        for s in sessions {
                            println!(
                                "{}: {} ({} windows, {} panes) [{}]",
                                s.name,
                                s.id,
                                s.window_count,
                                s.pane_count,
                                if s.attached_clients > 0 {
                                    "attached"
                                } else {
                                    "detached"
                                }
                            );
                        }
                    }
                }
                _ => eprintln!("Unexpected response"),
            }
        }

        Some(Commands::KillSession { target }) => {
            let mut client = connect_client(&pipe).await?;
            send_message(
                &mut client,
                &ClientMessage::KillSession(SessionTarget::Name(target)),
            )
            .await?;

            let response: ServerMessage = recv_message(&mut client).await?;
            match response {
                ServerMessage::Notification(msg) => println!("{}", msg),
                ServerMessage::Error(e) => eprintln!("Error: {}", e),
                _ => {}
            }
        }

        Some(Commands::KillServer) => {
            eprintln!("Server shutdown not yet implemented");
        }

        Some(Commands::StartServer) => {
            start_server().await?;
            println!("Server is running.");
        }
    }

    Ok(())
}

/// Run the interactive terminal session.
async fn run_interactive(
    mut pipe: tokio::net::windows::named_pipe::NamedPipeClient,
) -> Result<()> {
    // Enter raw mode
    terminal::enable_raw_mode()?;

    // Clear screen and enable mouse capture
    let mut stdout = io::stdout();
    execute!(
        stdout,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0),
        crossterm::event::EnableMouseCapture
    )?;

    let mut input_handler = InputHandler::new();
    let result = interactive_loop(&mut pipe, &mut input_handler).await;

    // Restore terminal
    terminal::disable_raw_mode()?;
    execute!(
        io::stdout(),
        crossterm::event::DisableMouseCapture,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0),
        cursor::Show
    )?;
    println!("[detached]");

    result
}

async fn interactive_loop(
    pipe: &mut tokio::net::windows::named_pipe::NamedPipeClient,
    input_handler: &mut InputHandler,
) -> Result<()> {
    use crossterm::event::{self, Event, KeyEventKind, MouseEventKind as CMouseEventKind};
    use wtmux_common::protocol::MouseEventKind;

    let mut stdout = io::stdout();

    loop {
        // Poll for terminal events with a short timeout
        if event::poll(std::time::Duration::from_millis(10))? {
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    // Process through input handler (handles prefix key, bindings)
                    match input_handler.handle_key(key_event) {
                        input_handler::KeyAction::SendBytes(bytes) => {
                            send_message(pipe, &ClientMessage::Input(bytes)).await?;
                        }
                        input_handler::KeyAction::Command(cmd) => {
                            send_message(pipe, &ClientMessage::Command(cmd)).await?;
                        }
                        input_handler::KeyAction::Detach => {
                            send_message(pipe, &ClientMessage::Detach).await?;
                        }
                        input_handler::KeyAction::None => {}
                    }
                }
                Event::Mouse(mouse_event) => {
                    let kind = match mouse_event.kind {
                        CMouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                            Some(MouseEventKind::Click)
                        }
                        CMouseEventKind::ScrollUp => Some(MouseEventKind::ScrollUp),
                        CMouseEventKind::ScrollDown => Some(MouseEventKind::ScrollDown),
                        _ => None,
                    };
                    if let Some(kind) = kind {
                        send_message(
                            pipe,
                            &ClientMessage::MouseEvent {
                                kind,
                                col: mouse_event.column,
                                row: mouse_event.row,
                            },
                        )
                        .await?;
                    }
                }
                Event::Resize(cols, rows) => {
                    send_message(pipe, &ClientMessage::Resize { cols, rows }).await?;
                }
                _ => {} // Ignore key release/repeat, focus events
            }
        }

        // Try to read server messages (non-blocking)
        let mut buf = [0u8; 4];
        match tokio::time::timeout(
            std::time::Duration::from_millis(5),
            pipe.read(&mut buf[..4]),
        )
        .await
        {
            Ok(Ok(4)) => {
                // Got the length prefix, read the rest
                let len = u32::from_le_bytes(buf);
                if len > 16 * 1024 * 1024 {
                    error!("Message too large: {}", len);
                    break;
                }
                let mut data = vec![0u8; len as usize];
                pipe.read_exact(&mut data).await?;
                let msg: ServerMessage = bincode::deserialize(&data)?;

                match msg {
                    ServerMessage::Output(output) => {
                        stdout.write_all(&output)?;
                        stdout.flush()?;
                    }
                    ServerMessage::Detached => {
                        break;
                    }
                    ServerMessage::Error(e) => {
                        debug!("Server error: {}", e);
                    }
                    ServerMessage::Notification(n) => {
                        debug!("Notification: {}", n);
                    }
                    ServerMessage::Shutdown => {
                        break;
                    }
                    ServerMessage::Pong => {}
                    _ => {}
                }
            }
            Ok(Ok(_)) => {
                // Partial read or connection closed
            }
            Ok(Err(_)) => {
                // Connection error
                break;
            }
            Err(_) => {
                // Timeout - no data available, continue loop
            }
        }
    }

    Ok(())
}

/// Connect to the server, starting it if necessary.
/// Returns the connected pipe client directly (no wasted probe connections).
async fn ensure_server_and_connect(
    pipe: &str,
) -> Result<tokio::net::windows::named_pipe::NamedPipeClient> {
    use tokio::net::windows::named_pipe::ClientOptions;

    // Try to connect directly (server may already be running).
    for _ in 0..5 {
        match ClientOptions::new().open(pipe) {
            Ok(client) => return Ok(client),
            Err(_) => tokio::time::sleep(std::time::Duration::from_millis(50)).await,
        }
    }

    // Could not connect — start the server.
    start_server().await?;

    // Wait for server to accept connections.
    for attempt in 0..30 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        match ClientOptions::new().open(pipe) {
            Ok(client) => return Ok(client),
            Err(e) if attempt == 29 => return Err(e.into()),
            Err(_) => {}
        }
    }

    anyhow::bail!("Server failed to start within timeout");
}

/// Start the server as a detached background process.
async fn start_server() -> Result<()> {
    let exe_path = std::env::current_exe()?;
    let server_path = exe_path.parent().unwrap().join("wtmux-server.exe");

    if !server_path.exists() {
        anyhow::bail!(
            "Server binary not found: {}. Build with `cargo build`.",
            server_path.display()
        );
    }

    std::process::Command::new(&server_path)
        .creation_flags(0x00000008) // DETACHED_PROCESS
        .spawn()?;

    Ok(())
}
