# plane-cli

CLI for Plane project management.

## Project structure

```
plane-cli/
  Cargo.toml
  src/
    main.rs        # CLI entry point (clap derive), subcommand enums, output formatting
```

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

## Dependencies

- `clap` (derive) — CLI parsing
- `serde` + `serde_json` — JSON handling
- `anyhow` — error handling
- `console` — terminal colors/styling
- `indicatif` — progress spinners
