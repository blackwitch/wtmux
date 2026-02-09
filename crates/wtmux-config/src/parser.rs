use crate::keybindings::{self, KeyTable};
use crate::options::Options;
use anyhow::Result;

/// Parse a `set-option` command line.
pub fn parse_set_option(options: &mut Options, args: &str) -> Result<()> {
    let args = args.trim();

    // Strip -g (global) flag
    let args = if args.starts_with("-g ") {
        &args[3..]
    } else {
        args
    };
    let args = args.trim();

    // Split into option name and value
    let (name, value) = match args.split_once(' ') {
        Some((n, v)) => (n.trim(), v.trim()),
        None => return Ok(()), // No value
    };

    options
        .set(name, value)
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Parse a `bind-key` command line.
pub fn parse_bind_key(table: &mut KeyTable, args: &str) -> Result<()> {
    let args = args.trim();

    // Optional -n flag (no prefix)
    let (_no_prefix, args) = if args.starts_with("-n ") {
        (true, &args[3..])
    } else {
        (false, args)
    };
    let args = args.trim();

    // Split into key and command
    let (key_str, command) = match args.split_once(' ') {
        Some((k, c)) => (k.trim(), c.trim()),
        None => return Ok(()),
    };

    if let Some(binding) = keybindings::parse_key(key_str) {
        table
            .bindings
            .insert(binding, command.to_string());
    }

    Ok(())
}

/// Parse an `unbind-key` command line.
pub fn parse_unbind_key(table: &mut KeyTable, args: &str) -> Result<()> {
    let key_str = args.trim();
    if let Some(binding) = keybindings::parse_key(key_str) {
        table.bindings.remove(&binding);
    }
    Ok(())
}
