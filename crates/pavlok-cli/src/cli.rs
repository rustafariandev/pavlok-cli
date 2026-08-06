//! Command-line interface definition (clap derive).

use clap::{Parser, Subcommand, ValueEnum};

/// A stimulus tool that the MCP server can expose.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum McpTool {
    Zap,
    Beep,
    Vibe,
}

impl McpTool {
    /// The name the tool is registered under in the MCP router.
    pub fn name(self) -> &'static str {
        match self {
            McpTool::Zap => "zap",
            McpTool::Beep => "beep",
            McpTool::Vibe => "vibe",
        }
    }
}

#[derive(Parser)]
#[command(
    name = "pavlok-cli",
    version,
    about = "Control a Pavlok device from the terminal and expose it to AI via MCP"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Log in with email/password and store an API token
    Login {
        /// Account email (prompted for if omitted)
        #[arg(long)]
        email: Option<String>,
    },
    /// Deliver a zap (electric stimulus)
    Zap {
        /// Intensity, 1-100
        #[arg(default_value_t = 50)]
        value: u8,
        /// Optional reason/label recorded with the stimulus
        #[arg(long)]
        reason: Option<String>,
    },
    /// Make the device beep
    Beep {
        /// Intensity, 1-100
        #[arg(default_value_t = 50)]
        value: u8,
        #[arg(long)]
        reason: Option<String>,
    },
    /// Make the device vibrate
    Vibe {
        /// Intensity, 1-100
        #[arg(default_value_t = 50)]
        value: u8,
        #[arg(long)]
        reason: Option<String>,
    },
    /// Show the logged-in account
    Whoami {
        /// Print the raw API response as JSON
        #[arg(long)]
        json: bool,
        /// Include settings and rarely-set profile fields
        #[arg(long, conflicts_with = "json")]
        all: bool,
    },
    /// Run as an MCP server over stdio (for AI assistants)
    Mcp {
        /// Restrict the server to these tools (comma-separated, e.g.
        /// `--tools beep,vibe`). If omitted, all tools are exposed.
        #[arg(long, value_enum, value_delimiter = ',')]
        tools: Vec<McpTool>,
    },
}
