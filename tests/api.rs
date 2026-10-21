// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Protocol and resilience tests against local HTTP servers; no Steam key needed.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use serde_json::{Value, json};
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use steam_api_web::{
    ApiKey, ClientBuilder, Error, Host, RetryPolicy, SteamClient, SteamId,
    user::{PublisherOwnershipVersion, VanityUrlType},
    user_auth::{AuthenticateUserRequest, WebApiTicket},
    user_stats::{GlobalStatsRange, StatUpdate},
};
use wiremock::{
    Mock, MockServer, Request, Respond, ResponseTemplate,
    matchers::{header, method, path, query_param},
};

const ID: u64 = 76561198000000000;
fn builder(server: &MockServer) -> ClientBuilder {
    SteamClient::builder()
        .base_url(server.uri())
        .api_key(ApiKey::publisher("test-key").unwrap())
}
fn no_backoff(retries: u32) -> RetryPolicy {
    RetryPolicy {
        max_retries: retries,
        initial_backoff: Duration::ZERO,
        max_backoff: Duration::ZERO,
    }
}
fn count_response() -> Value {
    json!({"response":{"player_count":12,"result":1}})
}
fn ownership() -> Value {
    json!({"ownsapp":true,"permanent":false,"timestamp":"2026-01-01T00:00:00Z","timeexpires":"never","ownersteamid":ID.to_string(),"sitelicense":false,"usercanceled":false})
}
async fn fixture(server: &MockServer, verb: &str, endpoint: &str, body: Value) {
    Mock::given(method(verb))
        .and(path(endpoint))
        .and(header("x-webapi-key", "test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(server)
        .await;
}

#[tokio::test]
async fn ticket_binds_identity_uses_effective_callback_bytes_and_redacts_secrets() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/ISteamUserAuth/AuthenticateUserTicket/v1/"))
        .and(query_param("appid", "480")).and(query_param("ticket", "00abff"))
        .and(query_param("identity", "my service&=中文")).and(header("x-webapi-key", "test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response":{"params":{
            "result":"OK","steamid":ID.to_string(),"ownersteamid":ID,"vacbanned":false,"publisherbanned":true}}})))
        .expect(1).mount(&server).await;
    let ticket = WebApiTicket::from_callback_bytes(&[0, 0xab, 0xff, 0, 0], 3).unwrap();
    let client = builder(&server).build().unwrap();
    let user = client
        .user_auth()
        .authenticate_user_ticket(480, &ticket, "my service&=中文")
        .await
        .unwrap();
    assert_eq!(user.steamid.raw(), ID);
    assert_eq!(user.ownersteamid.unwrap().raw(), ID);
    assert_eq!(user.publisherbanned, Some(true));
    assert!(!format!("{ticket:?}").contains("00abff"));
    let requests = server.received_requests().await.unwrap();
    assert!(!requests[0].url.query_pairs().any(|(key, _)| key == "key"));
    assert_eq!(
        requests[0]
            .url
            .query_pairs()
            .find(|(key, _)| key == "format")
            .unwrap()
            .1,
        "json"
    );
}

#[tokio::test]
async fn legacy_login_preserves_all_raw_bytes_in_form_body() {
    let server = MockServer::start().await;
    fixture(
        &server,
        "POST",
        "/ISteamUserAuth/AuthenticateUser/v1/",
        json!({"response":{"token":"private-token"}}),
    )
    .await;
    let raw: Vec<u8> = (0..=255).collect();
    let request = AuthenticateUserRequest {
        steamid: ID.into(),
        sessionkey: &raw,
        encrypted_loginkey: &[0xff, 0, b'+', b'&', b'='],
    };
    let result = builder(&server)
        .build()
        .unwrap()
        .user_auth()
        .authenticate_user(&request)
        .await
        .unwrap();
    assert_eq!(result.as_json()["response"]["token"], "private-token");
    assert!(!format!("{result:?}").contains("private-token"));
    let requests = server.received_requests().await.unwrap();
    let body = std::str::from_utf8(&requests[0].body).unwrap();
    assert!(body.contains("encrypted_loginkey=%FF%00%2B%26%3D"));
    let encoded = body
        .split('&')
        .find_map(|p| p.strip_prefix("sessionkey="))
        .unwrap();
    // Decode as bytes, not UTF-8, and verify every possible octet survives.
    let mut decoded = Vec::new();
    let bytes = encoded.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                decoded.push(u8::from_str_radix(&encoded[i + 1..i + 3], 16).unwrap());
                i += 3;
            }
            b'+' => {
                decoded.push(b' ');
                i += 1;
            }
            byte => {
                decoded.push(byte);
                i += 1;
            }
        }
    }
    assert_eq!(decoded, raw);
    assert_eq!(
        requests[0].headers.get("content-type").unwrap(),
        "application/x-www-form-urlencoded"
    );
    assert!(requests[0].url.query().is_none());
}

#[tokio::test]
async fn all_user_methods_match_versions_parameters_and_response_envelopes() {
    let server = MockServer::start().await;
    fixture(
        &server,
        "GET",
        "/ISteamUser/CheckAppOwnership/v4/",
        json!({"appownership":ownership()}),
    )
    .await;
    fixture(&server, "GET", "/ISteamUser/GetAppPriceInfo/v1/", json!({"price_info":{"480":{"packageid":1,"currency":"CNY","initial_price":999,"final_price":500,"discount_percent":50}}})).await;
    fixture(
        &server,
        "GET",
        "/ISteamUser/GetDeletedSteamIDs/v1/",
        json!({"response":{"rowversion":"18446744073709551615","steamids":[]}}),
    )
    .await;
    fixture(&server, "GET", "/ISteamUser/GetFriendList/v1/", json!({"friendslist":{"friends":[{"steamid":ID.to_string(),"relationship":"friend","friend_since":1}]}})).await;
    fixture(&server, "GET", "/ISteamUser/GetPlayerBans/v1/", json!({"players":[{"SteamId":ID.to_string(),"CommunityBanned":false,"VACBanned":true,"NumberOfVACBans":1,"DaysSinceLastBan":3,"NumberOfGameBans":2,"EconomyBan":"none","bans":[{"AppIdMin":60,"AppIdMax":60,"BanStartTime":1780525984_u64,"BanType":2,"BanProviderId":0}]}]})).await;
    fixture(&server, "GET", "/ISteamUser/GetPlayerSummaries/v2/", json!({"response":{"players":[{"steamid":ID.to_string(),"communityvisibilitystate":1,"personaname":"test","profileurl":"https://steamcommunity.com/","avatar":"a","avatarmedium":"b","avatarfull":"c","future_field":7}]}})).await;
    let mut app = ownership();
    app["appid"] = json!(480);
    fixture(
        &server,
        "GET",
        "/ISteamUser/GetPublisherAppOwnership/v4/",
        json!({"appownership":{"apps":[app]}}),
    )
    .await;
    fixture(
        &server,
        "GET",
        "/ISteamUser/GetPublisherAppOwnership/v5/",
        json!({"appownership":{"apps":[]}}),
    )
    .await;
    fixture(
        &server,
        "GET",
        "/ISteamUser/GetUserGroupList/v1/",
        json!({"response":{"success":true,"groups":[{"gid":"123"}]}}),
    )
    .await;
    fixture(
        &server,
        "GET",
        "/ISteamUser/ResolveVanityURL/v1/",
        json!({"response":{"success":1,"steamid":ID.to_string()}}),
    )
    .await;
    let client = builder(&server).build().unwrap();
    let user = client.user();
    assert!(user.check_app_ownership(ID, 480).await.unwrap().ownsapp);
    assert_eq!(
        user.get_app_price_info(ID, &[480, 220]).await.unwrap()[&480].final_price,
        500
    );
    assert_eq!(
        user.get_deleted_steam_ids(u64::MAX)
            .await
            .unwrap()
            .as_json()["response"]["rowversion"],
        u64::MAX.to_string()
    );
    assert_eq!(
        user.get_friend_list(ID, Some("friend"))
            .await
            .unwrap()
            .friends[0]
            .steamid
            .raw(),
        ID
    );
    assert_eq!(
        user.get_player_bans(&[ID.into()]).await.unwrap()[0].bans[0].app_id_min,
        60
    );
    let profiles = user.get_player_summaries(&[ID.into()]).await.unwrap();
    assert_eq!(profiles[0].extra["future_field"], 7);
    assert!(profiles[0].lastlogoff.is_none());
    assert_eq!(
        user.get_publisher_app_ownership(ID).await.unwrap().apps[0].appid,
        480
    );
    assert!(
        user.get_publisher_app_ownership_version(ID, PublisherOwnershipVersion::V5)
            .await
            .unwrap()
            .apps
            .is_empty()
    );
    assert_eq!(user.get_user_group_list(ID).await.unwrap()[0].gid, 123);
    assert_eq!(
        user.resolve_vanity_url("example", VanityUrlType::Group)
            .await
            .unwrap()
            .unwrap()
            .raw(),
        ID
    );
    let requests = server.received_requests().await.unwrap();
    assert!(requests.iter().all(|r| {
        r.url
            .query_pairs()
            .any(|(k, v)| k == "format" && v == "json")
    }));
    for request in requests {
        let params: std::collections::HashMap<_, _> =
            request.url.query_pairs().into_owned().collect();
        match request.url.path() {
            "/ISteamUser/GetAppPriceInfo/v1/" => assert_eq!(params["appids"], "480,220"),
            "/ISteamUser/GetDeletedSteamIDs/v1/" => {
                assert_eq!(params["rowversion"], u64::MAX.to_string())
            }
            "/ISteamUser/GetFriendList/v1/" => assert_eq!(params["relationship"], "friend"),
            "/ISteamUser/GetPlayerBans/v1/" | "/ISteamUser/GetPlayerSummaries/v2/" => {
                assert_eq!(params["steamids"], ID.to_string())
            }
            "/ISteamUser/ResolveVanityURL/v1/" => {
                assert_eq!(params["url_type"], "2");
                assert_eq!(params["vanityurl"], "example");
            }
            _ => assert_eq!(params["steamid"], ID.to_string()),
        }
    }
}

#[tokio::test]
async fn all_stats_methods_preserve_numeric_values_and_indexed_forms() {
    let server = MockServer::start().await;
    fixture(
        &server,
        "GET",
        "/ISteamUserStats/GetGlobalAchievementPercentagesForApp/v2/",
        json!({"achievementpercentages":{"achievements":[{"name":"WIN","percent":12.5}]}}),
    )
    .await;
    fixture(&server, "GET", "/ISteamUserStats/GetGlobalStatsForGame/v1/", json!({"response":{"result":1,"globalstats":{"total":{"total":"18446744073709551615"},"float":{"total":1.25}}}})).await;
    fixture(
        &server,
        "GET",
        "/ISteamUserStats/GetNumberOfCurrentPlayers/v1/",
        count_response(),
    )
    .await;
    fixture(&server, "GET", "/ISteamUserStats/GetPlayerAchievements/v1/", json!({"playerstats":{"steamID":ID.to_string(),"success":true,"achievements":[{"apiname":"WIN","achieved":1,"unlocktime":100}]}})).await;
    fixture(&server, "GET", "/ISteamUserStats/GetSchemaForGame/v2/", json!({"game":{"gameName":"test","gameVersion":"1","availableGameStats":{"stats":[{"name":"total","defaultvalue":0}],"achievements":[{"name":"WIN","defaultvalue":0,"displayName":"Win","hidden":1,"icon":"a","icongray":"b"}]}}})).await;
    fixture(&server, "GET", "/ISteamUserStats/GetUserStatsForGame/v2/", json!({"playerstats":{"steamID":ID,"gameName":"test","stats":[{"name":"negative","value":-5},{"name":"float","value":1.25}],"achievements":[{"name":"WIN","achieved":1}]}})).await;
    fixture(
        &server,
        "POST",
        "/ISteamUserStats/SetUserStatsForGame/v1/",
        json!({"response":{"result":1}}),
    )
    .await;
    let client = builder(&server).build().unwrap();
    let stats = client.user_stats();
    assert_eq!(
        stats
            .get_global_achievement_percentages_for_app(480)
            .await
            .unwrap()
            .achievements[0]
            .percent,
        12.5
    );
    let global = stats
        .get_global_stats_for_game(
            480,
            &["total", "float"],
            GlobalStatsRange {
                startdate: Some(1),
                enddate: Some(2),
            },
        )
        .await
        .unwrap();
    assert_eq!(
        global.globalstats["total"]
            .total
            .as_ref()
            .unwrap()
            .0
            .as_u64(),
        Some(u64::MAX)
    );
    assert_eq!(
        stats
            .get_number_of_current_players(480)
            .await
            .unwrap()
            .player_count,
        12
    );
    assert_eq!(
        stats
            .get_player_achievements(ID, 480, Some("schinese"))
            .await
            .unwrap()
            .achievements[0]
            .achieved,
        1
    );
    assert!(
        stats
            .get_schema_for_game(480, Some("english"))
            .await
            .unwrap()
            .available_game_stats
            .achievements[0]
            .description
            .is_none()
    );
    assert_eq!(
        stats.get_user_stats_for_game(ID, 480).await.unwrap().stats[0]
            .value
            .0
            .as_i64(),
        Some(-5)
    );
    stats
        .set_user_stats_for_game(
            ID,
            480,
            &[
                StatUpdate {
                    name: "with & space=中文",
                    value: u32::MAX,
                },
                StatUpdate {
                    name: "WIN",
                    value: 1,
                },
            ],
        )
        .await
        .unwrap();
    for request in server.received_requests().await.unwrap() {
        let source = if request.method == "POST" {
            std::str::from_utf8(&request.body).unwrap()
        } else {
            request.url.query().unwrap()
        };
        let params: std::collections::HashMap<_, _> =
            url::form_urlencoded::parse(source.as_bytes())
                .into_owned()
                .collect();
        match request.url.path() {
            "/ISteamUserStats/GetGlobalStatsForGame/v1/" => {
                assert_eq!(params["count"], "2");
                assert_eq!(params["name[1]"], "float");
                assert_eq!(params["startdate"], "1");
                assert_eq!(params["enddate"], "2");
            }
            "/ISteamUserStats/SetUserStatsForGame/v1/" => {
                assert_eq!(params["count"], "2");
                assert_eq!(params["name[0]"], "with & space=中文");
                assert_eq!(params["value[0]"], u32::MAX.to_string());
                assert_eq!(params["value[1]"], "1");
                assert!(request.url.query().is_none());
            }
            "/ISteamUserStats/GetGlobalAchievementPercentagesForApp/v2/" => {
                assert_eq!(params["gameid"], "480")
            }
            "/ISteamUserStats/GetPlayerAchievements/v1/" => assert_eq!(params["l"], "schinese"),
            "/ISteamUserStats/GetSchemaForGame/v2/" => assert_eq!(params["l"], "english"),
            _ => assert_eq!(params["appid"], "480"),
        }
    }
}

struct Transient {
    calls: AtomicUsize,
    failures: usize,
    status: u16,
}
impl Respond for Transient {
    fn respond(&self, _: &Request) -> ResponseTemplate {
        if self.calls.fetch_add(1, Ordering::SeqCst) < self.failures {
            ResponseTemplate::new(self.status)
        } else {
            ResponseTemplate::new(200).set_body_json(count_response())
        }
    }
}
#[tokio::test]
async fn transient_read_errors_retry_with_a_finite_budget() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(Transient {
            calls: AtomicUsize::new(0),
            failures: 2,
            status: 503,
        })
        .expect(3)
        .mount(&server)
        .await;
    let client = builder(&server)
        .retry_policy(no_backoff(2))
        .build()
        .unwrap();
    assert_eq!(
        client
            .user_stats()
            .get_number_of_current_players(480)
            .await
            .unwrap()
            .player_count,
        12
    );
    assert_eq!(
        client.metrics(),
        steam_api_web::ClientMetrics {
            attempts: 3,
            retries: 2,
            succeeded: 1,
            failed: 0
        }
    );
}
#[tokio::test]
async fn retries_exhaust_and_permanent_http_errors_do_not_replay() {
    for (status, attempts) in [
        (500, 3),
        (502, 3),
        (504, 3),
        (408, 3),
        (400, 1),
        (401, 1),
        (403, 1),
        (404, 1),
        (501, 1),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(status))
            .expect(attempts)
            .mount(&server)
            .await;
        let client = builder(&server)
            .retry_policy(no_backoff(2))
            .build()
            .unwrap();
        assert!(
            matches!(client.user_stats().get_number_of_current_players(480).await, Err(Error::Http{status:s,..}) if s==status)
        );
        assert_eq!(client.metrics().attempts, attempts);
    }
}
#[tokio::test]
async fn authentication_and_write_failures_are_never_replayed() {
    for post in [false, true] {
        let server = MockServer::start().await;
        Mock::given(method(if post { "POST" } else { "GET" }))
            .respond_with(ResponseTemplate::new(503))
            .expect(1)
            .mount(&server)
            .await;
        let client = builder(&server)
            .retry_policy(no_backoff(10))
            .build()
            .unwrap();
        let ticket = WebApiTicket::from_hex("aabb").unwrap();
        if post {
            assert!(matches!(
                client
                    .user_stats()
                    .set_user_stats_for_game(
                        ID,
                        480,
                        &[StatUpdate {
                            name: "score",
                            value: 1
                        }]
                    )
                    .await,
                Err(Error::Http { status: 503, .. })
            ));
        } else {
            assert!(matches!(
                client
                    .user_auth()
                    .authenticate_user_ticket(480, &ticket, "identity")
                    .await,
                Err(Error::Http { status: 503, .. })
            ));
        }
        assert_eq!(client.metrics().attempts, 1);
    }
}
#[tokio::test]
async fn retry_after_creates_shared_cooldown_even_when_retries_disabled() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(429).insert_header("Retry-After", "18446744073709551615"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = builder(&server)
        .retry_policy(no_backoff(0))
        .operation_timeout(Duration::from_millis(250))
        .build()
        .unwrap();
    assert!(matches!(
        client.user_stats().get_number_of_current_players(480).await,
        Err(Error::Http {
            status: 429,
            retry_after: Some(_),
            ..
        })
    ));
    assert!(matches!(
        client
            .clone()
            .user_stats()
            .get_number_of_current_players(480)
            .await,
        Err(Error::Deadline { .. })
    ));
    assert_eq!(client.metrics().attempts, 1);
}
#[tokio::test]
async fn operation_deadline_includes_retry_backoff() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(503))
        .expect(1)
        .mount(&server)
        .await;
    let client = builder(&server)
        .retry_policy(RetryPolicy {
            max_retries: 2,
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(1),
        })
        .operation_timeout(Duration::from_millis(100))
        .build()
        .unwrap();
    assert!(matches!(
        client.user_stats().get_number_of_current_players(480).await,
        Err(Error::Deadline { .. })
    ));
}
#[tokio::test]
async fn private_friend_list_is_an_error_and_vanity_no_match_is_none() {
    let server = MockServer::start().await;
    Mock::given(path("/ISteamUser/GetFriendList/v1/"))
        .respond_with(ResponseTemplate::new(401))
        .expect(1)
        .mount(&server)
        .await;
    fixture(
        &server,
        "GET",
        "/ISteamUser/ResolveVanityURL/v1/",
        json!({"response":{"success":42,"message":"No match"}}),
    )
    .await;
    let client = builder(&server).build().unwrap();
    assert!(matches!(
        client.user().get_friend_list(ID, None).await,
        Err(Error::Http { status: 401, .. })
    ));
    assert!(
        client
            .user()
            .resolve_vanity_url("nonexistent", VanityUrlType::Individual)
            .await
            .unwrap()
            .is_none()
    );
}
#[tokio::test]
async fn nested_business_errors_and_invalid_success_markers_fail_closed() {
    for body in [
        json!({"response":{"result":2}}),
        json!({"response":{"error":{"errorcode":5,"errordesc":"secret"}}}),
        json!({"response":{"success":false}}),
        json!({"response":{"success":"secret","result":1,"player_count":12}}),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .expect(1)
            .mount(&server)
            .await;
        let client = builder(&server).build().unwrap();
        assert!(
            client
                .user_stats()
                .get_number_of_current_players(480)
                .await
                .is_err()
        );
    }
    let server = MockServer::start().await;
    fixture(
        &server,
        "POST",
        "/ISteamUserStats/SetUserStatsForGame/v1/",
        json!({"response":{"result":2}}),
    )
    .await;
    let client = builder(&server).build().unwrap();
    assert!(matches!(
        client
            .user_stats()
            .set_user_stats_for_game(
                ID,
                480,
                &[StatUpdate {
                    name: "score",
                    value: 1
                }]
            )
            .await,
        Err(Error::Api { code: Some(2), .. })
    ));
}
#[tokio::test]
async fn malformed_tickets_and_json_never_appear_in_errors() {
    for body in [
        json!({"response":{"params":{"result":"OK","steamid":"secret-ticket"}}}),
        json!({"response":{"params":{"result":"FAIL secret-ticket","steamid":ID}}}),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .expect(1)
            .mount(&server)
            .await;
        let client = builder(&server).build().unwrap();
        let error = client
            .user_auth()
            .authenticate_user_ticket(
                480,
                &WebApiTicket::from_hex("deadbeef").unwrap(),
                "private-identity",
            )
            .await
            .unwrap_err();
        let printed = format!("{error} {error:?}");
        for secret in [
            "secret-ticket",
            "deadbeef",
            "private-identity",
            "test-key",
            "http://",
        ] {
            assert!(!printed.contains(secret));
        }
    }
}
#[tokio::test]
async fn redirects_are_not_followed_and_large_bodies_are_rejected() {
    let server = MockServer::start().await;
    let other = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(302).insert_header("Location", other.uri()))
        .expect(1)
        .mount(&server)
        .await;
    let client = builder(&server).build().unwrap();
    assert!(matches!(
        client.user_stats().get_number_of_current_players(480).await,
        Err(Error::Http { status: 302, .. })
    ));
    assert!(other.received_requests().await.unwrap().is_empty());
    server.reset().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("x".repeat(256)))
        .expect(1)
        .mount(&server)
        .await;
    let client = builder(&server).max_response_bytes(32).build().unwrap();
    assert!(matches!(
        client.user_stats().get_number_of_current_players(480).await,
        Err(Error::ResponseTooLarge { limit: 32, .. })
    ));
}
#[tokio::test]
async fn client_clones_share_concurrency_and_cancellation_releases_permits() {
    let server = MockServer::start().await;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    Mock::given(query_param("appid", "1"))
        .respond_with(move |_: &Request| {
            tx.send(()).unwrap();
            ResponseTemplate::new(200)
                .set_body_json(count_response())
                .set_delay(Duration::from_secs(10))
        })
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(query_param("appid", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(count_response()))
        .expect(1)
        .mount(&server)
        .await;
    let client = builder(&server)
        .max_in_flight(1)
        .operation_timeout(Duration::from_secs(1))
        .build()
        .unwrap();
    let first = client.clone();
    let first =
        tokio::spawn(async move { first.user_stats().get_number_of_current_players(1).await });
    rx.recv().await.unwrap();
    let second = client.clone();
    let second =
        tokio::spawn(async move { second.user_stats().get_number_of_current_players(2).await });
    // Wait until the second task has had an opportunity to queue on the semaphore.
    tokio::task::yield_now().await;
    assert_eq!(client.metrics().attempts, 1);
    first.abort();
    assert!(first.await.unwrap_err().is_cancelled());
    assert_eq!(second.await.unwrap().unwrap().player_count, 12);
    assert_eq!(client.metrics().attempts, 2);
}
#[tokio::test]
async fn pacing_is_shared_by_clones_and_bounds_request_start_rate() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(count_response()))
        .expect(2)
        .mount(&server)
        .await;
    let client = builder(&server)
        .min_request_interval(Duration::from_millis(80))
        .build()
        .unwrap();
    let start = std::time::Instant::now();
    let cloned = client.clone();
    let first = client.user_stats();
    let second = cloned.user_stats();
    let (a, b) = tokio::join!(
        first.get_number_of_current_players(1),
        second.get_number_of_current_players(2)
    );
    a.unwrap();
    b.unwrap();
    assert!(start.elapsed() >= Duration::from_millis(80));
}
#[tokio::test]
async fn profile_batching_splits_at_100_without_manufacturing_missing_users() {
    let server = MockServer::start().await;
    Mock::given(path("/ISteamUser/GetPlayerSummaries/v2/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response":{"players":[]}})))
        .expect(3)
        .mount(&server)
        .await;
    let client = builder(&server).build().unwrap();
    assert!(
        client
            .user()
            .get_player_summaries_batched(&(1..=201).map(SteamId::from).collect::<Vec<_>>())
            .await
            .unwrap()
            .is_empty()
    );
    let requests = server.received_requests().await.unwrap();
    let sizes: Vec<_> = requests
        .iter()
        .map(|r| {
            r.url
                .query_pairs()
                .find(|(k, _)| k == "steamids")
                .unwrap()
                .1
                .split(',')
                .count()
        })
        .collect();
    assert_eq!(sizes, vec![100, 100, 1]);
}
#[tokio::test]
async fn validation_rejects_bad_inputs_and_missing_keys_before_network() {
    let server = MockServer::start().await;
    let client = builder(&server).build().unwrap();
    assert!(client.user().get_player_summaries(&[]).await.is_err());
    assert!(
        client
            .user()
            .get_player_summaries(&vec![ID.into(); 101])
            .await
            .is_err()
    );
    assert!(client.user().check_app_ownership(0_u64, 480).await.is_err());
    assert!(client.user().get_app_price_info(ID, &[0]).await.is_err());
    assert!(
        client
            .user_stats()
            .get_global_stats_for_game(480, &[], GlobalStatsRange::default())
            .await
            .is_err()
    );
    assert!(
        client
            .user_stats()
            .get_global_stats_for_game(
                480,
                &["x"],
                GlobalStatsRange {
                    startdate: Some(2),
                    enddate: Some(1)
                }
            )
            .await
            .is_err()
    );
    assert!(
        client
            .user_stats()
            .set_user_stats_for_game(
                ID,
                480,
                &[
                    StatUpdate {
                        name: "x",
                        value: 1
                    },
                    StatUpdate {
                        name: "x",
                        value: 2
                    }
                ]
            )
            .await
            .is_err()
    );
    let anonymous = SteamClient::builder()
        .base_url(server.uri())
        .build()
        .unwrap();
    assert!(matches!(
        anonymous.user().get_friend_list(ID, None).await,
        Err(Error::MissingApiKey(_))
    ));
    let user = SteamClient::builder()
        .base_url(server.uri())
        .api_key(ApiKey::user("user-key").unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        user.user().check_app_ownership(ID, 480).await,
        Err(Error::MissingApiKey("publisher"))
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
}
#[test]
fn configuration_and_ticket_validation_have_safe_failures() {
    for builder in [
        SteamClient::builder().host(Host::Partner),
        SteamClient::builder().max_in_flight(0),
        SteamClient::builder().max_response_bytes(0),
        SteamClient::builder().operation_timeout(Duration::ZERO),
        SteamClient::builder().base_url("http://example.com"),
        SteamClient::builder().base_url("https://user:secret@example.com"),
        SteamClient::builder().base_url("https://example.com/?ticket=secret"),
    ] {
        let error = builder.build().unwrap_err();
        assert!(!format!("{error:?}").contains("secret"));
    }
    assert!(ApiKey::publisher("\nsecret").is_err());
    assert!(ApiKey::user("").is_err());
    for ticket in ["", "abc", "zz"] {
        assert!(WebApiTicket::from_hex(ticket).is_err());
    }
    for len in [-1, 0, 3] {
        assert!(WebApiTicket::from_callback_bytes(&[1, 2], len).is_err());
    }
    fn send_sync<T: Send + Sync>() {}
    send_sync::<SteamClient>();
    send_sync::<Error>();
}
#[test]
fn ids_preserve_full_u64_range_without_float_rounding() {
    for raw in [format!("\"{}\"", u64::MAX), u64::MAX.to_string()] {
        let id: SteamId = serde_json::from_str(&raw).unwrap();
        assert_eq!(id.raw(), u64::MAX);
        assert_eq!(
            serde_json::to_string(&id).unwrap(),
            format!("\"{}\"", u64::MAX)
        );
    }
    for raw in ["-1", "1.5", "\"18446744073709551616\""] {
        assert!(serde_json::from_str::<SteamId>(raw).is_err());
    }
}

#[tokio::test]
async fn chunked_body_limit_is_enforced_without_content_length() {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            assert_ne!(socket.read_buf(&mut request).await.unwrap(), 0);
        }
        socket.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n20\r\n01234567890123456789012345678901\r\n20\r\n01234567890123456789012345678901\r\n0\r\n\r\n").await.unwrap();
    });
    let client = SteamClient::builder()
        .base_url(format!("http://{address}"))
        .max_response_bytes(40)
        .build()
        .unwrap();
    assert!(matches!(
        client.user_stats().get_number_of_current_players(480).await,
        Err(Error::ResponseTooLarge { limit: 40, .. })
    ));
    server.await.unwrap();
}

#[tokio::test]
async fn per_attempt_timeouts_retry_reads_but_never_authentication() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(count_response())
                .set_delay(Duration::from_secs(1)),
        )
        .expect(3)
        .mount(&server)
        .await;
    let client = builder(&server)
        .request_timeout(Duration::from_millis(100))
        .retry_policy(no_backoff(2))
        .build()
        .unwrap();
    assert!(matches!(
        client.user_stats().get_number_of_current_players(480).await,
        Err(Error::Transport { timeout: true, .. })
    ));
    assert_eq!(client.metrics().attempts, 3);
    server.reset().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({}))
                .set_delay(Duration::from_secs(1)),
        )
        .expect(1)
        .mount(&server)
        .await;
    let ticket = WebApiTicket::from_hex("aabb").unwrap();
    assert!(matches!(
        client
            .user_auth()
            .authenticate_user_ticket(480, &ticket, "service")
            .await,
        Err(Error::Transport { timeout: true, .. })
    ));
    assert_eq!(client.metrics().attempts, 4);
}

#[tokio::test]
async fn anonymous_request_keeps_gateway_prefix_and_has_no_credential_header() {
    let server = MockServer::start().await;
    Mock::given(path(
        "/gateway/ISteamUserStats/GetNumberOfCurrentPlayers/v1/",
    ))
    .respond_with(ResponseTemplate::new(200).set_body_json(count_response()))
    .expect(1)
    .mount(&server)
    .await;
    let client = SteamClient::builder()
        .base_url(format!("{}/gateway", server.uri()))
        .build()
        .unwrap();
    assert_eq!(
        client
            .user_stats()
            .get_number_of_current_players(480)
            .await
            .unwrap()
            .player_count,
        12
    );
    assert!(
        !server.received_requests().await.unwrap()[0]
            .headers
            .contains_key("x-webapi-key")
    );
}

#[tokio::test]
async fn malformed_json_and_wrong_root_shape_are_not_retried() {
    for body in ["{not-json", "null", "[]"] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .expect(1)
            .mount(&server)
            .await;
        assert!(matches!(
            builder(&server)
                .build()
                .unwrap()
                .user_stats()
                .get_number_of_current_players(480)
                .await,
            Err(Error::Decode { .. })
        ));
    }
}
