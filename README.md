# wtmux

> **Built with Claude Opus 4.6** — The entire codebase (~6,000 lines of Rust across 7 crates) was converted from tmux's C source using approximately 1M tokens via Claude Opus 4.6. Architecture design, code generation, debugging, and iteration were all done through AI-assisted development.

A Windows-native terminal multiplexer — [tmux](https://github.com/tmux/tmux) rewritten in Rust. The core concepts, key bindings, and command interface are ported from tmux, rebuilt entirely on Windows APIs (ConPTY, Named Pipes) — no WSL, no Cygwin required.

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

## tmux Compatibility

wtmux aims to provide a tmux-compatible experience on Windows. Below is a comprehensive comparison of what is supported and what is not yet implemented.

### Overall Coverage

| Area | tmux Total | wtmux Supported | Coverage |
|------|-----------|----------------|----------|
| CLI Subcommands | ~80 | ~25 | ~31% |
| Default Key Bindings | ~40 | ~22 | ~55% |
| Copy Mode Keys | ~40 | ~15 | ~38% |
| Configuration Options | 100+ | ~16 | ~15% |

### CLI Subcommands — Sessions & Clients

| tmux Command | Supported | Notes |
|---|---|---|
| `new-session` / `new` | ✅ | `-s`, `-c` options |
| `attach-session` / `attach` / `a` | ✅ | `-t` option |
| `list-sessions` / `ls` | ✅ | |
| `kill-session` | ✅ | `-t` option |
| `start-server` | ✅ | |
| `kill-server` | ❌ | Not yet implemented |
| `has-session` | ❌ | |
| `list-clients` | ❌ | |
| `list-commands` | ❌ | |
| `lock-client` / `lock-server` / `lock-session` | ❌ | |
| `refresh-client` | ❌ | |
| `show-messages` | ❌ | |
| `suspend-client` | ❌ | |
| `switch-client` | ❌ | |

### CLI Subcommands — Windows & Panes

| tmux Command | Supported | Notes |
|---|---|---|
| `split-window` | ✅ | `-h`, `-v` flags |
| `select-pane` | ✅ | `-U/-D/-L/-R`, `-t :.+` |
| `resize-pane` | ✅ | `-U/-D/-L/-R N`, `-Z` (zoom) |
| `kill-pane` | ✅ | |
| `last-pane` | ✅ | |
| `swap-pane` | ✅ | `-U`, `-D` |
| `new-window` | ✅ | `-n` flag |
| `select-window` | ✅ | `-t` flag |
| `next-window` / `previous-window` | ✅ | |
| `last-window` | ✅ | |
| `rename-window` | ✅ | |
| `kill-window` | ✅ | |
| `next-layout` | ✅ | |
| `copy-mode` | ✅ | `-u` flag |
| `paste-buffer` | ✅ | |
| `display-message` | ✅ | |
| `break-pane` | ❌ | Break pane into its own window |
| `capture-pane` | ❌ | Capture pane contents |
| `join-pane` | ❌ | Join pane from another window |
| `move-pane` / `move-window` | ❌ | |
| `swap-window` | ❌ | |
| `rotate-window` | ❌ | |
| `link-window` / `unlink-window` | ❌ | |
| `find-window` | ❌ | |
| `list-panes` / `list-windows` | ❌ | |
| `pipe-pane` | ❌ | Pipe pane output to a command |
| `display-panes` | ❌ | Pane number overlay |
| `previous-layout` / `select-layout` | ❌ | Only `next-layout` exists |
| `resize-window` | ❌ | |
| `respawn-pane` / `respawn-window` | ❌ | |
| `choose-tree` / `choose-client` | ❌ | Interactive selection UI |
| `send-keys` / `send-prefix` | ❌ | Critical for scripting |

### Key Bindings & Options Commands

| tmux Command | Supported | Notes |
|---|---|---|
| `bind-key` | ✅ | `-n` flag for no-prefix |
| `unbind-key` | ✅ | |
| `list-keys` | ✅ | |
| `set-option` | ✅ | `-g` flag, ~16 options only |
| `source-file` | ✅ | |
| `send-keys` / `send-prefix` | ❌ | |
| `set-window-option` (setw) | ❌ | |
| `show-options` / `show-window-options` | ❌ | |

### Buffer Commands

| tmux Command | Supported | Notes |
|---|---|---|
| `paste-buffer` | ✅ | |
| `choose-buffer` | ❌ | |
| `list-buffers` | ❌ | |
| `load-buffer` / `save-buffer` | ❌ | |
| `set-buffer` / `delete-buffer` / `show-buffer` / `clear-history` | ❌ | |

### Other Commands

| tmux Command | Supported | Notes |
|---|---|---|
| `clock-mode` | ✅ | |
| `command-prompt` | ✅ | Basic only (tmux `-I`, `-p` etc. not supported) |
| `confirm-before` | ❌ | |
| `display-menu` / `display-popup` | ❌ | |
| `if-shell` | ❌ | Conditional execution |
| `run-shell` | ❌ | External command execution |
| `wait-for` | ❌ | Synchronization channels |
| `set-environment` / `show-environment` | ❌ | |

### Default Key Bindings

| Key | tmux Action | Supported |
|---|---|---|
| `C-b "` | split-window (vertical) | ✅ |
| `C-b %` | split-window -h | ✅ |
| `C-b &` | kill-window | ✅ |
| `C-b ,` | rename-window | ✅ |
| `C-b $` | rename-session | ✅ |
| `C-b 0-9` | select-window | ✅ |
| `C-b c` | new-window | ✅ |
| `C-b d` | detach-client | ✅ |
| `C-b n` / `C-b p` | next/prev window | ✅ |
| `C-b l` | last-window | ✅ |
| `C-b o` | select next pane | ✅ |
| `C-b ;` | last-pane | ✅ |
| `C-b x` | kill-pane | ✅ |
| `C-b z` | resize-pane -Z (zoom) | ✅ |
| `C-b {` / `C-b }` | swap-pane | ✅ |
| `C-b Space` | next-layout | ✅ |
| `C-b [` / `C-b ]` | copy-mode / paste | ✅ |
| `C-b :` | command-prompt | ✅ |
| `C-b ?` | list-keys | ✅ |
| `C-b t` | clock-mode | ✅ |
| `C-b w` | choose-window | ❌ |
| `C-b !` | break-pane | ❌ |
| `C-b #` | list-buffers | ❌ |
| `C-b '` | select window by index prompt | ❌ |
| `C-b (` / `C-b )` | switch-client prev/next | ❌ |
| `C-b -` | delete-buffer | ❌ |
| `C-b .` | move-window prompt | ❌ |
| `C-b =` | choose-buffer | ❌ |
| `C-b C-o` | rotate-window | ❌ |
| `C-b C-z` | suspend-client | ❌ |
| `C-b D` | choose-client | ❌ |
| `C-b L` | switch-client -l | ❌ |
| `C-b M-1` ~ `M-5` | select-layout presets | ❌ |
| `C-b f` | find-window | ❌ |
| `C-b i` | display-message (window info) | ❌ |
| `C-b m` / `C-b M` | mark/unmark pane | ❌ |
| `C-b q` | display-panes | ❌ |
| `C-b r` | refresh-client | ❌ |
| `C-b s` | choose-tree (session picker) | ❌ |
| `C-b ~` | show-messages | ❌ |

### Copy Mode (vi mode)

| Key | Action | Supported |
|---|---|---|
| `h/j/k/l` | Cursor movement | ✅ |
| `0` / `$` | Start/end of line | ✅ |
| `g` / `G` | Top/bottom of history | ✅ |
| `C-u` / `C-d` | Half page up/down | ✅ |
| `PgUp` / `PgDn` | Full page up/down | ✅ |
| `Space` | Begin selection | ✅ |
| `Enter` | Copy selection & exit | ✅ |
| `/` / `?` | Search forward/backward | ✅ |
| `n` / `N` | Next/previous search result | ✅ |
| `q` / `Escape` | Exit copy mode | ✅ |
| `w` / `b` / `e` | Word movement | ❌ |
| `W` / `B` / `E` | WORD movement | ❌ |
| `f` / `F` / `t` / `T` | Character jump | ❌ |
| `;` / `,` | Repeat jump | ❌ |
| `{` / `}` | Paragraph movement | ❌ |
| `H` / `M` / `L` | Screen top/middle/bottom | ❌ |
| `^` | First non-blank character | ❌ |
| `v` | Rectangle toggle | ❌ |
| `V` | Select line | ❌ |
| `o` | Other end of selection | ❌ |
| `D` | Copy to end of line | ❌ |
| `A` | Append to selection | ❌ |

### Missing Options

wtmux supports ~16 options (see [Available Options](#available-options)). Notable tmux options **not yet supported** include:

- `mode-keys` — vi/emacs mode selection
- `pane-base-index`
- `set-titles` / `set-titles-string`
- `visual-activity` / `visual-bell` / `visual-silence`
- `monitor-activity` / `monitor-bell` / `monitor-silence`
- `remain-on-exit`
- `synchronize-panes`
- `aggressive-resize`
- `window-style` / `window-active-style`
- `allow-rename` / `allow-passthrough`
- `focus-events`
- `set-clipboard`
- `word-separators`
- `wrap-search`
- `@user-options` — User-defined options

### Missing Feature Categories

| Feature Area | Status | Impact |
|---|---|---|
| **Scripting/Automation** (`send-keys`, `run-shell`, `if-shell`) | Not supported | tmux scripts incompatible |
| **Hooks** (`after-*`, `before-*` events) | Not supported | Automation workflows unavailable |
| **Format variables** (`#{}`) | Very limited | Only `session_name` and time; tmux has 100+ |
| **Target specification** (`session:window.pane`) | Limited | Complex targeting unavailable |
| **Multiple key tables** (`-T` prefix/root/copy-mode etc.) | Not supported | Only prefix/default tables exist |
| **Interactive UI** (`choose-tree`, `choose-buffer`) | Not supported | No session/buffer picker UI |
| **Popup windows** (`display-popup`) | Not supported | |
| **Pane linking/moving** | Not supported | Cannot move panes across sessions |
| **Emacs copy mode** | Not supported | Only partial vi mode |
| **Environment variable management** | Not supported | |

### Summary

wtmux covers the **core daily workflow** well — session create/attach/detach, pane splitting, window switching, basic copy mode, and configuration. However, it does not yet match tmux's full feature set. Advanced features like scripting (`send-keys`, `run-shell`), hooks, interactive UIs (`choose-tree`), and extensive format variables are not implemented. Existing tmux automation scripts (complex `.tmux.conf`, tmuxinator, tmux-resurrect, etc.) are not compatible.

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
