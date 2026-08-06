//! Integration tests for `pavlok-client`, exercising every public method
//! against a `wiremock` mock of the Pavlok v5 API.

use pavlok_client::{PavlokClient, StimulusType};
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// --- Pure logic (no network) ---------------------------------------------------

#[test]
fn stimulus_type_as_str_matches_api_strings() {
    assert_eq!(StimulusType::Zap.as_str(), "zap");
    assert_eq!(StimulusType::Beep.as_str(), "beep");
    assert_eq!(StimulusType::Vibe.as_str(), "vibe");
}

#[test]
fn stimulus_type_serializes_lowercase() {
    assert_eq!(
        serde_json::to_value(StimulusType::Zap).unwrap(),
        json!("zap")
    );
    assert_eq!(
        serde_json::to_value(StimulusType::Beep).unwrap(),
        json!("beep")
    );
}

#[tokio::test]
async fn send_stimulus_rejects_out_of_range_before_network() {
    // No token and an unreachable base URL: if validation did not run first,
    // this would fail on the token/connection instead of the range check.
    let client = PavlokClient::with_base_url("http://127.0.0.1:1", None);

    for bad in [0u8, 101, 250] {
        let err = client
            .send_stimulus(StimulusType::Zap, bad, None)
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("between 1 and 100"),
            "value {bad} should be rejected by range check, got: {err}"
        );
    }
}

#[tokio::test]
async fn send_stimulus_requires_token() {
    let client = PavlokClient::with_base_url("http://127.0.0.1:1", None);
    let err = client
        .send_stimulus(StimulusType::Zap, 50, None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("token"), "got: {err}");
}

#[tokio::test]
async fn whoami_requires_token() {
    let client = PavlokClient::with_base_url("http://127.0.0.1:1", None);
    let err = client.whoami().await.unwrap_err();
    assert!(err.to_string().contains("token"), "got: {err}");
}

// --- Networked methods (mock server) -------------------------------------------

#[tokio::test]
async fn login_sends_credentials_and_returns_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v5/users/login"))
        .and(body_json(json!({
            "user": { "email": "me@example.com", "password": "hunter2" }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "user": { "id": 1, "email": "me@example.com", "token": "tok-123" }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = PavlokClient::with_base_url(server.uri(), None);
    let token = client.login("me@example.com", "hunter2").await.unwrap();
    assert_eq!(token, "tok-123");
}

#[tokio::test]
async fn send_stimulus_posts_expected_body_with_bearer_auth() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v5/stimulus/send"))
        .and(header("authorization", "Bearer tok-123"))
        .and(body_json(json!({
            "stimulus": {
                "stimulusType": "zap",
                "stimulusValue": 60,
                "reason": "focus"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "success": true })))
        .expect(1)
        .mount(&server)
        .await;

    let client = PavlokClient::with_base_url(server.uri(), Some("tok-123".into()));
    client
        .send_stimulus(StimulusType::Zap, 60, Some("focus".into()))
        .await
        .unwrap();
}

#[tokio::test]
async fn send_stimulus_omits_reason_when_none() {
    let server = MockServer::start().await;
    // body_json requires an *exact* match, so the absence of "reason" is asserted.
    Mock::given(method("POST"))
        .and(path("/api/v5/stimulus/send"))
        .and(body_json(json!({
            "stimulus": { "stimulusType": "vibe", "stimulusValue": 25 }
        })))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let client = PavlokClient::with_base_url(server.uri(), Some("tok".into()));
    client
        .send_stimulus(StimulusType::Vibe, 25, None)
        .await
        .unwrap();
}

#[tokio::test]
async fn api_error_surfaces_status_and_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v5/stimulus/send"))
        .respond_with(ResponseTemplate::new(422).set_body_string("{\"error\":\"nope\"}"))
        .mount(&server)
        .await;

    let client = PavlokClient::with_base_url(server.uri(), Some("tok".into()));
    let err = client
        .send_stimulus(StimulusType::Beep, 40, None)
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("422"), "should include status, got: {msg}");
    assert!(msg.contains("nope"), "should include body, got: {msg}");
}

#[tokio::test]
async fn whoami_parses_user_response() {
    let server = MockServer::start().await;
    // Abridged from a real response; the endpoint is served at a trailing slash.
    Mock::given(method("GET"))
        .and(path("/api/v5/user/"))
        .and(header("authorization", "Bearer tok"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "user": {
                "id": 173,
                "username": "Granting Gray Zapdos",
                "email": "me@example.com",
                "token": "tok",
                "phone": null,
                "countryCode": "+1",
                "phoneConfirmed": false,
                "firstName": "Ada",
                "lastName": "Lovelace",
                "emailConfirmed": false,
                "anonymous": true,
                "role": null,
                "timezone": "America/Toronto",
                "profilePictureId": 918,
                "settings": {
                    "preferred_language": {
                        "createdAt": "2015-09-08T08:52:33.454443Z",
                        "updatedAt": "2023-07-11T14:00:20.477885Z",
                        "deletedAt": null,
                        "id": 173,
                        "settingKey": "preferred_language",
                        "settingValue": "en",
                        "settingType": "string",
                        "settingMeta": {},
                        "userId": 173
                    }
                }
            },
            "volts": 151920
        })))
        .mount(&server)
        .await;

    let client = PavlokClient::with_base_url(server.uri(), Some("tok".into()));
    let resp = client.whoami().await.unwrap();
    assert_eq!(resp.user.id, 173);
    assert_eq!(resp.user.email, "me@example.com");
    assert_eq!(resp.user.first_name.as_deref(), Some("Ada"));
    assert_eq!(resp.user.phone, None);
    assert!(resp.user.anonymous);
    assert_eq!(resp.volts, 151920);
    assert_eq!(resp.user.settings["preferred_language"].setting_value, "en");
}

#[tokio::test]
async fn whoami_tolerates_missing_and_unknown_fields() {
    // The upstream shape is undocumented: absent fields must fall back to their
    // defaults and newly added ones must be ignored, not fail the request.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v5/user/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "user": { "id": 1, "somethingNewUpstream": ["anything"] },
            "volts": 0
        })))
        .mount(&server)
        .await;

    let client = PavlokClient::with_base_url(server.uri(), Some("tok".into()));
    let resp = client.whoami().await.unwrap();
    assert_eq!(resp.user.id, 1);
    assert_eq!(resp.user.email, "");
    assert!(resp.user.settings.is_empty());
}
