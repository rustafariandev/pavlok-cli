//! MCP server exposing the Pavlok stimuli (zap/beep/vibe) as tools.
//!
//! Only the stimulus actions are exposed — account data is deliberately kept
//! off the AI-facing surface.

use std::sync::Arc;

use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::router::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use serde::Deserialize;

use pavlok_client::{PavlokClient, StimulusType};

/// Parameters shared by every stimulus tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StimulusArgs {
    /// Intensity of the stimulus, from 1 (weakest) to 100 (strongest).
    #[schemars(range(min = 1, max = 100))]
    pub value: u8,
    /// Optional short reason/label recorded alongside the stimulus.
    pub reason: Option<String>,
}

#[derive(Clone)]
pub struct PavlokServer {
    client: Arc<PavlokClient>,
    // Read by the macro-generated `ServerHandler` impl; invisible to dead-code analysis.
    #[allow(dead_code)]
    tool_router: ToolRouter<PavlokServer>,
}

#[tool_router]
impl PavlokServer {
    /// Build a server exposing every stimulus tool.
    pub fn new(client: Arc<PavlokClient>) -> Self {
        Self {
            client,
            tool_router: Self::tool_router(),
        }
    }

    /// Build a server exposing only the named tools.
    ///
    /// `allowed` is a list of tool names (e.g. `["beep", "vibe"]`). Any tool
    /// not in the list is dropped from the router, which removes it from both
    /// `tools/list` and `tools/call` — a filtered-out tool cannot be invoked.
    pub fn with_tools(client: Arc<PavlokClient>, allowed: &[&str]) -> Self {
        let mut tool_router = Self::tool_router();
        tool_router
            .map
            .retain(|name, _| allowed.contains(&name.as_ref()));
        Self {
            client,
            tool_router,
        }
    }

    /// Shared implementation: send the stimulus and render a tool result.
    async fn fire(
        &self,
        kind: StimulusType,
        args: StimulusArgs,
    ) -> Result<CallToolResult, McpError> {
        match self
            .client
            .send_stimulus(kind, args.value, args.reason)
            .await
        {
            Ok(()) => Ok(CallToolResult::success(vec![ContentBlock::text(format!(
                "Sent {} at intensity {}.",
                kind.as_str(),
                args.value
            ))])),
            Err(e) => Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                "Failed to send {}: {e}",
                kind.as_str()
            ))])),
        }
    }

    #[tool(
        description = "Deliver a zap (electric stimulus) to the Pavlok device. `value` is intensity from 1 to 100."
    )]
    async fn zap(
        &self,
        Parameters(args): Parameters<StimulusArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.fire(StimulusType::Zap, args).await
    }

    #[tool(description = "Make the Pavlok device beep. `value` is intensity/volume from 1 to 100.")]
    async fn beep(
        &self,
        Parameters(args): Parameters<StimulusArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.fire(StimulusType::Beep, args).await
    }

    #[tool(description = "Make the Pavlok device vibrate. `value` is intensity from 1 to 100.")]
    async fn vibe(
        &self,
        Parameters(args): Parameters<StimulusArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.fire(StimulusType::Vibe, args).await
    }
}

// `router = self.tool_router` makes list/call use our (possibly filtered)
// instance router; the macro otherwise defaults to a fresh `Self::tool_router()`
// with every tool, which would ignore `with_tools`.
#[tool_handler(router = self.tool_router)]
impl ServerHandler for PavlokServer {
    fn get_info(&self) -> ServerInfo {
        // `Implementation` is #[non_exhaustive]; build from default then set fields.
        let mut server_info = Implementation::default();
        server_info.name = env!("CARGO_PKG_NAME").to_string();
        server_info.version = env!("CARGO_PKG_VERSION").to_string();

        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(server_info)
            .with_instructions(
                "Trigger Pavlok stimuli. Tools: zap (electric), beep, vibe — each takes an \
                 intensity `value` from 1 to 100 and an optional `reason`.",
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool_names(server: &PavlokServer) -> Vec<String> {
        let mut names: Vec<String> = server
            .tool_router
            .map
            .keys()
            .map(|k| k.to_string())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn new_exposes_all_tools() {
        let server = PavlokServer::new(Arc::new(PavlokClient::new(None)));
        assert_eq!(tool_names(&server), ["beep", "vibe", "zap"]);
    }

    #[test]
    fn with_tools_restricts_to_allowlist() {
        let server = PavlokServer::with_tools(Arc::new(PavlokClient::new(None)), &["beep", "vibe"]);
        assert_eq!(tool_names(&server), ["beep", "vibe"]);
    }

    #[test]
    fn with_tools_ignores_unknown_names() {
        let server = PavlokServer::with_tools(Arc::new(PavlokClient::new(None)), &["zap", "bogus"]);
        assert_eq!(tool_names(&server), ["zap"]);
    }
}
