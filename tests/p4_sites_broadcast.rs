// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! PC Cafe and broadcast contracts against local HTTP servers only.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::{Value, json};
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use steam_api_web::{
    ApiKey, Error, Host, RetryPolicy, SteamClient,
    broadcast::PostGameDataFrameRequest,
    site_license::{SiteLicenseType, TotalPlaytimeRequest},
};
use wiremock::{
    Mock, MockServer, Request, Respond, ResponseTemplate,
    matchers::{header, method, path},
};

const ID: u64 = u64::MAX;
const PRIVATE: &str = "private +&=%/中文";
const START: &str = "2026-10-21T00:00:00.00000000000000000001Z";
const END: &str = "2026-10-22T00:00:00+00:00";
const ROUTES: [(&str, &str); 3] = [
    ("GET", "ISiteLicenseService/GetCurrentClientConnections/v1/"),
    ("GET", "ISiteLicenseService/GetTotalPlaytime/v1/"),
    ("POST", "IBroadcastService/PostGameDataFrame/v1/"),
];

fn client(server: &MockServer) -> SteamClient {
    SteamClient::builder()
        .base_url(server.uri())
        .host(Host::Public)
        .api_key(ApiKey::publisher("sites-key").unwrap())
        .retry_policy(RetryPolicy {
            max_retries: 2,
            initial_backoff: Duration::ZERO,
            max_backoff: Duration::ZERO,
        })
        .build()
        .unwrap()
}

fn report<'a>(start_time: &'a str, end_time: &'a str) -> TotalPlaytimeRequest<'a> {
    TotalPlaytimeRequest {
        start_time,
        end_time,
        siteid: Some(ID),
    }
}

fn frame(frame_data: &str) -> PostGameDataFrameRequest<'_> {
    PostGameDataFrameRequest {
        appid: 480,
        steamid: ID.into(),
        broadcast_id: ID,
        frame_data,
    }
}

fn current_json() -> Value {
    json!({"response":{
        "sites":[{"siteid":ID.to_string(),"site_name":PRIVATE,"clients":[{
            "ip_address":"192.0.2.5","hostname":PRIVATE,"games":[{
                "id":u32::MAX,"start_time":START,"license_type":"future-license", "extension":PRIVATE
            }],"extension":PRIVATE
        }],"extension":PRIVATE}],
        "game_names":[{"id":u32::MAX,"name":PRIVATE,"extension":PRIVATE}],
        "extension":{"id":ID,"private":PRIVATE}
    }})
}

fn playtime_json() -> Value {
    json!({"response":{
        "sites":[{"siteid":ID,"site_name":PRIVATE,"games":[{
            "id":u32::MAX,"license_type":"future-license","playtime_seconds":u32::MAX,"extension":PRIVATE
        }],"extension":PRIVATE}],
        "game_names":[{"id":u32::MAX,"name":PRIVATE,"extension":PRIVATE}],
        "extension":{"id":ID,"private":PRIVATE}
    }})
}

fn success(route: &str) -> Value {
    if route.contains("GetCurrentClientConnections") {
        current_json()
    } else if route.contains("GetTotalPlaytime") {
        playtime_json()
    } else {
        json!({"response":{"extension":PRIVATE,"broadcast_id":ID}})
    }
}

async fn call(client: &SteamClient, route: &str) -> steam_api_web::Result<()> {
    if route.contains("GetCurrentClientConnections") {
        client
            .site_license()
            .get_current_client_connections(Some(ID))
            .await
            .map(|_| ())
    } else if route.contains("GetTotalPlaytime") {
        client
            .site_license()
            .get_total_playtime(&report(START, END))
            .await
            .map(|_| ())
    } else {
        client
            .broadcast()
            .post_game_data_frame(&frame(PRIVATE))
            .await
            .map(|_| ())
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
    serde_json::from_str(&pairs.iter().find(|(key, _)| key == "input_json").unwrap().1).unwrap()
}

#[tokio::test]
async fn all_three_routes_use_exact_versions_publisher_credentials_and_lossless_json() {
    let server = MockServer::start().await;
    let client = client(&server);
    for (verb, route) in ROUTES {
        Mock::given(method(verb))
            .and(path(format!("/{route}")))
            .and(header("x-webapi-key", "sites-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(success(route)))
            .expect(1)
            .mount(&server)
            .await;
        call(&client, route).await.unwrap();
    }
    let requests = server.received_requests().await.unwrap();
    assert_eq!(input(&requests[0]), json!({"siteid":ID}));
    assert_eq!(
        input(&requests[1]),
        json!({"siteid":ID,"start_time":START,"end_time":END})
    );
    assert_eq!(
        input(&requests[2]),
        json!({"appid":480,"steamid":ID.to_string(),"broadcast_id":ID,"frame_data":PRIVATE})
    );
}

#[tokio::test]
async fn site_models_keep_private_fields_dictionary_keys_full_ids_and_unknown_license_categories() {
    let server = MockServer::start().await;
    let client = client(&server);
    Mock::given(path(format!("/{}", ROUTES[0].1)))
        .respond_with(ResponseTemplate::new(200).set_body_json(current_json()))
        .mount(&server)
        .await;
    let response = client
        .site_license()
        .get_current_client_connections(None)
        .await
        .unwrap();
    let site = &response.sites[0];
    let computer = &site.clients[0];
    let session = &computer.games[0];
    assert_eq!(site.siteid, ID);
    assert_eq!(computer.ip_address, "192.0.2.5");
    assert_eq!(computer.hostname, PRIVATE);
    assert_eq!(session.start_time, START);
    assert_eq!(session.id, u32::MAX);
    assert_eq!(session.id, response.game_names[0].id);
    assert_eq!(
        session.license_type,
        SiteLicenseType::Other("future-license".into())
    );
    assert_eq!(session.license_type.as_str(), "future-license");
    assert_eq!(response.extra["extension"]["id"].as_u64(), Some(ID));
    assert_eq!(site.extra["extension"], PRIVATE);
    assert_eq!(computer.extra["extension"], PRIVATE);
    assert_eq!(session.extra["extension"], PRIVATE);
    assert_eq!(response.game_names[0].extra["extension"], PRIVATE);
    for debug in [
        format!("{response:?}"),
        format!("{site:?}"),
        format!("{computer:?}"),
        format!("{session:?}"),
        format!("{:?}", response.game_names[0]),
    ] {
        assert!(!debug.contains(PRIVATE));
        assert!(!debug.contains("192.0.2.5"));
    }
}

#[tokio::test]
async fn playtime_models_preserve_uint32_aggregates_and_every_documented_license_type() {
    let server = MockServer::start().await;
    let client = client(&server);
    let mut payload = playtime_json();
    for category in ["user", "commercial", "free"] {
        payload["response"]["sites"][0]["games"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":1,"license_type":category,"playtime_seconds":0}));
    }
    Mock::given(path(format!("/{}", ROUTES[1].1)))
        .respond_with(ResponseTemplate::new(200).set_body_json(payload))
        .mount(&server)
        .await;
    let response = client
        .site_license()
        .get_total_playtime(&report(START, END))
        .await
        .unwrap();
    let site = &response.sites[0];
    assert_eq!(site.siteid, ID);
    assert_eq!(site.games[0].playtime_seconds, u32::MAX);
    assert_eq!(site.games[0].id, response.game_names[0].id);
    assert_eq!(site.games[0].extra["extension"], PRIVATE);
    assert_eq!(site.games[1].license_type, SiteLicenseType::User);
    assert_eq!(site.games[2].license_type, SiteLicenseType::Commercial);
    assert_eq!(site.games[3].license_type, SiteLicenseType::Free);
    assert_eq!(response.extra["extension"]["id"].as_u64(), Some(ID));
    for debug in [
        format!("{response:?}"),
        format!("{site:?}"),
        format!("{:?}", site.games[0]),
    ] {
        assert!(!debug.contains(PRIVATE));
    }
}

#[tokio::test]
async fn omitted_and_zero_site_filters_are_distinct_wire_spellings_for_all_sites() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"response":{"sites":[],"game_names":[]}})),
        )
        .expect(4)
        .mount(&server)
        .await;
    let client = client(&server);
    for siteid in [None, Some(0)] {
        client
            .site_license()
            .get_current_client_connections(siteid)
            .await
            .unwrap();
        client
            .site_license()
            .get_total_playtime(&TotalPlaytimeRequest {
                siteid,
                ..report(START, END)
            })
            .await
            .unwrap();
    }
    let requests = server.received_requests().await.unwrap();
    assert_eq!(input(&requests[0]), json!({}));
    assert_eq!(
        input(&requests[1]),
        json!({"start_time":START,"end_time":END})
    );
    assert_eq!(input(&requests[2]), json!({"siteid":0}));
    assert_eq!(
        input(&requests[3]),
        json!({"siteid":0,"start_time":START,"end_time":END})
    );
}

#[tokio::test]
async fn utc_intervals_keep_fractional_precision_equal_instants_and_original_boundaries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"response":{"sites":[],"game_names":[]}})),
        )
        .expect(4)
        .mount(&server)
        .await;
    let client = client(&server);
    let cases = [
        ("2024-02-29T23:59:59.9Z", "2024-03-01T00:00:00Z"),
        ("2026-10-21T00:00:00.01Z", "2026-10-21t00:00:00.0100z"),
        ("2026-10-21T00:00:00Z", "2026-10-21T00:00:00+00:00"),
        (START, "2026-10-21T00:00:00.00000000000000000002Z"),
    ];
    for (start, end) in cases {
        client
            .site_license()
            .get_total_playtime(&report(start, end))
            .await
            .unwrap();
    }
    for (request, (start, end)) in server.received_requests().await.unwrap().iter().zip(cases) {
        assert_eq!(input(request)["start_time"], start);
        assert_eq!(input(request)["end_time"], end);
    }
}

#[tokio::test]
async fn invalid_or_reversed_intervals_and_zero_broadcast_identifiers_never_reach_http() {
    let server = MockServer::start().await;
    let client = client(&server);
    let invalid = [
        ("2026-02-29T00:00:00Z", END),
        ("2026-10-21T24:00:00Z", END),
        ("2026-10-21T00:00:00+01:00", END),
        ("2026-10-21T00:00:00-00:00", END),
        ("2026-10-21T00:00:00.Z", END),
        ("2026-10-21T00:00:00Z", PRIVATE),
        (END, START),
        (
            "2026-10-21T00:00:00.10000000000000000001Z",
            "2026-10-21T00:00:00.10000000000000000000Z",
        ),
    ];
    for (start, end) in invalid {
        assert!(matches!(
            client
                .site_license()
                .get_total_playtime(&report(start, end))
                .await,
            Err(Error::InvalidParameter(_))
        ));
    }
    for request in [
        PostGameDataFrameRequest {
            appid: 0,
            ..frame(PRIVATE)
        },
        PostGameDataFrameRequest {
            steamid: 0.into(),
            ..frame(PRIVATE)
        },
        PostGameDataFrameRequest {
            broadcast_id: 0,
            ..frame(PRIVATE)
        },
    ] {
        assert!(matches!(
            client.broadcast().post_game_data_frame(&request).await,
            Err(Error::InvalidParameter(_))
        ));
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn opaque_broadcast_frames_include_empty_nul_and_non_json_strings_without_transformation() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"response":{"extension":PRIVATE,"id":ID}})),
        )
        .expect(3)
        .mount(&server)
        .await;
    let client = client(&server);
    let frames = ["", "\0", "not {json} and not base64 +&=%/中文"];
    for value in frames {
        let request = frame(value);
        assert!(!format!("{request:?}").contains("not {json}"));
        let response = client
            .broadcast()
            .post_game_data_frame(&request)
            .await
            .unwrap();
        assert_eq!(response.as_json()["response"]["id"].as_u64(), Some(ID));
        assert!(!format!("{response:?}").contains(PRIVATE));
    }
    for (request, expected) in server.received_requests().await.unwrap().iter().zip(frames) {
        assert_eq!(input(request)["frame_data"], expected);
    }
}

struct TemporaryFailure(AtomicUsize);
impl Respond for TemporaryFailure {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        if self.0.fetch_add(1, Ordering::Relaxed) == 0 {
            ResponseTemplate::new(503)
        } else {
            ResponseTemplate::new(200).set_body_json(success(request.url.path()))
        }
    }
}

#[tokio::test]
async fn observational_site_reads_retry_transient_failures_with_the_identical_filter_and_range() {
    let server = MockServer::start().await;
    let client = client(&server);
    for (verb, route) in ROUTES.into_iter().filter(|(verb, _)| *verb == "GET") {
        Mock::given(method(verb))
            .and(path(format!("/{route}")))
            .respond_with(TemporaryFailure(AtomicUsize::new(0)))
            .expect(2)
            .mount(&server)
            .await;
        call(&client, route).await.unwrap();
    }
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 4);
    assert_eq!(requests[0].url, requests[1].url);
    assert_eq!(requests[2].url, requests[3].url);
    assert_eq!(client.metrics().attempts, 4);
    assert_eq!(client.metrics().retries, 2);
}

#[tokio::test]
async fn broadcast_metadata_is_not_replayed_after_throttling_server_failure_or_timeout() {
    for (status, delay) in [
        (429, Duration::ZERO),
        (503, Duration::ZERO),
        (200, Duration::from_millis(500)),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(status)
                    .insert_header("retry-after", "0")
                    .set_delay(delay)
                    .set_body_json(json!({"response":{}})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let client = SteamClient::builder()
            .base_url(server.uri())
            .api_key(ApiKey::publisher("sites-key").unwrap())
            .request_timeout(if delay.is_zero() {
                Duration::from_secs(5)
            } else {
                Duration::from_millis(100)
            })
            .retry_policy(RetryPolicy {
                max_retries: 2,
                initial_backoff: Duration::ZERO,
                max_backoff: Duration::ZERO,
            })
            .build()
            .unwrap();
        let error = client
            .broadcast()
            .post_game_data_frame(&frame(PRIVATE))
            .await
            .unwrap_err();
        if status == 200 {
            assert!(matches!(error, Error::Transport { timeout: true, .. }));
        } else {
            assert!(matches!(error, Error::Http { status: actual, .. } if actual == status));
        }
        assert_eq!(client.metrics().attempts, 1);
        assert_eq!(client.metrics().retries, 0);
    }
}

#[tokio::test]
async fn absent_and_user_credentials_never_authorize_sites_or_broadcast_writes() {
    let server = MockServer::start().await;
    let anonymous = SteamClient::builder()
        .base_url(server.uri())
        .build()
        .unwrap();
    let user = SteamClient::builder()
        .base_url(server.uri())
        .api_key(ApiKey::user("sites-key").unwrap())
        .build()
        .unwrap();
    for (_, route) in ROUTES {
        for client in [&anonymous, &user] {
            assert!(matches!(
                call(client, route).await,
                Err(Error::MissingApiKey("publisher"))
            ));
        }
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn explicit_public_host_client_does_not_reroute_or_fall_back_to_another_client() {
    let primary = MockServer::start().await;
    let secondary = MockServer::start().await;
    let client = client(&primary);
    let _other = SteamClient::builder()
        .host(Host::Partner)
        .base_url(secondary.uri())
        .api_key(ApiKey::publisher("other-key").unwrap())
        .build()
        .unwrap();
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(503))
        .expect(3)
        .mount(&primary)
        .await;
    assert!(matches!(
        client
            .site_license()
            .get_current_client_connections(None)
            .await,
        Err(Error::Http { status: 503, .. })
    ));
    assert!(secondary.received_requests().await.unwrap().is_empty());
    for request in primary.received_requests().await.unwrap() {
        assert_eq!(input(&request), json!({}));
        assert_eq!(request.headers.get("x-webapi-key").unwrap(), "sites-key");
    }
}

#[tokio::test]
async fn nested_business_errors_keep_the_code_and_hide_server_private_text_without_retry() {
    let server = MockServer::start().await;
    let client = client(&server);
    for (verb, route) in ROUTES {
        Mock::given(method(verb))
            .and(path(format!("/{route}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                json!({"response":{"params":{"error":{"errorcode":42,"errordesc":PRIVATE}}}}),
            ))
            .expect(1)
            .mount(&server)
            .await;
        let error = call(&client, route).await.unwrap_err();
        assert!(matches!(error, Error::Api { code: Some(42), .. }));
        assert!(!format!("{error:?} {error}").contains(PRIVATE));
    }
    assert_eq!(client.metrics().retries, 0);
}

#[tokio::test]
async fn malformed_json_and_site_schema_mismatches_are_redacted_and_count_as_failure() {
    let server = MockServer::start().await;
    let client = client(&server);
    for (verb, route) in ROUTES {
        Mock::given(method(verb))
            .and(path(format!("/{route}")))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(format!("{{\"response\":\"{PRIVATE}")),
            )
            .expect(1)
            .mount(&server)
            .await;
        let error = call(&client, route).await.unwrap_err();
        assert!(matches!(error, Error::Decode { .. }));
        assert!(!format!("{error:?} {error}").contains(PRIVATE));
    }
    server.reset().await;
    for payload in [
        json!({"sites":[],"game_names":[]}),
        json!({"response":{"sites":[]}}),
        json!({"response":{"sites":[{"siteid":"18446744073709551616","site_name":PRIVATE,"clients":[]}],"game_names":[]}}),
    ] {
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(payload))
            .expect(1)
            .mount(&server)
            .await;
        let error = client
            .site_license()
            .get_current_client_connections(None)
            .await
            .unwrap_err();
        assert!(matches!(error, Error::Decode { .. }));
        assert!(!format!("{error:?} {error}").contains(PRIVATE));
        server.reset().await;
    }
    assert_eq!(client.metrics().succeeded, 0);
    assert_eq!(client.metrics().failed, 6);
    assert_eq!(client.metrics().retries, 0);
}
