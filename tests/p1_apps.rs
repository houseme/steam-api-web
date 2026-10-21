// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Wire contracts and administrative safety for apps and game server statistics.
#![allow(clippy::unwrap_used, clippy::expect_used, deprecated)]

use serde_json::json;
use std::{collections::BTreeMap, time::Duration};
use steam_api_web::{
    ApiKey, Error, RetryPolicy, SteamClient,
    apps::{AppType, BuildLiveState, SetAppBuildLiveRequest},
    game_server_stats::GameServerPlayerStatsRequest,
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path, query_param},
};

const ID: u64 = 76561198000000000;
const BUILD_LIVE: &str = "/ISteamApps/SetAppBuildLive/v2/";
const SERVER_STATS: &str = "/ISteamGameServerStats/GetGameServerPlayerStatsForGame/v1/";

fn client(server: &MockServer, key: Option<ApiKey>, retries: u32) -> SteamClient {
    let mut builder = SteamClient::builder()
        .base_url(server.uri())
        .retry_policy(RetryPolicy {
            max_retries: retries,
            initial_backoff: Duration::ZERO,
            max_backoff: Duration::ZERO,
        });
    if let Some(key) = key {
        builder = builder.api_key(key);
    }
    builder.build().unwrap()
}

fn publisher(server: &MockServer) -> SteamClient {
    client(server, Some(ApiKey::publisher("test-key").unwrap()), 0)
}

fn build_request() -> SetAppBuildLiveRequest<'static> {
    SetAppBuildLiveRequest {
        appid: 480,
        buildid: 1234,
        betakey: "public",
        steamid: Some(ID.into()),
        description: Some("Release & confirm=ready + QA"),
    }
}

fn stats_request() -> GameServerPlayerStatsRequest<'static> {
    GameServerPlayerStatsRequest {
        gameid: 480,
        appid: 480,
        rangestart: "2024-02-29 00:00:00",
        rangeend: "2024-02-29 23:59:59",
        maxresults: Some(1000),
    }
}

#[tokio::test]
async fn publisher_app_reads_send_documented_versions_and_options() {
    let server = MockServer::start().await;
    for endpoint in [
        "GetAppBetas",
        "GetAppBuilds",
        "GetAppDepotVersions",
        "GetPlayersBanned",
    ] {
        Mock::given(method("GET"))
            .and(path(format!("/ISteamApps/{endpoint}/v1/")))
            .and(header("x-webapi-key", "test-key"))
            .and(query_param("appid", "480"))
            .and(query_param("format", "json"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"response":{"endpoint":endpoint}})),
            )
            .expect(1)
            .mount(&server)
            .await;
    }
    Mock::given(method("GET"))
        .and(path("/ISteamApps/GetServerList/v1/"))
        .and(header("x-webapi-key", "test-key"))
        .and(query_param("filter", "\\appid\\480\\secure\\1"))
        .and(query_param("limit", "200"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response":{"servers":[]}})))
        .expect(1)
        .mount(&server)
        .await;
    let client = publisher(&server);
    assert_eq!(
        client.apps().get_app_betas(480).await.unwrap().as_json()["response"]["endpoint"],
        "GetAppBetas"
    );
    assert_eq!(
        client
            .apps()
            .get_app_builds(480, Some(17))
            .await
            .unwrap()
            .as_json()["response"]["endpoint"],
        "GetAppBuilds"
    );
    assert_eq!(
        client
            .apps()
            .get_app_depot_versions(480)
            .await
            .unwrap()
            .as_json()["response"]["endpoint"],
        "GetAppDepotVersions"
    );
    assert_eq!(
        client
            .apps()
            .get_players_banned(480)
            .await
            .unwrap()
            .as_json()["response"]["endpoint"],
        "GetPlayersBanned"
    );
    assert_eq!(
        client
            .apps()
            .get_server_list(Some("\\appid\\480\\secure\\1"), Some(200))
            .await
            .unwrap()
            .as_json()["response"]["servers"],
        json!([])
    );
    let requests = server.received_requests().await.unwrap();
    let builds = requests
        .iter()
        .find(|request| request.url.path().contains("GetAppBuilds"))
        .unwrap();
    assert!(
        builds
            .url
            .query_pairs()
            .any(|(key, value)| key == "count" && value == "17")
    );
    assert!(
        requests
            .iter()
            .all(|request| !request.url.query_pairs().any(|(key, _)| key == "key"))
    );
}

#[tokio::test]
async fn public_app_reads_need_no_key_and_preserve_original_json() {
    let server = MockServer::start().await;
    let catalog = json!({"applist":{"apps":[{"appid":480,"name":"Spacewar"}]}});
    Mock::given(method("GET"))
        .and(path("/ISteamApps/GetAppList/v2/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&catalog))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ISteamApps/GetServersAtAddress/v1/"))
        .and(query_param("addr", "[2001:db8::1]:27015"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"response":{"success":true,"servers":[]}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ISteamApps/UpToDateCheck/v1/"))
        .and(query_param("appid", "480"))
        .and(query_param("version", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"response":{"success":true,"up_to_date":false,"required_version":5}}),
        ))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server, None, 0);
    assert_eq!(
        client.apps().get_app_list().await.unwrap().as_json(),
        &catalog
    );
    assert_eq!(
        client
            .apps()
            .get_servers_at_address("[2001:db8::1]:27015")
            .await
            .unwrap()
            .as_json()["response"]["servers"],
        json!([])
    );
    assert_eq!(
        client
            .apps()
            .up_to_date_check(480, 0)
            .await
            .unwrap()
            .as_json()["response"]["required_version"],
        5
    );
    assert!(
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .all(|request| !request.headers.contains_key("x-webapi-key"))
    );
}

#[tokio::test]
async fn partner_app_list_decodes_documented_v2_nesting_and_encodes_type_filter() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ISteamApps/GetPartnerAppListForWebAPIKey/v2/"))
        .and(header("x-webapi-key", "test-key"))
        .and(query_param("type_filter", "game,application,tool,demo,dlc,music"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"applist":{"apps":{"app":[{"appid":480,"app_type":"game","app_name":"Spacewar"},{"appid":481,"app_type":"future_type","app_name":"Future"}]}}})))
        .expect(1)
        .mount(&server)
        .await;
    let apps = publisher(&server)
        .apps()
        .get_partner_app_list_for_web_api_key(&[
            AppType::Game,
            AppType::Application,
            AppType::Tool,
            AppType::Demo,
            AppType::Dlc,
            AppType::Music,
        ])
        .await
        .unwrap();
    assert_eq!(apps.len(), 2);
    assert_eq!(apps[0].appid, 480);
    assert_eq!(apps[0].app_name, "Spacewar");
    assert_eq!(apps[1].app_type, "future_type");
}

#[tokio::test]
async fn optional_publisher_fields_are_absent_when_not_requested() {
    let server = MockServer::start().await;
    for endpoint in [
        "GetAppBuilds/v1",
        "GetServerList/v1",
        "GetPartnerAppListForWebAPIKey/v2",
    ] {
        Mock::given(method("GET"))
            .and(path(format!("/ISteamApps/{endpoint}/")))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"applist":{"apps":{"app":[]}}})),
            )
            .expect(1)
            .mount(&server)
            .await;
    }
    Mock::given(method("GET"))
        .and(path(SERVER_STATS))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response":{}})))
        .expect(1)
        .mount(&server)
        .await;
    let client = publisher(&server);
    client.apps().get_app_builds(480, None).await.unwrap();
    client.apps().get_server_list(None, None).await.unwrap();
    assert!(
        client
            .apps()
            .get_partner_app_list_for_web_api_key(&[])
            .await
            .unwrap()
            .is_empty()
    );
    let request = GameServerPlayerStatsRequest {
        maxresults: None,
        ..stats_request()
    };
    client
        .game_server_stats()
        .get_game_server_player_stats_for_game(&request)
        .await
        .unwrap();
    for request in server.received_requests().await.unwrap() {
        assert!(!request.url.query_pairs().any(|(key, _)| {
            ["count", "filter", "limit", "type_filter", "maxresults"].contains(&key.as_ref())
        }));
    }
}

#[tokio::test]
async fn live_build_uses_v2_post_and_preserves_mobile_confirmation_status() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(BUILD_LIVE))
        .and(header("x-webapi-key", "test-key"))
        .and(header("content-type", "application/x-www-form-urlencoded"))
        .respond_with(
            ResponseTemplate::new(201)
                .set_body_json(json!({"response":{"result":1,"confirmation":"pending"}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let result = publisher(&server)
        .apps()
        .set_app_build_live(&build_request())
        .await
        .unwrap();
    assert_eq!(result.http_status, 201);
    assert_eq!(result.state, BuildLiveState::ConfirmationRequired);
    assert_eq!(
        result.response.unwrap().as_json()["response"]["confirmation"],
        "pending"
    );
    let requests = server.received_requests().await.unwrap();
    assert!(requests[0].url.query().is_none());
    let params: BTreeMap<_, _> = url::form_urlencoded::parse(&requests[0].body)
        .into_owned()
        .collect();
    assert_eq!(params.get("appid").unwrap(), "480");
    assert_eq!(params.get("buildid").unwrap(), "1234");
    assert_eq!(params.get("betakey").unwrap(), "public");
    assert_eq!(params.get("steamid").unwrap(), &ID.to_string());
    assert_eq!(
        params.get("description").unwrap(),
        "Release & confirm=ready + QA"
    );
    assert_eq!(params.get("format").unwrap(), "json");
    assert!(!params.contains_key("key"));
}

#[tokio::test]
async fn live_build_accepts_empty_201_confirmation_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(BUILD_LIVE))
        .respond_with(ResponseTemplate::new(201))
        .expect(1)
        .mount(&server)
        .await;
    let result = publisher(&server)
        .apps()
        .set_app_build_live(&build_request())
        .await
        .unwrap();
    assert_eq!(result.http_status, 201);
    assert_eq!(result.state, BuildLiveState::ConfirmationRequired);
    assert!(result.response.is_none());
}

#[tokio::test]
async fn live_build_rejects_empty_200_instead_of_claiming_acceptance() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(BUILD_LIVE))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    assert!(matches!(
        publisher(&server)
            .apps()
            .set_app_build_live(&build_request())
            .await,
        Err(Error::Decode { .. })
    ));
}

#[tokio::test]
async fn live_build_200_accepts_beta_branch_without_confirmation_account() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(BUILD_LIVE))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response":{"result":1}})))
        .expect(1)
        .mount(&server)
        .await;
    let request = SetAppBuildLiveRequest {
        betakey: "internal",
        steamid: None,
        description: None,
        ..build_request()
    };
    let result = publisher(&server)
        .apps()
        .set_app_build_live(&request)
        .await
        .unwrap();
    assert_eq!(result.http_status, 200);
    assert_eq!(result.state, BuildLiveState::RequestAccepted);
    let requests = server.received_requests().await.unwrap();
    let params: BTreeMap<_, _> = url::form_urlencoded::parse(&requests[0].body)
        .into_owned()
        .collect();
    assert!(!params.contains_key("steamid"));
    assert!(!params.contains_key("description"));
    assert_eq!(params.get("betakey").unwrap(), "internal");
}

#[tokio::test]
async fn live_build_is_never_replayed_on_transient_failure() {
    for status in [429, 503] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(BUILD_LIVE))
            .respond_with(ResponseTemplate::new(status))
            .expect(1)
            .mount(&server)
            .await;
        let client = client(&server, Some(ApiKey::publisher("test-key").unwrap()), 3);
        let result = client.apps().set_app_build_live(&build_request()).await;
        assert!(matches!(result, Err(Error::Http { status: code, .. }) if code == status));
        assert_eq!(client.metrics().attempts, 1);
        assert_eq!(client.metrics().retries, 0);
    }
}

#[tokio::test]
async fn game_server_stats_preserve_64_bit_game_id_and_seattle_local_range() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(SERVER_STATS))
        .and(header("x-webapi-key", "test-key"))
        .and(query_param("gameid", u64::MAX.to_string()))
        .and(query_param("appid", "480"))
        .and(query_param("rangestart", "2024-02-29 00:00:00"))
        .and(query_param("rangeend", "2024-02-29 23:59:59"))
        .and(query_param("maxresults", "1000"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(
                json!({"response":{"future_stats":{"value":"18446744073709551615"}}}),
            ),
        )
        .expect(1)
        .mount(&server)
        .await;
    let request = GameServerPlayerStatsRequest {
        gameid: u64::MAX,
        ..stats_request()
    };
    let result = publisher(&server)
        .game_server_stats()
        .get_game_server_player_stats_for_game(&request)
        .await
        .unwrap();
    assert_eq!(
        result.as_json()["response"]["future_stats"]["value"],
        "18446744073709551615"
    );
}

#[tokio::test]
async fn every_new_method_propagates_nested_steam_business_errors() {
    let cases = [
        ("GET", "/ISteamApps/GetAppBetas/v1/"),
        ("GET", "/ISteamApps/GetAppBuilds/v1/"),
        ("GET", "/ISteamApps/GetAppDepotVersions/v1/"),
        ("GET", "/ISteamApps/GetAppList/v2/"),
        ("GET", "/ISteamApps/GetPartnerAppListForWebAPIKey/v2/"),
        ("GET", "/ISteamApps/GetPlayersBanned/v1/"),
        ("GET", "/ISteamApps/GetServerList/v1/"),
        ("GET", "/ISteamApps/GetServersAtAddress/v1/"),
        ("POST", BUILD_LIVE),
        ("GET", "/ISteamApps/UpToDateCheck/v1/"),
        ("GET", SERVER_STATS),
    ];
    let server = MockServer::start().await;
    for (verb, endpoint) in cases {
        Mock::given(method(verb))
            .and(path(endpoint))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                json!({"response":{"error":{"errorcode":15,"errordesc":"private server error"}}}),
            ))
            .expect(1)
            .mount(&server)
            .await;
    }
    let client = publisher(&server);
    for (index, (_, endpoint)) in cases.iter().enumerate() {
        let result = match index {
            0 => client.apps().get_app_betas(480).await.map(|_| ()),
            1 => client.apps().get_app_builds(480, None).await.map(|_| ()),
            2 => client.apps().get_app_depot_versions(480).await.map(|_| ()),
            3 => client.apps().get_app_list().await.map(|_| ()),
            4 => client
                .apps()
                .get_partner_app_list_for_web_api_key(&[])
                .await
                .map(|_| ()),
            5 => client.apps().get_players_banned(480).await.map(|_| ()),
            6 => client.apps().get_server_list(None, None).await.map(|_| ()),
            7 => client
                .apps()
                .get_servers_at_address("127.0.0.1")
                .await
                .map(|_| ()),
            8 => client
                .apps()
                .set_app_build_live(&build_request())
                .await
                .map(|_| ()),
            9 => client.apps().up_to_date_check(480, 1).await.map(|_| ()),
            _ => client
                .game_server_stats()
                .get_game_server_player_stats_for_game(&stats_request())
                .await
                .map(|_| ()),
        };
        let error = result.unwrap_err();
        assert!(
            matches!(error, Error::Api { code: Some(15), .. }),
            "{}: {error:?}",
            endpoint
        );
        assert!(!error.to_string().contains("private server error"));
    }
}

#[tokio::test]
async fn all_publisher_methods_reject_anonymous_and_user_keys_before_network() {
    let server = MockServer::start().await;
    for key in [None, Some(ApiKey::user("test-user-key").unwrap())] {
        let client = client(&server, key, 0);
        let errors = [
            client
                .apps()
                .get_app_betas(480)
                .await
                .map(|_| ())
                .unwrap_err(),
            client
                .apps()
                .get_app_builds(480, None)
                .await
                .map(|_| ())
                .unwrap_err(),
            client
                .apps()
                .get_app_depot_versions(480)
                .await
                .map(|_| ())
                .unwrap_err(),
            client
                .apps()
                .get_partner_app_list_for_web_api_key(&[])
                .await
                .map(|_| ())
                .unwrap_err(),
            client
                .apps()
                .get_players_banned(480)
                .await
                .map(|_| ())
                .unwrap_err(),
            client
                .apps()
                .get_server_list(None, None)
                .await
                .map(|_| ())
                .unwrap_err(),
            client
                .apps()
                .set_app_build_live(&build_request())
                .await
                .map(|_| ())
                .unwrap_err(),
            client
                .game_server_stats()
                .get_game_server_player_stats_for_game(&stats_request())
                .await
                .map(|_| ())
                .unwrap_err(),
        ];
        for error in errors {
            assert!(matches!(error, Error::MissingApiKey("publisher")));
        }
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn app_arguments_are_validated_before_network() {
    let server = MockServer::start().await;
    let client = publisher(&server);
    let errors = [
        client
            .apps()
            .get_app_betas(0)
            .await
            .map(|_| ())
            .unwrap_err(),
        client
            .apps()
            .get_app_builds(0, None)
            .await
            .map(|_| ())
            .unwrap_err(),
        client
            .apps()
            .get_app_depot_versions(0)
            .await
            .map(|_| ())
            .unwrap_err(),
        client
            .apps()
            .get_players_banned(0)
            .await
            .map(|_| ())
            .unwrap_err(),
        client
            .apps()
            .up_to_date_check(0, 1)
            .await
            .map(|_| ())
            .unwrap_err(),
        client
            .apps()
            .get_app_builds(480, Some(0))
            .await
            .map(|_| ())
            .unwrap_err(),
        client
            .apps()
            .get_server_list(None, Some(0))
            .await
            .map(|_| ())
            .unwrap_err(),
        client
            .apps()
            .get_server_list(Some("bad\0filter"), None)
            .await
            .map(|_| ())
            .unwrap_err(),
        client
            .apps()
            .get_partner_app_list_for_web_api_key(&[AppType::Game, AppType::Game])
            .await
            .map(|_| ())
            .unwrap_err(),
    ];
    for error in errors {
        assert!(matches!(error, Error::InvalidParameter(_)));
    }
    for address in [
        "",
        "localhost",
        "https://example.com",
        "127.0.0.1:0",
        "127.0.0.1:65536",
        "127.0.0.1&appid=1",
        "[::1]:0",
    ] {
        assert!(matches!(
            client.apps().get_servers_at_address(address).await,
            Err(Error::InvalidParameter(_))
        ));
    }
    for request in [
        SetAppBuildLiveRequest {
            appid: 0,
            ..build_request()
        },
        SetAppBuildLiveRequest {
            buildid: 0,
            ..build_request()
        },
        SetAppBuildLiveRequest {
            betakey: " ",
            ..build_request()
        },
        SetAppBuildLiveRequest {
            betakey: "bad\0branch",
            ..build_request()
        },
        SetAppBuildLiveRequest {
            steamid: Some(0.into()),
            ..build_request()
        },
        SetAppBuildLiveRequest {
            description: Some("bad\0description"),
            ..build_request()
        },
    ] {
        assert!(matches!(
            client.apps().set_app_build_live(&request).await,
            Err(Error::InvalidParameter(_))
        ));
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn game_server_ranges_reject_invalid_calendar_values_and_bounds_before_network() {
    let server = MockServer::start().await;
    let client = publisher(&server);
    for value in [
        "",
        "2024-02-29T00:00:00",
        "2024-02-29 00:00:00Z",
        "2023-02-29 00:00:00",
        "1900-02-29 00:00:00",
        "2024-04-31 00:00:00",
        "2024-00-01 00:00:00",
        "2024-13-01 00:00:00",
        "2024-01-00 00:00:00",
        "0000-01-01 00:00:00",
        "2024-02-29 24:00:00",
        "2024-02-29 12:60:00",
        "2024-02-29 12:00:60",
        "2024-02-29 00:00:0x",
        "2024-02-29 00:00:00\0",
        "２０２４-02-29 00:00:00",
    ] {
        let request = GameServerPlayerStatsRequest {
            rangestart: value,
            ..stats_request()
        };
        assert!(
            matches!(
                client
                    .game_server_stats()
                    .get_game_server_player_stats_for_game(&request)
                    .await,
                Err(Error::InvalidParameter(_))
            ),
            "{value}"
        );
    }
    for request in [
        GameServerPlayerStatsRequest {
            gameid: 0,
            ..stats_request()
        },
        GameServerPlayerStatsRequest {
            appid: 0,
            ..stats_request()
        },
        GameServerPlayerStatsRequest {
            rangeend: "2024-02-28 23:59:59",
            ..stats_request()
        },
        GameServerPlayerStatsRequest {
            maxresults: Some(0),
            ..stats_request()
        },
        GameServerPlayerStatsRequest {
            maxresults: Some(1001),
            ..stats_request()
        },
    ] {
        assert!(matches!(
            client
                .game_server_stats()
                .get_game_server_player_stats_for_game(&request)
                .await,
            Err(Error::InvalidParameter(_))
        ));
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn partner_app_list_does_not_replace_a_missing_typed_schema_with_an_empty_list() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ISteamApps/GetPartnerAppListForWebAPIKey/v2/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"applist":{"apps":{}}})))
        .expect(1)
        .mount(&server)
        .await;
    assert!(matches!(
        publisher(&server)
            .apps()
            .get_partner_app_list_for_web_api_key(&[])
            .await,
        Err(Error::Decode { .. })
    ));
}

#[tokio::test]
async fn game_server_read_uses_safe_read_retry_policy() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(SERVER_STATS))
        .respond_with(ResponseTemplate::new(503))
        .expect(3)
        .mount(&server)
        .await;
    let client = client(&server, Some(ApiKey::publisher("test-key").unwrap()), 2);
    assert!(matches!(
        client
            .game_server_stats()
            .get_game_server_player_stats_for_game(&stats_request())
            .await,
        Err(Error::Http { status: 503, .. })
    ));
    assert_eq!(client.metrics().attempts, 3);
    assert_eq!(client.metrics().retries, 2);
}

#[tokio::test]
async fn game_server_stats_accept_century_leap_day_and_an_equal_range() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(SERVER_STATS))
        .and(query_param("rangestart", "2000-02-29 12:00:00"))
        .and(query_param("rangeend", "2000-02-29 12:00:00"))
        .and(query_param("maxresults", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response":{}})))
        .expect(1)
        .mount(&server)
        .await;
    let request = GameServerPlayerStatsRequest {
        rangestart: "2000-02-29 12:00:00",
        rangeend: "2000-02-29 12:00:00",
        maxresults: Some(1),
        ..stats_request()
    };
    publisher(&server)
        .game_server_stats()
        .get_game_server_player_stats_for_game(&request)
        .await
        .unwrap();
}

#[tokio::test]
async fn public_build_without_confirmation_account_preserves_unknown_release_state() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ISteamApps/SetAppBuildLive/v2/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response":{"result":1}})))
        .expect(1)
        .mount(&server)
        .await;
    let client = publisher(&server);
    let request = SetAppBuildLiveRequest {
        betakey: "public",
        steamid: None,
        ..build_request()
    };
    let response = client.apps().set_app_build_live(&request).await.unwrap();
    assert_eq!(response.http_status, 200);
    let requests = server.received_requests().await.unwrap();
    let fields: BTreeMap<_, _> = url::form_urlencoded::parse(&requests[0].body)
        .into_owned()
        .collect();
    assert_eq!(fields["betakey"], "public");
    assert!(!fields.contains_key("steamid"));
}
