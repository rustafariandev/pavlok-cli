//! Thin async client for the Pavlok v5 HTTP API.
//!
//! Endpoints used:
//!   POST /api/v5/users/login    -> obtain a bearer token
//!   POST /api/v5/stimulus/send  -> trigger zap/beep/vibe
//!   GET  /api/v5/user/          -> current account

#![deny(missing_docs)]

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

/// Base URL of the public Pavlok API, used by [`PavlokClient::new`].
pub const DEFAULT_BASE_URL: &str = "https://api.pavlok.com";

/// Install the `ring` rustls crypto provider exactly once per process.
///
/// We build reqwest with `rustls-no-provider` so that no C toolchain or cmake is
/// needed to compile this crate; the tradeoff is that reqwest panics when
/// constructing a `Client` if no provider has been installed. Doing it here
/// keeps [`PavlokClient::new`] infallible for callers who know nothing about
/// rustls.
///
/// An `Err` from `install_default` only means some other part of the process got
/// there first, which is equally fine — a provider is installed either way — so
/// it is deliberately ignored.
fn install_crypto_provider() {
    static TLS_PROVIDER: std::sync::Once = std::sync::Once::new();
    TLS_PROVIDER.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// The kinds of stimulus a Pavlok device can deliver.
///
/// Serializes to the lowercase strings the API expects (`"zap"`, `"beep"`,
/// `"vibe"`), so this single enum is the source of truth for both the CLI and
/// the MCP server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StimulusType {
    /// A brief electric shock.
    Zap,
    /// An audible tone.
    Beep,
    /// A silent vibration.
    Vibe,
}

impl StimulusType {
    /// The lowercase wire name the API uses for this stimulus (`"zap"`,
    /// `"beep"`, `"vibe"`).
    pub fn as_str(self) -> &'static str {
        match self {
            StimulusType::Zap => "zap",
            StimulusType::Beep => "beep",
            StimulusType::Vibe => "vibe",
        }
    }
}

/// An async client for the Pavlok v5 HTTP API.
///
/// Construct one with [`PavlokClient::new`], or [`PavlokClient::with_base_url`]
/// to point at a staging server or a mock. The token is optional so the same
/// type can drive the unauthenticated [`login`](PavlokClient::login) flow.
pub struct PavlokClient {
    http: reqwest::Client,
    token: Option<String>,
    base_url: String,
}

// --- Request / response bodies -------------------------------------------------

#[derive(Serialize)]
struct LoginRequest {
    user: LoginCredentials,
}

#[derive(Serialize)]
struct LoginCredentials {
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct LoginResponse {
    user: LoginUser,
}

#[derive(Deserialize)]
struct LoginUser {
    token: String,
}

/// The response of [`PavlokClient::whoami`] — the current account plus its
/// reward-point balance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhoamiResponse {
    /// The authenticated account.
    pub user: User,
    /// Reward points ("volts") the account has accumulated.
    pub volts: i64,
}

/// A Pavlok account, as returned by `GET /api/v5/user/`.
///
/// The upstream API is undocumented and adds fields over time, so unknown
/// fields are ignored and missing ones fall back to [`Default`] rather than
/// failing the whole request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct User {
    /// Numeric account id.
    pub id: i64,
    /// Display name; auto-generated (e.g. "Granting Gray Zapdos") until changed.
    pub username: String,
    /// Account email address.
    pub email: String,
    /// Bearer token for this account, echoed back by the endpoint.
    pub token: String,
    /// Phone number, without the [`country_code`](User::country_code).
    pub phone: Option<String>,
    /// Dialling code for [`phone`](User::phone), e.g. `"+1"`.
    pub country_code: Option<String>,
    /// Whether the phone number has been verified.
    pub phone_confirmed: bool,
    /// Given name.
    pub first_name: Option<String>,
    /// Family name.
    pub last_name: Option<String>,
    /// Date of birth, as sent by the API.
    pub birth_date: Option<String>,
    /// Body weight, in [`weight_measurement_unit`](User::weight_measurement_unit).
    pub weight: Option<f64>,
    /// Unit [`weight`](User::weight) is expressed in.
    pub weight_measurement_unit: Option<String>,
    /// Body height, in [`height_measurement_unit`](User::height_measurement_unit).
    pub height: Option<f64>,
    /// Unit [`height`](User::height) is expressed in.
    pub height_measurement_unit: Option<String>,
    /// Outstanding password-reset token, if a reset is in flight.
    pub reset_password_token: Option<String>,
    /// When the password-reset email was sent.
    pub reset_password_sent_at: Option<String>,
    /// Id of the uploaded profile picture.
    pub profile_picture_id: Option<i64>,
    /// Public URL of the profile picture.
    pub profile_picture_url: Option<String>,
    /// Whether the email address has been verified.
    pub email_confirmed: bool,
    /// Whether this is an anonymous (not fully registered) account.
    pub anonymous: bool,
    /// Account role, if the API assigned one.
    pub role: Option<String>,
    /// IANA timezone name, e.g. `"America/Toronto"`.
    pub timezone: Option<String>,
    /// Free-form answer to the "how did you hear about us" question.
    pub heard_about_us: Option<String>,
    /// Per-account settings, keyed by [`Setting::setting_key`].
    pub settings: BTreeMap<String, Setting>,
}

/// One entry of [`User::settings`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Setting {
    /// When the setting was created.
    pub created_at: Option<String>,
    /// When the setting was last updated.
    pub updated_at: Option<String>,
    /// When the setting was soft-deleted, if it was.
    pub deleted_at: Option<String>,
    /// Numeric id of the setting row.
    pub id: i64,
    /// The setting's name, matching its key in [`User::settings`].
    pub setting_key: String,
    /// The value, always sent as a string regardless of
    /// [`setting_type`](Setting::setting_type).
    pub setting_value: String,
    /// How to interpret [`setting_value`](Setting::setting_value), e.g.
    /// `"boolean"` or `"string"`.
    pub setting_type: String,
    /// Additional metadata; shape varies per setting.
    pub setting_meta: serde_json::Value,
    /// Id of the account the setting belongs to.
    pub user_id: i64,
}

#[derive(Serialize)]
struct StimulusRequest {
    stimulus: StimulusBody,
}

#[derive(Serialize)]
struct StimulusBody {
    #[serde(rename = "stimulusType")]
    stimulus_type: StimulusType,
    #[serde(rename = "stimulusValue")]
    stimulus_value: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

impl PavlokClient {
    /// Create a client against the default Pavlok base URL. `token` may be
    /// `None` for the login flow (which does not require authentication) or for
    /// surfacing a friendly error otherwise.
    pub fn new(token: Option<String>) -> Self {
        Self::with_base_url(DEFAULT_BASE_URL, token)
    }

    /// Create a client against a custom base URL (e.g. a staging server or, in
    /// tests, a mock server). The URL should not have a trailing slash.
    pub fn with_base_url(base_url: impl Into<String>, token: Option<String>) -> Self {
        install_crypto_provider();
        Self {
            http: reqwest::Client::new(),
            token,
            base_url: base_url.into(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn require_token(&self) -> Result<&str> {
        self.token
            .as_deref()
            .context("no Pavlok token configured — run `pavlok-cli login` or set PAVLOK_TOKEN")
    }

    /// Exchange email/password for a bearer token.
    pub async fn login(&self, email: &str, password: &str) -> Result<String> {
        let body = LoginRequest {
            user: LoginCredentials {
                email: email.to_string(),
                password: password.to_string(),
            },
        };
        let resp = self
            .http
            .post(self.url("/api/v5/users/login"))
            .json(&body)
            .send()
            .await
            .context("login request failed")?;
        let resp = ensure_ok(resp).await?;
        let parsed: LoginResponse = resp.json().await.context("parsing login response")?;
        Ok(parsed.user.token)
    }

    /// Send a stimulus to the device. Validates the intensity range *before*
    /// touching the network or requiring a token.
    pub async fn send_stimulus(
        &self,
        kind: StimulusType,
        value: u8,
        reason: Option<String>,
    ) -> Result<()> {
        if !(1..=100).contains(&value) {
            bail!("stimulus value must be between 1 and 100 (got {value})");
        }
        let token = self.require_token()?;
        let body = StimulusRequest {
            stimulus: StimulusBody {
                stimulus_type: kind,
                stimulus_value: value,
                reason,
            },
        };
        let resp = self
            .http
            .post(self.url("/api/v5/stimulus/send"))
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .context("stimulus request failed")?;
        ensure_ok(resp).await?;
        Ok(())
    }

    /// Fetch the current account, as a [`WhoamiResponse`].
    pub async fn whoami(&self) -> Result<WhoamiResponse> {
        let token = self.require_token()?;
        let resp = self
            .http
            .get(self.url("/api/v5/user/"))
            .bearer_auth(token)
            .send()
            .await
            .context("user request failed")?;
        let resp = ensure_ok(resp).await?;
        resp.json().await.context("parsing user response")
    }
}

/// Turn a non-2xx response into an error that includes the server's body.
async fn ensure_ok(resp: reqwest::Response) -> Result<reqwest::Response> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    let body = resp.text().await.unwrap_or_default();
    bail!("Pavlok API error {status}: {body}");
}
