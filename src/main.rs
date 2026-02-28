mod client;
mod commands;
mod settings;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use client::Client;
use commands::{IssuesCreateParams, IssuesListParams};
use settings::{CliOverrides, Settings};

#[derive(Parser)]
#[command(name = "plane", version, about = "CLI for Plane project management")]
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

        /// Priority level
        #[arg(long, value_enum)]
        priority: Option<Priority>,

        /// Assignee member IDs (can be repeated)
        #[arg(long)]
        assignee: Vec<String>,

        /// Label IDs (can be repeated)
        #[arg(long)]
        label: Vec<String>,
    },
}

#[derive(Clone, ValueEnum)]
enum Priority {
    None,
    Urgent,
    High,
    Medium,
    Low,
}

impl Priority {
    fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Urgent => "urgent",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

fn main() {
    let cli = Cli::parse();

    if let Err(err) = run(cli) {
        let style = console::Style::new().red().bold();
        eprintln!("{} {err:#}", style.apply_to("error:"));
        std::process::exit(1);
    }
}

#[tokio::main]
async fn run(cli: Cli) -> Result<()> {
    let json_mode = cli.json;

    let settings = Settings::load(CliOverrides {
        api_key: cli.api_key,
        base_url: cli.base_url,
        workspace: cli.workspace,
        timeout: cli.timeout,
    })?;

    let workspace = settings.workspace.as_deref().context(
        "workspace is required — set it via --workspace, PLANE_CLI_WORKSPACE, or config file",
    )?;

    let client = Client::new(&settings, json_mode)?;

    match cli.command {
        Command::Projects { action } => match action {
            ProjectsAction::List => {
                commands::projects_list(&client, workspace, json_mode).await?;
            }
        },
        Command::States { action } => match action {
            StatesAction::List { project } => {
                commands::states_list(&client, workspace, &project, json_mode).await?;
            }
        },
        Command::Labels { action } => match action {
            LabelsAction::List { project } => {
                commands::labels_list(&client, workspace, &project, json_mode).await?;
            }
        },
        Command::Members { action } => match action {
            MembersAction::List { project } => {
                commands::members_list(&client, workspace, &project, json_mode).await?;
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
                commands::issues_list(
                    &client,
                    workspace,
                    &IssuesListParams {
                        project: &project,
                        state: state.as_deref(),
                        assignee: assignee.as_deref(),
                        per_page,
                        cursor: cursor.as_deref(),
                    },
                    json_mode,
                )
                .await?;
            }
            IssuesAction::Get { project, id } => {
                commands::issues_get(&client, workspace, &project, &id, json_mode).await?;
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
                commands::issues_create(
                    &client,
                    workspace,
                    &IssuesCreateParams {
                        project: &project,
                        title: &title,
                        description: description.as_deref(),
                        state: state.as_deref(),
                        priority: priority.as_ref().map(Priority::as_str),
                        assignees: &assignee,
                        labels: &label,
                    },
                    json_mode,
                )
                .await?;
            }
        },
    }

    Ok(())
}
