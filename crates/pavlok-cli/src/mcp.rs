//! MCP server exposing the Pavlok stimuli (zap/beep/vibe) as tools.
//!
//! Only the stimulus actions are exposed — account/history data is deliberately
//! kept off the AI-facing surface.

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
    pub fn new(client: Arc<PavlokClient>) -> Self {
        Self {
            client,
            tool_router: Self::tool_router(),
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

    #[tool(
        description = "Make the Pavlok device beep. `value` is intensity/volume from 1 to 100."
    )]
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

#[tool_handler]
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
