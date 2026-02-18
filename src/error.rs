use rmcp::model::ErrorData;

#[derive(Debug, thiserror::Error)]
pub enum McpGithubError {
    #[error("GitHub API error: {0}")]
    GitHub(#[from] octocrab::Error),

    #[error("Missing required parameter: {0}")]
    MissingParam(String),

    #[error("Repository not found: {0}")]
    RepoNotFound(String),

    #[error("Authentication required")]
    Unauthenticated,

    #[error("{0}")]
    Other(String),
}

impl McpGithubError {
    pub fn to_mcp_error(&self) -> ErrorData {
        match self {
            McpGithubError::MissingParam(_) | McpGithubError::RepoNotFound(_) => {
                ErrorData::invalid_params(self.to_string(), None)
            }
            McpGithubError::Unauthenticated => {
                ErrorData::invalid_params(self.to_string(), None)
            }
            McpGithubError::GitHub(_) | McpGithubError::Other(_) => {
                ErrorData::internal_error(self.to_string(), None)
            }
        }
    }
}
