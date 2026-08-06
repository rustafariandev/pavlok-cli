# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 1.0.0

A complete rewrite. The version jumps straight from 0.1.0 (published January
2023) to 1.0.0 because essentially nothing is shared with that release; the
0.1.0 line is superseded and will not receive further updates.

### Added

- **MCP server mode** — `pavlok-cli mcp` runs a [Model Context
  Protocol](https://modelcontextprotocol.io) server over stdio, letting an AI
  assistant trigger stimuli as tool calls. `--tools zap,beep,vibe` restricts
  which stimuli are exposed; account data is deliberately never exposed as a
  tool.
- **`pavlok-client`** — the API client is now a separate, reusable library crate
  published alongside the CLI. `whoami()` returns a typed `WhoamiResponse` — a
  `User` (including a `settings` map of `Setting` records) plus the volts
  balance — with camelCase wire names handled by serde. Unknown fields are
  ignored and missing ones fall back to their defaults, so additions to the
  undocumented upstream API do not break the parse.
- Prebuilt binaries for Linux, macOS, and Windows attached to each GitHub
  release, so a Rust toolchain is no longer required to install.
- `pavlok-cli login` stores the token at `~/.config/pavlok-cli/config.toml` with
  `0600` permissions on Unix. `PAVLOK_TOKEN` takes precedence when set.

### Changed

- TLS now uses `rustls` with the pure-Rust `ring` backend instead of
  `aws-lc-rs`. This removes cmake and the bundled AWS-LC C library from the
  build, so `cargo install pavlok-cli` no longer needs a cmake installation.
- Stimulus intensity is validated to `1..=100` before any network request.

### Fixed

- `--help` output and error messages now refer to `pavlok-cli`, which is the
  actual installed binary name. They previously said `pavlok`, a command that
  does not exist.
