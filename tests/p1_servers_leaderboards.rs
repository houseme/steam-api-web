// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Wire contracts for game server accounts and publisher leaderboards.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::{Value, json};
use std::{collections::BTreeMap, time::Duration};
use steam_api_web::{
    ApiKey, Error, RetryPolicy, SteamClient, SteamId,
    game_servers::GameServerLoginToken,
    leaderboards::{
        FindLeaderboardOptions, LeaderboardDataRequest, LeaderboardEntriesRequest,
        LeaderboardScoreMethod, LeaderboardScoreUpdate,
    },
    user_auth::OpaqueResponse,
};
use wiremock::{
    Mock, MockServer, Request, ResponseTemplate,
    matchers::{header, method, path},
};

const ID: u64 = 76561198000000000;
const TOKEN: &str = "private-token&=+中文";
const GAME_METHODS: [(&str, &str); 9] = [
    ("GetAccountList", "GET"),
    ("CreateAccount", "POST"),
    ("SetMemo", "POST"),
    ("ResetLoginToken", "POST"),
    ("DeleteAccount", "POST"),
    ("GetAccountPublicInfo", "GET"),
    ("QueryLoginToken", "GET"),
    ("GetServerSteamIDsByIP", "GET"),
    ("GetServerIPsBySteamID", "GET"),
];
const LEADERBOARD_METHODS: [(&str, &str, u8); 7] = [
    ("DeleteLeaderboard", "POST", 1),
    ("DeleteLeaderboardScore", "POST", 1),
    ("FindOrCreateLeaderboard", "POST", 2),
    ("GetLeaderboardEntries", "GET", 1),
    ("GetLeaderboardsForGame", "GET", 2),
    ("ResetLeaderboard", "POST", 1),
    ("SetLeaderboardScore", "POST", 1),
];

fn client(server: &MockServer, publisher: bool) -> SteamClient {
    let key = if publisher {
        ApiKey::publisher("wire-key").unwrap()
    } else {
        ApiKey::user("wire-key").unwrap()
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

fn form(request: &Request) -> BTreeMap<String, String> {
    if request.method.as_str() == "POST" {
        url::form_urlencoded::parse(&request.body)
            .into_owned()
            .collect()
    } else {
        request.url.query_pairs().into_owned().collect()
    }
}

fn entries(mode: LeaderboardDataRequest) -> LeaderboardEntriesRequest {
    LeaderboardEntriesRequest {
        appid: 480,
        leaderboardid: 42,
        rangestart: -10,
        rangeend: 10,
        datarequest: mode,
        steamid: Some(SteamId(ID)),
    }
}

fn score() -> LeaderboardScoreUpdate<'static> {
    LeaderboardScoreUpdate {
        appid: 480,
        leaderboardid: 42,
        steamid: SteamId(ID),
        score: -123,
        scoremethod: LeaderboardScoreMethod::ForceUpdate,
        details: Some(&[0, 255, b'+', b'&', b'=']),
    }
}

async fn game_call(client: &SteamClient, name: &str) -> steam_api_web::Result<OpaqueResponse> {
    let servers = client.game_servers();
    match name {
        "GetAccountList" => servers.get_account_list().await,
        "CreateAccount" => servers.create_account(480, "memo & 中文").await,
        "SetMemo" => servers.set_memo(ID.into(), "").await,
        "ResetLoginToken" => servers.reset_login_token(ID.into()).await,
        "DeleteAccount" => servers.delete_account(ID.into()).await,
        "GetAccountPublicInfo" => servers.get_account_public_info(ID.into()).await,
        "QueryLoginToken" => {
            let token = GameServerLoginToken::new(TOKEN).unwrap();
            servers.query_login_token(&token).await
        }
        "GetServerSteamIDsByIP" => {
            servers
                .get_server_steam_ids_by_ip(&["192.0.2.1:27015", "2001:db8::1"])
                .await
        }
        "GetServerIPsBySteamID" => {
            servers
                .get_server_ips_by_steam_id(&[ID.into(), u64::MAX.into()])
                .await
        }
        _ => panic!("unknown test method"),
    }
}

async fn leaderboard_call(
    client: &SteamClient,
    name: &str,
) -> steam_api_web::Result<OpaqueResponse> {
    let leaderboards = client.leaderboards();
    match name {
        "DeleteLeaderboard" => leaderboards.delete_leaderboard(480, "score & 中文").await,
        "DeleteLeaderboardScore" => {
            leaderboards
                .delete_leaderboard_score(480, 42, ID.into())
                .await
        }
        "FindOrCreateLeaderboard" => {
            leaderboards
                .find_or_create_leaderboard(
                    480,
                    "score & 中文",
                    FindLeaderboardOptions {
                        sortmethod: Some("Descending"),
                        displaytype: Some("Numeric"),
                        createifnotfound: Some(false),
                        onlytrustedwrites: Some(true),
                        onlyfriendsreads: Some(false),
                    },
                )
                .await
        }
        "GetLeaderboardEntries" => {
            leaderboards
                .get_leaderboard_entries(entries(LeaderboardDataRequest::AroundUser))
                .await
        }
        "GetLeaderboardsForGame" => leaderboards.get_leaderboards_for_game(480).await,
        "ResetLeaderboard" => leaderboards.reset_leaderboard(480, 42).await,
        "SetLeaderboardScore" => leaderboards.set_leaderboard_score(score()).await,
        _ => panic!("unknown test method"),
    }
}

#[tokio::test]
async fn all_game_server_methods_use_authenticated_service_json_and_preserve_opaque_results() {
    let server = MockServer::start().await;
    let client = client(&server, false);
    for (name, verb) in GAME_METHODS {
        let endpoint = format!("/IGameServersService/{name}/v1/");
        Mock::given(method(verb))
            .and(path(endpoint))
            .and(header("x-webapi-key", "wire-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "response": {"logon_token": TOKEN, "extension": [1, 2, 3]}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let result = game_call(&client, name).await.unwrap();
        assert_eq!(result.as_json()["response"]["logon_token"], TOKEN);
        assert_eq!(result.as_json()["response"]["extension"], json!([1, 2, 3]));
        assert!(!format!("{result:?}").contains(TOKEN));
    }
    let requests = server.received_requests().await.unwrap();
    for request in requests {
        let params = form(&request);
        assert_eq!(params.get("format").unwrap(), "json");
        assert_eq!(params.len(), 2);
        assert!(!params.contains_key("key"));
        let payload: Value = serde_json::from_str(params.get("input_json").unwrap()).unwrap();
        let method_name = request.url.path().split('/').nth(2).unwrap();
        let expected = match method_name {
            "GetAccountList" => json!({}),
            "CreateAccount" => json!({"appid": 480, "memo": "memo & 中文"}),
            "SetMemo" => json!({"steamid": ID.to_string(), "memo": ""}),
            "ResetLoginToken" | "DeleteAccount" | "GetAccountPublicInfo" => {
                json!({"steamid": ID.to_string()})
            }
            "QueryLoginToken" => json!({"login_token": TOKEN}),
            "GetServerSteamIDsByIP" => {
                json!({"server_ips": ["192.0.2.1:27015", "2001:db8::1"]})
            }
            "GetServerIPsBySteamID" => {
                json!({"server_steamids": [ID.to_string(), u64::MAX.to_string()]})
            }
            _ => panic!("unknown request"),
        };
        assert_eq!(payload, expected);
        if request.method.as_str() == "POST" {
            assert!(request.url.query().is_none());
            assert_eq!(
                request.headers.get("content-type").unwrap(),
                "application/x-www-form-urlencoded"
            );
        }
    }
}

#[tokio::test]
async fn all_leaderboards_use_publisher_contracts_latest_versions_and_binary_forms() {
    let server = MockServer::start().await;
    let client = client(&server, true);
    for (name, verb, version) in LEADERBOARD_METHODS {
        Mock::given(method(verb))
            .and(path(format!("/ISteamLeaderboards/{name}/v{version}/")))
            .and(header("x-webapi-key", "wire-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "response": {"extension": {"id": "18446744073709551615"}}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let result = leaderboard_call(&client, name).await.unwrap();
        assert_eq!(
            result.as_json()["response"]["extension"]["id"],
            u64::MAX.to_string()
        );
    }
    for request in server.received_requests().await.unwrap() {
        let params = form(&request);
        assert_eq!(params.get("appid").unwrap(), "480");
        assert_eq!(params.get("format").unwrap(), "json");
        assert!(!params.contains_key("key"));
        assert!(!params.contains_key("input_json"));
        match request.url.path().split('/').nth(2).unwrap() {
            "DeleteLeaderboard" => assert_eq!(params.get("name").unwrap(), "score & 中文"),
            "DeleteLeaderboardScore" => {
                assert_eq!(params.get("leaderboardid").unwrap(), "42");
                assert_eq!(params.get("steamid").unwrap(), &ID.to_string());
            }
            "FindOrCreateLeaderboard" => {
                assert_eq!(params.get("name").unwrap(), "score & 中文");
                assert_eq!(params.get("sortmethod").unwrap(), "Descending");
                assert_eq!(params.get("displaytype").unwrap(), "Numeric");
                assert_eq!(params.get("createifnotfound").unwrap(), "false");
                assert_eq!(params.get("onlytrustedwrites").unwrap(), "true");
                assert_eq!(params.get("onlyfriendsreads").unwrap(), "false");
            }
            "GetLeaderboardEntries" => {
                assert_eq!(params.get("leaderboardid").unwrap(), "42");
                assert_eq!(params.get("datarequest").unwrap(), "RequestAroundUser");
                assert_eq!(params.get("rangestart").unwrap(), "-10");
                assert_eq!(params.get("rangeend").unwrap(), "10");
                assert_eq!(params.get("steamid").unwrap(), &ID.to_string());
            }
            "GetLeaderboardsForGame" => assert_eq!(params.len(), 2),
            "ResetLeaderboard" => assert_eq!(params.get("leaderboardid").unwrap(), "42"),
            "SetLeaderboardScore" => {
                assert_eq!(params.get("steamid").unwrap(), &ID.to_string());
                assert_eq!(params.get("score").unwrap(), "-123");
                assert_eq!(params.get("scoremethod").unwrap(), "ForceUpdate");
                let body = std::str::from_utf8(&request.body).unwrap();
                assert!(
                    body.split('&')
                        .any(|field| field == "details=%00%FF%2B%26%3D")
                );
            }
            _ => panic!("unknown leaderboard request"),
        }
    }
}

#[tokio::test]
async fn every_mutation_is_sent_once_even_when_server_requests_retry() {
    let server = MockServer::start().await;
    let client = client(&server, true);
    for (name, verb) in GAME_METHODS.into_iter().filter(|(_, verb)| *verb == "POST") {
        Mock::given(method(verb))
            .and(path(format!("/IGameServersService/{name}/v1/")))
            .respond_with(ResponseTemplate::new(503).insert_header("retry-after", "0"))
            .expect(1)
            .mount(&server)
            .await;
        assert!(matches!(
            game_call(&client, name).await,
            Err(Error::Http { status: 503, .. })
        ));
    }
    for (name, verb, version) in LEADERBOARD_METHODS
        .into_iter()
        .filter(|(_, verb, _)| *verb == "POST")
    {
        Mock::given(method(verb))
            .and(path(format!("/ISteamLeaderboards/{name}/v{version}/")))
            .respond_with(ResponseTemplate::new(503).insert_header("retry-after", "0"))
            .expect(1)
            .mount(&server)
            .await;
        assert!(matches!(
            leaderboard_call(&client, name).await,
            Err(Error::Http { status: 503, .. })
        ));
    }
    assert_eq!(client.metrics().attempts, 9);
    assert_eq!(client.metrics().retries, 0);
}

#[tokio::test]
async fn keys_of_insufficient_privilege_are_rejected_without_network_calls() {
    let server = MockServer::start().await;
    let anonymous = SteamClient::builder()
        .base_url(server.uri())
        .build()
        .unwrap();
    for (name, _) in GAME_METHODS {
        assert!(matches!(
            game_call(&anonymous, name).await,
            Err(Error::MissingApiKey(_))
        ));
    }
    let user = client(&server, false);
    for (name, _, _) in LEADERBOARD_METHODS {
        assert!(matches!(
            leaderboard_call(&user, name).await,
            Err(Error::MissingApiKey("publisher"))
        ));
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn invalid_bounds_are_rejected_before_any_request() {
    let server = MockServer::start().await;
    let client = client(&server, true);
    let servers = client.game_servers();
    assert!(GameServerLoginToken::new("").is_err());
    assert!(GameServerLoginToken::new("token\0value").is_err());
    assert!(servers.create_account(0, "memo").await.is_err());
    assert!(servers.create_account(480, "memo\0value").await.is_err());
    assert!(servers.set_memo(SteamId(0), "").await.is_err());
    assert!(servers.reset_login_token(SteamId(0)).await.is_err());
    assert!(servers.delete_account(SteamId(0)).await.is_err());
    assert!(servers.get_account_public_info(SteamId(0)).await.is_err());
    assert!(servers.get_server_steam_ids_by_ip(&[]).await.is_err());
    assert!(
        servers
            .get_server_steam_ids_by_ip(&["example.com:27015"])
            .await
            .is_err()
    );
    assert!(servers.get_server_ips_by_steam_id(&[]).await.is_err());
    assert!(
        servers
            .get_server_ips_by_steam_id(&[ID.into(), SteamId(0)])
            .await
            .is_err()
    );
    let leaderboards = client.leaderboards();
    assert!(leaderboards.delete_leaderboard(480, " ").await.is_err());
    assert!(
        leaderboards
            .delete_leaderboard_score(480, 0, ID.into())
            .await
            .is_err()
    );
    assert!(leaderboards.reset_leaderboard(0, 42).await.is_err());
    assert!(leaderboards.get_leaderboards_for_game(0).await.is_err());
    let mut request = entries(LeaderboardDataRequest::AroundUser);
    request.steamid = None;
    assert!(leaderboards.get_leaderboard_entries(request).await.is_err());
    request = entries(LeaderboardDataRequest::Friends);
    request.steamid = Some(SteamId(0));
    assert!(leaderboards.get_leaderboard_entries(request).await.is_err());
    request = entries(LeaderboardDataRequest::AroundUser);
    request.rangestart = 11;
    assert!(leaderboards.get_leaderboard_entries(request).await.is_err());
    request = entries(LeaderboardDataRequest::Global);
    assert!(leaderboards.get_leaderboard_entries(request).await.is_err());
    request = entries(LeaderboardDataRequest::AroundUser);
    request.leaderboardid = -1;
    assert!(leaderboards.get_leaderboard_entries(request).await.is_err());
    let oversized = [0; 257];
    let mut update = score();
    update.details = Some(&oversized);
    assert!(leaderboards.set_leaderboard_score(update).await.is_err());
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn optional_defaults_are_omitted_and_detail_limit_is_inclusive() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response": {}})))
        .expect(4)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response": {}})))
        .expect(2)
        .mount(&server)
        .await;
    let client = client(&server, true);
    let leaderboards = client.leaderboards();
    leaderboards
        .find_or_create_leaderboard(480, "score", FindLeaderboardOptions::default())
        .await
        .unwrap();
    let full = [255; 256];
    let mut update = score();
    update.details = Some(&full);
    leaderboards.set_leaderboard_score(update).await.unwrap();
    update.details = None;
    update.scoremethod = LeaderboardScoreMethod::KeepBest;
    leaderboards.set_leaderboard_score(update).await.unwrap();
    update.details = Some(&[]);
    leaderboards.set_leaderboard_score(update).await.unwrap();
    let mut request = entries(LeaderboardDataRequest::Global);
    request.rangestart = 0;
    request.steamid = None;
    leaderboards.get_leaderboard_entries(request).await.unwrap();
    request = entries(LeaderboardDataRequest::Friends);
    leaderboards.get_leaderboard_entries(request).await.unwrap();
    let requests = server.received_requests().await.unwrap();
    assert_eq!(form(&requests[0]).len(), 3);
    let encoded = std::str::from_utf8(&requests[1].body).unwrap();
    let details = encoded
        .split('&')
        .find_map(|part| part.strip_prefix("details="))
        .unwrap();
    assert_eq!(details, "%FF".repeat(256));
    assert!(!form(&requests[2]).contains_key("details"));
    assert_eq!(form(&requests[2]).get("scoremethod").unwrap(), "KeepBest");
    assert_eq!(form(&requests[3]).get("details").unwrap(), "");
    assert!(!form(&requests[4]).contains_key("steamid"));
    assert_eq!(
        form(&requests[4]).get("datarequest").unwrap(),
        "RequestGlobal"
    );
    assert_eq!(
        form(&requests[5]).get("datarequest").unwrap(),
        "RequestFriends"
    );
}

#[tokio::test]
async fn opaque_methods_reject_nested_errors_without_leaking_tokens_or_upstream_text() {
    let server = MockServer::start().await;
    let client = client(&server, true);
    let token = GameServerLoginToken::new(TOKEN).unwrap();
    assert!(!format!("{token:?}").contains(TOKEN));
    for (name, verb) in GAME_METHODS {
        Mock::given(method(verb))
            .and(path(format!("/IGameServersService/{name}/v1/")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "response": {"params": {"result": 15, "message": TOKEN}}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let error = game_call(&client, name).await.unwrap_err();
        assert!(matches!(error, Error::Api { code: Some(15), .. }));
        assert!(!format!("{error:?} {error}").contains(TOKEN));
    }
    for (name, verb, version) in LEADERBOARD_METHODS {
        Mock::given(method(verb))
            .and(path(format!("/ISteamLeaderboards/{name}/v{version}/")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "response": {"error": {"errorcode": 2, "description": TOKEN}}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let error = leaderboard_call(&client, name).await.unwrap_err();
        assert!(matches!(error, Error::Api { code: Some(2), .. }));
        assert!(!format!("{error:?} {error}").contains(TOKEN));
    }
}

#[tokio::test]
async fn malformed_token_response_does_not_expose_request_or_response_secrets() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/IGameServersService/QueryLoginToken/v1/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(format!(
            "{{\"response\":{{\"private-token&=+中文\":{TOKEN}}}}}"
        )))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server, true);
    let error = game_call(&client, "QueryLoginToken").await.unwrap_err();
    assert!(matches!(error, Error::Decode { .. }));
    assert!(!format!("{error:?} {error}").contains(TOKEN));
    assert_eq!(client.metrics().attempts, 1);
    assert_eq!(client.metrics().retries, 0);
}
