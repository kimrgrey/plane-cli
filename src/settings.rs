use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub api_key: Option<String>,
    pub base_url: String,
    pub workspace: Option<String>,
    pub timeout: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: "https://api.plane.so".to_string(),
            workspace: None,
            timeout: 30,
        }
    }
}

/// CLI overrides passed from clap arguments.
pub struct CliOverrides {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub workspace: Option<String>,
    pub timeout: Option<u64>,
}

impl Settings {
    /// Load settings with layered merging:
    /// defaults → config/settings.json → config/settings.local.json → env vars → CLI args
    pub fn load(cli: CliOverrides) -> Result<Self> {
        let mut value = serde_json::to_value(Settings::default())
            .context("failed to serialize default settings")?;

        let home = home_dir();

        let config_dir = home.join("config");

        // Layer 2: config/settings.json
        merge_file(&mut value, &config_dir.join("settings.json"))?;

        // Layer 3: config/settings.local.json
        merge_file(&mut value, &config_dir.join("settings.local.json"))?;

        // Layer 4: environment variables
        merge_env(&mut value);

        // Layer 5: CLI arguments (highest priority)
        merge_cli(&mut value, cli);

        let settings: Settings =
            serde_json::from_value(value).context("failed to parse merged settings")?;

        Ok(settings)
    }
}

fn merge_file(base: &mut serde_json::Value, path: &Path) -> Result<()> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            return Err(e).with_context(|| format!("failed to read {}", path.display()));
        }
    };

    let overlay: serde_json::Value =
        serde_json::from_str(&content).with_context(|| format!("invalid JSON in {}", path.display()))?;

    deep_merge(base, &overlay);
    Ok(())
}

/// Returns the settings home directory.
/// `PLANE_CLI_HOME` env var if set, otherwise the current directory.
fn home_dir() -> PathBuf {
    std::env::var("PLANE_CLI_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn merge_env(base: &mut serde_json::Value) {
    let map = base.as_object_mut().expect("base must be an object");

    if let Ok(v) = std::env::var("PLANE_CLI_API_KEY") {
        map.insert("api_key".to_string(), serde_json::Value::String(v));
    }
    if let Ok(v) = std::env::var("PLANE_CLI_BASE_URL") {
        map.insert("base_url".to_string(), serde_json::Value::String(v));
    }
    if let Ok(v) = std::env::var("PLANE_CLI_WORKSPACE") {
        map.insert("workspace".to_string(), serde_json::Value::String(v));
    }
    if let Ok(v) = std::env::var("PLANE_CLI_TIMEOUT") {
        if let Ok(n) = v.parse::<u64>() {
            map.insert("timeout".to_string(), serde_json::json!(n));
        }
    }
}

fn merge_cli(base: &mut serde_json::Value, cli: CliOverrides) {
    let map = base.as_object_mut().expect("base must be an object");

    if let Some(v) = cli.api_key {
        map.insert("api_key".to_string(), serde_json::Value::String(v));
    }
    if let Some(v) = cli.base_url {
        map.insert("base_url".to_string(), serde_json::Value::String(v));
    }
    if let Some(v) = cli.workspace {
        map.insert("workspace".to_string(), serde_json::Value::String(v));
    }
    if let Some(v) = cli.timeout {
        map.insert("timeout".to_string(), serde_json::json!(v));
    }
}

fn deep_merge(base: &mut serde_json::Value, overlay: &serde_json::Value) {
    match (base, overlay) {
        (serde_json::Value::Object(base_map), serde_json::Value::Object(overlay_map)) => {
            for (key, overlay_val) in overlay_map {
                let entry = base_map
                    .entry(key.clone())
                    .or_insert(serde_json::Value::Null);
                deep_merge(entry, overlay_val);
            }
        }
        (base, overlay) => {
            *base = overlay.clone();
        }
    }
}
