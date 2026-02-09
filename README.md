# wtmux

A Windows-native terminal multiplexer written in Rust. Inspired by tmux, built entirely on Windows APIs (ConPTY, Named Pipes) - no WSL, no Cygwin required.

```
Client (wtmux-client.exe)       Server (wtmux-server.exe)
+-----------------+             +------------------------------+
| Raw Terminal    |<--Named-->  | Session Manager              |
| Input/Output    |   Pipe      |  +-- Session 1               |
| Key Bindings    |   IPC       |  |   +-- Window 1            |
| Status Bar      |             |  |   |   +-- Pane 1 (ConPTY) |
+-----------------+             |  |   |   +-- Pane 2 (ConPTY) |
                                |  |   +-- Window 2            |
                                |  +-- Session 2               |
                                +------------------------------+
```

## Features

- **Persistent sessions** - Detach and reattach without losing state
- **Multi-pane split** - Horizontal and vertical splits with tree-based layout
- **Multi-window** - Create, switch, rename, and close windows
- **tmux-compatible keybindings** - Ctrl-B prefix with familiar shortcuts
- **Status bar** - Session name, window list, clock
- **Copy mode** - Vi-style navigation, text selection, paste buffer
- **Configuration** - `~/.wtmux.conf` with tmux-compatible syntax
- **VT100/ANSI terminal emulation** - Colors, attributes, cursor movement, alt screen
- **CJK wide character support** - Correct rendering of double-width characters

## Requirements

- Windows 10 version 1809+ (ConPTY support)
- Rust 1.70+ (for building from source)

## Installation

### PowerShell (recommended for quick setup)

```powershell
# Clone and install (builds from source, adds to PATH)
git clone https://github.com/petermn2/wtmux.git
cd wtmux
.\install.ps1
```

The installer will:
- Build release binaries (or use `-SkipBuild` if pre-built)
- Install to `%LOCALAPPDATA%\wtmux\`
- Add the install directory to user PATH
- Create a default config at `%USERPROFILE%\.wtmux.conf`

To uninstall:

```powershell
.\uninstall.ps1
```

### Scoop

```powershell
scoop bucket add wtmux https://github.com/petermn2/wtmux
scoop install wtmux
```

### Cargo

```powershell
# Install both binaries
cargo install --path crates/wtmux-client
cargo install --path crates/wtmux-server
```

### MSI Installer

Download the `.msi` from the [latest release](https://github.com/petermn2/wtmux/releases) and run it. This installs to `Program Files\wtmux` and adds to system PATH.

### Manual

```powershell
cargo build --release
```

Copy `target/release/wtmux-client.exe` and `target/release/wtmux-server.exe` to a directory in your PATH. Both binaries must be accessible.

## Quick Start

```powershell
# Start a new session (server starts automatically)
wtmux

# Start a named session
wtmux new-session -s work

# Detach with Ctrl-B d, then reattach
wtmux attach -t work

# List all sessions
wtmux ls
```

## CLI Commands

```
wtmux [COMMAND]

Commands:
  new-session   Create a new session (aliases: new)
  attach        Attach to an existing session (aliases: a)
  list-sessions List all sessions (aliases: ls)
  kill-session  Kill a session
  start-server  Start the server manually
  kill-server   Stop the server

Options for new-session:
  -s, --name <NAME>       Session name
  -c, --command <COMMAND>  Shell command (default: %COMSPEC%)

Options for attach:
  -t, --target <TARGET>   Target session name

Options for kill-session:
  -t, --target <TARGET>   Target session name
```

Running `wtmux` without arguments is equivalent to `wtmux new-session`.

## Key Bindings

All keybindings use the prefix key **Ctrl-B** (press Ctrl-B first, release, then press the action key).

### Pane Management

| Key | Action |
|-----|--------|
| `Ctrl-B %` | Split pane horizontally (left/right) |
| `Ctrl-B "` | Split pane vertically (top/bottom) |
| `Ctrl-B Arrow` | Navigate to pane in direction |
| `Ctrl-B Ctrl-Arrow` | Resize pane in direction |
| `Ctrl-B z` | Toggle zoom (maximize/restore active pane) |
| `Ctrl-B x` | Kill active pane |
| `Ctrl-B o` | Cycle to next pane |
| `Ctrl-B ;` | Switch to last active pane |
| `Ctrl-B {` | Swap pane up |
| `Ctrl-B }` | Swap pane down |
| `Ctrl-B Space` | Cycle through layout presets |

### Window Management

| Key | Action |
|-----|--------|
| `Ctrl-B c` | Create new window |
| `Ctrl-B n` | Next window |
| `Ctrl-B p` | Previous window |
| `Ctrl-B l` | Last (most recently used) window |
| `Ctrl-B 0-9` | Select window by number |
| `Ctrl-B ,` | Rename current window |
| `Ctrl-B &` | Kill current window |
| `Ctrl-B w` | Choose window from list |

### Session Management

| Key | Action |
|-----|--------|
| `Ctrl-B d` | Detach from session |
| `Ctrl-B $` | Rename current session |

### Copy Mode

| Key | Action |
|-----|--------|
| `Ctrl-B [` | Enter copy mode |
| `Ctrl-B ]` | Paste from buffer |
| `Ctrl-B PgUp` | Enter copy mode and scroll up |

In copy mode (vi-style):

| Key | Action |
|-----|--------|
| `h/j/k/l` | Move cursor left/down/up/right |
| `Ctrl-U / Ctrl-D` | Half page up/down |
| `PgUp / PgDn` | Full page up/down |
| `g / G` | Jump to top/bottom |
| `0 / $` | Start/end of line |
| `Space` | Start selection |
| `Enter` | Copy selection and exit |
| `/ / ?` | Search forward/backward |
| `n / N` | Next/previous search result |
| `q / Escape` | Exit copy mode |

### Other

| Key | Action |
|-----|--------|
| `Ctrl-B :` | Open command prompt |
| `Ctrl-B ?` | List all key bindings |
| `Ctrl-B t` | Show clock |

## Command Prompt

Press `Ctrl-B :` to open the command prompt. Type a command and press Enter.

### Available Commands

```
# Pane
split-window -h              # Split horizontally
split-window -v              # Split vertically
select-pane -U/-D/-L/-R      # Select pane by direction
resize-pane -Z               # Toggle zoom
kill-pane                     # Close active pane

# Window
new-window                    # Create window
new-window -n <name>          # Create named window
select-window -t <index>      # Select window by index
next-window                   # Next window
previous-window               # Previous window
rename-window <name>          # Rename window
kill-window                   # Close window

# Session
detach-client                 # Detach
rename-session <name>         # Rename session
list-sessions                 # List all sessions
kill-session -t <name>        # Kill a session

# Copy/Paste
copy-mode                     # Enter copy mode
paste-buffer                  # Paste from buffer

# Configuration
set-option -g <option> <val>  # Set a global option
source-file <path>            # Load config file
list-keys                     # Show key bindings
display-message <text>        # Show a message
```

## Configuration

wtmux reads `%USERPROFILE%\.wtmux.conf` on startup. The syntax is tmux-compatible.

### Example Configuration

```bash
# Change prefix key to Ctrl-A
set-option -g prefix C-a

# Set default shell to PowerShell
set-option -g default-shell "C:\Program Files\PowerShell\7\pwsh.exe"

# Status bar
set-option -g status-left "[#{session_name}] "
set-option -g status-right " %H:%M %Y-%m-%d"
set-option -g status-style fg=white,bg=blue

# Start window numbering at 1
set-option -g base-index 1

# Scrollback buffer size
set-option -g history-limit 5000

# Escape key delay (ms)
set-option -g escape-time 100

# Custom key bindings
bind-key v split-window -h
bind-key s split-window -v
unbind-key %
```

### Available Options

| Option | Default | Description |
|--------|---------|-------------|
| `prefix` | `C-b` | Prefix key combination |
| `default-shell` | `%COMSPEC%` | Default shell for new panes |
| `default-terminal` | `xterm-256color` | Terminal type |
| `base-index` | `0` | Starting index for windows |
| `history-limit` | `2000` | Scrollback buffer lines |
| `escape-time` | `500` | Escape key delay (ms) |
| `status` | `on` | Show/hide status bar |
| `status-left` | `[#{session_name}] ` | Status bar left format |
| `status-right` | ` %H:%M %Y-%m-%d` | Status bar right format |
| `status-style` | `fg=black,bg=green` | Status bar colors |
| `status-interval` | `1` | Status refresh interval (s) |
| `mouse` | `off` | Enable mouse support |
| `renumber-windows` | `off` | Renumber after closing |
| `automatic-rename` | `on` | Auto-rename windows |
| `pane-border-style` | `default` | Inactive pane border style |
| `pane-active-border-style` | `fg=green` | Active pane border style |
| `display-time` | `750` | Message display duration (ms) |

### Format Variables

Use in `status-left` and `status-right`:

| Variable | Description |
|----------|-------------|
| `#{session_name}` | Current session name |
| `%H` | Hour (00-23) |
| `%M` | Minute (00-59) |
| `%Y` | Year (4 digits) |
| `%m` | Month (01-12) |
| `%d` | Day (01-31) |

## Architecture

```
wtmux/
+-- Cargo.toml                  # Workspace root
+-- install.ps1                 # PowerShell installer
+-- uninstall.ps1               # PowerShell uninstaller
+-- Makefile.toml               # cargo-make automation
+-- LICENSE                     # MIT license
+-- crates/
|   +-- wtmux-server/           # Server binary
|   |   +-- server.rs           #   Event loop, client handling
|   |   +-- session.rs          #   Session management
|   |   +-- window.rs           #   Window + pane container
|   |   +-- pane.rs             #   Pane (ConPTY + terminal)
|   |   +-- renderer.rs         #   Screen composition
|   |   +-- command_executor.rs #   Command dispatch
|   |   +-- copymode.rs         #   Copy mode state
|   |   +-- pastebuffer.rs      #   Paste buffer stack
|   +-- wtmux-client/           # Client binary
|   |   +-- main.rs             #   CLI, interactive loop
|   |   +-- input_handler.rs    #   Prefix key state machine
|   +-- wtmux-common/           # Shared library
|   |   +-- protocol.rs         #   Client/Server message types
|   |   +-- ipc.rs              #   Named pipe helpers
|   |   +-- error.rs            #   Error types
|   +-- wtmux-pty/              # PTY library
|   |   +-- conpty.rs           #   ConPTY lifecycle
|   |   +-- process.rs          #   Job Object management
|   +-- wtmux-terminal/         # Terminal emulation library
|   |   +-- terminal.rs         #   VT parser + renderer
|   |   +-- grid.rs             #   2D cell grid
|   |   +-- cell.rs             #   Cell (char, color, attrs)
|   |   +-- parser.rs           #   vte::Perform implementation
|   |   +-- scrollback.rs       #   Ring buffer
|   |   +-- statusbar.rs        #   Status bar rendering
|   +-- wtmux-layout/           # Layout library
|   |   +-- lib.rs              #   Tree-based layout engine
|   |   +-- geometry.rs         #   Rect type
|   +-- wtmux-config/           # Configuration library
|       +-- config.rs           #   Config loading
|       +-- keybindings.rs      #   Key table + parsing
|       +-- options.rs          #   Option definitions
|       +-- parser.rs           #   Config syntax parser
+-- scoop/                      # Scoop package manifest
|   +-- wtmux.json
+-- wix/                        # WiX MSI installer definition
|   +-- main.wxs
+-- .github/workflows/          # CI/CD
    +-- release.yml             #   Automated release on tag push
```

### IPC Protocol

Client and server communicate over Windows Named Pipes (`\\.\pipe\wtmux-{username}`) using length-prefixed bincode serialization.

```
+--------+-------------------+
| 4 byte |   N bytes         |
| length |   bincode payload |
| (LE)   |   (ClientMessage  |
|        |    or ServerMsg)   |
+--------+-------------------+
```

### Layout Engine

Panes are arranged using a binary tree. Each node is either a leaf (single pane) or a split (horizontal/vertical with child nodes and ratios).

Built-in layouts: even-horizontal, even-vertical, main-horizontal, main-vertical, tiled.

## Development

### Build

```powershell
cargo build            # Debug
cargo build --release  # Release
cargo test             # Run tests
```

### cargo-make

Install [cargo-make](https://github.com/sagiegurari/cargo-make) for automated tasks:

```powershell
cargo install cargo-make

cargo make build          # Debug build
cargo make release        # Release build
cargo make test           # Run tests
cargo make install-local  # Build + install to LOCALAPPDATA
cargo make package-zip    # Create release zip archive
cargo make install-cargo  # cargo install both binaries
```

### Release Process

Releases are automated via GitHub Actions. Push a version tag to trigger:

```powershell
git tag v0.1.0
git push origin v0.1.0
```

This will:
1. Build release binaries (`x86_64-pc-windows-msvc`)
2. Run tests
3. Create a zip archive
4. Build an MSI installer
5. Publish a GitHub Release with all artifacts

## Logging

Set the `RUST_LOG` environment variable to control log output:

```powershell
$env:RUST_LOG="debug"
wtmux-server.exe    # Logs to stdout

$env:RUST_LOG="info"
wtmux                # Client logs to stderr
```

## License

[MIT](LICENSE)
