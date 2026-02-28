use anyhow::{Context, Result};
use comfy_table::{Cell, Color, Table, presets::UTF8_BORDERS_ONLY};

use crate::client::Client;

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

pub async fn projects_list(client: &Client, workspace: &str, json_mode: bool) -> Result<()> {
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

    Ok(())
}

pub async fn states_list(
    client: &Client,
    workspace: &str,
    project: &str,
    json_mode: bool,
) -> Result<()> {
    let data = client
        .get(&format!(
            "workspaces/{workspace}/projects/{project}/states/"
        ))
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

    Ok(())
}

pub async fn labels_list(
    client: &Client,
    workspace: &str,
    project: &str,
    json_mode: bool,
) -> Result<()> {
    let data = client
        .get(&format!(
            "workspaces/{workspace}/projects/{project}/labels/"
        ))
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

    Ok(())
}

pub async fn members_list(
    client: &Client,
    workspace: &str,
    project: &str,
    json_mode: bool,
) -> Result<()> {
    let data = client
        .get(&format!(
            "workspaces/{workspace}/projects/{project}/members/"
        ))
        .await?;

    if json_mode {
        println!("{}", serde_json::to_string_pretty(&data)?);
    } else {
        let results = match data.as_array() {
            Some(arr) => arr,
            None => data["results"]
                .as_array()
                .context("unexpected response format")?,
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

    Ok(())
}

pub struct IssuesListParams<'a> {
    pub project: &'a str,
    pub state: Option<&'a str>,
    pub assignee: Option<&'a str>,
    pub per_page: u32,
    pub cursor: Option<&'a str>,
}

pub async fn issues_list(
    client: &Client,
    workspace: &str,
    params: &IssuesListParams<'_>,
    json_mode: bool,
) -> Result<()> {
    let per_page_str = params.per_page.to_string();
    let mut query: Vec<(&str, &str)> = vec![("per_page", &per_page_str)];
    if let Some(s) = params.state {
        query.push(("state", s));
    }
    if let Some(a) = params.assignee {
        query.push(("assignee", a));
    }
    if let Some(c) = params.cursor {
        query.push(("cursor", c));
    }

    let data = client
        .get_with_params(
            &format!("workspaces/{workspace}/projects/{}/issues/", params.project),
            &query,
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
        table.set_header(vec![
            header("#"),
            header("Name"),
            header("Priority"),
            header("ID"),
        ]);
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

    Ok(())
}

pub async fn issues_get(
    client: &Client,
    workspace: &str,
    project: &str,
    id: &str,
    json_mode: bool,
) -> Result<()> {
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
            let ids: Vec<&str> = assignees.iter().filter_map(|a| a.as_str()).collect();
            if !ids.is_empty() {
                println!("  {} {}", cyan.apply_to("assignees:"), ids.join(", "));
            }
        }

        if let Some(labels) = data["labels"].as_array() {
            let ids: Vec<&str> = labels.iter().filter_map(|l| l.as_str()).collect();
            if !ids.is_empty() {
                println!("  {} {}", cyan.apply_to("labels:  "), ids.join(", "));
            }
        }

        let desc = data["description_html"].as_str().unwrap_or("");
        if !desc.is_empty() {
            println!("\n  {}", dim.apply_to(desc));
        }
    }

    Ok(())
}

pub struct IssuesCreateParams<'a> {
    pub project: &'a str,
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub state: Option<&'a str>,
    pub priority: Option<&'a str>,
    pub assignees: &'a [String],
    pub labels: &'a [String],
}

pub async fn issues_create(
    client: &Client,
    workspace: &str,
    params: &IssuesCreateParams<'_>,
    json_mode: bool,
) -> Result<()> {
    let mut body = serde_json::json!({ "name": params.title });
    let obj = body.as_object_mut().unwrap();

    if let Some(desc) = params.description {
        obj.insert("description_html".to_string(), serde_json::json!(desc));
    }
    if let Some(s) = params.state {
        obj.insert("state".to_string(), serde_json::json!(s));
    }
    if let Some(p) = params.priority {
        obj.insert("priority".to_string(), serde_json::json!(p));
    }
    if !params.assignees.is_empty() {
        obj.insert("assignees".to_string(), serde_json::json!(params.assignees));
    }
    if !params.labels.is_empty() {
        obj.insert("labels".to_string(), serde_json::json!(params.labels));
    }

    let data = client
        .post(
            &format!("workspaces/{workspace}/projects/{}/issues/", params.project),
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

    Ok(())
}
