// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Anti-cheat and abuse-report contracts tested locally without reporting users.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::{Value, json};
use std::{collections::BTreeMap, time::Duration};
use steam_api_web::{
    ApiKey, Error, RetryPolicy, SteamClient, SteamId,
    cheat_reporting::{
        CheatingReportsRequest, EndSecureMultiplayerSessionRequest, GameBanDuration,
        ReportPlayerCheatingRequest, RequestPlayerGameBanRequest, VacStatusRequest,
    },
    community::ReportAbuseRequest,
};
use wiremock::{
    Mock, MockServer, Request, ResponseTemplate,
    matchers::{header, method, path},
};

const ID: SteamId = SteamId(u64::MAX);
const PRIVATE: &str = "private evidence &+=中文";
const METHODS: [(&str, &str, &str); 8] = [
    ("ICheatReportingService", "ReportPlayerCheating", "POST"),
    ("ICheatReportingService", "RequestPlayerGameBan", "POST"),
    ("ICheatReportingService", "RemovePlayerGameBan", "POST"),
    ("ICheatReportingService", "GetCheatingReports", "GET"),
    ("ICheatReportingService", "RequestVacStatusForUser", "POST"),
    (
        "ICheatReportingService",
        "StartSecureMultiplayerSession",
        "POST",
    ),
    (
        "ICheatReportingService",
        "EndSecureMultiplayerSession",
        "POST",
    ),
    ("ISteamCommunity", "ReportAbuse", "POST"),
];

fn client(server: &MockServer, publisher: bool) -> SteamClient {
    let key = if publisher {
        ApiKey::publisher("test-publisher-key")
    } else {
        ApiKey::user("test-user-key")
    };
    SteamClient::builder()
        .base_url(server.uri())
        .api_key(key.unwrap())
        .retry_policy(RetryPolicy {
            max_retries: 2,
            initial_backoff: Duration::ZERO,
            max_backoff: Duration::ZERO,
        })
        .build()
        .unwrap()
}

fn report() -> ReportPlayerCheatingRequest {
    ReportPlayerCheatingRequest {
        steamid: ID,
        appid: 480,
        steamidreporter: Some(SteamId(u64::MAX - 1)),
        appdata: Some(u64::MAX),
        heuristic: Some(false),
        detection: Some(true),
        playerreport: Some(true),
        noreportid: Some(false),
        gamemode: Some(0),
        suspicionstarttime: Some(u32::MAX),
        severity: Some(u32::MAX),
    }
}

fn ban() -> RequestPlayerGameBanRequest<'static> {
    RequestPlayerGameBanRequest {
        steamid: ID,
        appid: 480,
        reportid: u64::MAX,
        cheatdescription: PRIVATE,
        duration: GameBanDuration::Permanent,
        delayban: false,
        flags: u32::MAX,
    }
}

fn reports() -> CheatingReportsRequest {
    CheatingReportsRequest {
        appid: 480,
        timeend: u32::MAX,
        timebegin: 0,
        reportidmin: u64::MAX,
        includereports: Some(true),
        includebans: Some(false),
        steamid: Some(ID),
    }
}

fn vac() -> VacStatusRequest {
    VacStatusRequest {
        steamid: ID,
        appid: 480,
        session_id: Some(u64::MAX),
    }
}

fn end() -> EndSecureMultiplayerSessionRequest {
    EndSecureMultiplayerSessionRequest {
        steamid: ID,
        appid: 480,
        session_id: u64::MAX,
    }
}

fn abuse() -> ReportAbuseRequest<'static> {
    ReportAbuseRequest {
        steamid_actor: ID,
        steamid_target: SteamId(u64::MAX - 1),
        appid: 480,
        abuse_type: u32::MAX,
        content_type: u32::MAX,
        description: PRIVATE,
        gid: Some(u64::MAX),
    }
}

fn form(request: &Request) -> BTreeMap<String, String> {
    if request.method.as_str() == "POST" {
        assert!(request.url.query().is_none());
        assert_eq!(
            request.headers.get("content-type").unwrap(),
            "application/x-www-form-urlencoded"
        );
        url::form_urlencoded::parse(&request.body)
            .into_owned()
            .collect()
    } else {
        assert!(request.body.is_empty());
        request.url.query_pairs().into_owned().collect()
    }
}

fn input(request: &Request) -> Value {
    let params = form(request);
    assert_eq!(params.len(), 2);
    assert_eq!(params["format"], "json");
    serde_json::from_str(&params["input_json"]).unwrap()
}

async fn call(client: &SteamClient, name: &str) -> steam_api_web::Result<()> {
    let service = client.cheat_reporting();
    match name {
        "ReportPlayerCheating" => service.report_player_cheating(&report()).await.map(|_| ()),
        "RequestPlayerGameBan" => service.request_player_game_ban(&ban()).await.map(|_| ()),
        "RemovePlayerGameBan" => service.remove_player_game_ban(ID, 480).await.map(|_| ()),
        "GetCheatingReports" => service.get_cheating_reports(&reports()).await.map(|_| ()),
        "RequestVacStatusForUser" => service
            .request_vac_status_for_user(&vac())
            .await
            .map(|_| ()),
        "StartSecureMultiplayerSession" => service
            .start_secure_multiplayer_session(ID, 480)
            .await
            .map(|_| ()),
        "EndSecureMultiplayerSession" => service
            .end_secure_multiplayer_session(&end())
            .await
            .map(|_| ()),
        "ReportAbuse" => client.community().report_abuse(&abuse()).await.map(|_| ()),
        _ => panic!("unknown test method"),
    }
}

#[tokio::test]
async fn all_eight_methods_use_exact_paths_key_kind_and_request_shapes() {
    let server = MockServer::start().await;
    for (interface, name, verb) in METHODS {
        Mock::given(method(verb))
            .and(path(format!("/{interface}/{name}/v1/")))
            .and(header("x-webapi-key", "test-publisher-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response": {"success": true, "session_verified": false, "reportid": u64::MAX.to_string(), "future": PRIVATE}})))
            .expect(1).mount(&server).await;
    }
    let client = client(&server, true);
    for (_, name, _) in METHODS {
        call(&client, name).await.unwrap();
    }
    for request in server.received_requests().await.unwrap() {
        let name = request.url.path().split('/').nth(2).unwrap();
        let fields = form(&request);
        assert!(!fields.contains_key("key"));
        if name == "ReportAbuse" {
            assert_eq!(
                fields,
                BTreeMap::from([
                    ("steamidActor".to_owned(), ID.to_string()),
                    ("steamidTarget".to_owned(), (u64::MAX - 1).to_string()),
                    ("appid".to_owned(), "480".to_owned()),
                    ("abuseType".to_owned(), u32::MAX.to_string()),
                    ("contentType".to_owned(), u32::MAX.to_string()),
                    ("description".to_owned(), PRIVATE.to_owned()),
                    ("gid".to_owned(), u64::MAX.to_string()),
                    ("format".to_owned(), "json".to_owned()),
                ])
            );
            continue;
        }
        let value = input(&request);
        assert_eq!(value["appid"], 480);
        match name {
            "ReportPlayerCheating" => assert_eq!(
                value,
                json!({
                    "steamid": ID.to_string(), "appid":480,
                    "steamidreporter":(u64::MAX - 1).to_string(), "appdata":u64::MAX,
                    "heuristic":false, "detection":true, "playerreport":true,
                    "noreportid":false, "gamemode":0, "suspicionstarttime":u32::MAX,
                    "severity":u32::MAX
                })
            ),
            "RequestPlayerGameBan" => assert_eq!(
                value,
                json!({
                    "steamid":ID.to_string(), "appid":480, "reportid":u64::MAX,
                    "cheatdescription":PRIVATE, "duration":0, "delayban":false,
                    "flags":u32::MAX
                })
            ),
            "GetCheatingReports" => assert_eq!(
                value,
                json!({
                    "appid":480, "timeend":u32::MAX, "timebegin":0,
                    "reportidmin":u64::MAX, "includereports":true,
                    "includebans":false, "steamid":ID.to_string()
                })
            ),
            "RequestVacStatusForUser" | "EndSecureMultiplayerSession" => assert_eq!(
                value,
                json!({
                    "steamid":ID.to_string(), "appid":480, "session_id":u64::MAX
                })
            ),
            "RemovePlayerGameBan" | "StartSecureMultiplayerSession" => {
                assert_eq!(value, json!({"steamid":ID.to_string(), "appid":480}))
            }
            _ => panic!("unknown test method"),
        }
    }
}

#[tokio::test]
async fn reporting_without_a_report_id_does_not_implicitly_request_a_ban() {
    let server = MockServer::start().await;
    Mock::given(path("/ICheatReportingService/ReportPlayerCheating/v1/"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"response":{"success":true}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let result = client(&server, true)
        .cheat_reporting()
        .report_player_cheating(&ReportPlayerCheatingRequest {
            noreportid: Some(true),
            ..report()
        })
        .await
        .unwrap();
    assert!(result.as_json()["response"].get("reportid").is_none());
    assert!(format!("{result:?}").contains("REDACTED"));
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(input(&requests[0])["noreportid"], true);
}

#[tokio::test]
async fn permanent_and_temporary_durations_are_explicit_and_preserve_seconds() {
    let server = MockServer::start().await;
    Mock::given(path("/ICheatReportingService/RequestPlayerGameBan/v1/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response":{}})))
        .expect(3)
        .mount(&server)
        .await;
    let client = client(&server, true);
    for duration in [
        GameBanDuration::Permanent,
        GameBanDuration::Seconds(1),
        GameBanDuration::Seconds(u32::MAX),
    ] {
        client
            .cheat_reporting()
            .request_player_game_ban(&RequestPlayerGameBanRequest { duration, ..ban() })
            .await
            .unwrap();
    }
    let requests = server.received_requests().await.unwrap();
    assert_eq!(input(&requests[0])["duration"], 0);
    assert_eq!(input(&requests[1])["duration"], 1);
    assert_eq!(input(&requests[2])["duration"], u32::MAX);
}

#[tokio::test]
async fn pending_and_unverified_vac_states_remain_distinct_from_permission_to_join() {
    for body in [
        json!({"response":{"success":false,"private_evidence":PRIVATE}}),
        json!({"response":{"success":true,"session_verified":false,"private_evidence":PRIVATE}}),
        json!({"response":{"success":true,"session_verified":true,"private_evidence":PRIVATE}}),
    ] {
        let server = MockServer::start().await;
        Mock::given(path("/ICheatReportingService/RequestVacStatusForUser/v1/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&server)
            .await;
        let client = client(&server, true);
        let response = client
            .cheat_reporting()
            .request_vac_status_for_user(&vac())
            .await
            .unwrap();
        assert_eq!(
            response.success,
            body["response"]["success"].as_bool().unwrap()
        );
        assert_eq!(
            response.session_verified,
            body["response"]["session_verified"].as_bool()
        );
        assert_eq!(response.extra["private_evidence"], PRIVATE);
        assert!(!format!("{response:?}").contains(PRIVATE));
        assert_eq!(client.metrics().succeeded, 1);
        assert_eq!(client.metrics().retries, 0);
    }
}

#[tokio::test]
async fn vac_pending_policy_does_not_suppress_errors_invalid_markers_or_siblings() {
    for body in [
        json!({"response":{"success":false,"error":{"errorcode":15,"message":PRIVATE}}}),
        json!({"response":{"success":false,"result":15}}),
        json!({"response":{"success":false},"params":{"eresult":15}}),
        json!({"success":false,"response":{"success":true}}),
        json!({"response":{"success":"false"}}),
        json!({"response":{"success":true,"session_verified":"false"}}),
        json!({"response":{}}),
    ] {
        let server = MockServer::start().await;
        Mock::given(path("/ICheatReportingService/RequestVacStatusForUser/v1/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .expect(1)
            .mount(&server)
            .await;
        let error = client(&server, true)
            .cheat_reporting()
            .request_vac_status_for_user(&vac())
            .await
            .unwrap_err();
        assert!(!format!("{error:?} {error}").contains(PRIVATE));
    }
}

#[tokio::test]
async fn optional_fields_are_omitted_and_zero_report_cursor_is_preserved() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"response":{"success":true}})),
        )
        .expect(3)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response":{}})))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server, true);
    client
        .cheat_reporting()
        .report_player_cheating(&ReportPlayerCheatingRequest {
            steamid: ID,
            appid: 480,
            steamidreporter: None,
            appdata: None,
            heuristic: None,
            detection: None,
            playerreport: None,
            noreportid: None,
            gamemode: None,
            suspicionstarttime: None,
            severity: None,
        })
        .await
        .unwrap();
    client
        .cheat_reporting()
        .get_cheating_reports(&CheatingReportsRequest {
            reportidmin: 0,
            includereports: None,
            includebans: None,
            steamid: None,
            ..reports()
        })
        .await
        .unwrap();
    client
        .cheat_reporting()
        .request_vac_status_for_user(&VacStatusRequest {
            session_id: None,
            ..vac()
        })
        .await
        .unwrap();
    client
        .community()
        .report_abuse(&ReportAbuseRequest {
            gid: None,
            ..abuse()
        })
        .await
        .unwrap();
    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        input(&requests[0]),
        json!({"appid":480,"steamid":ID.to_string()})
    );
    assert_eq!(
        input(&requests[1]),
        json!({"appid":480,"timeend":u32::MAX,"timebegin":0,"reportidmin":0})
    );
    assert_eq!(
        input(&requests[2]),
        json!({"appid":480,"steamid":ID.to_string()})
    );
    assert!(!form(&requests[3]).contains_key("gid"));
}

#[tokio::test]
async fn every_post_is_single_send_and_the_report_read_uses_bounded_retries() {
    for (interface, name, verb) in METHODS {
        for status in [429, 503] {
            let server = MockServer::start().await;
            let attempts = if verb == "GET" { 3 } else { 1 };
            Mock::given(method(verb))
                .and(path(format!("/{interface}/{name}/v1/")))
                .respond_with(ResponseTemplate::new(status).insert_header("Retry-After", "0"))
                .expect(attempts)
                .mount(&server)
                .await;
            let client = client(&server, true);
            assert!(
                matches!(call(&client,name).await,Err(Error::Http{status:actual,..}) if actual == status)
            );
            assert_eq!(client.metrics().attempts, attempts);
        }
    }
}

#[tokio::test]
async fn user_and_anonymous_credentials_cannot_reach_any_method() {
    let server = MockServer::start().await;
    let anonymous = SteamClient::builder()
        .base_url(server.uri())
        .build()
        .unwrap();
    let user = client(&server, false);
    for (_, name, _) in METHODS {
        for client in [&anonymous, &user] {
            assert!(matches!(
                call(client, name).await,
                Err(Error::MissingApiKey("publisher"))
            ));
        }
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn business_failures_and_malformed_json_never_leak_report_narratives_or_retry() {
    for (interface, name, verb) in METHODS {
        for (body, business_error) in [
            (
                json!({"response":{"error":{"errorcode":15,"description":PRIVATE}}}).to_string(),
                true,
            ),
            (format!("{{\"private\":\"{PRIVATE}"), false),
        ] {
            let server = MockServer::start().await;
            Mock::given(method(verb))
                .and(path(format!("/{interface}/{name}/v1/")))
                .respond_with(ResponseTemplate::new(200).set_body_string(body))
                .expect(1)
                .mount(&server)
                .await;
            let client = client(&server, true);
            let error = call(&client, name).await.unwrap_err();
            if business_error {
                assert!(
                    matches!(error, Error::Api { code: Some(15), .. }),
                    "{name}: {error:?}"
                );
            } else {
                assert!(matches!(error, Error::Decode { .. }), "{name}: {error:?}");
            }
            let text = format!("{error:?} {error}");
            assert!(!text.contains(PRIVATE));
            assert!(!text.contains("test-publisher-key"));
            assert_eq!(client.metrics().attempts, 1);
            assert_eq!(client.metrics().retries, 0);
        }
    }
}

#[tokio::test]
async fn ended_session_requires_its_documented_boolean_success_marker() {
    for body in [
        json!({"response":{"success":false}}),
        json!({"response":{}}),
        json!({"response":{"success":1}}),
    ] {
        let server = MockServer::start().await;
        Mock::given(path(
            "/ICheatReportingService/EndSecureMultiplayerSession/v1/",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(&server)
        .await;
        assert!(
            client(&server, true)
                .cheat_reporting()
                .end_secure_multiplayer_session(&end())
                .await
                .is_err()
        );
    }
}

#[tokio::test]
async fn invalid_identifiers_intervals_ban_duration_and_text_fail_before_network() {
    let server = MockServer::start().await;
    let client = client(&server, true);
    assert!(
        client
            .cheat_reporting()
            .report_player_cheating(&ReportPlayerCheatingRequest {
                steamidreporter: Some(SteamId(0)),
                ..report()
            })
            .await
            .is_err()
    );
    for request in [
        RequestPlayerGameBanRequest {
            reportid: 0,
            ..ban()
        },
        RequestPlayerGameBanRequest {
            duration: GameBanDuration::Seconds(0),
            ..ban()
        },
        RequestPlayerGameBanRequest {
            cheatdescription: "\0",
            ..ban()
        },
    ] {
        assert!(
            client
                .cheat_reporting()
                .request_player_game_ban(&request)
                .await
                .is_err()
        );
    }
    for request in [
        CheatingReportsRequest {
            timebegin: 2,
            timeend: 1,
            ..reports()
        },
        CheatingReportsRequest {
            includereports: Some(false),
            includebans: Some(false),
            ..reports()
        },
        CheatingReportsRequest {
            includereports: None,
            includebans: Some(false),
            ..reports()
        },
        CheatingReportsRequest {
            steamid: Some(SteamId(0)),
            ..reports()
        },
    ] {
        assert!(
            client
                .cheat_reporting()
                .get_cheating_reports(&request)
                .await
                .is_err()
        );
    }
    assert!(
        client
            .cheat_reporting()
            .request_vac_status_for_user(&VacStatusRequest {
                session_id: Some(0),
                ..vac()
            })
            .await
            .is_err()
    );
    assert!(
        client
            .cheat_reporting()
            .end_secure_multiplayer_session(&EndSecureMultiplayerSessionRequest {
                session_id: 0,
                ..end()
            })
            .await
            .is_err()
    );
    assert!(
        client
            .cheat_reporting()
            .start_secure_multiplayer_session(SteamId(0), 480)
            .await
            .is_err()
    );
    assert!(
        client
            .cheat_reporting()
            .remove_player_game_ban(ID, 0)
            .await
            .is_err()
    );
    assert!(
        client
            .community()
            .report_abuse(&ReportAbuseRequest {
                description: " ",
                ..abuse()
            })
            .await
            .is_err()
    );
    assert!(
        client
            .community()
            .report_abuse(&ReportAbuseRequest {
                steamid_target: SteamId(0),
                ..abuse()
            })
            .await
            .is_err()
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[test]
fn request_debug_output_redacts_evidence_and_report_session_identities() {
    for debug in [
        format!("{:?}", report()),
        format!("{:?}", ban()),
        format!("{:?}", reports()),
        format!("{:?}", vac()),
        format!("{:?}", end()),
        format!("{:?}", abuse()),
    ] {
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains(PRIVATE));
        assert!(!debug.contains(&u64::MAX.to_string()));
    }
}
