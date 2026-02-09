use anyhow::Result;
use wtmux_common::PaneId;
use wtmux_pty::ConPty;
use wtmux_terminal::Terminal;

/// A pane is a single terminal within a window.
pub struct Pane {
    pub id: PaneId,
    pub pty: ConPty,
    pub terminal: Terminal,
    pub title: String,
    pub cols: u16,
    pub rows: u16,
    pub exited: bool,
}

impl Pane {
    /// Create a new pane by spawning a process.
    pub fn new(command: &str, cols: u16, rows: u16) -> Result<Self> {
        let id = PaneId::new();
        let pty = ConPty::spawn(command, cols, rows)?;
        let terminal = Terminal::new(cols, rows);

        Ok(Pane {
            id,
            pty,
            terminal,
            title: command.to_string(),
            cols,
            rows,
            exited: false,
        })
    }

    /// Resize this pane.
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        if cols != self.cols || rows != self.rows {
            self.pty.resize(cols, rows)?;
            self.terminal.resize(cols, rows);
            self.cols = cols;
            self.rows = rows;
        }
        Ok(())
    }

    /// Write input to the PTY.
    pub async fn write_input(&mut self, data: &[u8]) -> Result<()> {
        self.pty.write(data).await
    }

    /// Read output from the PTY and process it through the VT parser.
    pub async fn read_output(&mut self) -> Result<Option<Vec<u8>>> {
        let mut buf = vec![0u8; 4096];
        match self.pty.read(&mut buf).await {
            Ok(0) => {
                self.exited = true;
                Ok(None)
            }
            Ok(n) => {
                buf.truncate(n);
                self.terminal.process_bytes(&buf);
                if let Some(title) = self.get_title_update() {
                    self.title = title;
                }
                Ok(Some(buf))
            }
            Err(e) => {
                self.exited = true;
                Err(e)
            }
        }
    }

    fn get_title_update(&self) -> Option<String> {
        let title = &self.terminal.state.title;
        if !title.is_empty() && *title != self.title {
            Some(title.clone())
        } else {
            None
        }
    }
}
