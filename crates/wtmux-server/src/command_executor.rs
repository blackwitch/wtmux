use anyhow::Result;
use tracing::debug;
use wtmux_common::protocol::Direction;

use crate::server::ServerState;

/// Parse and execute a tmux-style command string.
pub fn execute_command(state: &mut ServerState, command: &str) -> Result<Option<String>> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Ok(None);
    }

    debug!("Executing command: {}", command);

    match parts[0] {
        "split-window" => {
            let horizontal = parts.contains(&"-h");
            let shell = state.config.options.default_shell.clone();
            if let Some(session) = state.active_session_mut() {
                session.active_window_mut().split_pane(&shell, horizontal)?;
            }
            Ok(None)
        }

        "new-window" => {
            let name = find_flag_value(&parts, "-n");
            let shell = state.config.options.default_shell.clone();
            if let Some(session) = state.active_session_mut() {
                let cols = session.active_window().area_width();
                let rows = session.active_window().area_height();
                session.new_window(name, &shell, cols, rows)?;
            }
            Ok(None)
        }

        "select-window" => {
            if let Some(target) = find_flag_value(&parts, "-t") {
                if let Ok(idx) = target.parse::<usize>() {
                    if let Some(session) = state.active_session_mut() {
                        session.select_window(idx);
                    }
                }
            }
            Ok(None)
        }

        "next-window" => {
            if let Some(session) = state.active_session_mut() {
                session.next_window();
            }
            Ok(None)
        }

        "previous-window" => {
            if let Some(session) = state.active_session_mut() {
                session.prev_window();
            }
            Ok(None)
        }

        "select-pane" => {
            // Check for -t :.+ (next pane)
            if let Some(target) = find_flag_value(&parts, "-t") {
                if target == ":.+" {
                    if let Some(session) = state.active_session_mut() {
                        session.active_window_mut().select_next_pane();
                    }
                    return Ok(None);
                }
            }

            let direction = if parts.contains(&"-U") {
                Some(Direction::Up)
            } else if parts.contains(&"-D") {
                Some(Direction::Down)
            } else if parts.contains(&"-L") {
                Some(Direction::Left)
            } else if parts.contains(&"-R") {
                Some(Direction::Right)
            } else {
                None
            };

            if let Some(dir) = direction {
                if let Some(session) = state.active_session_mut() {
                    session.active_window_mut().select_pane_direction(dir);
                }
            }
            Ok(None)
        }

        "resize-pane" => {
            if parts.contains(&"-Z") {
                // Zoom toggle
                if let Some(session) = state.active_session_mut() {
                    session.active_window_mut().toggle_zoom();
                }
            } else {
                // Direction-based resize
                let direction = if parts.contains(&"-U") {
                    Some(Direction::Up)
                } else if parts.contains(&"-D") {
                    Some(Direction::Down)
                } else if parts.contains(&"-L") {
                    Some(Direction::Left)
                } else if parts.contains(&"-R") {
                    Some(Direction::Right)
                } else {
                    None
                };

                let amount = parts.last()
                    .and_then(|s| s.parse::<u16>().ok())
                    .unwrap_or(1);

                if let Some(dir) = direction {
                    if let Some(session) = state.active_session_mut() {
                        let _ = session.active_window_mut().resize_pane_direction(dir, amount);
                    }
                }
            }
            Ok(None)
        }

        "kill-pane" => {
            if let Some(session) = state.active_session_mut() {
                let pane_id = session.active_pane_id();
                let window_empty = session.active_window_mut().close_pane(pane_id);
                if window_empty {
                    let win_id = session.active_window().id;
                    session.close_window(win_id);
                }
            }
            Ok(None)
        }

        "kill-window" => {
            if let Some(session) = state.active_session_mut() {
                let win_id = session.active_window().id;
                session.close_window(win_id);
            }
            Ok(None)
        }

        "rename-window" => {
            if let Some(name) = parts.get(1) {
                if let Some(session) = state.active_session_mut() {
                    session.active_window_mut().name = name.to_string();
                }
            }
            Ok(None)
        }

        "rename-session" => {
            if let Some(name) = parts.get(1) {
                if let Some(session) = state.active_session_mut() {
                    session.name = name.to_string();
                }
            }
            Ok(None)
        }

        "detach-client" => Ok(Some("__detach__".to_string())),

        "copy-mode" => Ok(Some("__copy_mode__".to_string())),

        "paste-buffer" => Ok(Some("__paste__".to_string())),

        "command-prompt" => Ok(Some("__command_prompt__".to_string())),

        "list-keys" => {
            let mut keys_text = String::from("Key bindings:\n");
            for (binding, cmd) in &state.config.key_table.bindings {
                keys_text.push_str(&format!("  {:?} -> {}\n", binding, cmd));
            }
            Ok(Some(keys_text))
        }

        "next-layout" => {
            if let Some(session) = state.active_session_mut() {
                let _ = session.active_window_mut().next_layout();
            }
            Ok(None)
        }

        "swap-pane" => {
            let up = parts.contains(&"-U");
            // -D is default if no flag
            if let Some(session) = state.active_session_mut() {
                let _ = session.active_window_mut().swap_pane(up);
            }
            Ok(None)
        }

        "last-pane" => {
            if let Some(session) = state.active_session_mut() {
                session.active_window_mut().select_last_pane();
            }
            Ok(None)
        }

        "last-window" => {
            if let Some(session) = state.active_session_mut() {
                session.select_last_window();
            }
            Ok(None)
        }

        "list-sessions" => Ok(Some("__list_sessions__".to_string())),

        "kill-session" => {
            if let Some(target) = find_flag_value(&parts, "-t") {
                return Ok(Some(format!("__kill_session__:{}", target)));
            }
            Ok(None)
        }

        "source-file" | "source" => {
            if let Some(path) = parts.get(1) {
                match std::fs::read_to_string(path) {
                    Ok(content) => {
                        if let Err(e) = state.config.apply_config_string(&content) {
                            return Ok(Some(format!("Error loading config: {}", e)));
                        }
                    }
                    Err(e) => {
                        return Ok(Some(format!("Error reading file: {}", e)));
                    }
                }
            }
            Ok(None)
        }

        "set-option" | "set" => {
            let args = parts[1..].join(" ");
            if let Err(e) = wtmux_config::parser::parse_set_option(&mut state.config.options, &args) {
                return Ok(Some(format!("Error: {}", e)));
            }
            Ok(None)
        }

        "clock-mode" => {
            // Display a clock in the current pane
            Ok(None)
        }

        "display-message" => {
            let msg = parts[1..].join(" ");
            Ok(Some(msg))
        }

        _ => Ok(Some(format!("Unknown command: {}", parts[0]))),
    }
}

fn find_flag_value<'a>(parts: &'a [&'a str], flag: &str) -> Option<String> {
    parts
        .iter()
        .position(|&p| p == flag)
        .and_then(|i| parts.get(i + 1))
        .map(|s| s.to_string())
}
