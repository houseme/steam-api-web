// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! P1 read-service contracts against local servers; no Steam credentials needed.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::{Value, json};
use std::time::Duration;
use steam_api_web::{
    ApiKey, Error, RetryPolicy, SteamClient,
    news::NewsOptions,
    player::OwnedGamesOptions,
    reviews::{AppReviewsOptions, PurchaseType, ReviewFilter, ReviewType},
    store::AppListOptions,
};
use wiremock::{
    Mock, MockServer, Request, ResponseTemplate,
    matchers::{header, method, path},
};

const ID: u64 = u64::MAX;

fn client(server: &MockServer) -> SteamClient {
    SteamClient::builder()
        .base_url(server.uri())
        .api_key(ApiKey::user("test-key").unwrap())
        .build()
        .unwrap()
}

async fn fixture(server: &MockServer, endpoint: &str, value: Value, keyed: bool) {
    let mock = Mock::given(method("GET")).and(path(endpoint));
    let mock = if keyed {
        mock.and(header("x-webapi-key", "test-key"))
    } else {
        mock
    };
    mock.respond_with(ResponseTemplate::new(200).set_body_json(value))
        .expect(1)
        .mount(server)
        .await;
}

fn input(request: &Request) -> Value {
    assert!(request.body.is_empty());
    let pairs = request.url.query_pairs().collect::<Vec<_>>();
    assert_eq!(pairs.len(), 2);
    assert!(
        pairs
            .iter()
            .any(|(key, value)| key == "format" && value == "json")
    );
    let json = &pairs.iter().find(|(key, _)| key == "input_json").unwrap().1;
    serde_json::from_str(json).unwrap()
}

fn page(reviews: Value) -> Value {
    json!({"response": {
        "query_summary": {"num_reviews": reviews.as_array().unwrap().len()},
        "reviews": reviews,
        "cursor": "AoJ+/=&opaque 中文",
        "total_matching": 1
    }})
}

fn review() -> Value {
    json!({
        "recommendationid": u64::MAX.to_string(),
        "author": {"steamid": ID.to_string(), "playtime_forever": 120, "new_author_field": 7},
        "language": "english",
        "review": "A review with Unicode: 中文",
        "timestamp_created": 10,
        "timestamp_updated": 20,
        "voted_up": true,
        "votes_up": 2,
        "votes_funny": 1,
        "weighted_vote_score": 0.75,
        "comment_count": 0,
        "steam_purchase": true,
        "received_for_free": false,
        "written_during_early_access": false,
        "primarily_steam_deck": true,
        "refunded": false,
        "new_review_field": {"x": 1}
    })
}

#[tokio::test]
async fn anonymous_discovery_and_news_use_documented_versions_and_preserve_raw_shapes() {
    let server = MockServer::start().await;
    fixture(
        &server,
        "/ISteamWebAPIUtil/GetServerInfo/v1/",
        json!({"servertime": 123, "future": true}),
        false,
    )
    .await;
    fixture(
        &server,
        "/ISteamWebAPIUtil/GetSupportedAPIList/v1/",
        json!({"apilist": {"interfaces": [], "future": 42}}),
        false,
    )
    .await;
    fixture(
        &server,
        "/ISteamNews/GetNewsForApp/v2/",
        json!({"appnews": {"appid": 480, "newsitems": [], "future": "kept"}}),
        false,
    )
    .await;
    let client = SteamClient::builder()
        .base_url(server.uri())
        .build()
        .unwrap();
    assert_eq!(
        client
            .web_api_util()
            .get_server_info()
            .await
            .unwrap()
            .as_json()["future"],
        true
    );
    assert_eq!(
        client
            .web_api_util()
            .get_supported_api_list()
            .await
            .unwrap()
            .as_json()["apilist"]["future"],
        42
    );
    let news = client
        .news()
        .get_news_for_app(
            480,
            NewsOptions {
                maxlength: Some(0),
                enddate: Some(u32::MAX),
                count: Some(3),
                feeds: Some("steam_community_announcements,external&=feed"),
            },
        )
        .await
        .unwrap();
    assert_eq!(news.as_json()["appnews"]["future"], "kept");
    let requests = server.received_requests().await.unwrap();
    assert!(
        requests
            .iter()
            .all(|request| !request.headers.contains_key("x-webapi-key"))
    );
    let news = requests
        .iter()
        .find(|request| request.url.path().contains("GetNewsForApp"))
        .unwrap();
    let pairs = news
        .url
        .query_pairs()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(pairs["appid"], "480");
    assert_eq!(pairs["maxlength"], "0");
    assert_eq!(pairs["enddate"], u32::MAX.to_string());
    assert_eq!(pairs["count"], "3");
    assert_eq!(
        pairs["feeds"],
        "steam_community_announcements,external&=feed"
    );
    assert!(!pairs.contains_key("input_json"));
}

#[tokio::test]
async fn all_player_methods_encode_service_json_and_preserve_full_u64_and_private_results() {
    let server = MockServer::start().await;
    for endpoint in [
        "GetRecentlyPlayedGames",
        "GetSingleGamePlaytime",
        "GetOwnedGames",
        "GetSteamLevel",
        "GetBadges",
        "GetCommunityBadgeProgress",
    ] {
        fixture(
            &server,
            &format!("/IPlayerService/{endpoint}/v1/"),
            json!({"response": {}}),
            true,
        )
        .await;
    }
    let client = client(&server);
    let player = client.player();
    player.get_recently_played_games(ID, 0).await.unwrap();
    player.get_single_game_playtime(ID, 480).await.unwrap();
    let hidden = player
        .get_owned_games(
            ID,
            OwnedGamesOptions {
                include_appinfo: true,
                include_played_free_games: false,
                appids_filter: &[480, 570],
            },
        )
        .await
        .unwrap();
    assert_eq!(hidden.as_json(), &json!({"response": {}}));
    player.get_steam_level(ID).await.unwrap();
    player.get_badges(ID).await.unwrap();
    player
        .get_community_badge_progress(ID, i32::MIN)
        .await
        .unwrap();
    let requests = server.received_requests().await.unwrap();
    for request in &requests {
        let value = input(request);
        assert_eq!(value["steamid"], ID.to_string());
        match request.url.path().split('/').nth(2).unwrap() {
            "GetRecentlyPlayedGames" => assert_eq!(value["count"], 0),
            "GetSingleGamePlaytime" => assert_eq!(value["appid"], 480),
            "GetOwnedGames" => {
                assert_eq!(value["appids_filter"], json!([480, 570]));
                assert_eq!(value["include_appinfo"], true);
                assert_eq!(value["include_played_free_games"], false);
            }
            "GetCommunityBadgeProgress" => assert_eq!(value["badgeid"], i32::MIN),
            _ => assert_eq!(value.as_object().unwrap().len(), 1),
        }
    }
}

#[tokio::test]
async fn store_service_encodes_every_filter_and_keeps_steam_pagination_metadata() {
    let server = MockServer::start().await;
    let original = json!({"response": {"apps": [{"appid": 480, "name": "Spacewar", "last_modified": 123, "price_change_number": 456}], "last_appid": 480, "have_more_results": true}});
    fixture(
        &server,
        "/IStoreService/GetAppList/v1/",
        original.clone(),
        true,
    )
    .await;
    let result = client(&server)
        .store()
        .get_app_list(AppListOptions {
            if_modified_since: Some(0),
            have_description_language: Some("schinese"),
            include_games: Some(false),
            include_dlc: Some(true),
            include_software: Some(true),
            include_videos: Some(false),
            include_hardware: Some(true),
            last_appid: Some(479),
            max_results: Some(50_000),
        })
        .await
        .unwrap();
    assert_eq!(result.as_json(), &original);
    let request = &server.received_requests().await.unwrap()[0];
    assert_eq!(
        input(request),
        json!({
            "if_modified_since": 0, "have_description_language": "schinese",
            "include_games": false, "include_dlc": true, "include_software": true,
            "include_videos": false, "include_hardware": true,
            "last_appid": 479, "max_results": 50_000
        })
    );
}

#[tokio::test]
async fn review_service_encodes_all_filters_as_numeric_json_and_decodes_typed_page() {
    let server = MockServer::start().await;
    let mut body = page(json!([review()]));
    body["response"]["query_summary"]["review_score"] = json!(8);
    body["response"]["query_summary"]["review_score_desc"] = json!("Very Positive");
    body["response"]["future_page"] = json!(17);
    fixture(
        &server,
        "/IUserReviewsService/GetAppReviews/v1/",
        body,
        false,
    )
    .await;
    let client = SteamClient::builder()
        .base_url(server.uri())
        .build()
        .unwrap();
    let options = AppReviewsOptions {
        filter: Some(ReviewFilter::Updated),
        languages: &["english", "schinese"],
        day_range: Some(0),
        cursor: Some("AoJ+/=&opaque 中文"),
        review_type: Some(ReviewType::Negative),
        purchase_type: Some(PurchaseType::All),
        num_per_page: Some(100),
        filter_offtopic_activity: Some(false),
        display_language: Some("english"),
        use_review_quality: Some(false),
        date_range_start: Some(10),
        date_range_end: Some(20),
        date_range_exclude: Some(true),
        playtime_min_hours: Some(0),
        playtime_max_hours: Some(99),
        primarily_steam_deck: Some(true),
        hardware_os: Some("Linux&="),
        hardware_cpu: Some("CPU"),
        hardware_gpu: Some("GPU"),
        hardware_device_type: Some("Handheld"),
    };
    let response = client
        .reviews()
        .get_app_reviews(480, options)
        .await
        .unwrap();
    let review = &response.reviews[0];
    assert_eq!(review.recommendationid, u64::MAX);
    assert_eq!(review.author.steamid.raw(), ID);
    assert_eq!(review.weighted_vote_score, 0.75);
    assert_eq!(review.author.playtime_last_two_weeks, None);
    assert_eq!(review.developer_response, None);
    assert_eq!(review.author.extra["new_author_field"], 7);
    assert_eq!(review.extra["new_review_field"]["x"], 1);
    assert_eq!(response.extra["future_page"], 17);
    assert_eq!(response.query_summary.review_score, Some(8));
    assert_eq!(response.query_summary.total_reviews, None);
    assert_eq!(response.cursor, options.cursor.unwrap());
    let request = &server.received_requests().await.unwrap()[0];
    assert!(!request.headers.contains_key("x-webapi-key"));
    assert_eq!(
        input(request),
        json!({
            "appid": 480, "filter": 2, "languages": ["english", "schinese"],
            "day_range": 0, "cursor": "AoJ+/=&opaque 中文", "review_type": 2,
            "purchase_type": 1, "num_per_page": 100, "filter_offtopic_activity": false,
            "display_language": "english", "use_review_quality": false,
            "date_range_start": 10, "date_range_end": 20, "date_range_exclude": true,
            "playtime_min_hours": 0, "playtime_max_hours": 99, "primarily_steam_deck": true,
            "hardware_os": "Linux&=", "hardware_cpu": "CPU", "hardware_gpu": "GPU",
            "hardware_device_type": "Handheld"
        })
    );
}

#[tokio::test]
async fn default_service_options_omit_optional_fields_and_preserve_server_defaults() {
    let server = MockServer::start().await;
    fixture(
        &server,
        "/IStoreService/GetAppList/v1/",
        json!({"response": {}}),
        true,
    )
    .await;
    fixture(
        &server,
        "/IPlayerService/GetOwnedGames/v1/",
        json!({"response": {}}),
        true,
    )
    .await;
    fixture(
        &server,
        "/IUserReviewsService/GetAppReviews/v1/",
        page(json!([])),
        true,
    )
    .await;
    let client = client(&server);
    client
        .store()
        .get_app_list(AppListOptions::default())
        .await
        .unwrap();
    client
        .player()
        .get_owned_games(ID, OwnedGamesOptions::default())
        .await
        .unwrap();
    let response = client
        .reviews()
        .get_app_reviews(480, AppReviewsOptions::default())
        .await
        .unwrap();
    assert!(response.reviews.is_empty());
    assert_eq!(response.query_summary.review_score, None);
    let requests = server.received_requests().await.unwrap();
    assert_eq!(input(&requests[0]), json!({}));
    assert_eq!(
        input(&requests[1]),
        json!({"steamid": ID.to_string(), "include_appinfo": false, "include_played_free_games": false})
    );
    assert_eq!(input(&requests[2]), json!({"appid": 480}));
}

#[tokio::test]
async fn publisher_news_uses_its_distinct_v2_path_and_enforces_key_kind() {
    let server = MockServer::start().await;
    fixture(
        &server,
        "/ISteamNews/GetNewsForAppAuthed/v2/",
        json!({"appnews": {"private": true}}),
        true,
    )
    .await;
    let user = client(&server);
    assert!(matches!(
        user.news()
            .get_news_for_app_authed(480, NewsOptions::default())
            .await,
        Err(Error::MissingApiKey("publisher"))
    ));
    let publisher = SteamClient::builder()
        .base_url(server.uri())
        .api_key(ApiKey::publisher("test-key").unwrap())
        .build()
        .unwrap();
    assert_eq!(
        publisher
            .news()
            .get_news_for_app_authed(480, NewsOptions::default())
            .await
            .unwrap()
            .as_json()["appnews"]["private"],
        true
    );
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let pairs = requests[0]
        .url
        .query_pairs()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs["appid"], "480");
}

#[tokio::test]
async fn opaque_services_still_reject_nested_steam_business_errors() {
    let server = MockServer::start().await;
    for endpoint in [
        "/ISteamWebAPIUtil/GetServerInfo/v1/",
        "/IPlayerService/GetOwnedGames/v1/",
        "/IStoreService/GetAppList/v1/",
        "/ISteamNews/GetNewsForApp/v2/",
    ] {
        fixture(
            &server,
            endpoint,
            json!({"response": {"error": {"errorcode": 15, "errordesc": "private-server-text"}}}),
            true,
        )
        .await;
    }
    let client = client(&server);
    let errors = [
        client.web_api_util().get_server_info().await.unwrap_err(),
        client
            .player()
            .get_owned_games(ID, OwnedGamesOptions::default())
            .await
            .unwrap_err(),
        client
            .store()
            .get_app_list(AppListOptions::default())
            .await
            .unwrap_err(),
        client
            .news()
            .get_news_for_app(480, NewsOptions::default())
            .await
            .unwrap_err(),
    ];
    for error in errors {
        assert!(matches!(error, Error::Api { code: Some(15), .. }));
        assert!(!format!("{error:?} {error}").contains("private-server-text"));
    }
}

#[tokio::test]
async fn reviews_fail_on_http_errors_or_malformed_contracts_without_inventing_success() {
    let server = MockServer::start().await;
    let client = SteamClient::builder()
        .base_url(server.uri())
        .retry_policy(RetryPolicy {
            max_retries: 0,
            initial_backoff: Duration::ZERO,
            max_backoff: Duration::ZERO,
        })
        .build()
        .unwrap();
    Mock::given(path("/IUserReviewsService/GetAppReviews/v1/"))
        .respond_with(ResponseTemplate::new(429))
        .expect(1)
        .mount(&server)
        .await;
    assert!(matches!(
        client
            .reviews()
            .get_app_reviews(480, AppReviewsOptions::default())
            .await,
        Err(Error::Http { status: 429, .. })
    ));
    server.reset().await;
    for body in [
        json!({"response": {}}),
        json!({"response": {"result": 15}}),
        json!({"response": {"query_summary": {"num_reviews": 0}, "total_matching": 0}}),
    ] {
        fixture(
            &server,
            "/IUserReviewsService/GetAppReviews/v1/",
            body,
            false,
        )
        .await;
        assert!(
            client
                .reviews()
                .get_app_reviews(480, AppReviewsOptions::default())
                .await
                .is_err()
        );
        server.reset().await;
    }
    let mut malformed = review();
    malformed["recommendationid"] = json!("18446744073709551616");
    fixture(
        &server,
        "/IUserReviewsService/GetAppReviews/v1/",
        page(json!([malformed])),
        false,
    )
    .await;
    assert!(matches!(
        client
            .reviews()
            .get_app_reviews(480, AppReviewsOptions::default())
            .await,
        Err(Error::Decode { .. })
    ));
}

#[tokio::test]
async fn invalid_options_and_missing_keys_fail_before_network() {
    let server = MockServer::start().await;
    let client = client(&server);
    assert!(
        client
            .player()
            .get_recently_played_games(0_u64, 0)
            .await
            .is_err()
    );
    assert!(
        client
            .player()
            .get_single_game_playtime(ID, 0)
            .await
            .is_err()
    );
    assert!(
        client
            .player()
            .get_owned_games(
                ID,
                OwnedGamesOptions {
                    appids_filter: &[0],
                    ..Default::default()
                }
            )
            .await
            .is_err()
    );
    assert!(
        client
            .news()
            .get_news_for_app(0, NewsOptions::default())
            .await
            .is_err()
    );
    assert!(
        client
            .news()
            .get_news_for_app(
                480,
                NewsOptions {
                    feeds: Some("bad\0feed"),
                    ..Default::default()
                }
            )
            .await
            .is_err()
    );
    for limit in [0, 50_001] {
        assert!(
            client
                .store()
                .get_app_list(AppListOptions {
                    max_results: Some(limit),
                    ..Default::default()
                })
                .await
                .is_err()
        );
    }
    for options in [
        AppReviewsOptions {
            day_range: Some(366),
            ..Default::default()
        },
        AppReviewsOptions {
            num_per_page: Some(0),
            ..Default::default()
        },
        AppReviewsOptions {
            num_per_page: Some(101),
            ..Default::default()
        },
        AppReviewsOptions {
            date_range_start: Some(2),
            date_range_end: Some(1),
            ..Default::default()
        },
        AppReviewsOptions {
            playtime_min_hours: Some(2),
            playtime_max_hours: Some(1),
            ..Default::default()
        },
        AppReviewsOptions {
            cursor: Some("\0"),
            ..Default::default()
        },
        AppReviewsOptions {
            languages: &[""],
            ..Default::default()
        },
    ] {
        assert!(
            client
                .reviews()
                .get_app_reviews(480, options)
                .await
                .is_err()
        );
    }
    let anonymous = SteamClient::builder()
        .base_url(server.uri())
        .build()
        .unwrap();
    assert!(matches!(
        anonymous.player().get_steam_level(ID).await,
        Err(Error::MissingApiKey(_))
    ));
    assert!(matches!(
        anonymous
            .store()
            .get_app_list(AppListOptions::default())
            .await,
        Err(Error::MissingApiKey(_))
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[test]
fn every_review_enum_has_its_documented_integer_representation() {
    for (value, expected) in [
        (ReviewFilter::Helpful, 0),
        (ReviewFilter::Recent, 1),
        (ReviewFilter::Updated, 2),
        (ReviewFilter::Funny, 3),
    ] {
        assert_eq!(serde_json::to_value(value).unwrap(), expected);
    }
    for (value, expected) in [
        (ReviewType::All, 0),
        (ReviewType::Positive, 1),
        (ReviewType::Negative, 2),
    ] {
        assert_eq!(serde_json::to_value(value).unwrap(), expected);
    }
    for (value, expected) in [
        (PurchaseType::Steam, 0),
        (PurchaseType::All, 1),
        (PurchaseType::NonSteamPurchase, 2),
    ] {
        assert_eq!(serde_json::to_value(value).unwrap(), expected);
    }
}
