# mcp-github

MCP server that lets LLMs interact with GitHub repositories, issues, and pull requests. Single binary, read-only.

## Install

```bash
cargo install mcp-github
```

## Usage

```bash
# With token from environment (GITHUB_TOKEN)
mcp-github

# Explicit token
mcp-github --token ghp_xxxxx

# Default owner for all operations
mcp-github --owner myorg

# Custom results limit
mcp-github --owner myorg --max-results 50
```

## Configuration

### Claude Code

```bash
claude mcp add github -- mcp-github --owner myorg
```

### Claude Desktop

```json
{
  "mcpServers": {
    "github": {
      "command": "mcp-github",
      "args": ["--owner", "myorg"],
      "env": {
        "GITHUB_TOKEN": "ghp_xxxxx"
      }
    }
  }
}
```

### Cursor / VS Code

```json
{
  "mcpServers": {
    "github": {
      "command": "mcp-github",
      "args": ["--owner", "myorg"],
      "env": {
        "GITHUB_TOKEN": "ghp_xxxxx"
      }
    }
  }
}
```

## Tools

| Tool | Description |
|------|-------------|
| `list_repos` | List repositories for a user or organization |
| `get_repo` | Get repository info (stars, forks, language, default branch) |
| `list_issues` | List issues with state and label filters |
| `get_issue` | Get issue details with comments |
| `list_pulls` | List pull requests with state filter |
| `get_pull` | Get PR details with review summary and diff stats |
| `search_code` | Search code across repositories |
| `list_actions_runs` | List recent GitHub Actions workflow runs |

## CLI Options

| Flag | Default | Description |
|------|---------|-------------|
| `--token` | — | GitHub personal access token |
| `--token-env` | `GITHUB_TOKEN` | Environment variable containing the token |
| `--owner` | — | Default repository owner/org |
| `--max-results` | `30` | Maximum results per API call |

## Authentication

Token is resolved in this order:
1. `--token` flag
2. `--token-env` environment variable
3. `GITHUB_TOKEN` environment variable
4. Unauthenticated (rate limited to 60 requests/hour)

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
