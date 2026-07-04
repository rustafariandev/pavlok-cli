# pavlok-cli

A Rust command-line tool for the [Pavlok](https://pavlok.com) v5 API. It lets you
trigger your device's stimuli (zap / beep / vibe) from the terminal, and can run
as an **MCP server** so an AI assistant (e.g. Claude) can trigger stimuli through
tool calls.

## Build

```sh
cargo build --release
# binary at ./target/release/pavlok-cli
```

## Authentication

Log in once to store a bearer token at `~/.config/pavlok-cli/config.toml`
(written `0600`):

```sh
pavlok-cli login              # prompts for email + password
pavlok-cli login --email you@example.com
```

Alternatively, skip the login flow and provide a token directly via the
`PAVLOK_TOKEN` environment variable, which always takes priority over the config
file:

```sh
export PAVLOK_TOKEN="your-token"
```

## CLI usage

```sh
pavlok-cli zap 50                 # electric stimulus, intensity 1-100 (default 50)
pavlok-cli beep 30
pavlok-cli vibe 40 --reason "pomodoro over"
pavlok-cli whoami                 # print current account as JSON
pavlok-cli history                # print recently received stimuli as JSON
```

Intensity is validated to `1..=100` locally before any request is made.

## MCP server (for AI assistants)

Run the server over stdio:

```sh
PAVLOK_TOKEN="your-token" pavlok-cli mcp
```

It exposes three tools — `zap`, `beep`, `vibe` — each taking an intensity
`value` (1–100) and an optional `reason`. Account/history data is intentionally
**not** exposed to the AI.

Register it with Claude Code:

```sh
claude mcp add pavlok --env PAVLOK_TOKEN=your-token -- /absolute/path/to/pavlok-cli mcp
```

Or add it to a client's MCP config manually:

```json
{
  "mcpServers": {
    "pavlok": {
      "command": "/absolute/path/to/pavlok-cli",
      "args": ["mcp"],
      "env": { "PAVLOK_TOKEN": "your-token" }
    }
  }
}
```

## Notes

- All logging goes to **stderr**; stdout carries only command output and MCP
  JSON-RPC, so the protocol is never corrupted.
- Built on the official [`rmcp`](https://crates.io/crates/rmcp) SDK and
  [`reqwest`](https://crates.io/crates/reqwest).
