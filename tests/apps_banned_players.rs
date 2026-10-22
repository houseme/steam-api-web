// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Filter and cursor contracts for the complete GetPlayersBanned request.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;
use steam_api_web::{ApiKey, Error, RetryPolicy, SteamClient, apps::BannedPlayersOptions};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path, query_param},
};

const PATH: &str = "/ISteamApps/GetPlayersBanned/v1/";
const CURSOR: &str = "next/+?&=cursor%2F";

fn publisher(server: &MockServer, retries: u32) -> SteamClient {
    SteamClient::builder()
        .api_key(ApiKey::publisher("test-key").unwrap())
        .base_url(server.uri())
        .retry_policy(RetryPolicy {
            max_retries: retries,
            initial_backoff: Duration::ZERO,
            max_backoff: Duration::ZERO,
        })
        .build()
        .unwrap()
}

#[tokio::test]
async fn time_filters_and_continuation_preserve_wire_values() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(PATH))
        .and(header("x-webapi-key", "test-key"))
        .and(query_param("appid", "480"))
        .and(query_param("starttime", "0"))
        .and(query_param("endtime", "4294967295"))
        .and(query_param("continue_token", CURSOR))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"response":{"future_field":7}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let response = publisher(&server, 0)
        .apps()
        .get_players_banned_with_options(
            480,
            BannedPlayersOptions {
                starttime: Some(0),
                endtime: Some(u32::MAX),
                continue_token: Some(CURSOR),
            },
        )
        .await
        .unwrap();
    assert_eq!(response.as_json()["response"]["future_field"], 7);
    let requests = server.received_requests().await.unwrap();
    assert!(requests[0].body.is_empty());
    assert!(!requests[0].url.query_pairs().any(|(key, _)| key == "key"));
}

#[tokio::test]
async fn original_method_and_default_options_omit_new_parameters() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .expect(2)
        .mount(&server)
        .await;
    let client = publisher(&server, 0);
    client.apps().get_players_banned(480).await.unwrap();
    client
        .apps()
        .get_players_banned_with_options(480, BannedPlayersOptions::default())
        .await
        .unwrap();
    for request in server.received_requests().await.unwrap() {
        assert!(
            !request.url.query_pairs().any(
                |(name, _)| ["starttime", "endtime", "continue_token"].contains(&name.as_ref())
            )
        );
    }
}

#[tokio::test]
async fn independent_time_boundaries_and_explicit_zero_are_preserved() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .expect(2)
        .mount(&server)
        .await;
    let client = publisher(&server, 0);
    for options in [
        BannedPlayersOptions {
            starttime: Some(0),
            ..Default::default()
        },
        BannedPlayersOptions {
            endtime: Some(0),
            ..Default::default()
        },
    ] {
        client
            .apps()
            .get_players_banned_with_options(480, options)
            .await
            .unwrap();
    }
    let requests = server.received_requests().await.unwrap();
    for (request, present, absent) in [
        (&requests[0], "starttime", "endtime"),
        (&requests[1], "endtime", "starttime"),
    ] {
        assert!(
            request
                .url
                .query_pairs()
                .any(|(key, value)| key == present && value == "0")
        );
        assert!(
            !request
                .url
                .query_pairs()
                .any(|(key, _)| key == absent || key == "continue_token")
        );
    }
}

#[tokio::test]
async fn invalid_filters_are_rejected_before_any_network_request() {
    let server = MockServer::start().await;
    let client = publisher(&server, 0);
    for options in [
        BannedPlayersOptions {
            starttime: Some(2),
            endtime: Some(1),
            continue_token: None,
        },
        BannedPlayersOptions {
            starttime: None,
            endtime: Some(1),
            continue_token: Some(CURSOR),
        },
        BannedPlayersOptions {
            starttime: Some(0),
            endtime: None,
            continue_token: Some(CURSOR),
        },
        BannedPlayersOptions {
            starttime: None,
            endtime: None,
            continue_token: Some(CURSOR),
        },
        BannedPlayersOptions {
            starttime: Some(0),
            endtime: Some(1),
            continue_token: Some(""),
        },
        BannedPlayersOptions {
            starttime: Some(0),
            endtime: Some(1),
            continue_token: Some(" "),
        },
        BannedPlayersOptions {
            starttime: Some(0),
            endtime: Some(1),
            continue_token: Some("x\0y"),
        },
    ] {
        assert!(matches!(
            client
                .apps()
                .get_players_banned_with_options(480, options)
                .await,
            Err(Error::InvalidParameter(_))
        ));
    }
    assert!(matches!(
        client
            .apps()
            .get_players_banned_with_options(0, BannedPlayersOptions::default())
            .await,
        Err(Error::InvalidParameter(_))
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn read_retries_keep_the_same_interval_and_cursor() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(PATH))
        .and(query_param("starttime", "0"))
        .and(query_param("endtime", "1"))
        .and(query_param("continue_token", CURSOR))
        .respond_with(ResponseTemplate::new(503))
        .expect(3)
        .mount(&server)
        .await;
    let result = publisher(&server, 2)
        .apps()
        .get_players_banned_with_options(
            480,
            BannedPlayersOptions {
                starttime: Some(0),
                endtime: Some(1),
                continue_token: Some(CURSOR),
            },
        )
        .await;
    assert!(matches!(result, Err(Error::Http { status: 503, .. })));
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 3);
    assert!(
        requests
            .iter()
            .all(|request| request.url == requests[0].url)
    );
}

#[tokio::test]
async fn filtered_bans_require_publisher_credentials() {
    let server = MockServer::start().await;
    let client = SteamClient::builder()
        .base_url(server.uri())
        .api_key(ApiKey::user("test-key").unwrap())
        .build()
        .unwrap();
    let result = client
        .apps()
        .get_players_banned_with_options(
            480,
            BannedPlayersOptions {
                starttime: Some(0),
                endtime: Some(1),
                continue_token: None,
            },
        )
        .await;
    assert!(matches!(result, Err(Error::MissingApiKey("publisher"))));
    assert!(server.received_requests().await.unwrap().is_empty());
}
