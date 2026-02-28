# plane-cli

CLI for Plane project management.

## Project structure

```
plane-cli/
  Cargo.toml
  config/
    settings.json        # Base settings (committed)
    settings.local.json  # Local overrides (git-ignored)
  src/
    main.rs        # CLI entry point (clap derive), subcommand enums, output formatting
    settings.rs    # Settings struct & layered config loader
    client.rs      # HTTP client wrapper for Plane API
```

## Plane API documentation

https://github.com/makeplane/developer-docs/tree/master/docs/api-reference

## Key conventions

- All errors use `anyhow` for context-rich error chains
- Two output modes: human (colored, spinners) and JSON (`--json` flag)

## Git & PR conventions

- Do not add Claude as a co-author to commit messages and PR descriptions
- **NEVER commit or push unless the user explicitly asks to** — this is a hard rule, no exceptions
- Do not push branches unless explicitly asked to
- Never force push (`git push --force` / `git push -f`) branches to GitHub
- To resolve conflicts with main, use `git pull origin main` instead of `git rebase` (rebase requires force push)
- Always ask before using `--admin` flag when merging PRs — it bypasses branch protection checks
- Try to keep branch names short but readable
- Always remove previously added but now unused code

## Build & run

```bash
cargo build
cargo run -- <command>
```

## Settings

Layered configuration (each layer overrides the previous):
1. Hardcoded defaults (`Settings::default()`)
2. `{home}/config/settings.json` — base settings
3. `{home}/config/settings.local.json` — local overrides (git-ignored)
4. Environment variables: `PLANE_CLI_API_KEY`, `PLANE_CLI_BASE_URL`, `PLANE_CLI_WORKSPACE`, `PLANE_CLI_TIMEOUT`
5. CLI arguments (`--api-key`, `--base-url`, `--workspace`, `--timeout`)

`{home}` is `PLANE_CLI_HOME` env var if set, otherwise the current directory.

## Dependencies

- `clap` (derive) — CLI parsing
- `serde` + `serde_json` — JSON handling
- `anyhow` — error handling
- `console` — terminal colors/styling
- `indicatif` — progress spinners
- `reqwest` (json, rustls-tls) — HTTP client
- `tokio` (rt-multi-thread, macros) — async runtime
