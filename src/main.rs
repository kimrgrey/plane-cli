mod client;
mod settings;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use comfy_table::{presets::UTF8_BORDERS_ONLY, Cell, Color, Table};
use client::Client;
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

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Manage projects
    Projects {
        #[command(subcommand)]
        action: ProjectsAction,
    },
    /// Manage issues
    Issues {
        #[command(subcommand)]
        action: IssuesAction,
    },
    /// Manage states
    States {
        #[command(subcommand)]
        action: StatesAction,
    },
    /// Manage labels
    Labels {
        #[command(subcommand)]
        action: LabelsAction,
    },
    /// Manage members
    Members {
        #[command(subcommand)]
        action: MembersAction,
    },
}

#[derive(Subcommand)]
enum ProjectsAction {
    /// List projects in the workspace
    List,
}

#[derive(Subcommand)]
enum StatesAction {
    /// List states in a project
    List {
        /// Project ID
        #[arg(short, long)]
        project: String,
    },
}

#[derive(Subcommand)]
enum LabelsAction {
    /// List labels in a project
    List {
        /// Project ID
        #[arg(short, long)]
        project: String,
    },
}

#[derive(Subcommand)]
enum MembersAction {
    /// List members of a project
    List {
        /// Project ID
        #[arg(short, long)]
        project: String,
    },
}

#[derive(Subcommand)]
enum IssuesAction {
    /// List issues in a project
    List {
        /// Project ID
        #[arg(short, long)]
        project: String,

        /// Filter by state ID
        #[arg(long)]
        state: Option<String>,

        /// Filter by assignee ID
        #[arg(long)]
        assignee: Option<String>,

        /// Results per page
        #[arg(long, default_value = "50")]
        per_page: u32,

        /// Cursor for pagination (from previous response)
        #[arg(long)]
        cursor: Option<String>,
    },
    /// Get a single issue by ID
    Get {
        /// Project ID
        #[arg(short, long)]
        project: String,

        /// Issue ID
        #[arg(short, long)]
        id: String,
    },
    /// Create a new issue
    Create {
        /// Project ID
        #[arg(short, long)]
        project: String,

        /// Issue title
        #[arg(long)]
        title: String,

        /// Issue description (HTML)
        #[arg(long)]
        description: Option<String>,

        /// State ID
        #[arg(long)]
        state: Option<String>,

        /// Priority: none, urgent, high, medium, low
        #[arg(long)]
        priority: Option<String>,

        /// Assignee member IDs (can be repeated)
        #[arg(long)]
        assignee: Vec<String>,

        /// Label IDs (can be repeated)
        #[arg(long)]
        label: Vec<String>,
    },
}

fn header(name: &str) -> Cell {
    Cell::new(name).fg(Color::Cyan)
}

fn priority_cell(priority: &str) -> Cell {
    let color = match priority {
        "urgent" => Color::Red,
        "high" => Color::Yellow,
        "medium" => Color::Blue,
        "low" => Color::DarkGrey,
        _ => Color::Reset,
    };
    Cell::new(priority).fg(color)
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

    let workspace = settings
        .workspace
        .as_deref()
        .context("workspace is required — set it via --workspace, PLANE_CLI_WORKSPACE, or config file")?;

    let client = Client::new(&settings, json_mode)?;

    match cli.command {
        Command::Projects { action } => match action {
            ProjectsAction::List => {
                let data = client
                    .get(&format!("workspaces/{workspace}/projects/"))
                    .await?;

                if json_mode {
                    println!("{}", serde_json::to_string_pretty(&data)?);
                } else {
                    let results = data["results"]
                        .as_array()
                        .context("unexpected response format: missing 'results' array")?;

                    if results.is_empty() {
                        println!("No projects found.");
                        return Ok(());
                    }

                    let mut table = Table::new();
                    table.load_preset(UTF8_BORDERS_ONLY);
                    table.set_header(vec![header("Name"), header("Identifier"), header("ID")]);
                    for project in results {
                        table.add_row(vec![
                            Cell::new(project["name"].as_str().unwrap_or("(unnamed)")).fg(Color::White),
                            Cell::new(project["identifier"].as_str().unwrap_or("")),
                            Cell::new(project["id"].as_str().unwrap_or("")).fg(Color::DarkGrey),
                        ]);
                    }
                    println!("{table}");
                }
            }
        },
        Command::States { action } => match action {
            StatesAction::List { project } => {
                let data = client
                    .get(&format!("workspaces/{workspace}/projects/{project}/states/"))
                    .await?;

                if json_mode {
                    println!("{}", serde_json::to_string_pretty(&data)?);
                } else {
                    let results = data["results"]
                        .as_array()
                        .context("unexpected response format: missing 'results' array")?;

                    if results.is_empty() {
                        println!("No states found.");
                        return Ok(());
                    }

                    let mut table = Table::new();
                    table.load_preset(UTF8_BORDERS_ONLY);
                    table.set_header(vec![header("Name"), header("Group"), header("ID")]);
                    for state in results {
                        table.add_row(vec![
                            Cell::new(state["name"].as_str().unwrap_or("(unnamed)")).fg(Color::White),
                            Cell::new(state["group"].as_str().unwrap_or("")),
                            Cell::new(state["id"].as_str().unwrap_or("")).fg(Color::DarkGrey),
                        ]);
                    }
                    println!("{table}");
                }
            }
        },
        Command::Labels { action } => match action {
            LabelsAction::List { project } => {
                let data = client
                    .get(&format!("workspaces/{workspace}/projects/{project}/labels/"))
                    .await?;

                if json_mode {
                    println!("{}", serde_json::to_string_pretty(&data)?);
                } else {
                    let results = data["results"]
                        .as_array()
                        .context("unexpected response format: missing 'results' array")?;

                    if results.is_empty() {
                        println!("No labels found.");
                        return Ok(());
                    }

                    let mut table = Table::new();
                    table.load_preset(UTF8_BORDERS_ONLY);
                    table.set_header(vec![header("Name"), header("ID")]);
                    for label in results {
                        table.add_row(vec![
                            Cell::new(label["name"].as_str().unwrap_or("(unnamed)")).fg(Color::White),
                            Cell::new(label["id"].as_str().unwrap_or("")).fg(Color::DarkGrey),
                        ]);
                    }
                    println!("{table}");
                }
            }
        },
        Command::Members { action } => match action {
            MembersAction::List { project } => {
                let data = client
                    .get(&format!("workspaces/{workspace}/projects/{project}/members/"))
                    .await?;

                if json_mode {
                    println!("{}", serde_json::to_string_pretty(&data)?);
                } else {
                    let results = match data.as_array() {
                        Some(arr) => arr,
                        None => {
                            data["results"]
                                .as_array()
                                .context("unexpected response format")?
                        }
                    };

                    if results.is_empty() {
                        println!("No members found.");
                        return Ok(());
                    }

                    let mut table = Table::new();
                    table.load_preset(UTF8_BORDERS_ONLY);
                    table.set_header(vec![header("Name"), header("ID")]);
                    for member in results {
                        table.add_row(vec![
                            Cell::new(member["display_name"].as_str().unwrap_or("(unnamed)")).fg(Color::White),
                            Cell::new(member["id"].as_str().unwrap_or("")).fg(Color::DarkGrey),
                        ]);
                    }
                    println!("{table}");
                }
            }
        },
        Command::Issues { action } => match action {
            IssuesAction::List {
                project,
                state,
                assignee,
                per_page,
                cursor,
            } => {
                let per_page_str = per_page.to_string();
                let mut params: Vec<(&str, &str)> = vec![("per_page", &per_page_str)];
                if let Some(ref s) = state {
                    params.push(("state", s));
                }
                if let Some(ref a) = assignee {
                    params.push(("assignee", a));
                }
                if let Some(ref c) = cursor {
                    params.push(("cursor", c));
                }

                let data = client
                    .get_with_params(
                        &format!("workspaces/{workspace}/projects/{project}/issues/"),
                        &params,
                    )
                    .await?;

                if json_mode {
                    println!("{}", serde_json::to_string_pretty(&data)?);
                } else {
                    let results = data["results"]
                        .as_array()
                        .context("unexpected response format: missing 'results' array")?;

                    if results.is_empty() {
                        println!("No issues found.");
                        return Ok(());
                    }

                    let mut table = Table::new();
                    table.load_preset(UTF8_BORDERS_ONLY);
                    table.set_header(vec![header("#"), header("Name"), header("Priority"), header("ID")]);
                    for issue in results {
                        let prio = issue["priority"].as_str().unwrap_or("none");
                        table.add_row(vec![
                            Cell::new(issue["sequence_id"].to_string()).fg(Color::White),
                            Cell::new(issue["name"].as_str().unwrap_or("(unnamed)")),
                            priority_cell(prio),
                            Cell::new(issue["id"].as_str().unwrap_or("")).fg(Color::DarkGrey),
                        ]);
                    }
                    println!("{table}");
                }
            }
            IssuesAction::Get { project, id } => {
                let data = client
                    .get(&format!(
                        "workspaces/{workspace}/projects/{project}/issues/{id}/"
                    ))
                    .await?;

                if json_mode {
                    println!("{}", serde_json::to_string_pretty(&data)?);
                } else {
                    let cyan = console::Style::new().cyan();
                    let bold = console::Style::new().bold();
                    let dim = console::Style::new().dim();

                    let name = data["name"].as_str().unwrap_or("(unnamed)");
                    let seq = &data["sequence_id"];
                    let priority = data["priority"].as_str().unwrap_or("none");
                    let state_id = data["state"].as_str().unwrap_or("");
                    let created = data["created_at"].as_str().unwrap_or("");

                    println!("{} {}", bold.apply_to(seq), bold.apply_to(name));
                    println!("  {} {priority}", cyan.apply_to("priority:"));
                    println!("  {} {state_id}", cyan.apply_to("state:   "));
                    println!("  {} {created}", cyan.apply_to("created: "));

                    if let Some(assignees) = data["assignees"].as_array() {
                        let ids: Vec<&str> = assignees
                            .iter()
                            .filter_map(|a| a.as_str())
                            .collect();
                        if !ids.is_empty() {
                            println!("  {} {}", cyan.apply_to("assignees:"), ids.join(", "));
                        }
                    }

                    if let Some(labels) = data["labels"].as_array() {
                        let ids: Vec<&str> = labels
                            .iter()
                            .filter_map(|l| l.as_str())
                            .collect();
                        if !ids.is_empty() {
                            println!("  {} {}", cyan.apply_to("labels:  "), ids.join(", "));
                        }
                    }

                    let desc = data["description_html"].as_str().unwrap_or("");
                    if !desc.is_empty() {
                        println!("\n  {}", dim.apply_to(desc));
                    }
                }
            }
            IssuesAction::Create {
                project,
                title,
                description,
                state,
                priority,
                assignee,
                label,
            } => {
                let mut body = serde_json::json!({ "name": title });
                let obj = body.as_object_mut().unwrap();

                if let Some(desc) = description {
                    obj.insert("description_html".to_string(), serde_json::json!(desc));
                }
                if let Some(s) = state {
                    obj.insert("state".to_string(), serde_json::json!(s));
                }
                if let Some(p) = priority {
                    obj.insert("priority".to_string(), serde_json::json!(p));
                }
                if !assignee.is_empty() {
                    obj.insert("assignees".to_string(), serde_json::json!(assignee));
                }
                if !label.is_empty() {
                    obj.insert("labels".to_string(), serde_json::json!(label));
                }

                let data = client
                    .post(
                        &format!("workspaces/{workspace}/projects/{project}/issues/"),
                        &body,
                    )
                    .await?;

                if json_mode {
                    println!("{}", serde_json::to_string_pretty(&data)?);
                } else {
                    let id = data["id"].as_str().unwrap_or("");
                    let seq = &data["sequence_id"];
                    let name = data["name"].as_str().unwrap_or("");
                    let green = console::Style::new().green().bold();
                    let dim = console::Style::new().dim();
                    println!("{} #{} {}", green.apply_to("Created"), seq, name);
                    println!("  {}", dim.apply_to(id));
                }
            }
        },
    }

    Ok(())
}
