# pavlok-cli

Command-line tool for the [Pavlok](https://pavlok.com) v5 API. Trigger your
device's stimuli (zap / beep / vibe) from the terminal, and run as an **MCP
server** so an AI assistant (e.g. Claude) can trigger stimuli through tool calls.

Built on the
[`pavlok-client`](https://github.com/rustafariandev/pavlok-cli/tree/main/crates/pavlok-client)
library.

## Install

```sh
cargo install pavlok-cli
```

Prebuilt binaries for Linux, macOS, and Windows are also attached to every
[release](https://github.com/rustafariandev/pavlok-cli/releases). From a
checkout of the workspace:

```sh
cargo install --path .
cargo build --release   # binary at ../../target/release/pavlok-cli
```

## Authentication

Log in once to store a bearer token at `~/.config/pavlok-cli/config.toml`
(written `0600`):

```sh
pavlok-cli login                        # prompts for email + password
pavlok-cli login --email you@example.com
```

Or provide a token directly via `PAVLOK_TOKEN`, which always takes priority over
the config file:

```sh
export PAVLOK_TOKEN="your-token"
```

## Commands

```sh
pavlok-cli zap 50                 # electric stimulus, intensity 1-100 (default 50)
pavlok-cli beep 30
pavlok-cli vibe 40 --reason "pomodoro over"
pavlok-cli whoami                 # print current account as JSON
pavlok-cli history                # print recently received stimuli as JSON
pavlok-cli mcp                    # run as an MCP server over stdio
```

Intensity is validated to `1..=100` locally before any request is made.

## MCP server

```sh
PAVLOK_TOKEN="your-token" pavlok-cli mcp
```

Exposes three tools — `zap`, `beep`, `vibe` — each taking an intensity `value`
(1–100) and an optional `reason`. Account/history data is intentionally **not**
exposed to the AI. All logging goes to stderr so the JSON-RPC on stdout is never
corrupted.

Register with Claude Code:

```sh
claude mcp add pavlok --env PAVLOK_TOKEN=your-token -- /absolute/path/to/pavlok-cli mcp
```

Note that `--env PAVLOK_TOKEN=...` leaves the token in your shell history and in
the MCP client's config file in plaintext; `pavlok-cli login` stores it at
`~/.config/pavlok-cli/config.toml` (mode `0600`) instead, and the server reads it
from there with no `--env` needed.

See the [workspace README](https://github.com/rustafariandev/pavlok-cli) for the full project overview and a
manual MCP client config example.

## License

MIT — see [LICENSE](LICENSE).
