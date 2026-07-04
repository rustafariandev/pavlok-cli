//! Command-line interface definition (clap derive).

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "pavlok",
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
    /// Print the current account as JSON
    Whoami,
    /// Print recently received stimuli as JSON
    History,
    /// Run as an MCP server over stdio (for AI assistants)
    Mcp,
}
