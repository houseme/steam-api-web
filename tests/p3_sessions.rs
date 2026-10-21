// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Session and lobby Web API contracts against local HTTP servers.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::{Value, json};
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use steam_api_web::{
    ApiKey, Error, RetryPolicy, SteamClient, SteamId,
    game_notifications::{
        CreateSessionRequest, DeleteSessionRequest, EnumerateSessionsRequest, SessionDetailsQuery,
        SessionDetailsRequest, UpdateSessionRequest,
    },
    lobby_matchmaking::{CreateLobbyRequest, LobbyMetadata},
    user_auth::OpaqueResponse,
};
use wiremock::{
    Mock, MockServer, Request, Respond, ResponseTemplate,
    matchers::{header, method, path},
};

const ID: u64 = u64::MAX;
const PRIVATE: &str = "private +&=%/中文";
const METHODS: [(&str, &str, &str, bool); 10] = [
    ("IGameNotificationsService", "CreateSession", "POST", true),
    ("IGameNotificationsService", "UpdateSession", "POST", true),
    (
        "IGameNotificationsService",
        "EnumerateSessionsForApp",
        "GET",
        true,
    ),
    (
        "IGameNotificationsService",
        "GetSessionDetailsForApp",
        "GET",
        true,
    ),
    (
        "IGameNotificationsService",
        "RequestNotifications",
        "POST",
        true,
    ),
    ("IGameNotificationsService", "DeleteSession", "POST", true),
    (
        "IGameNotificationsService",
        "DeleteSessionBatch",
        "POST",
        true,
    ),
    ("ILobbyMatchmakingService", "CreateLobby", "POST", false),
    (
        "ILobbyMatchmakingService",
        "RemoveUserFromLobby",
        "POST",
        true,
    ),
    ("ILobbyMatchmakingService", "GetLobbyData", "GET", true),
];

fn client(server: &MockServer, publisher: bool) -> SteamClient {
    let key = if publisher {
        ApiKey::publisher("sessions-key").unwrap()
    } else {
        ApiKey::user("sessions-key").unwrap()
    };
    SteamClient::builder()
        .base_url(server.uri())
        .api_key(key)
        .retry_policy(RetryPolicy {
            max_retries: 2,
            initial_backoff: Duration::ZERO,
            max_backoff: Duration::ZERO,
        })
        .build()
        .unwrap()
}

fn title() -> Value {
    json!({"token":"#SessionTitle", "variables":[{"key":"user", "value":PRIVATE}], "future":{"id":ID}})
}

fn users() -> Vec<Value> {
    vec![
        json!({"steamid":ID.to_string(), "state":"waiting", "title":title(), "message":{"token":"#UserMessage", "future":PRIVATE}}),
        json!({"steamid":42, "state":"future-state", "custom":{"values":[true, 0, PRIVATE]}}),
    ]
}

fn enumeration() -> EnumerateSessionsRequest<'static> {
    EnumerateSessionsRequest {
        steamid: ID.into(),
        appid: Some(480),
        include_all_user_messages: Some(false),
        include_auth_user_message: Some(true),
        language: Some(PRIVATE),
    }
}

fn lobby<'a>(members: &'a [SteamId], metadata: &'a [LobbyMetadata<'a>]) -> CreateLobbyRequest<'a> {
    CreateLobbyRequest {
        appid: 480,
        max_members: i32::MAX,
        lobby_type: 4,
        lobby_name: Some(PRIVATE),
        steamid_invited_members: members,
        lobby_metadata: Some(metadata),
    }
}

async fn call(client: &SteamClient, name: &str) -> steam_api_web::Result<OpaqueResponse> {
    let sessions = client.game_notifications();
    match name {
        "CreateSession" => {
            sessions
                .create_session(&CreateSessionRequest {
                    appid: 480,
                    context: ID,
                    title: &title(),
                    users: &users(),
                    steamid: Some(ID.into()),
                })
                .await
        }
        "UpdateSession" => {
            sessions
                .update_session(&UpdateSessionRequest {
                    sessionid: ID,
                    appid: 480,
                    title: Some(&title()),
                    users: Some(&users()),
                    steamid: Some(ID.into()),
                })
                .await
        }
        "EnumerateSessionsForApp" => sessions.enumerate_sessions_for_app(&enumeration()).await,
        "GetSessionDetailsForApp" => {
            sessions
                .get_session_details_for_app(&SessionDetailsRequest {
                    appid: 480,
                    language: PRIVATE,
                    sessions: &[
                        SessionDetailsQuery {
                            sessionid: ID,
                            include_all_user_messages: Some(false),
                        },
                        SessionDetailsQuery {
                            sessionid: 42,
                            include_all_user_messages: None,
                        },
                    ],
                })
                .await
        }
        "RequestNotifications" => sessions.request_notifications(ID.into(), 480).await,
        "DeleteSession" => {
            sessions
                .delete_session(&DeleteSessionRequest {
                    sessionid: ID,
                    appid: 480,
                    steamid: Some(ID.into()),
                })
                .await
        }
        "DeleteSessionBatch" => sessions.delete_session_batch(ID, 480).await,
        "CreateLobby" => {
            client
                .lobby_matchmaking()
                .create_lobby(&lobby(
                    &[ID.into(), 42.into()],
                    &[
                        LobbyMetadata {
                            key_name: "motd",
                            key_value: PRIVATE,
                        },
                        LobbyMetadata {
                            key_name: "empty",
                            key_value: "",
                        },
                    ],
                ))
                .await
        }
        "RemoveUserFromLobby" => {
            client
                .lobby_matchmaking()
                .remove_user_from_lobby(480, ID.into(), ID.into())
                .await
        }
        "GetLobbyData" => {
            client
                .lobby_matchmaking()
                .get_lobby_data(480, ID.into())
                .await
        }
        _ => panic!("unknown session test method"),
    }
}

fn input(request: &Request) -> Value {
    let pairs = if request.method.as_str() == "POST" {
        assert!(request.url.query().is_none());
        assert_eq!(
            request.headers.get("content-type").unwrap(),
            "application/x-www-form-urlencoded"
        );
        url::form_urlencoded::parse(&request.body)
            .into_owned()
            .collect::<Vec<_>>()
    } else {
        assert!(request.body.is_empty());
        request.url.query_pairs().into_owned().collect::<Vec<_>>()
    };
    assert_eq!(pairs.len(), 2);
    assert!(
        pairs
            .iter()
            .any(|(key, value)| key == "format" && value == "json")
    );
    let value = &pairs.iter().find(|(key, _)| key == "input_json").unwrap().1;
    serde_json::from_str(value).unwrap()
}

#[tokio::test]
async fn all_ten_methods_use_documented_paths_key_kinds_service_json_and_exact_fields() {
    let server = MockServer::start().await;
    let publisher = client(&server, true);
    let user = client(&server, false);
    for (interface, name, verb, needs_publisher) in METHODS {
        Mock::given(method(verb))
            .and(path(format!("/{interface}/{name}/v1/")))
            .and(header("x-webapi-key", "sessions-key"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"response":{"private":PRIVATE,"extension":[ID, true]}})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let response = call(if needs_publisher { &publisher } else { &user }, name)
            .await
            .unwrap();
        assert_eq!(response.as_json()["response"]["private"], PRIVATE);
        assert_eq!(
            response.as_json()["response"]["extension"][0].as_u64(),
            Some(ID)
        );
        assert!(!format!("{response:?}").contains(PRIVATE));
    }
    for request in server.received_requests().await.unwrap() {
        let value = input(&request);
        let name = request.url.path().split('/').nth(2).unwrap();
        let expected = match name {
            "CreateSession" => {
                json!({"appid":480,"context":ID,"title":title(),"users":users(),"steamid":ID.to_string()})
            }
            "UpdateSession" => {
                json!({"sessionid":ID,"appid":480,"title":title(),"users":users(),"steamid":ID.to_string()})
            }
            "EnumerateSessionsForApp" => {
                json!({"steamid":ID.to_string(),"appid":480,"include_all_user_messages":false,"include_auth_user_message":true,"language":PRIVATE})
            }
            "GetSessionDetailsForApp" => {
                json!({"appid":480,"language":PRIVATE,"sessions":[{"sessionid":ID,"include_all_user_messages":false},{"sessionid":42}]})
            }
            "RequestNotifications" => json!({"steamid":ID.to_string(),"appid":480}),
            "DeleteSession" => json!({"sessionid":ID,"appid":480,"steamid":ID.to_string()}),
            "DeleteSessionBatch" => json!({"sessionid":ID,"appid":480}),
            "CreateLobby" => {
                json!({"appid":480,"max_members":i32::MAX,"lobby_type":4,"lobby_name":PRIVATE,"steamid_invited_members":[ID.to_string(),"42"],"lobby_metadata":[{"key_name":"motd","key_value":PRIVATE},{"key_name":"empty","key_value":""}]})
            }
            "RemoveUserFromLobby" => {
                json!({"appid":480,"steamid_to_remove":ID.to_string(),"steamid_lobby":ID.to_string()})
            }
            "GetLobbyData" => json!({"appid":480,"steamid_lobby":ID.to_string()}),
            _ => panic!("unknown session request"),
        };
        assert_eq!(value, expected, "{name}");
    }
}

#[tokio::test]
async fn all_seven_mutations_including_notification_dispatch_are_sent_once() {
    let server = MockServer::start().await;
    let client = client(&server, true);
    for (interface, name, verb, _) in METHODS
        .into_iter()
        .filter(|(_, _, verb, _)| *verb == "POST")
    {
        Mock::given(method(verb))
            .and(path(format!("/{interface}/{name}/v1/")))
            .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "0"))
            .expect(1)
            .mount(&server)
            .await;
        assert!(matches!(
            call(&client, name).await,
            Err(Error::Http { status: 429, .. })
        ));
    }
    assert_eq!(client.metrics().attempts, 7);
    assert_eq!(client.metrics().retries, 0);
}

struct TemporaryFailure(AtomicUsize);
impl Respond for TemporaryFailure {
    fn respond(&self, _: &Request) -> ResponseTemplate {
        if self.0.fetch_add(1, Ordering::Relaxed) == 0 {
            ResponseTemplate::new(503)
        } else {
            ResponseTemplate::new(200).set_body_json(json!({"response":{}}))
        }
    }
}

#[tokio::test]
async fn three_observational_reads_retry_temporary_failures() {
    let server = MockServer::start().await;
    let client = client(&server, true);
    for (interface, name, verb, _) in METHODS.into_iter().filter(|(_, _, verb, _)| *verb == "GET") {
        Mock::given(method(verb))
            .and(path(format!("/{interface}/{name}/v1/")))
            .respond_with(TemporaryFailure(AtomicUsize::new(0)))
            .expect(2)
            .mount(&server)
            .await;
        call(&client, name).await.unwrap();
    }
    assert_eq!(client.metrics().attempts, 6);
    assert_eq!(client.metrics().retries, 3);
}

#[tokio::test]
async fn anonymous_cannot_call_any_method_and_user_keys_only_authorize_lobby_creation() {
    let server = MockServer::start().await;
    let anonymous = SteamClient::builder()
        .base_url(server.uri())
        .build()
        .unwrap();
    let user = client(&server, false);
    for (_, name, _, publisher) in METHODS {
        assert!(matches!(
            call(&anonymous, name).await,
            Err(Error::MissingApiKey(_))
        ));
        if publisher {
            assert!(matches!(
                call(&user, name).await,
                Err(Error::MissingApiKey("publisher"))
            ));
        }
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn every_method_rejects_nested_errors_and_redacts_server_text_without_replay() {
    let server = MockServer::start().await;
    let client = client(&server, true);
    for (interface, name, verb, _) in METHODS {
        Mock::given(method(verb))
            .and(path(format!("/{interface}/{name}/v1/")))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(
                    json!({"response":{"params":{"success":false,"private":PRIVATE}}}),
                ),
            )
            .expect(1)
            .mount(&server)
            .await;
        let error = call(&client, name).await.unwrap_err();
        assert!(matches!(error, Error::Api { .. }));
        assert!(!format!("{error:?} {error}").contains(PRIVATE));
    }
    assert_eq!(client.metrics().retries, 0);
}

#[tokio::test]
async fn every_method_rejects_malformed_responses_without_exposing_private_contents() {
    let server = MockServer::start().await;
    let client = client(&server, true);
    for (interface, name, verb, _) in METHODS {
        Mock::given(method(verb))
            .and(path(format!("/{interface}/{name}/v1/")))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(format!("{{\"response\":\"{PRIVATE}")),
            )
            .expect(1)
            .mount(&server)
            .await;
        let error = call(&client, name).await.unwrap_err();
        assert!(matches!(error, Error::Decode { .. }));
        assert!(!format!("{error:?} {error}").contains(PRIVATE));
    }
    assert_eq!(client.metrics().retries, 0);
}

#[tokio::test]
async fn optional_omissions_explicit_empty_members_and_context_zero_survive_unchanged() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response":{}})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response":{}})))
        .expect(7)
        .mount(&server)
        .await;
    let client = client(&server, true);
    let sessions = client.game_notifications();
    sessions
        .create_session(&CreateSessionRequest {
            appid: 480,
            context: 0,
            title: &json!({}),
            users: &[],
            steamid: None,
        })
        .await
        .unwrap();
    sessions
        .update_session(&UpdateSessionRequest {
            sessionid: ID,
            appid: 480,
            title: None,
            users: None,
            steamid: None,
        })
        .await
        .unwrap();
    sessions
        .update_session(&UpdateSessionRequest {
            sessionid: ID,
            appid: 480,
            title: Some(&json!({})),
            users: Some(&[]),
            steamid: None,
        })
        .await
        .unwrap();
    sessions
        .enumerate_sessions_for_app(&EnumerateSessionsRequest {
            steamid: ID.into(),
            appid: None,
            include_all_user_messages: None,
            include_auth_user_message: None,
            language: None,
        })
        .await
        .unwrap();
    sessions
        .delete_session(&DeleteSessionRequest {
            sessionid: ID,
            appid: 480,
            steamid: None,
        })
        .await
        .unwrap();
    client
        .lobby_matchmaking()
        .create_lobby(&CreateLobbyRequest {
            appid: 480,
            max_members: 1,
            lobby_type: 0,
            lobby_name: None,
            steamid_invited_members: &[],
            lobby_metadata: None,
        })
        .await
        .unwrap();
    client
        .lobby_matchmaking()
        .create_lobby(&CreateLobbyRequest {
            appid: 480,
            max_members: 1,
            lobby_type: 4,
            lobby_name: Some("unique"),
            steamid_invited_members: &[],
            lobby_metadata: Some(&[]),
        })
        .await
        .unwrap();
    sessions.delete_session_batch(ID, 480).await.unwrap();
    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        input(&requests[0]),
        json!({"appid":480,"context":0,"title":{},"users":[]})
    );
    assert_eq!(input(&requests[1]), json!({"sessionid":ID,"appid":480}));
    assert_eq!(
        input(&requests[2]),
        json!({"sessionid":ID,"appid":480,"title":{},"users":[]})
    );
    assert_eq!(input(&requests[3]), json!({"steamid":ID.to_string()}));
    assert_eq!(input(&requests[4]), json!({"sessionid":ID,"appid":480}));
    assert_eq!(
        input(&requests[5]),
        json!({"appid":480,"max_members":1,"lobby_type":0,"steamid_invited_members":[]})
    );
    assert_eq!(input(&requests[6])["lobby_metadata"], json!([]));
    assert_eq!(input(&requests[7]), json!({"sessionid":ID,"appid":480}));
    assert!(
        !requests
            .iter()
            .any(|r| r.url.path().contains("RequestNotifications"))
    );
}

#[test]
fn private_message_user_and_lobby_request_debug_is_redacted() {
    let title = title();
    let users = users();
    let create = CreateSessionRequest {
        appid: 480,
        context: 0,
        title: &title,
        users: &users,
        steamid: None,
    };
    let update = UpdateSessionRequest {
        appid: 480,
        sessionid: ID,
        title: Some(&title),
        users: Some(&users),
        steamid: None,
    };
    let metadata = [LobbyMetadata {
        key_name: "private",
        key_value: PRIVATE,
    }];
    let lobby = lobby(&[], &metadata);
    for output in [
        format!("{create:?}"),
        format!("{update:?}"),
        format!("{lobby:?}"),
        format!("{:?}", metadata[0]),
    ] {
        assert!(!output.contains(PRIVATE));
    }
}

#[tokio::test]
async fn invalid_ids_arrays_localization_shapes_counts_and_names_fail_before_network() {
    let server = MockServer::start().await;
    let client = client(&server, true);
    let sessions = client.game_notifications();
    let title = title();
    let users = users();
    let mut create = CreateSessionRequest {
        appid: 480,
        context: 0,
        title: &title,
        users: &users,
        steamid: None,
    };
    create.appid = 0;
    assert!(sessions.create_session(&create).await.is_err());
    create.appid = 480;
    let scalar = json!(PRIVATE);
    create.title = &scalar;
    assert!(sessions.create_session(&create).await.is_err());
    create.title = &title;
    let invalid_users = [json!({"steamid":"18446744073709551616"})];
    create.users = &invalid_users;
    assert!(sessions.create_session(&create).await.is_err());
    let zero_user = [json!({"steamid":0})];
    create.users = &zero_user;
    assert!(sessions.create_session(&create).await.is_err());
    let nonobject = [json!(PRIVATE)];
    create.users = &nonobject;
    assert!(sessions.create_session(&create).await.is_err());
    create.users = &users;
    create.steamid = Some(0.into());
    assert!(sessions.create_session(&create).await.is_err());
    assert!(
        sessions
            .update_session(&UpdateSessionRequest {
                appid: 480,
                sessionid: 0,
                title: None,
                users: None,
                steamid: None
            })
            .await
            .is_err()
    );
    assert!(
        sessions
            .get_session_details_for_app(&SessionDetailsRequest {
                appid: 480,
                language: "english",
                sessions: &[]
            })
            .await
            .is_err()
    );
    assert!(
        sessions
            .get_session_details_for_app(&SessionDetailsRequest {
                appid: 480,
                language: "english",
                sessions: &[SessionDetailsQuery {
                    sessionid: 0,
                    include_all_user_messages: None
                }]
            })
            .await
            .is_err()
    );
    let mut enumeration = enumeration();
    enumeration.appid = Some(0);
    assert!(
        sessions
            .enumerate_sessions_for_app(&enumeration)
            .await
            .is_err()
    );
    enumeration.appid = Some(480);
    enumeration.language = Some("\0");
    assert!(
        sessions
            .enumerate_sessions_for_app(&enumeration)
            .await
            .is_err()
    );
    assert!(sessions.request_notifications(0.into(), 480).await.is_err());
    assert!(
        sessions
            .delete_session(&DeleteSessionRequest {
                sessionid: ID,
                appid: 0,
                steamid: None
            })
            .await
            .is_err()
    );
    assert!(sessions.delete_session_batch(0, 480).await.is_err());
    let lobbies = client.lobby_matchmaking();
    let mut request = lobby(&[], &[]);
    request.max_members = 0;
    assert!(lobbies.create_lobby(&request).await.is_err());
    request.max_members = 1;
    request.lobby_name = None;
    assert!(lobbies.create_lobby(&request).await.is_err());
    request.lobby_name = Some(" ");
    assert!(lobbies.create_lobby(&request).await.is_err());
    request.lobby_name = Some("unique");
    request.lobby_type = -1;
    assert!(lobbies.create_lobby(&request).await.is_err());
    request.lobby_type = 4;
    let zero_members = [SteamId(0)];
    request.steamid_invited_members = &zero_members;
    assert!(lobbies.create_lobby(&request).await.is_err());
    request.steamid_invited_members = &[];
    let invalid_metadata = [LobbyMetadata {
        key_name: "name",
        key_value: "private\0",
    }];
    request.lobby_metadata = Some(&invalid_metadata);
    assert!(lobbies.create_lobby(&request).await.is_err());
    assert!(
        lobbies
            .remove_user_from_lobby(480, 0.into(), ID.into())
            .await
            .is_err()
    );
    assert!(lobbies.get_lobby_data(480, 0.into()).await.is_err());
    assert!(server.received_requests().await.unwrap().is_empty());
}
