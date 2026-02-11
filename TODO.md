# wtmux — tmux Compatibility TODO

Track progress toward full tmux feature parity.

## Priority 1 — Core Commands (High Impact)

- [ ] `kill-server` — Stop the server process
- [ ] `has-session` — Check if a session exists (exit code based)
- [ ] `send-keys` — Send key sequences to a pane (critical for scripting)
- [ ] `send-prefix` — Send the prefix key to a pane
- [ ] `list-panes` — List all panes in a window
- [ ] `list-windows` — List all windows in a session
- [ ] `list-clients` — List all connected clients
- [ ] `display-panes` — Show pane numbers as overlay (`C-b q`)
- [ ] `choose-tree` — Interactive session/window picker (`C-b s`)
- [ ] `choose-window` — Interactive window picker (`C-b w`)

## Priority 2 — Window & Pane Operations

- [ ] `break-pane` — Detach pane into its own window (`C-b !`)
- [ ] `join-pane` — Move pane from another window into current
- [ ] `swap-window` — Swap two windows
- [ ] `move-window` — Move window to another index (`C-b .`)
- [ ] `rotate-window` — Rotate pane positions (`C-b C-o`)
- [ ] `find-window` — Search for window by name/content (`C-b f`)
- [ ] `select-layout` — Set a specific layout preset
- [ ] `previous-layout` — Cycle layouts in reverse
- [ ] `resize-window` — Resize window to fit smallest/largest client
- [ ] `move-pane` — Move pane to another window
- [ ] `respawn-pane` — Restart command in a pane
- [ ] `respawn-window` — Restart command in a window
- [ ] `capture-pane` — Capture pane contents to a buffer
- [ ] `pipe-pane` — Pipe pane output to a shell command

## Priority 3 — Copy Mode Enhancements (vi mode)

- [ ] `w` / `b` / `e` — Word movement
- [ ] `W` / `B` / `E` — WORD movement (whitespace-delimited)
- [ ] `f` / `F` / `t` / `T` — Character jump (find/till)
- [ ] `;` / `,` — Repeat last character jump
- [ ] `{` / `}` — Paragraph movement
- [ ] `H` / `M` / `L` — Screen top/middle/bottom
- [ ] `^` — First non-blank character
- [ ] `v` — Rectangle (block) selection toggle
- [ ] `V` — Select entire line
- [ ] `o` — Move cursor to other end of selection
- [ ] `D` — Copy from cursor to end of line
- [ ] `A` — Append to existing selection

## Priority 4 — Buffer Management

- [ ] `list-buffers` — List all paste buffers (`C-b #`)
- [ ] `choose-buffer` — Interactive buffer picker (`C-b =`)
- [ ] `show-buffer` — Display buffer contents
- [ ] `set-buffer` — Set buffer contents manually
- [ ] `load-buffer` — Load buffer from file
- [ ] `save-buffer` — Save buffer to file
- [ ] `delete-buffer` — Delete a paste buffer (`C-b -`)
- [ ] `clear-history` — Clear pane scrollback history

## Priority 5 — Key Bindings & Default Bindings

- [ ] `C-b '` — Prompt to select window by index
- [ ] `C-b (` / `C-b )` — Switch to previous/next client session
- [ ] `C-b C-z` — Suspend client
- [ ] `C-b D` — Choose client to detach
- [ ] `C-b L` — Switch to last client session
- [ ] `C-b M-1` ~ `M-5` — Select layout presets by number
- [ ] `C-b i` — Display window info
- [ ] `C-b m` / `C-b M` — Mark/unmark pane
- [ ] `C-b r` — Refresh client
- [ ] `C-b ~` — Show message log
- [ ] Multiple key tables (`-T` flag for bind-key)

## Priority 6 — Options & Configuration

- [ ] `set-window-option` (setw) — Per-window options
- [ ] `show-options` — Display current option values
- [ ] `show-window-options` — Display window option values
- [ ] `mode-keys` — vi/emacs mode selection for copy mode
- [ ] `pane-base-index` — Starting index for panes
- [ ] `synchronize-panes` — Send input to all panes simultaneously
- [ ] `remain-on-exit` — Keep pane open after command exits
- [ ] `monitor-activity` / `monitor-bell` / `monitor-silence`
- [ ] `visual-activity` / `visual-bell` / `visual-silence`
- [ ] `aggressive-resize` — Resize based on smallest active client
- [ ] `window-style` / `window-active-style` — Per-window styling
- [ ] `set-titles` / `set-titles-string` — Terminal title
- [ ] `allow-rename` / `allow-passthrough`
- [ ] `focus-events` — Pass focus events to applications
- [ ] `set-clipboard` — Clipboard integration
- [ ] `word-separators` — Characters for word boundary detection
- [ ] `wrap-search` — Wrap search in copy mode
- [ ] `@user-options` — User-defined custom options

## Priority 7 — Scripting & Automation

- [ ] `if-shell` — Conditional command execution
- [ ] `run-shell` — Run external command
- [ ] `wait-for` — Synchronization channels between commands
- [ ] `confirm-before` — Confirmation prompt before command
- [ ] `display-menu` — Display interactive menu
- [ ] `display-popup` — Display popup window
- [ ] `set-environment` / `show-environment` — Environment variable management
- [ ] Hooks (`after-*`, `before-*` event callbacks)

## Priority 8 — Advanced Features

- [ ] `list-commands` — List all available commands
- [ ] `show-messages` — Show server message log
- [ ] `switch-client` — Switch client to a different session
- [ ] `link-window` / `unlink-window` — Share windows between sessions
- [ ] `lock-client` / `lock-server` / `lock-session` — Lock interface
- [ ] `refresh-client` — Force client redraw
- [ ] `suspend-client` — Suspend client process
- [ ] Emacs copy mode — Alternative key bindings for copy mode
- [ ] Extended format variables (`#{}`) — 100+ tmux format variables
- [ ] Full target specification (`session:window.pane` syntax)

---

_Last updated: 2025-02-11_
