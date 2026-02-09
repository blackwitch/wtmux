use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A key combination that can trigger a binding.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyBinding {
    pub key: Key,
    pub modifiers: Modifiers,
}

/// Key identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Key {
    Char(char),
    F(u8),
    Enter,
    Escape,
    Backspace,
    Tab,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
    Space,
}

/// Key modifiers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

/// A table of key bindings mapping keys to command strings.
pub struct KeyTable {
    /// The prefix key (default: Ctrl-B).
    pub prefix: KeyBinding,
    /// Bindings active after prefix key is pressed.
    pub bindings: HashMap<KeyBinding, String>,
}

impl KeyTable {
    pub fn new(prefix: KeyBinding) -> Self {
        KeyTable {
            prefix,
            bindings: HashMap::new(),
        }
    }

    /// Create the default tmux-compatible key bindings.
    pub fn default_tmux_bindings() -> Self {
        let prefix = KeyBinding {
            key: Key::Char('b'),
            modifiers: Modifiers {
                ctrl: true,
                ..Default::default()
            },
        };

        let mut table = KeyTable::new(prefix);

        // Split panes
        table.bind(Key::Char('%'), Modifiers::default(), "split-window -h");
        table.bind(Key::Char('"'), Modifiers::default(), "split-window -v");

        // Window management
        table.bind(Key::Char('c'), Modifiers::default(), "new-window");
        table.bind(Key::Char('d'), Modifiers::default(), "detach-client");
        table.bind(Key::Char('n'), Modifiers::default(), "next-window");
        table.bind(Key::Char('p'), Modifiers::default(), "previous-window");
        table.bind(Key::Char('l'), Modifiers::default(), "last-window");
        table.bind(Key::Char('w'), Modifiers::default(), "choose-window");
        table.bind(Key::Char(','), Modifiers::default(), "rename-window");
        table.bind(Key::Char('$'), Modifiers::default(), "rename-session");
        table.bind(Key::Char('&'), Modifiers::default(), "kill-window");
        table.bind(Key::Char('x'), Modifiers::default(), "kill-pane");

        // Window select by number
        for i in 0..=9 {
            let ch = std::char::from_digit(i, 10).unwrap();
            table.bind(
                Key::Char(ch),
                Modifiers::default(),
                &format!("select-window -t {}", i),
            );
        }

        // Pane navigation
        table.bind(Key::Up, Modifiers::default(), "select-pane -U");
        table.bind(Key::Down, Modifiers::default(), "select-pane -D");
        table.bind(Key::Left, Modifiers::default(), "select-pane -L");
        table.bind(Key::Right, Modifiers::default(), "select-pane -R");

        // Pane resize
        table.bind(
            Key::Up,
            Modifiers { ctrl: true, ..Default::default() },
            "resize-pane -U 1",
        );
        table.bind(
            Key::Down,
            Modifiers { ctrl: true, ..Default::default() },
            "resize-pane -D 1",
        );
        table.bind(
            Key::Left,
            Modifiers { ctrl: true, ..Default::default() },
            "resize-pane -L 1",
        );
        table.bind(
            Key::Right,
            Modifiers { ctrl: true, ..Default::default() },
            "resize-pane -R 1",
        );

        // Zoom
        table.bind(Key::Char('z'), Modifiers::default(), "resize-pane -Z");

        // Copy mode
        table.bind(Key::Char('['), Modifiers::default(), "copy-mode");
        table.bind(Key::Char(']'), Modifiers::default(), "paste-buffer");
        table.bind(Key::PageUp, Modifiers::default(), "copy-mode -u");

        // Command prompt
        table.bind(Key::Char(':'), Modifiers::default(), "command-prompt");

        // Other
        table.bind(Key::Char('t'), Modifiers::default(), "clock-mode");
        table.bind(Key::Char('?'), Modifiers::default(), "list-keys");
        table.bind(Key::Char('o'), Modifiers::default(), "select-pane -t :.+");
        table.bind(Key::Char(';'), Modifiers::default(), "last-pane");
        table.bind(Key::Char('{'), Modifiers::default(), "swap-pane -U");
        table.bind(Key::Char('}'), Modifiers::default(), "swap-pane -D");
        table.bind(Key::Space, Modifiers::default(), "next-layout");

        table
    }

    /// Add a binding.
    pub fn bind(&mut self, key: Key, modifiers: Modifiers, command: &str) {
        let binding = KeyBinding {
            key,
            modifiers,
        };
        self.bindings.insert(binding, command.to_string());
    }

    /// Remove a binding.
    pub fn unbind(&mut self, key: Key, modifiers: Modifiers) {
        let binding = KeyBinding {
            key,
            modifiers,
        };
        self.bindings.remove(&binding);
    }

    /// Look up a command for a key binding.
    pub fn lookup(&self, binding: &KeyBinding) -> Option<&String> {
        self.bindings.get(binding)
    }
}

/// Parse a key string like "C-b", "M-a", "Up", "F1" into a KeyBinding.
pub fn parse_key(s: &str) -> Option<KeyBinding> {
    let mut modifiers = Modifiers::default();
    let mut remaining = s;

    // Parse modifiers
    loop {
        if remaining.starts_with("C-") || remaining.starts_with("c-") {
            modifiers.ctrl = true;
            remaining = &remaining[2..];
        } else if remaining.starts_with("M-") || remaining.starts_with("m-") {
            modifiers.alt = true;
            remaining = &remaining[2..];
        } else if remaining.starts_with("S-") || remaining.starts_with("s-") {
            modifiers.shift = true;
            remaining = &remaining[2..];
        } else {
            break;
        }
    }

    let key = match remaining {
        "Enter" | "enter" => Key::Enter,
        "Escape" | "escape" | "Esc" | "esc" => Key::Escape,
        "Space" | "space" => Key::Space,
        "Backspace" | "BSpace" | "bspace" => Key::Backspace,
        "Tab" | "tab" => Key::Tab,
        "Up" | "up" => Key::Up,
        "Down" | "down" => Key::Down,
        "Left" | "left" => Key::Left,
        "Right" | "right" => Key::Right,
        "Home" | "home" => Key::Home,
        "End" | "end" => Key::End,
        "PageUp" | "PgUp" | "pgup" => Key::PageUp,
        "PageDown" | "PgDn" | "pgdn" => Key::PageDown,
        "Insert" | "insert" => Key::Insert,
        "Delete" | "delete" | "DC" | "dc" => Key::Delete,
        s if s.starts_with('F') || s.starts_with('f') => {
            s[1..].parse::<u8>().ok().map(Key::F)?
        }
        s if s.len() == 1 => Key::Char(s.chars().next().unwrap()),
        _ => return None,
    };

    Some(KeyBinding { key, modifiers })
}
