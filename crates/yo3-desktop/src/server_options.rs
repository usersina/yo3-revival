//! One source of truth for advanced run flags: clap's server definitions. Values are argv entries, never shell text.
use anyhow::{ensure, Context};
use crate::Settings;

/// Settings with dedicated controls, or paths owned by the app so process attachment and log export keep working.
pub fn managed(name: &str) -> Option<&'static str> {
    match name {
        "bind" | "auth-port" | "advertise" => Some("Set in Hosting → Network."),
        "data-dir" | "players" => Some("Set in Hosting → Server data."),
        "game-dir" => Some("Set in Game setup."),
        "regulation" => Some("Set in Hosting → Overview → Banlist."),
        "log-dir" | "diagnostics-dir" | "diagnostic-parent" => Some("Managed by the app for Activity and Export logs. Use Scripts edition to choose these paths directly."),
        _ => None,
    }
}
pub fn arguments(settings: &Settings) -> anyhow::Result<Vec<String>> {
    let catalogue=yo3_server::cli::server_options(); let mut args=Vec::new();
    for (name,value) in &settings.server_options {
        ensure!(managed(name).is_none(), "--{name} has a dedicated application control and cannot be overridden here.");
        let option=catalogue.iter().find(|o| &o.name==name).with_context(||format!("This server does not support --{name}. Remove the saved override."))?;
        if option.toggle {
            match value.as_str() { "true" => args.push(format!("--{name}")), "false" => {}, _ => anyhow::bail!("--{name} must be true or false.") }
        } else { args.push(format!("--{name}={value}")); }
    }
    yo3_server::cli::validate_server_args(args.iter().map(String::as_str)).map_err(anyhow::Error::msg)?;
    Ok(args)
}
