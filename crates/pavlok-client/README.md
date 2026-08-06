# pavlok-client

A small async Rust client for the [Pavlok](https://pavlok.com) v5 HTTP API.

It wraps the endpoints needed to authenticate and drive a Pavlok device — send
stimuli (zap / beep / vibe) and read the current account — with a minimal
dependency footprint (just `reqwest` + `serde`). It powers the
[`pavlok-cli`](https://github.com/rustafariandev/pavlok-cli/tree/main/crates/pavlok-cli)
binary but is usable on its own.

TLS is provided by `rustls` with the pure-Rust `ring` backend, so building this
crate needs no cmake and no system OpenSSL. The provider is installed
automatically on first client construction.

## Add it

```toml
[dependencies]
pavlok-client = "1.0"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

## Usage

```rust
use pavlok_client::{PavlokClient, StimulusType};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Exchange credentials for a bearer token (or bring your own).
    let client = PavlokClient::new(None);
    let token = client.login("me@example.com", "hunter2").await?;

    // Authenticated client.
    let client = PavlokClient::new(Some(token));

    // Trigger a vibration at intensity 40 (valid range: 1-100).
    client
        .send_stimulus(StimulusType::Vibe, 40, Some("break time".into()))
        .await?;

    // Read the current account.
    let me = client.whoami().await?;
    println!("{} has {} volts", me.user.username, me.volts);

    Ok(())
}
```

## API

| Method | Description |
|---|---|
| `PavlokClient::new(token)` | Client against the default base URL (`https://api.pavlok.com`). |
| `PavlokClient::with_base_url(url, token)` | Client against a custom base URL (staging or a mock server in tests). |
| `login(email, password) -> String` | Obtain a bearer token. |
| `send_stimulus(kind, value, reason)` | Send a zap/beep/vibe; `value` is validated to `1..=100` before any request. |
| `whoami() -> WhoamiResponse` | Fetch the current account (typed `User` plus the volts balance). |

`StimulusType` (`Zap` / `Beep` / `Vibe`) serializes to the lowercase strings the
API expects and is the single source of truth for the stimulus kinds.

## License

MIT — see [LICENSE](LICENSE). Part of the
[`pavlok-cli`](https://github.com/rustafariandev/pavlok-cli) project.
