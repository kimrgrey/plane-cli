# plane-cli

CLI for [Plane](https://plane.so) project management.

## Why

This CLI is designed to be used by AI coding agents (like Claude) to automate routine project management tasks: creating follow-up issues after implementing a feature (e.g. frontend issues after backend work), moving issues between statuses, assigning work, adding labels, and other repetitive operations that shouldn't require opening a browser. The `--json` mode makes it easy for agents to parse responses, while the human-readable mode with colored tables and spinners keeps it pleasant for manual use too.

## Installation

```bash
cargo install --path .
```

## Configuration

Settings are loaded in layers (each overrides the previous):

1. Hardcoded defaults
2. `{home}/config/settings.json`
3. `{home}/config/settings.local.json` (git-ignored)
4. Environment variables
5. CLI arguments

`{home}` is the `PLANE_CLI_HOME` env var if set, otherwise the current directory.

### Environment variables

| Variable | Description |
|---|---|
| `PLANE_CLI_HOME` | Settings home directory |
| `PLANE_CLI_API_KEY` | Plane API key |
| `PLANE_CLI_BASE_URL` | Plane API base URL |
| `PLANE_CLI_WORKSPACE` | Default workspace slug |
| `PLANE_CLI_TIMEOUT` | Request timeout in seconds |

### Example `config/settings.json`

```json
{
    "base_url": "https://api.plane.so",
    "timeout": 30
}
```

### Example `config/settings.local.json`

```json
{
    "base_url": "https://plane.example.com",
    "api_key": "plane_api_...",
    "workspace": "my-team"
}
```

## Global options

| Option | Description |
|---|---|
| `--api-key <KEY>` | Plane API key |
| `--base-url <URL>` | Plane API base URL |
| `--workspace <SLUG>` | Default workspace slug |
| `--timeout <SECS>` | Request timeout in seconds |
| `--json` | Output in JSON format |

## Commands

### Projects

```bash
# List all projects in the workspace
plane-cli projects list
```

### Issues

```bash
# List issues in a project
plane-cli issues list -p <PROJECT_ID>
plane-cli issues list -p <PROJECT_ID> --per-page 10

# Get a single issue
plane-cli issues get -p <PROJECT_ID> -i <ISSUE_ID>

# Create an issue
plane-cli issues create -p <PROJECT_ID> --title "Fix login bug"
plane-cli issues create -p <PROJECT_ID> \
  --title "Add feature" \
  --description "<p>Description in HTML</p>" \
  --state <STATE_ID> \
  --priority high \
  --assignee <MEMBER_ID> \
  --label <LABEL_ID>
```

Priority values: `none`, `low`, `medium`, `high`, `urgent`.

### States

```bash
# List states in a project
plane-cli states list -p <PROJECT_ID>
```

### Labels

```bash
# List labels in a project
plane-cli labels list -p <PROJECT_ID>
```

### Members

```bash
# List members of a project
plane-cli members list -p <PROJECT_ID>
```

### JSON output

Append `--json` to any command to get raw JSON output, suitable for piping to `jq`:

```bash
plane-cli --json projects list | jq '.results[].name'
plane-cli --json issues list -p <PROJECT_ID> | jq '.results[] | {name, priority}'
```

## License

[MIT](LICENSE)
