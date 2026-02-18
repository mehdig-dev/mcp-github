use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::{schemars, tool, tool_handler, tool_router, ServerHandler};
use serde::Deserialize;

use crate::error::McpGithubError;

#[derive(Clone)]
pub struct McpGithubServer {
    github: Arc<octocrab::Octocrab>,
    default_owner: Option<String>,
    max_results: u32,
    tool_router: ToolRouter<Self>,
}

// -- Tool parameter types --

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct OwnerParam {
    #[schemars(description = "GitHub user or organization name")]
    #[serde(default)]
    pub owner: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RepoParams {
    #[schemars(description = "Repository owner (user or org)")]
    #[serde(default)]
    pub owner: Option<String>,

    #[schemars(description = "Repository name")]
    pub repo: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListIssuesParams {
    #[schemars(description = "Repository owner (user or org)")]
    #[serde(default)]
    pub owner: Option<String>,

    #[schemars(description = "Repository name")]
    pub repo: String,

    #[schemars(description = "Filter by state: open, closed, or all (default: open)")]
    #[serde(default)]
    pub state: Option<String>,

    #[schemars(description = "Filter by comma-separated label names")]
    #[serde(default)]
    pub labels: Option<String>,

    #[schemars(description = "Maximum number of results")]
    #[serde(default)]
    pub per_page: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IssueParams {
    #[schemars(description = "Repository owner (user or org)")]
    #[serde(default)]
    pub owner: Option<String>,

    #[schemars(description = "Repository name")]
    pub repo: String,

    #[schemars(description = "Issue number")]
    pub issue_number: u64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListPullsParams {
    #[schemars(description = "Repository owner (user or org)")]
    #[serde(default)]
    pub owner: Option<String>,

    #[schemars(description = "Repository name")]
    pub repo: String,

    #[schemars(description = "Filter by state: open, closed, or all (default: open)")]
    #[serde(default)]
    pub state: Option<String>,

    #[schemars(description = "Maximum number of results")]
    #[serde(default)]
    pub per_page: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PullParams {
    #[schemars(description = "Repository owner (user or org)")]
    #[serde(default)]
    pub owner: Option<String>,

    #[schemars(description = "Repository name")]
    pub repo: String,

    #[schemars(description = "Pull request number")]
    pub pr_number: u64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchCodeParams {
    #[schemars(description = "Search query (GitHub code search syntax)")]
    pub query: String,

    #[schemars(description = "Scope search to this owner/org")]
    #[serde(default)]
    pub owner: Option<String>,

    #[schemars(description = "Scope search to this repository")]
    #[serde(default)]
    pub repo: Option<String>,

    #[schemars(description = "Maximum number of results")]
    #[serde(default)]
    pub per_page: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ActionsParams {
    #[schemars(description = "Repository owner (user or org)")]
    #[serde(default)]
    pub owner: Option<String>,

    #[schemars(description = "Repository name")]
    pub repo: String,

    #[schemars(description = "Filter by status: completed, in_progress, queued")]
    #[serde(default)]
    pub status: Option<String>,

    #[schemars(description = "Maximum number of results")]
    #[serde(default)]
    pub per_page: Option<u32>,
}

impl McpGithubServer {
    pub fn new(
        github: octocrab::Octocrab,
        default_owner: Option<String>,
        max_results: u32,
    ) -> Self {
        Self {
            github: Arc::new(github),
            default_owner,
            max_results,
            tool_router: Self::tool_router(),
        }
    }

    fn resolve_owner(&self, param: Option<&str>) -> Result<String, McpGithubError> {
        param
            .map(String::from)
            .or_else(|| self.default_owner.clone())
            .ok_or_else(|| {
                McpGithubError::MissingParam(
                    "owner is required (or set --owner default)".to_string(),
                )
            })
    }

    fn err(&self, e: McpGithubError) -> ErrorData {
        e.to_mcp_error()
    }
}

#[tool_router]
impl McpGithubServer {
    #[tool(
        name = "list_repos",
        description = "List repositories for a user or organization"
    )]
    async fn list_repos(
        &self,
        Parameters(params): Parameters<OwnerParam>,
    ) -> Result<CallToolResult, ErrorData> {
        let owner = self
            .resolve_owner(params.owner.as_deref())
            .map_err(|e| self.err(e))?;

        let page = self
            .github
            .orgs(&owner)
            .list_repos()
            .per_page(self.max_results as u8)
            .send()
            .await;

        // If org fails, try as user
        let repos = match page {
            Ok(page) => page.items,
            Err(_) => {
                self.github
                    .users(&owner)
                    .repos()
                    .per_page(self.max_results as u8)
                    .send()
                    .await
                    .map_err(|e| self.err(McpGithubError::GitHub(e)))?
                    .items
            }
        };

        let results: Vec<serde_json::Value> = repos
            .iter()
            .map(|r| {
                serde_json::json!({
                    "name": r.name,
                    "full_name": r.full_name.as_deref().unwrap_or(""),
                    "description": r.description.as_deref().unwrap_or(""),
                    "language": r.language.as_ref().map(|l| l.to_string()).unwrap_or_default(),
                    "stars": r.stargazers_count.unwrap_or(0),
                    "forks": r.forks_count.unwrap_or(0),
                    "private": r.private.unwrap_or(false),
                })
            })
            .collect();

        let text = serde_json::to_string_pretty(&serde_json::json!({
            "owner": owner,
            "repos": results,
            "count": results.len(),
        }))
        .unwrap_or_else(|_| "{}".to_string());
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        name = "get_repo",
        description = "Get repository info including description, stars, forks, language, and default branch"
    )]
    async fn get_repo(
        &self,
        Parameters(params): Parameters<RepoParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let owner = self
            .resolve_owner(params.owner.as_deref())
            .map_err(|e| self.err(e))?;

        let repo = self
            .github
            .repos(&owner, &params.repo)
            .get()
            .await
            .map_err(|e| self.err(McpGithubError::GitHub(e)))?;

        let text = serde_json::to_string_pretty(&serde_json::json!({
            "name": repo.name,
            "full_name": repo.full_name,
            "description": repo.description,
            "language": repo.language,
            "default_branch": repo.default_branch,
            "stars": repo.stargazers_count,
            "forks": repo.forks_count,
            "open_issues": repo.open_issues_count,
            "private": repo.private,
            "created_at": repo.created_at.map(|t| t.to_string()),
            "updated_at": repo.updated_at.map(|t| t.to_string()),
        }))
        .unwrap_or_else(|_| "{}".to_string());
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        name = "list_issues",
        description = "List issues in a repository, optionally filtered by state and labels"
    )]
    async fn list_issues(
        &self,
        Parameters(params): Parameters<ListIssuesParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let owner = self
            .resolve_owner(params.owner.as_deref())
            .map_err(|e| self.err(e))?;

        let issue_handler = self.github.issues(&owner, &params.repo);
        let mut request = issue_handler
            .list()
            .per_page(params.per_page.unwrap_or(self.max_results) as u8);

        if let Some(ref state) = params.state {
            request = match state.as_str() {
                "open" => request.state(octocrab::params::State::Open),
                "closed" => request.state(octocrab::params::State::Closed),
                "all" => request.state(octocrab::params::State::All),
                _ => request,
            };
        }

        let label_list: Vec<String>;
        if let Some(ref labels) = params.labels {
            label_list = labels.split(',').map(|s| s.trim().to_string()).collect();
            request = request.labels(&label_list);
        }

        let issues = request
            .send()
            .await
            .map_err(|e| self.err(McpGithubError::GitHub(e)))?;

        let results: Vec<serde_json::Value> = issues
            .items
            .iter()
            .map(|i| {
                let labels: Vec<String> = i.labels.iter().map(|l| l.name.clone()).collect();
                serde_json::json!({
                    "number": i.number,
                    "title": i.title,
                    "state": format!("{:?}", i.state),
                    "author": i.user.login,
                    "labels": labels,
                    "comments": i.comments,
                    "created_at": i.created_at.to_string(),
                })
            })
            .collect();

        let text = serde_json::to_string_pretty(&serde_json::json!({
            "repo": format!("{}/{}", owner, params.repo),
            "issues": results,
            "count": results.len(),
        }))
        .unwrap_or_else(|_| "{}".to_string());
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        name = "get_issue",
        description = "Get issue details including body and comments"
    )]
    async fn get_issue(
        &self,
        Parameters(params): Parameters<IssueParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let owner = self
            .resolve_owner(params.owner.as_deref())
            .map_err(|e| self.err(e))?;

        let issue = self
            .github
            .issues(&owner, &params.repo)
            .get(params.issue_number)
            .await
            .map_err(|e| self.err(McpGithubError::GitHub(e)))?;

        // Fetch comments
        let comments = self
            .github
            .issues(&owner, &params.repo)
            .list_comments(params.issue_number)
            .send()
            .await
            .map_err(|e| self.err(McpGithubError::GitHub(e)))?;

        let comment_items: Vec<serde_json::Value> = comments
            .items
            .iter()
            .map(|c| {
                serde_json::json!({
                    "author": c.user.login,
                    "body": c.body.as_deref().unwrap_or(""),
                    "created_at": c.created_at.to_string(),
                })
            })
            .collect();

        let labels: Vec<String> = issue.labels.iter().map(|l| l.name.clone()).collect();

        let text = serde_json::to_string_pretty(&serde_json::json!({
            "number": issue.number,
            "title": issue.title,
            "state": format!("{:?}", issue.state),
            "author": issue.user.login,
            "labels": labels,
            "body": issue.body.as_deref().unwrap_or(""),
            "comments": comment_items,
            "created_at": issue.created_at.to_string(),
        }))
        .unwrap_or_else(|_| "{}".to_string());
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        name = "list_pulls",
        description = "List pull requests in a repository"
    )]
    async fn list_pulls(
        &self,
        Parameters(params): Parameters<ListPullsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let owner = self
            .resolve_owner(params.owner.as_deref())
            .map_err(|e| self.err(e))?;

        let pulls_handler = self.github.pulls(&owner, &params.repo);
        let mut request = pulls_handler
            .list()
            .per_page(params.per_page.unwrap_or(self.max_results) as u8);

        if let Some(ref state) = params.state {
            request = match state.as_str() {
                "open" => request.state(octocrab::params::State::Open),
                "closed" => request.state(octocrab::params::State::Closed),
                "all" => request.state(octocrab::params::State::All),
                _ => request,
            };
        }

        let pulls = request
            .send()
            .await
            .map_err(|e| self.err(McpGithubError::GitHub(e)))?;

        let results: Vec<serde_json::Value> = pulls
            .items
            .iter()
            .map(|p| {
                serde_json::json!({
                    "number": p.number,
                    "title": p.title.as_deref().unwrap_or(""),
                    "state": format!("{:?}", p.state.as_ref()),
                    "author": p.user.as_ref().map(|u| u.login.as_str()).unwrap_or("unknown"),
                    "head": p.head.ref_field,
                    "base": p.base.ref_field,
                    "draft": p.draft,
                    "created_at": p.created_at.map(|t| t.to_string()),
                })
            })
            .collect();

        let text = serde_json::to_string_pretty(&serde_json::json!({
            "repo": format!("{}/{}", owner, params.repo),
            "pulls": results,
            "count": results.len(),
        }))
        .unwrap_or_else(|_| "{}".to_string());
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        name = "get_pull",
        description = "Get pull request details including review summary and changed files count"
    )]
    async fn get_pull(
        &self,
        Parameters(params): Parameters<PullParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let owner = self
            .resolve_owner(params.owner.as_deref())
            .map_err(|e| self.err(e))?;

        let pr = self
            .github
            .pulls(&owner, &params.repo)
            .get(params.pr_number)
            .await
            .map_err(|e| self.err(McpGithubError::GitHub(e)))?;

        let text = serde_json::to_string_pretty(&serde_json::json!({
            "number": pr.number,
            "title": pr.title.as_deref().unwrap_or(""),
            "state": format!("{:?}", pr.state.as_ref()),
            "author": pr.user.as_ref().map(|u| u.login.as_str()).unwrap_or("unknown"),
            "body": pr.body.as_deref().unwrap_or(""),
            "head": pr.head.ref_field,
            "base": pr.base.ref_field,
            "draft": pr.draft,
            "mergeable": pr.mergeable,
            "additions": pr.additions,
            "deletions": pr.deletions,
            "changed_files": pr.changed_files,
            "commits": pr.commits,
            "created_at": pr.created_at.map(|t| t.to_string()),
            "merged_at": pr.merged_at.map(|t| t.to_string()),
        }))
        .unwrap_or_else(|_| "{}".to_string());
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        name = "search_code",
        description = "Search code across GitHub repositories using GitHub's code search syntax"
    )]
    async fn search_code(
        &self,
        Parameters(params): Parameters<SearchCodeParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let mut query = params.query.clone();

        // Scope to owner/repo if specified
        if let Some(ref owner) = params.owner.as_ref().or(self.default_owner.as_ref()) {
            if let Some(ref repo) = params.repo {
                query = format!("{} repo:{}/{}", query, owner, repo);
            } else {
                query = format!("{} org:{}", query, owner);
            }
        }

        let results = self
            .github
            .search()
            .code(&query)
            .per_page(params.per_page.unwrap_or(self.max_results) as u8)
            .send()
            .await
            .map_err(|e| self.err(McpGithubError::GitHub(e)))?;

        let items: Vec<serde_json::Value> = results
            .items
            .iter()
            .map(|item| {
                serde_json::json!({
                    "name": item.name,
                    "path": item.path,
                    "repository": item.repository.full_name.as_deref().unwrap_or(""),
                    "url": item.html_url,
                })
            })
            .collect();

        let text = serde_json::to_string_pretty(&serde_json::json!({
            "query": params.query,
            "results": items,
            "count": items.len(),
        }))
        .unwrap_or_else(|_| "{}".to_string());
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        name = "list_actions_runs",
        description = "List recent GitHub Actions workflow runs for a repository"
    )]
    async fn list_actions_runs(
        &self,
        Parameters(params): Parameters<ActionsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let owner = self
            .resolve_owner(params.owner.as_deref())
            .map_err(|e| self.err(e))?;

        let route = format!(
            "/repos/{}/{}/actions/runs?per_page={}",
            owner,
            params.repo,
            params.per_page.unwrap_or(self.max_results)
        );

        let response: serde_json::Value = self
            .github
            .get(route, None::<&()>)
            .await
            .map_err(|e| self.err(McpGithubError::GitHub(e)))?;

        let runs = response
            .get("workflow_runs")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|run| {
                        serde_json::json!({
                            "id": run.get("id"),
                            "name": run.get("name"),
                            "status": run.get("status"),
                            "conclusion": run.get("conclusion"),
                            "branch": run.get("head_branch"),
                            "event": run.get("event"),
                            "created_at": run.get("created_at"),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let text = serde_json::to_string_pretty(&serde_json::json!({
            "repo": format!("{}/{}", owner, params.repo),
            "runs": runs,
            "count": runs.len(),
        }))
        .unwrap_or_else(|_| "{}".to_string());
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }
}

#[tool_handler]
impl ServerHandler for McpGithubServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "mcp-github".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                ..Default::default()
            },
            instructions: Some(
                "GitHub server. Use list_repos to see repositories, get_repo for repo details, \
                 list_issues and get_issue for issues, list_pulls and get_pull for PRs, \
                 search_code to search code, and list_actions_runs for CI/CD runs."
                    .to_string(),
            ),
        }
    }
}
