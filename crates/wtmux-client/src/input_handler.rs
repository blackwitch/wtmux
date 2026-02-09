use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use wtmux_config::keybindings::{Key, KeyBinding, KeyTable, Modifiers};

/// The result of processing a key event.
pub enum KeyAction {
    /// Send raw bytes to the server (PTY input).
    SendBytes(Vec<u8>),
    /// Execute a command string.
    Command(String),
    /// Detach from the session.
    Detach,
    /// No action.
    None,
}

/// Input handler state machine: Normal → PrefixReceived → dispatch binding.
pub struct InputHandler {
    state: InputState,
    key_table: KeyTable,
    command_buffer: String,
    in_command_prompt: bool,
}

enum InputState {
    Normal,
    PrefixReceived,
}

impl InputHandler {
    pub fn new() -> Self {
        InputHandler {
            state: InputState::Normal,
            key_table: KeyTable::default_tmux_bindings(),
            command_buffer: String::new(),
            in_command_prompt: false,
        }
    }

    pub fn handle_key(&mut self, event: KeyEvent) -> KeyAction {
        // Command prompt mode
        if self.in_command_prompt {
            return self.handle_command_prompt_key(event);
        }

        match self.state {
            InputState::Normal => self.handle_normal(event),
            InputState::PrefixReceived => self.handle_prefix(event),
        }
    }

    fn handle_normal(&mut self, event: KeyEvent) -> KeyAction {
        // Check if this is the prefix key
        let prefix = &self.key_table.prefix;
        if key_event_matches(event, prefix) {
            self.state = InputState::PrefixReceived;
            return KeyAction::None;
        }

        // Convert key event to bytes
        key_event_to_bytes(event)
    }

    fn handle_prefix(&mut self, event: KeyEvent) -> KeyAction {
        self.state = InputState::Normal;

        // Look up the binding
        if let Some(binding) = crossterm_to_binding(event) {
            if let Some(command) = self.key_table.lookup(&binding) {
                let command = command.clone();

                // Handle special commands
                if command == "detach-client" {
                    return KeyAction::Detach;
                }
                if command == "command-prompt" {
                    self.in_command_prompt = true;
                    self.command_buffer.clear();
                    // Show command prompt indicator
                    return KeyAction::SendBytes(b"\x1b[999;1H\x1b[2K:".to_vec());
                }

                return KeyAction::Command(command);
            }
        }

        // If no binding matched, send the key as regular input
        key_event_to_bytes(event)
    }

    fn handle_command_prompt_key(&mut self, event: KeyEvent) -> KeyAction {
        match event.code {
            KeyCode::Enter => {
                self.in_command_prompt = false;
                let cmd = self.command_buffer.clone();
                self.command_buffer.clear();
                if cmd.is_empty() {
                    KeyAction::None
                } else {
                    KeyAction::Command(cmd)
                }
            }
            KeyCode::Esc => {
                self.in_command_prompt = false;
                self.command_buffer.clear();
                KeyAction::None
            }
            KeyCode::Backspace => {
                self.command_buffer.pop();
                // Redraw prompt
                let display = format!("\x1b[999;1H\x1b[2K:{}", self.command_buffer);
                KeyAction::SendBytes(display.into_bytes())
            }
            KeyCode::Char(c) => {
                self.command_buffer.push(c);
                let display = format!("\x1b[999;1H\x1b[2K:{}", self.command_buffer);
                KeyAction::SendBytes(display.into_bytes())
            }
            _ => KeyAction::None,
        }
    }
}

/// Check if a crossterm KeyEvent matches a KeyBinding.
fn key_event_matches(event: KeyEvent, binding: &KeyBinding) -> bool {
    let event_binding = match crossterm_to_binding(event) {
        Some(b) => b,
        None => return false,
    };
    event_binding == *binding
}

/// Convert a crossterm KeyEvent to a KeyBinding.
fn crossterm_to_binding(event: KeyEvent) -> Option<KeyBinding> {
    let modifiers = Modifiers {
        ctrl: event.modifiers.contains(KeyModifiers::CONTROL),
        alt: event.modifiers.contains(KeyModifiers::ALT),
        shift: event.modifiers.contains(KeyModifiers::SHIFT),
    };

    let key = match event.code {
        KeyCode::Char(c) => Key::Char(c),
        KeyCode::F(n) => Key::F(n),
        KeyCode::Enter => Key::Enter,
        KeyCode::Esc => Key::Escape,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Tab => Key::Tab,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Insert => Key::Insert,
        KeyCode::Delete => Key::Delete,
        _ => return None,
    };

    Some(KeyBinding { key, modifiers })
}

/// Convert a crossterm KeyEvent to raw terminal bytes.
fn key_event_to_bytes(event: KeyEvent) -> KeyAction {
    let bytes = match event.code {
        KeyCode::Char(c) => {
            if event.modifiers.contains(KeyModifiers::CONTROL) {
                // Ctrl+A = 0x01, Ctrl+B = 0x02, etc.
                if c.is_ascii_lowercase() {
                    vec![c as u8 - b'a' + 1]
                } else if c.is_ascii_uppercase() {
                    vec![c as u8 - b'A' + 1]
                } else {
                    let mut buf = [0u8; 4];
                    let s = c.encode_utf8(&mut buf);
                    s.as_bytes().to_vec()
                }
            } else if event.modifiers.contains(KeyModifiers::ALT) {
                let mut bytes = vec![0x1b]; // ESC prefix for Alt
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                bytes.extend_from_slice(s.as_bytes());
                bytes
            } else {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                s.as_bytes().to_vec()
            }
        }
        KeyCode::Enter => vec![0x0D],
        KeyCode::Backspace => vec![0x7F],
        KeyCode::Tab => vec![0x09],
        KeyCode::Esc => vec![0x1B],
        KeyCode::Up => b"\x1b[A".to_vec(),
        KeyCode::Down => b"\x1b[B".to_vec(),
        KeyCode::Right => b"\x1b[C".to_vec(),
        KeyCode::Left => b"\x1b[D".to_vec(),
        KeyCode::Home => b"\x1b[H".to_vec(),
        KeyCode::End => b"\x1b[F".to_vec(),
        KeyCode::PageUp => b"\x1b[5~".to_vec(),
        KeyCode::PageDown => b"\x1b[6~".to_vec(),
        KeyCode::Insert => b"\x1b[2~".to_vec(),
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        KeyCode::F(n) => match n {
            1 => b"\x1bOP".to_vec(),
            2 => b"\x1bOQ".to_vec(),
            3 => b"\x1bOR".to_vec(),
            4 => b"\x1bOS".to_vec(),
            5 => b"\x1b[15~".to_vec(),
            6 => b"\x1b[17~".to_vec(),
            7 => b"\x1b[18~".to_vec(),
            8 => b"\x1b[19~".to_vec(),
            9 => b"\x1b[20~".to_vec(),
            10 => b"\x1b[21~".to_vec(),
            11 => b"\x1b[23~".to_vec(),
            12 => b"\x1b[24~".to_vec(),
            _ => return KeyAction::None,
        },
        _ => return KeyAction::None,
    };

    KeyAction::SendBytes(bytes)
}
