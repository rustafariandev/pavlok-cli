mod cli;
mod config;
mod mcp;

use std::io::{self, Write};
use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use pavlok_client::{PavlokClient, StimulusType};
use rmcp::{ServiceExt, transport::stdio};

use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    // Log to stderr; stdout is reserved for MCP JSON-RPC and command output.
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_target(false)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Login { email } => login(email).await,
        Commands::Zap { value, reason } => stimulus(StimulusType::Zap, value, reason).await,
        Commands::Beep { value, reason } => stimulus(StimulusType::Beep, value, reason).await,
        Commands::Vibe { value, reason } => stimulus(StimulusType::Vibe, value, reason).await,
        Commands::Whoami => whoami().await,
        Commands::History => history().await,
        Commands::Mcp => serve_mcp().await,
    }
}

async fn login(email: Option<String>) -> Result<()> {
    let email = match email {
        Some(e) => e,
        None => prompt("Email: ")?,
    };
    let password = rpassword::prompt_password("Password: ")?;

    let client = PavlokClient::new(None);
    let token = client.login(&email, &password).await?;
    let path = config::save_token(&token)?;
    println!("Logged in. Token saved to {}", path.display());
    Ok(())
}

async fn stimulus(kind: StimulusType, value: u8, reason: Option<String>) -> Result<()> {
    // Note: value validation happens inside send_stimulus before any token or
    // network is needed, so out-of-range input fails fast.
    let client = PavlokClient::new(config::resolve_token()?);
    client.send_stimulus(kind, value, reason).await?;
    println!("Sent {} at intensity {}.", kind.as_str(), value);
    Ok(())
}

async fn whoami() -> Result<()> {
    let client = PavlokClient::new(Some(config::require_token()?));
    let user = client.whoami().await?;
    println!("{}", serde_json::to_string_pretty(&user)?);
    Ok(())
}

async fn history() -> Result<()> {
    let client = PavlokClient::new(Some(config::require_token()?));
    let events = client.history().await?;
    println!("{}", serde_json::to_string_pretty(&events)?);
    Ok(())
}

async fn serve_mcp() -> Result<()> {
    let client = Arc::new(PavlokClient::new(config::resolve_token()?));
    let service = mcp::PavlokServer::new(client).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

/// Prompt on stderr and read a trimmed line from stdin.
fn prompt(label: &str) -> Result<String> {
    eprint!("{label}");
    io::stderr().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(line.trim().to_string())
}
