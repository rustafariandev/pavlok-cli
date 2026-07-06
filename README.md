# pavlok-cli

A Rust command-line tool for the [Pavlok](https://pavlok.com) v5 API. It lets you
trigger your device's stimuli (zap / beep / vibe) from the terminal, and can run
as an **MCP server** so an AI assistant (e.g. Claude) can trigger stimuli through
tool calls.

## Project layout

A Cargo workspace with two crates:

- `crates/pavlok-client` — a standalone async library for the Pavlok v5 API
  (`PavlokClient`, `StimulusType`). Depends only on reqwest/serde; reusable on
  its own.
- `crates/pavlok-cli` — the `pavlok-cli` binary (CLI + MCP server) that depends
  on the library.

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

Restrict which tools the server exposes with `--tools` (comma-separated). Any
tool left out is removed from both `tools/list` and `tools/call`, so the AI can
neither see nor invoke it:

```sh
PAVLOK_TOKEN="your-token" pavlok-cli mcp --tools beep,vibe   # no zap
```

Register it with Claude Code:

```sh
claude mcp add pavlok --env PAVLOK_TOKEN=your-token -- /absolute/path/to/pavlok-cli mcp
# or, to expose only some tools:
claude mcp add pavlok --env PAVLOK_TOKEN=your-token -- /absolute/path/to/pavlok-cli mcp --tools beep,vibe
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
