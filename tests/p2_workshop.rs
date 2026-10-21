// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Wire, authorization and replay contracts for all nineteen Workshop methods.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use steam_api_web::{
    ApiKey, Error, RetryPolicy, SteamClient, SteamId,
    published_file::{QueryFilesOptions, QueryFilesPagination, QueryFilesRequest},
    published_file_moderation::{
        ApprovalRequestsOptions, ProcessApprovalRequest, PublishedFileRevision,
    },
    published_item_search::{PublishedItemSearchOptions, PublishedItemSearchRequest},
    user_auth::OpaqueResponse,
    workshop::{ItemDailyRevenueRequest, ItemPaymentRulesRequest},
};
use wiremock::{
    Mock, MockServer, Request, Respond, ResponseTemplate,
    matchers::{header, method, path},
};

const PRIVATE: &str = "private&=+工作坊";
const CURSOR: &str = "cursor+/=&%25工作坊";
const METHODS: [(&str, &str, &str); 19] = [
    ("IPublishedFileService", "Delete", "GET"),
    ("IPublishedFileService", "QueryFiles", "GET"),
    ("IPublishedFileService", "SetDeveloperMetadata", "POST"),
    ("IPublishedFileService", "UpdateAppUGCBan", "POST"),
    ("IPublishedFileService", "UpdateBanStatus", "POST"),
    ("IPublishedFileService", "UpdateIncompatibleStatus", "POST"),
    ("IPublishedFileService", "UpdateTags", "POST"),
    (
        "IPublishedFileModerationService",
        "GetAllApprovalRequests",
        "GET",
    ),
    (
        "IPublishedFileModerationService",
        "ProcessApprovalRequest",
        "POST",
    ),
    (
        "ISteamPublishedItemSearch",
        "RankedByPublicationOrder",
        "POST",
    ),
    ("ISteamPublishedItemSearch", "RankedByTrend", "POST"),
    ("ISteamPublishedItemSearch", "RankedByVote", "POST"),
    ("ISteamPublishedItemSearch", "ResultSetSummary", "POST"),
    ("ISteamPublishedItemVoting", "ItemVoteSummary", "POST"),
    ("ISteamPublishedItemVoting", "UserVoteSummary", "POST"),
    ("IWorkshopService", "SetItemPaymentRules", "POST"),
    ("IWorkshopService", "GetFinalizedContributors", "GET"),
    ("IWorkshopService", "GetItemDailyRevenue", "GET"),
    ("IWorkshopService", "PopulateItemDescriptions", "POST"),
];

fn client(server: &MockServer, publisher: Option<bool>) -> SteamClient {
    let mut builder = SteamClient::builder()
        .base_url(server.uri())
        .retry_policy(RetryPolicy {
            max_retries: 2,
            initial_backoff: Duration::ZERO,
            max_backoff: Duration::ZERO,
        });
    if let Some(publisher) = publisher {
        builder = builder.api_key(if publisher {
            ApiKey::publisher("wire-key").unwrap()
        } else {
            ApiKey::user("wire-key").unwrap()
        });
    }
    builder.build().unwrap()
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

fn query(pagination: QueryFilesPagination<'_>) -> QueryFilesRequest<'_> {
    QueryFilesRequest {
        query_type: 0,
        creator_appid: 480,
        appid: 570,
        pagination,
        options: QueryFilesOptions::default(),
    }
}

fn search_options() -> PublishedItemSearchOptions<'static> {
    PublishedItemSearchOptions {
        tags: &["tag&=+工作坊", "second"],
        user_tags: &["mine"],
        hasappadminaccess: Some(false),
        file_type: Some(18),
    }
}

fn search() -> PublishedItemSearchRequest<'static> {
    PublishedItemSearchRequest {
        steamid: SteamId(u64::MAX),
        appid: 480,
        startidx: 7,
        count: 23,
        options: search_options(),
    }
}

async fn call(client: &SteamClient, name: &str) -> steam_api_web::Result<OpaqueResponse> {
    let raw = json!({"opaque_key": PRIVATE, "unsigned": u64::MAX, "extension": [false, 17]});
    match name {
        "Delete" => client.published_file().delete(480, u64::MAX).await,
        "QueryFiles" => {
            client
                .published_file()
                .query_files(query(QueryFilesPagination::Cursor(CURSOR)))
                .await
        }
        "SetDeveloperMetadata" => {
            client
                .published_file()
                .set_developer_metadata(480, u64::MAX, "")
                .await
        }
        "UpdateAppUGCBan" => {
            client
                .published_file()
                .update_app_ugc_ban(480, u64::MAX.into(), 0, Some(PRIVATE))
                .await
        }
        "UpdateBanStatus" => {
            client
                .published_file()
                .update_ban_status(480, u64::MAX, false, "")
                .await
        }
        "UpdateIncompatibleStatus" => {
            client
                .published_file()
                .update_incompatible_status(480, u64::MAX, true)
                .await
        }
        "UpdateTags" => {
            client
                .published_file()
                .update_tags(480, u64::MAX, &["tag&=+工作坊"], &["old"])
                .await
        }
        "GetAllApprovalRequests" => {
            client
                .published_file_moderation()
                .get_all_approval_requests(
                    480,
                    ApprovalRequestsOptions {
                        revision: Some(PublishedFileRevision::RejectedSnapshot),
                        start: Some(7),
                        count: Some(1000),
                    },
                )
                .await
        }
        "ProcessApprovalRequest" => {
            client
                .published_file_moderation()
                .process_approval_request(ProcessApprovalRequest {
                    appid: 480,
                    publishedfileid: u64::MAX,
                    change_number: u64::MAX,
                    approved: false,
                })
                .await
        }
        "RankedByPublicationOrder" => {
            client
                .published_item_search()
                .ranked_by_publication_order(search())
                .await
        }
        "RankedByTrend" => {
            client
                .published_item_search()
                .ranked_by_trend(search(), Some(7))
                .await
        }
        "RankedByVote" => {
            client
                .published_item_search()
                .ranked_by_vote(search())
                .await
        }
        "ResultSetSummary" => {
            client
                .published_item_search()
                .result_set_summary(u64::MAX.into(), u64::MAX, search_options())
                .await
        }
        "ItemVoteSummary" => {
            client
                .published_item_voting()
                .item_vote_summary(u64::MAX.into(), 480, &[u64::MAX, 42])
                .await
        }
        "UserVoteSummary" => {
            client
                .published_item_voting()
                .user_vote_summary(u64::MAX.into(), &[u64::MAX, 42])
                .await
        }
        "SetItemPaymentRules" => {
            client
                .workshop()
                .set_item_payment_rules(ItemPaymentRulesRequest {
                    appid: 480,
                    gameitemid: 7,
                    associated_workshop_files: &raw,
                    partner_accounts: &raw,
                    validate_only: Some(true),
                    make_workshop_files_subscribable: false,
                })
                .await
        }
        "GetFinalizedContributors" => client.workshop().get_finalized_contributors(480, 7).await,
        "GetItemDailyRevenue" => {
            client
                .workshop()
                .get_item_daily_revenue(ItemDailyRevenueRequest {
                    appid: 480,
                    item_id: 7,
                    date_start: 86400,
                    date_end: 172800,
                })
                .await
        }
        "PopulateItemDescriptions" => {
            client
                .workshop()
                .populate_item_descriptions(480, &raw)
                .await
        }
        _ => panic!("unknown test method"),
    }
}

#[tokio::test]
async fn all_nineteen_methods_use_exact_paths_credentials_and_preserve_opaque_results() {
    let server = MockServer::start().await;
    let client = client(&server, Some(true));
    for (interface, name, verb) in METHODS {
        Mock::given(method(verb)).and(path(format!("/{interface}/{name}/v1/"))).and(header("x-webapi-key", "wire-key"))
            .respond_with(ResponseTemplate::new(200).insert_header("x-eresult", "1").set_body_json(json!({"response":{"private":PRIVATE, "extension":{"id":u64::MAX}, "next_cursor":CURSOR}})))
            .expect(1).mount(&server).await;
        let response = call(&client, name).await.unwrap();
        assert_eq!(
            response.as_json()["response"]["extension"]["id"].as_u64(),
            Some(u64::MAX),
            "{name}"
        );
        assert_eq!(response.as_json()["response"]["next_cursor"], CURSOR);
        assert!(!format!("{response:?}").contains(PRIVATE));
    }
    for request in server.received_requests().await.unwrap() {
        let params = form(&request);
        assert_eq!(params.get("format").unwrap(), "json");
        assert!(!params.contains_key("key"));
        if request.method.as_str() == "POST" {
            assert!(request.url.query().is_none());
            assert_eq!(
                request.headers.get("content-type").unwrap(),
                "application/x-www-form-urlencoded"
            );
        }
        let name = request.url.path().split('/').nth(2).unwrap();
        let service = request
            .url
            .path()
            .split('/')
            .nth(1)
            .unwrap()
            .ends_with("Service");
        if !service {
            assert!(!params.contains_key("input_json"));
            continue;
        }
        assert_eq!(params.len(), 2);
        let payload: Value = serde_json::from_str(params.get("input_json").unwrap()).unwrap();
        let raw = json!({"opaque_key":PRIVATE, "unsigned":u64::MAX, "extension":[false,17]});
        let expected = match name {
            "Delete" => json!({"publishedfileid":u64::MAX,"appid":480}),
            "QueryFiles" => {
                assert_eq!(payload["cursor"], CURSOR);
                assert_eq!(payload["page"], 1);
                assert_eq!(payload["creator_appid"], 480);
                assert_eq!(payload["appid"], 570);
                assert_eq!(payload["filetype"], 0);
                assert_eq!(payload["child_publishedfileid"], 0);
                assert_eq!(payload["search_text"], "");
                for optional in [
                    "numperpage",
                    "match_all_tags",
                    "cache_max_age_seconds",
                    "language",
                    "required_kv_tags",
                    "return_metadata",
                ] {
                    assert!(payload.get(optional).is_none());
                }
                continue;
            }
            "SetDeveloperMetadata" => json!({"publishedfileid":u64::MAX,"appid":480,"metadata":""}),
            "UpdateAppUGCBan" => {
                json!({"steamid":u64::MAX.to_string(),"appid":480,"expiration_time":0,"reason":PRIVATE})
            }
            "UpdateBanStatus" => {
                json!({"publishedfileid":u64::MAX,"appid":480,"banned":false,"reason":""})
            }
            "UpdateIncompatibleStatus" => {
                json!({"publishedfileid":u64::MAX,"appid":480,"incompatible":true})
            }
            "UpdateTags" => {
                json!({"publishedfileid":u64::MAX,"appid":480,"add_tags":["tag&=+工作坊"],"remove_tags":["old"]})
            }
            "GetAllApprovalRequests" => json!({"appid":480,"revision":4,"start":7,"count":1000}),
            "ProcessApprovalRequest" => {
                json!({"appid":480,"publishedfileid":u64::MAX,"revision":2,"change_number":u64::MAX,"approved":false})
            }
            "SetItemPaymentRules" => {
                json!({"appid":480,"gameitemid":7,"associated_workshop_files":raw,"partner_accounts":raw,"validate_only":true,"make_workshop_files_subscribable":false})
            }
            "GetFinalizedContributors" => json!({"appid":480,"gameitemid":7}),
            "GetItemDailyRevenue" => {
                json!({"appid":480,"item_id":7,"date_start":86400,"date_end":172800})
            }
            "PopulateItemDescriptions" => json!({"appid":480,"languages":raw}),
            _ => panic!("unexpected service method"),
        };
        assert_eq!(payload, expected, "{name}");
    }
}

#[tokio::test]
async fn search_and_voting_derive_indexed_counts_and_preserve_case_and_full_width_ids() {
    let server = MockServer::start().await;
    let client = client(&server, Some(true));
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response":{}})))
        .expect(6)
        .mount(&server)
        .await;
    for name in [
        "RankedByPublicationOrder",
        "RankedByTrend",
        "RankedByVote",
        "ResultSetSummary",
        "ItemVoteSummary",
        "UserVoteSummary",
    ] {
        call(&client, name).await.unwrap();
    }
    for request in server.received_requests().await.unwrap() {
        let params = form(&request);
        let name = request.url.path().split('/').nth(2).unwrap();
        assert_eq!(params["steamid"], u64::MAX.to_string());
        if name.ends_with("VoteSummary") {
            assert_eq!(params["count"], "2");
            assert_eq!(params["publishedfileid[0]"], u64::MAX.to_string());
            assert_eq!(params["publishedfileid[1]"], "42");
            assert_eq!(params.contains_key("appid"), name == "ItemVoteSummary");
        } else {
            assert_eq!(params["tagcount"], "2");
            assert_eq!(params["usertagcount"], "1");
            assert_eq!(params["tag[0]"], "tag&=+工作坊");
            assert_eq!(params["tag[1]"], "second");
            assert_eq!(params["usertag[0]"], "mine");
            assert_eq!(params["fileType"], "18");
            assert_eq!(params["hasappadminaccess"], "false");
            assert!(!params.contains_key("filetype"));
            if name == "ResultSetSummary" {
                assert_eq!(params["appid"], u64::MAX.to_string());
                assert!(!params.contains_key("startidx"));
                assert!(!params.contains_key("count"));
            } else {
                assert_eq!(params["appid"], "480");
                assert_eq!(params["startidx"], "7");
                assert_eq!(params["count"], "23");
            }
            assert_eq!(
                params.get("days").map(String::as_str),
                (name == "RankedByTrend").then_some("7")
            );
        }
    }
}

#[tokio::test]
async fn all_publishers_reject_user_credentials_and_all_methods_reject_anonymous_clients() {
    let server = MockServer::start().await;
    let user = client(&server, Some(false));
    let anonymous = client(&server, None);
    for (_, name, _) in METHODS {
        if name != "QueryFiles" {
            assert!(
                matches!(
                    call(&user, name).await,
                    Err(Error::MissingApiKey("publisher"))
                ),
                "{name}"
            );
        }
        assert!(
            matches!(call(&anonymous, name).await, Err(Error::MissingApiKey(_))),
            "{name}"
        );
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn get_delete_and_every_post_are_never_automatically_replayed() {
    let server = MockServer::start().await;
    let client = client(&server, Some(true));
    for (interface, name, verb) in METHODS {
        if verb == "GET" && name != "Delete" {
            continue;
        }
        Mock::given(method(verb))
            .and(path(format!("/{interface}/{name}/v1/")))
            .respond_with(ResponseTemplate::new(503))
            .expect(1)
            .mount(&server)
            .await;
        assert!(
            matches!(
                call(&client, name).await,
                Err(Error::Http { status: 503, .. })
            ),
            "{name}"
        );
    }
    assert_eq!(client.metrics().attempts, 15);
    assert_eq!(client.metrics().retries, 0);
}

struct ReadTransient(AtomicUsize);
impl Respond for ReadTransient {
    fn respond(&self, _: &Request) -> ResponseTemplate {
        if self.0.fetch_add(1, Ordering::SeqCst) < 2 {
            ResponseTemplate::new(503)
        } else {
            ResponseTemplate::new(200).set_body_json(json!({"response":{"kept":true}}))
        }
    }
}

#[tokio::test]
async fn all_four_safe_gets_can_retry_transient_failures_with_a_finite_budget() {
    let server = MockServer::start().await;
    let client = client(&server, Some(true));
    for (interface, name, verb) in METHODS {
        if verb != "GET" || name == "Delete" {
            continue;
        }
        Mock::given(method(verb))
            .and(path(format!("/{interface}/{name}/v1/")))
            .respond_with(ReadTransient(AtomicUsize::new(0)))
            .expect(3)
            .mount(&server)
            .await;
        assert_eq!(
            call(&client, name).await.unwrap().as_json()["response"]["kept"],
            true
        );
    }
    assert_eq!(client.metrics().attempts, 12);
    assert_eq!(client.metrics().retries, 8);
}

#[tokio::test]
async fn query_pagination_and_optional_controls_are_lossless() {
    let server = MockServer::start().await;
    let client = client(&server, Some(false));
    Mock::given(method("GET"))
        .and(path("/IPublishedFileService/QueryFiles/v1/"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"response":{"next_cursor":CURSOR}})),
        )
        .expect(2)
        .mount(&server)
        .await;
    let message = json!([{"opaque_key":PRIVATE,"id":u64::MAX}]);
    let mut request = query(QueryFilesPagination::Page(1000));
    request.query_type = 3;
    request.options = QueryFilesOptions {
        numperpage: Some(1000),
        requiredtags: "tag expression",
        excludedtags: "excluded",
        match_all_tags: Some(false),
        required_flags: "required",
        omitted_flags: "omitted",
        search_text: PRIVATE,
        filetype: 18,
        child_publishedfileid: u64::MAX,
        days: 7,
        include_recent_votes_only: true,
        cache_max_age_seconds: Some(0),
        language: Some(-1),
        required_kv_tags: Some(&message),
        totalonly: false,
        ids_only: true,
        return_vote_data: true,
        return_tags: true,
        return_kv_tags: true,
        return_previews: true,
        return_children: true,
        return_short_description: true,
        return_for_sale_data: true,
        return_metadata: Some(false),
        return_playtime_stats: 30,
    };
    let first = client.published_file().query_files(request).await.unwrap();
    request.pagination =
        QueryFilesPagination::Cursor(first.as_json()["response"]["next_cursor"].as_str().unwrap());
    client.published_file().query_files(request).await.unwrap();
    let requests = server.received_requests().await.unwrap();
    let first: Value = serde_json::from_str(&form(&requests[0])["input_json"]).unwrap();
    let second: Value = serde_json::from_str(&form(&requests[1])["input_json"]).unwrap();
    assert_eq!(first["page"], 1000);
    assert!(first.get("cursor").is_none());
    assert_eq!(second["page"], 1);
    assert_eq!(second["cursor"], CURSOR);
    assert_eq!(first["child_publishedfileid"].as_u64(), Some(u64::MAX));
    assert_eq!(first["required_kv_tags"], message);
    assert_eq!(first["language"], -1);
    assert_eq!(first["return_metadata"], false);
    assert_eq!(first["search_text"], PRIVATE);
}

#[tokio::test]
async fn invalid_pagination_scopes_windows_ids_and_message_shapes_fail_before_io() {
    let server = MockServer::start().await;
    let client = client(&server, Some(true));
    for page in [0, 1001] {
        assert!(
            client
                .published_file()
                .query_files(query(QueryFilesPagination::Page(page)))
                .await
                .is_err()
        );
    }
    for cursor in ["", "bad\0cursor"] {
        assert!(
            client
                .published_file()
                .query_files(query(QueryFilesPagination::Cursor(cursor)))
                .await
                .is_err()
        );
    }
    let mut request = query(QueryFilesPagination::Page(1));
    request.options.numperpage = Some(0);
    assert!(client.published_file().query_files(request).await.is_err());
    request.options.numperpage = None;
    request.query_type = 3;
    for days in [0, 8] {
        request.options.days = days;
        assert!(client.published_file().query_files(request).await.is_err());
    }
    for count in [0, 1001] {
        assert!(
            client
                .published_file_moderation()
                .get_all_approval_requests(
                    480,
                    ApprovalRequestsOptions {
                        count: Some(count),
                        ..Default::default()
                    }
                )
                .await
                .is_err()
        );
    }
    assert!(client.published_file().delete(0, 42).await.is_err());
    assert!(client.published_file().delete(480, 0).await.is_err());
    assert!(
        client
            .published_file()
            .set_developer_metadata(480, 42, "bad\0metadata")
            .await
            .is_err()
    );
    assert!(
        client
            .published_file()
            .update_tags(480, 42, &[""], &[])
            .await
            .is_err()
    );
    assert!(
        client
            .published_file()
            .update_app_ugc_ban(480, 0.into(), 0, None)
            .await
            .is_err()
    );
    assert!(
        client
            .published_item_search()
            .ranked_by_trend(search(), Some(8))
            .await
            .is_err()
    );
    let mut ranked = search();
    ranked.count = 0;
    assert!(
        client
            .published_item_search()
            .ranked_by_vote(ranked)
            .await
            .is_err()
    );
    assert!(
        client
            .published_item_voting()
            .item_vote_summary(1.into(), 480, &[])
            .await
            .is_err()
    );
    assert!(
        client
            .published_item_voting()
            .user_vote_summary(1.into(), &[0])
            .await
            .is_err()
    );
    for raw in [
        Value::Null,
        json!("not a message"),
        json!([{"valid":true},7]),
    ] {
        assert!(
            client
                .workshop()
                .populate_item_descriptions(480, &raw)
                .await
                .is_err()
        );
        let valid = json!({});
        assert!(
            client
                .workshop()
                .set_item_payment_rules(ItemPaymentRulesRequest {
                    appid: 480,
                    gameitemid: 7,
                    associated_workshop_files: &raw,
                    partner_accounts: &valid,
                    validate_only: None,
                    make_workshop_files_subscribable: false
                })
                .await
                .is_err()
        );
        let mut query = query(QueryFilesPagination::Page(1));
        query.options.required_kv_tags = Some(&raw);
        assert!(client.published_file().query_files(query).await.is_err());
    }
    assert!(
        client
            .workshop()
            .get_item_daily_revenue(ItemDailyRevenueRequest {
                appid: 480,
                item_id: 7,
                date_start: 86400,
                date_end: 86400
            })
            .await
            .is_err()
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn defaults_are_omitted_and_unknown_messages_are_explicit_without_leaking_debug() {
    let server = MockServer::start().await;
    let client = client(&server, Some(true));
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response":{}})))
        .expect(3)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response":{}})))
        .expect(1)
        .mount(&server)
        .await;
    client
        .published_file_moderation()
        .get_all_approval_requests(480, ApprovalRequestsOptions::default())
        .await
        .unwrap();
    client
        .published_item_search()
        .ranked_by_trend(
            PublishedItemSearchRequest {
                options: PublishedItemSearchOptions::default(),
                ..search()
            },
            None,
        )
        .await
        .unwrap();
    client
        .published_file()
        .update_tags(480, 42, &[], &[])
        .await
        .unwrap();
    let raw = json!([{"opaque":PRIVATE}]);
    let request = ItemPaymentRulesRequest {
        appid: 480,
        gameitemid: 7,
        associated_workshop_files: &raw,
        partner_accounts: &raw,
        validate_only: None,
        make_workshop_files_subscribable: true,
    };
    assert!(!format!("{request:?}").contains(PRIVATE));
    client
        .workshop()
        .set_item_payment_rules(request)
        .await
        .unwrap();
    for request in server.received_requests().await.unwrap() {
        let params = form(&request);
        match request.url.path().split('/').nth(2).unwrap() {
            "GetAllApprovalRequests" => {
                let payload: Value = serde_json::from_str(&params["input_json"]).unwrap();
                assert_eq!(payload, json!({"appid":480}));
            }
            "RankedByTrend" => {
                assert_eq!(params["tagcount"], "0");
                assert_eq!(params["usertagcount"], "0");
                for name in [
                    "days",
                    "fileType",
                    "hasappadminaccess",
                    "tag[0]",
                    "usertag[0]",
                ] {
                    assert!(!params.contains_key(name));
                }
            }
            "UpdateTags" => {
                let payload: Value = serde_json::from_str(&params["input_json"]).unwrap();
                assert_eq!(payload, json!({"appid":480,"publishedfileid":42}));
            }
            "SetItemPaymentRules" => {
                let payload: Value = serde_json::from_str(&params["input_json"]).unwrap();
                assert!(payload.get("validate_only").is_none());
                assert_eq!(payload["partner_accounts"], raw);
            }
            _ => panic!("unexpected method"),
        }
    }
}

#[tokio::test]
async fn empty_or_failed_mutations_never_invent_success_and_error_text_stays_redacted() {
    let server = MockServer::start().await;
    let client = client(&server, Some(true));
    Mock::given(method("GET"))
        .and(path("/IPublishedFileService/Delete/v1/"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"response":{"error":{"errorcode":2,"description":PRIVATE}}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let error = client.published_file().delete(480, 42).await.unwrap_err();
    assert!(matches!(error, Error::Api { code: Some(2), .. }));
    assert!(!format!("{error:?} {error}").contains(PRIVATE));
    Mock::given(method("POST"))
        .and(path("/IWorkshopService/SetItemPaymentRules/v1/"))
        .respond_with(ResponseTemplate::new(201))
        .expect(1)
        .mount(&server)
        .await;
    let error = call(&client, "SetItemPaymentRules").await.unwrap_err();
    assert!(matches!(error, Error::Decode { .. }));
    Mock::given(method("POST"))
        .and(path(
            "/IPublishedFileModerationService/ProcessApprovalRequest/v1/",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-eresult", "2")
                .set_body_json(json!({"response":{}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    assert!(matches!(
        call(&client, "ProcessApprovalRequest").await,
        Err(Error::Api { code: Some(2), .. })
    ));
    assert_eq!(client.metrics().attempts, 3);
    assert_eq!(client.metrics().retries, 0);
}
