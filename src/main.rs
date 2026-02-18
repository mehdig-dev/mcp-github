use anyhow::Result;
use clap::Parser;
use mcp_github::server;
use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::EnvFilter;

/// MCP server for GitHub — lets LLMs explore repositories, issues, and pull requests
#[derive(Parser)]
#[command(name = "mcp-github", version, about)]
struct Cli {
    /// GitHub personal access token.
    /// Can also be set via GITHUB_TOKEN environment variable.
    #[arg(long)]
    token: Option<String>,

    /// Read GitHub token from an environment variable.
    /// Default: GITHUB_TOKEN
    #[arg(long = "token-env")]
    token_env: Option<String>,

    /// Default repository owner/org for operations
    #[arg(long)]
    owner: Option<String>,

    /// Maximum results per API call (default: 30)
    #[arg(long, default_value = "30")]
    max_results: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    // Resolve token: --token > --token-env > GITHUB_TOKEN
    let token = if let Some(t) = cli.token {
        Some(t)
    } else {
        let env_name = cli.token_env.as_deref().unwrap_or("GITHUB_TOKEN");
        match std::env::var(env_name) {
            Ok(t) if !t.is_empty() => {
                tracing::info!(env = env_name, "Read GitHub token from environment variable");
                Some(t)
            }
            _ => None,
        }
    };

    let github = if let Some(ref t) = token {
        octocrab::OctocrabBuilder::new()
            .personal_token(t.clone())
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create GitHub client: {}", e))?
    } else {
        tracing::warn!("No GitHub token provided — API rate limits will be very restrictive");
        octocrab::Octocrab::default()
    };

    let authenticated = token.is_some();

    tracing::info!(
        authenticated,
        owner = cli.owner.as_deref().unwrap_or("none"),
        max_results = cli.max_results,
        "Starting mcp-github server"
    );

    let service = server::McpGithubServer::new(github, cli.owner, cli.max_results);
    let running = service.serve(stdio()).await?;
    running.waiting().await?;

    Ok(())
}
