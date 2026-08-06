//! The `whoami` command: a readable account summary, or the raw API response.
//!
//! The account's bearer token is deliberately never printed. The endpoint
//! echoes it back in every response, and `whoami` output is the natural thing
//! to paste into a bug report.

use std::io::{self, Write};

use anstyle::Style;
use anyhow::Result;
use pavlok_client::{PavlokClient, User, WhoamiResponse};

use crate::config;

const HEADER: Style = Style::new().bold();
const LABEL: Style = Style::new().dimmed();
const NOTE: Style = Style::new().dimmed();

pub async fn run(json: bool, all: bool) -> Result<()> {
    let client = PavlokClient::new(Some(config::require_token()?));
    let resp = client.whoami().await?;

    if json {
        println!("{}", redacted_json(&resp)?);
    } else {
        // AutoStream strips the escape sequences when stdout is not a terminal,
        // or when NO_COLOR / CLICOLOR / TERM=dumb say so.
        render(&mut anstream::stdout().lock(), &resp, all)?;
    }
    Ok(())
}

/// The raw response as pretty JSON, minus the echoed bearer token.
///
/// Redacting here rather than with `skip_serializing` on the field keeps
/// `pavlok_client::User` round-trippable for library consumers; hiding the
/// token is a presentation choice that belongs to the CLI.
fn redacted_json(resp: &WhoamiResponse) -> Result<String> {
    let mut value = serde_json::to_value(resp)?;
    if let Some(user) = value.get_mut("user").and_then(|u| u.as_object_mut()) {
        user.remove("token");
    }
    Ok(serde_json::to_string_pretty(&value)?)
}

/// Write the human-readable summary. `all` adds the settings map and the
/// profile fields most accounts leave unset.
pub fn render(w: &mut impl Write, resp: &WhoamiResponse, all: bool) -> io::Result<()> {
    let user = &resp.user;
    let header = display_name(user);

    writeln!(w, "{}{header}{}", HEADER.render(), HEADER.render_reset())?;

    let mut rows: Vec<(&str, String)> = Vec::new();

    if let Some(name) = full_name(user).filter(|name| *name != header) {
        rows.push(("Name", name));
    }
    if let Some(email) = present(&user.email) {
        rows.push((
            "Email",
            format!("{email} {}", confirmation(user.email_confirmed)),
        ));
    }
    if let Some(phone) = opt(&user.phone) {
        let number = match opt(&user.country_code) {
            Some(code) => format!("{code} {phone}"),
            None => phone.to_string(),
        };
        rows.push((
            "Phone",
            format!("{number} {}", confirmation(user.phone_confirmed)),
        ));
    }
    rows.push(("Account", format!("#{}", user.id)));
    rows.push(("Volts", thousands(resp.volts)));
    if let Some(timezone) = opt(&user.timezone) {
        rows.push(("Timezone", timezone.to_string()));
    }
    if let Some(role) = opt(&user.role) {
        rows.push(("Role", role.to_string()));
    }
    if all {
        // Not shown by default: upstream reports `anonymous: true` even for
        // fully registered accounts, so it is noise rather than signal.
        if user.anonymous {
            rows.push(("Anonymous", "yes".to_string()));
        }
        if let Some(birth_date) = opt(&user.birth_date) {
            rows.push(("Born", birth_date.to_string()));
        }
        if let Some(weight) = measurement(user.weight, &user.weight_measurement_unit) {
            rows.push(("Weight", weight));
        }
        if let Some(height) = measurement(user.height, &user.height_measurement_unit) {
            rows.push(("Height", height));
        }
        if let Some(url) = opt(&user.profile_picture_url) {
            rows.push(("Picture", url.to_string()));
        }
        if let Some(heard) = opt(&user.heard_about_us) {
            rows.push(("Heard via", heard.to_string()));
        }
    }

    write_rows(w, &rows)?;

    if all && !user.settings.is_empty() {
        writeln!(w)?;
        writeln!(w, "{}Settings{}", HEADER.render(), HEADER.render_reset())?;
        // `settings` is a BTreeMap, so this is already key-sorted.
        let settings: Vec<(&str, String)> = user
            .settings
            .iter()
            .map(|(key, setting)| (key.as_str(), setting.setting_value.clone()))
            .collect();
        write_rows(w, &settings)?;
    }

    Ok(())
}

/// Write indented `label  value` pairs, aligned to the widest label.
fn write_rows(w: &mut impl Write, rows: &[(&str, String)]) -> io::Result<()> {
    let width = rows.iter().map(|(label, _)| label.len()).max().unwrap_or(0);
    for (label, value) in rows {
        writeln!(
            w,
            "  {}{label:<width$}{}  {value}",
            LABEL.render(),
            LABEL.render_reset(),
        )?;
    }
    Ok(())
}

/// The account's best available name. `User` derives `Default`, and the API
/// omits fields freely, so every candidate can be empty.
fn display_name(user: &User) -> String {
    present(&user.username)
        .or_else(|| present(&user.email))
        .map(str::to_string)
        .unwrap_or_else(|| format!("Account #{}", user.id))
}

fn full_name(user: &User) -> Option<String> {
    let parts: Vec<&str> = [&user.first_name, &user.last_name]
        .into_iter()
        .filter_map(opt)
        .collect();
    (!parts.is_empty()).then(|| parts.join(" "))
}

fn measurement(value: Option<f64>, unit: &Option<String>) -> Option<String> {
    let value = value?;
    Some(match opt(unit) {
        Some(unit) => format!("{value} {unit}"),
        None => value.to_string(),
    })
}

fn confirmation(confirmed: bool) -> String {
    let text = if confirmed {
        "(confirmed)"
    } else {
        "(unconfirmed)"
    };
    format!("{}{text}{}", NOTE.render(), NOTE.render_reset())
}

/// The trimmed contents of a string field, if it has any. The API sends `""`
/// about as often as it omits a field.
fn present(field: &str) -> Option<&str> {
    Some(field.trim()).filter(|s| !s.is_empty())
}

/// [`present`] for the fields the API models as nullable.
fn opt(field: &Option<String>) -> Option<&str> {
    field.as_deref().and_then(present)
}

/// Group digits for readability: `151920` -> `151,920`.
fn thousands(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    if n < 0 {
        out.push('-');
    }
    for (i, c) in digits.char_indices() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use pavlok_client::Setting;

    use super::*;

    /// Render with the escape sequences stripped, the way a piped or
    /// `NO_COLOR` terminal sees it.
    fn plain(resp: &WhoamiResponse, all: bool) -> String {
        let mut out = anstream::StripStream::new(Vec::new());
        render(&mut out, resp, all).unwrap();
        String::from_utf8(out.into_inner()).unwrap()
    }

    fn setting(key: &str, value: &str) -> (String, Setting) {
        (
            key.to_string(),
            Setting {
                setting_key: key.to_string(),
                setting_value: value.to_string(),
                ..Default::default()
            },
        )
    }

    fn full_account() -> WhoamiResponse {
        WhoamiResponse {
            user: User {
                id: 48213,
                username: "Granting Gray Zapdos".to_string(),
                email: "user@example.com".to_string(),
                token: "super-secret-token".to_string(),
                phone: Some("5551234567".to_string()),
                country_code: Some("+1".to_string()),
                email_confirmed: true,
                first_name: Some("Ada".to_string()),
                last_name: Some("Lovelace".to_string()),
                timezone: Some("America/Toronto".to_string()),
                settings: BTreeMap::from([
                    setting("preferred_language", "en"),
                    setting("notifications_enabled", "true"),
                ]),
                ..Default::default()
            },
            volts: 151_920,
        }
    }

    #[test]
    fn summary_shows_identity_and_never_the_token() {
        let out = plain(&full_account(), false);

        // The label column is sized to the widest label present, here "Timezone".
        assert_eq!(
            out,
            "Granting Gray Zapdos\n\
             \x20 Name      Ada Lovelace\n\
             \x20 Email     user@example.com (confirmed)\n\
             \x20 Phone     +1 5551234567 (unconfirmed)\n\
             \x20 Account   #48213\n\
             \x20 Volts     151,920\n\
             \x20 Timezone  America/Toronto\n"
        );
        assert!(!out.contains("super-secret-token"), "token leaked: {out}");
    }

    #[test]
    fn settings_are_hidden_until_all_is_passed() {
        assert!(!plain(&full_account(), false).contains("Settings"));

        let out = plain(&full_account(), true);
        let settings = out.split_once("Settings\n").expect("settings section").1;
        // BTreeMap iteration order: notifications_enabled before preferred_language.
        assert_eq!(
            settings,
            "  notifications_enabled  true\n  preferred_language     en\n"
        );
    }

    /// Upstream sets `anonymous` on registered accounts too, so it must not
    /// contradict the name and email sitting right above it.
    #[test]
    fn anonymous_flag_is_hidden_until_all_is_passed() {
        let mut resp = full_account();
        resp.user.anonymous = true;

        assert!(!plain(&resp, false).contains("Anonymous"));
        assert!(plain(&resp, true).contains("Anonymous  yes"));
    }

    #[test]
    fn sparse_account_omits_unset_fields() {
        let resp = WhoamiResponse {
            user: User {
                id: 7,
                ..Default::default()
            },
            volts: 0,
        };
        let out = plain(&resp, true);

        assert_eq!(out, "Account #7\n  Account  #7\n  Volts    0\n");
    }

    #[test]
    fn header_falls_back_to_email_then_id() {
        let mut resp = full_account();
        resp.user.username = String::new();
        assert!(plain(&resp, false).starts_with("user@example.com\n"));

        resp.user.email = String::new();
        assert!(plain(&resp, false).starts_with("Account #48213\n"));
    }

    #[test]
    fn styling_is_strippable() {
        // The styled form carries escapes; StripStream removes every one.
        let mut styled = Vec::new();
        render(&mut styled, &full_account(), true).unwrap();
        assert!(styled.contains(&0x1b));
        assert!(!plain(&full_account(), true).contains('\u{1b}'));
    }

    #[test]
    fn json_output_drops_the_token() {
        let json = redacted_json(&full_account()).unwrap();

        assert!(!json.contains("super-secret-token"), "{json}");
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(value["user"].get("token").is_none());
        // Everything else survives, including the settings map.
        assert_eq!(value["user"]["id"], 48213);
        assert_eq!(value["volts"], 151_920);
        assert_eq!(
            value["user"]["settings"]["preferred_language"]["settingValue"],
            "en"
        );
    }

    #[test]
    fn thousands_groups_digits() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1_000), "1,000");
        assert_eq!(thousands(151_920), "151,920");
        assert_eq!(thousands(-1_234_567), "-1,234,567");
    }
}
