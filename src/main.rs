mod client;
mod settings;

use anyhow::Result;
use clap::Parser;
use settings::{CliOverrides, Settings};

#[derive(Parser)]
#[command(name = "plane", about = "CLI for Plane project management")]
struct Cli {
    /// Plane API key
    #[arg(long)]
    api_key: Option<String>,

    /// Plane API base URL
    #[arg(long)]
    base_url: Option<String>,

    /// Default workspace slug
    #[arg(long)]
    workspace: Option<String>,

    /// Request timeout in seconds
    #[arg(long)]
    timeout: Option<u64>,

    /// Output in JSON format
    #[arg(long)]
    json: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let json_mode = cli.json;

    let settings = Settings::load(CliOverrides {
        api_key: cli.api_key,
        base_url: cli.base_url,
        workspace: cli.workspace,
        timeout: cli.timeout,
    })?;

    if json_mode {
        println!("{}", serde_json::to_string_pretty(&settings)?);
    } else {
        let style = console::Style::new().bold();
        println!("{}", style.apply_to("Resolved settings:"));
        println!(
            "  api_key:   {}",
            settings
                .api_key
                .as_deref()
                .map(|k| format!("{}...", &k[..k.len().min(4)]))
                .unwrap_or_else(|| "(not set)".to_string())
        );
        println!("  base_url:  {}", settings.base_url);
        println!(
            "  workspace: {}",
            settings.workspace.as_deref().unwrap_or("(not set)")
        );
        println!("  timeout:   {}s", settings.timeout);
    }

    Ok(())
}
