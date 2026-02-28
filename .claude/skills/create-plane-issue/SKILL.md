---
name: create-plane-issue
description: Create a new issue in Plane using plane-cli. Interactively asks for project, state, priority, assignees, and labels.
allowed-tools: Bash, AskUserQuestion
---

# Create Plane Issue

Create a new issue in Plane via the CLI. The user provides a title and description; remaining fields are resolved interactively.

## Context

Available projects: !`cargo run -- projects list 2>/dev/null`

## Steps

1. **Identify the project** from the user's request or ask which project to use. Use the project list from Context above to match by name. Extract the project ID (UUID).

2. **Prepare title and description** from the user's request.
   - Title: short summary
   - Description: must be HTML (the API field is `description_html`). See **HTML formatting reference** below.

3. **Fetch project metadata** in parallel using `--json` flag and `jq` to extract names and IDs:
   ```
   cargo run -- --json states list -p <PROJECT_ID> 2>/dev/null | jq '[.results[] | {name, id, group}]'
   cargo run -- --json labels list -p <PROJECT_ID> 2>/dev/null | jq '[.results[] | {name, id}]'
   cargo run -- --json members list -p <PROJECT_ID> 2>/dev/null | jq '[.[] | {display_name, id}]'
   ```

4. **Ask the user interactively** using AskUserQuestion with the fetched data. Ask all four questions at once:
   - **State** (single select): present available states with their group in the description (e.g. "backlog group", "started group")
   - **Priority** (single select): `none`, `low`, `medium`, `high`, `urgent`
   - **Assignee** (single select): present project members by display_name, plus a "No assignee" option
   - **Labels** (multi select): present available labels, plus a "None" option

5. **Run the create command**:
   ```
   cargo run -- issues create \
     -p <PROJECT_ID> \
     --title "<TITLE>" \
     --description "<HTML>" \
     --state <STATE_ID> \
     --priority <PRIORITY> \
     --assignee <MEMBER_ID> \
     --label <LABEL_ID> \
     2>/dev/null
   ```
   - Omit `--state`, `--assignee`, `--label` if the user chose "no assignee" or "none"
   - Multiple `--assignee` and `--label` flags can be repeated

6. **Report the result** — show the created issue name, sequence ID, and a link:
   `{base_url}/{workspace}/projects/{project_id}/issues/{issue_id}`
   where `base_url` and `workspace` come from `config/settings.local.json`.

## HTML formatting reference

The Plane editor uses specific CSS classes. Use these exact patterns:

**Paragraph:**
```html
<p class="editor-paragraph-block">Text here</p>
```

**Heading (h2):**
```html
<h2 class="editor-heading-block">Section title</h2>
```

**Inline code** (for short references like variable names, endpoints, commands):
```html
<code class="rounded bg-custom-background-80 px-[6px] py-[1.5px] font-mono font-medium text-orange-500 border-[0.5px] border-custom-border-200" spellcheck="false">some_variable</code>
```

**Code block** (for multi-line code, shell commands, JSON, etc.):
```html
<pre class=""><code class="language-bash">ssh -L 8080:10.0.0.1:80 user@host -N</code></pre>
```
Supported language classes: `language-json`, `language-bash`, `language-rust`, etc. Omit the class for plain code blocks: `<pre class=""><code>plain code</code></pre>`

**Bulleted list:**
```html
<ul class="list-disc pl-7 space-y-[--list-spacing-y] tight" data-tight="true">
  <li class="not-prose space-y-2"><p class="editor-paragraph-block">Item one</p></li>
  <li class="not-prose space-y-2"><p class="editor-paragraph-block">Item two</p></li>
</ul>
```

## Rules

- Always use `2>/dev/null` when running cargo to suppress build warnings
- Always pass `--json` when fetching data for parsing
- Description must be HTML — use the exact CSS classes from the formatting reference above
- Priority is a string: `none`, `urgent`, `high`, `medium`, `low`
- Do not create the issue without asking the user about state, priority, assignee, and labels first
