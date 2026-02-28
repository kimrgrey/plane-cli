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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    fn empty_cli() -> CliOverrides {
        CliOverrides {
            api_key: None,
            base_url: None,
            workspace: None,
            timeout: None,
        }
    }

    // ── deep_merge ──

    #[test]
    fn test_deep_merge_flat_override() {
        let mut base = serde_json::json!({"a": 1, "b": 2});
        let overlay = serde_json::json!({"b": 99});
        deep_merge(&mut base, &overlay);
        assert_eq!(base, serde_json::json!({"a": 1, "b": 99}));
    }

    #[test]
    fn test_deep_merge_adds_new_keys() {
        let mut base = serde_json::json!({"a": 1});
        let overlay = serde_json::json!({"b": 2});
        deep_merge(&mut base, &overlay);
        assert_eq!(base, serde_json::json!({"a": 1, "b": 2}));
    }

    #[test]
    fn test_deep_merge_nested_objects() {
        let mut base = serde_json::json!({"outer": {"a": 1, "b": 2}});
        let overlay = serde_json::json!({"outer": {"b": 99, "c": 3}});
        deep_merge(&mut base, &overlay);
        assert_eq!(base, serde_json::json!({"outer": {"a": 1, "b": 99, "c": 3}}));
    }

    // ── merge_file ──

    #[test]
    fn test_merge_file_applies_values() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, r#"{"timeout": 60}"#).unwrap();

        let mut base = serde_json::json!({"timeout": 30, "base_url": "https://example.com"});
        merge_file(&mut base, &path).unwrap();

        assert_eq!(base["timeout"], 60);
        assert_eq!(base["base_url"], "https://example.com");
    }

    #[test]
    fn test_merge_file_missing_file_is_ok() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.json");

        let mut base = serde_json::json!({"timeout": 30});
        let result = merge_file(&mut base, &path);

        assert!(result.is_ok());
        assert_eq!(base["timeout"], 30);
    }

    #[test]
    fn test_merge_file_invalid_json_returns_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bad.json");
        std::fs::write(&path, "not json!").unwrap();

        let mut base = serde_json::json!({"timeout": 30});
        let result = merge_file(&mut base, &path);

        assert!(result.is_err());
        let msg = format!("{:#}", result.unwrap_err());
        assert!(msg.contains("bad.json"), "error should mention file: {msg}");
    }

    // ── merge_env ──

    #[test]
    #[serial]
    fn test_merge_env_reads_all_vars() {
        temp_env::with_vars(
            [
                ("PLANE_CLI_API_KEY", Some("key123")),
                ("PLANE_CLI_BASE_URL", Some("https://custom.api")),
                ("PLANE_CLI_WORKSPACE", Some("my-ws")),
                ("PLANE_CLI_TIMEOUT", Some("120")),
            ],
            || {
                let mut base = serde_json::to_value(Settings::default()).unwrap();
                merge_env(&mut base);

                assert_eq!(base["api_key"], "key123");
                assert_eq!(base["base_url"], "https://custom.api");
                assert_eq!(base["workspace"], "my-ws");
                assert_eq!(base["timeout"], 120);
            },
        );
    }

    #[test]
    #[serial]
    fn test_merge_env_skips_unset_vars() {
        temp_env::with_vars(
            [
                ("PLANE_CLI_API_KEY", None::<&str>),
                ("PLANE_CLI_BASE_URL", None::<&str>),
                ("PLANE_CLI_WORKSPACE", None::<&str>),
                ("PLANE_CLI_TIMEOUT", None::<&str>),
            ],
            || {
                let mut base = serde_json::to_value(Settings::default()).unwrap();
                let before = base.clone();
                merge_env(&mut base);

                assert_eq!(base, before);
            },
        );
    }

    #[test]
    #[serial]
    fn test_merge_env_ignores_non_numeric_timeout() {
        temp_env::with_vars(
            [
                ("PLANE_CLI_TIMEOUT", Some("abc")),
                ("PLANE_CLI_API_KEY", None::<&str>),
                ("PLANE_CLI_BASE_URL", None::<&str>),
                ("PLANE_CLI_WORKSPACE", None::<&str>),
            ],
            || {
                let mut base = serde_json::to_value(Settings::default()).unwrap();
                merge_env(&mut base);

                assert_eq!(base["timeout"], 30); // default unchanged
            },
        );
    }

    // ── merge_cli ──

    #[test]
    fn test_merge_cli_applies_all_overrides() {
        let mut base = serde_json::to_value(Settings::default()).unwrap();
        merge_cli(
            &mut base,
            CliOverrides {
                api_key: Some("cli-key".to_string()),
                base_url: Some("https://cli.api".to_string()),
                workspace: Some("cli-ws".to_string()),
                timeout: Some(99),
            },
        );

        assert_eq!(base["api_key"], "cli-key");
        assert_eq!(base["base_url"], "https://cli.api");
        assert_eq!(base["workspace"], "cli-ws");
        assert_eq!(base["timeout"], 99);
    }

    #[test]
    fn test_merge_cli_skips_none_fields() {
        let mut base = serde_json::to_value(Settings::default()).unwrap();
        let before = base.clone();
        merge_cli(&mut base, empty_cli());

        assert_eq!(base, before);
    }

    // ── Settings::load integration ──

    #[test]
    #[serial]
    fn test_load_defaults_only() {
        let dir = TempDir::new().unwrap();
        temp_env::with_vars(
            [
                ("PLANE_CLI_HOME", Some(dir.path().to_str().unwrap())),
                ("PLANE_CLI_API_KEY", None::<&str>),
                ("PLANE_CLI_BASE_URL", None::<&str>),
                ("PLANE_CLI_WORKSPACE", None::<&str>),
                ("PLANE_CLI_TIMEOUT", None::<&str>),
            ],
            || {
                let s = Settings::load(empty_cli()).unwrap();
                assert_eq!(s.base_url, "https://api.plane.so");
                assert_eq!(s.timeout, 30);
                assert!(s.api_key.is_none());
                assert!(s.workspace.is_none());
            },
        );
    }

    #[test]
    #[serial]
    fn test_load_file_overrides_defaults() {
        let dir = TempDir::new().unwrap();
        let config_dir = dir.path().join("config");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("settings.json"),
            r#"{"timeout": 60}"#,
        )
        .unwrap();

        temp_env::with_vars(
            [
                ("PLANE_CLI_HOME", Some(dir.path().to_str().unwrap())),
                ("PLANE_CLI_API_KEY", None::<&str>),
                ("PLANE_CLI_BASE_URL", None::<&str>),
                ("PLANE_CLI_WORKSPACE", None::<&str>),
                ("PLANE_CLI_TIMEOUT", None::<&str>),
            ],
            || {
                let s = Settings::load(empty_cli()).unwrap();
                assert_eq!(s.timeout, 60);
                assert_eq!(s.base_url, "https://api.plane.so"); // default kept
            },
        );
    }

    #[test]
    #[serial]
    fn test_load_local_overrides_base() {
        let dir = TempDir::new().unwrap();
        let config_dir = dir.path().join("config");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("settings.json"),
            r#"{"timeout": 60, "base_url": "https://base.api"}"#,
        )
        .unwrap();
        std::fs::write(
            config_dir.join("settings.local.json"),
            r#"{"timeout": 90}"#,
        )
        .unwrap();

        temp_env::with_vars(
            [
                ("PLANE_CLI_HOME", Some(dir.path().to_str().unwrap())),
                ("PLANE_CLI_API_KEY", None::<&str>),
                ("PLANE_CLI_BASE_URL", None::<&str>),
                ("PLANE_CLI_WORKSPACE", None::<&str>),
                ("PLANE_CLI_TIMEOUT", None::<&str>),
            ],
            || {
                let s = Settings::load(empty_cli()).unwrap();
                assert_eq!(s.timeout, 90);
                assert_eq!(s.base_url, "https://base.api"); // from settings.json
            },
        );
    }

    #[test]
    #[serial]
    fn test_load_env_overrides_file() {
        let dir = TempDir::new().unwrap();
        let config_dir = dir.path().join("config");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("settings.json"),
            r#"{"timeout": 60}"#,
        )
        .unwrap();

        temp_env::with_vars(
            [
                ("PLANE_CLI_HOME", Some(dir.path().to_str().unwrap())),
                ("PLANE_CLI_TIMEOUT", Some("200")),
                ("PLANE_CLI_API_KEY", None::<&str>),
                ("PLANE_CLI_BASE_URL", None::<&str>),
                ("PLANE_CLI_WORKSPACE", None::<&str>),
            ],
            || {
                let s = Settings::load(empty_cli()).unwrap();
                assert_eq!(s.timeout, 200);
            },
        );
    }

    #[test]
    #[serial]
    fn test_load_cli_overrides_everything() {
        let dir = TempDir::new().unwrap();
        let config_dir = dir.path().join("config");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("settings.json"),
            r#"{"timeout": 60}"#,
        )
        .unwrap();

        temp_env::with_vars(
            [
                ("PLANE_CLI_HOME", Some(dir.path().to_str().unwrap())),
                ("PLANE_CLI_TIMEOUT", Some("200")),
                ("PLANE_CLI_API_KEY", Some("env-key")),
                ("PLANE_CLI_BASE_URL", None::<&str>),
                ("PLANE_CLI_WORKSPACE", None::<&str>),
            ],
            || {
                let cli = CliOverrides {
                    api_key: Some("cli-key".to_string()),
                    base_url: None,
                    workspace: None,
                    timeout: Some(999),
                };
                let s = Settings::load(cli).unwrap();
                assert_eq!(s.timeout, 999);
                assert_eq!(s.api_key.as_deref(), Some("cli-key"));
            },
        );
    }
}
