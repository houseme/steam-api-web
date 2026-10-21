// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Sandbox and production contracts use only local mock billing servers.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::{Value, json};
use std::{collections::BTreeMap, time::Duration};
use steam_api_web::{
    ApiKey, Error, Result, RetryPolicy, SteamClient,
    micro_txn::{
        AdjustAgreementRequest, AgreementRequest, InitTxnRequest, MicroTxn, MicroTxnBundle,
        MicroTxnEnvironment, MicroTxnItem, MicroTxnReportRequest, MicroTxnReportType,
        MicroTxnUserRequest, MicroTxnUserSession, OrderRequest, ProcessAgreementRequest,
        PurchasingUserRequest, QueryTxnRequest, RecurringBilling, RecurringBillingType,
        RecurringPeriod,
    },
    user_auth::OpaqueResponse,
};
use wiremock::{
    Mock, MockServer, Request, ResponseTemplate,
    matchers::{header, method, path},
};

const PRIVATE: &str = "private-billing-data&=+中文";
const METHODS: [(&str, &str, u32); 10] = [
    ("AdjustAgreement", "POST", 1),
    ("CancelAgreement", "POST", 1),
    ("FinalizeTxn", "POST", 2),
    ("GetReport", "GET", 5),
    ("GetUserAgreementInfo", "GET", 2),
    ("GetUserInfo", "GET", 2),
    ("InitTxn", "POST", 3),
    ("ProcessAgreement", "POST", 1),
    ("QueryTxn", "GET", 3),
    ("RefundTxn", "POST", 2),
];

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

fn publisher(server: &MockServer, retries: u32) -> SteamClient {
    client(
        server,
        Some(ApiKey::publisher("billing-key").unwrap()),
        retries,
    )
}

fn api(client: &SteamClient, sandbox: bool) -> MicroTxn<'_> {
    if sandbox {
        client.micro_txn_sandbox()
    } else {
        client.micro_txn_production()
    }
}

fn interface(sandbox: bool) -> &'static str {
    if sandbox {
        "ISteamMicroTxnSandbox"
    } else {
        "ISteamMicroTxn"
    }
}

fn user() -> MicroTxnUserRequest {
    MicroTxnUserRequest {
        appid: 480,
        steamid: u64::MAX.into(),
    }
}
fn agreement() -> AgreementRequest {
    AgreementRequest {
        appid: 480,
        steamid: u64::MAX.into(),
        agreementid: u64::MAX,
    }
}
fn order() -> OrderRequest {
    OrderRequest {
        appid: 480,
        orderid: u64::MAX,
    }
}

fn items() -> [MicroTxnItem<'static>; 2] {
    [
        MicroTxnItem {
            itemid: 0,
            qty: i16::MAX,
            amount: i64::MAX,
            description: "Sword & 中文=+",
            category: Some("gear&=+"),
            associated_bundle: Some(7),
            recurring: Some(RecurringBilling {
                billingtype: RecurringBillingType::Steam,
                period: RecurringPeriod::Month,
                frequency: 255,
                recurringamt: i64::MIN,
            }),
        },
        MicroTxnItem {
            itemid: u32::MAX,
            qty: 1,
            amount: i64::MIN,
            description: "Second",
            category: None,
            associated_bundle: None,
            recurring: None,
        },
    ]
}

fn init<'a>(
    items: &'a [MicroTxnItem<'a>],
    bundles: &'a [MicroTxnBundle<'a>],
) -> InitTxnRequest<'a> {
    InitTxnRequest {
        appid: 480,
        orderid: u64::MAX,
        steamid: u64::MAX.into(),
        language: "zh-CN",
        currency: "JPY",
        items,
        bundles,
        usersession: Some(MicroTxnUserSession::Web),
        ipaddress: Some("203.0.113.42"),
    }
}

async fn call(client: &SteamClient, sandbox: bool, name: &str) -> Result<OpaqueResponse> {
    let api = api(client, sandbox);
    match name {
        "AdjustAgreement" => {
            api.adjust_agreement(AdjustAgreementRequest {
                agreement: agreement(),
                nextprocessdate: "20280229",
                oldnextprocessdate: Some("20270228"),
            })
            .await
        }
        "CancelAgreement" => api.cancel_agreement(agreement()).await,
        "FinalizeTxn" => api.finalize_txn(order()).await,
        "GetReport" => {
            api.get_report(MicroTxnReportRequest {
                appid: 480,
                time: "2026-10-21T12:34:56.123456Z",
                report_type: Some(MicroTxnReportType::Subscription),
                maxresults: Some(50_000),
            })
            .await
        }
        "GetUserAgreementInfo" => api.get_user_agreement_info(user()).await,
        "GetUserInfo" => {
            api.get_user_info(PurchasingUserRequest {
                user: user(),
                ipaddress: Some("203.0.113.42"),
            })
            .await
        }
        "InitTxn" => {
            api.init_txn(init(
                &items(),
                &[MicroTxnBundle {
                    bundleid: 7,
                    bundle_qty: u32::MAX,
                    bundle_desc: "Bundle & 中文",
                    bundle_category: Some("bundle-category"),
                }],
            ))
            .await
        }
        "ProcessAgreement" => {
            api.process_agreement(ProcessAgreementRequest {
                agreement: agreement(),
                orderid: 0,
                amount: i32::MIN,
                currency: "USD",
            })
            .await
        }
        "QueryTxn" => {
            api.query_txn(QueryTxnRequest {
                appid: 480,
                orderid: Some(u64::MAX),
                transid: Some(u64::MAX),
            })
            .await
        }
        "RefundTxn" => api.refund_txn(order()).await,
        _ => unreachable!(),
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
        request.url.query_pairs().into_owned().collect()
    }
}

fn ok_response() -> Value {
    json!({"response":{"result":"OK","params":{"orderid":u64::MAX.to_string(),"transid":u64::MAX.to_string(),"agreementid":u64::MAX.to_string(),"status":"Init","steamurl":PRIVATE,"future_field":true}}})
}

#[tokio::test]
async fn all_twenty_routes_share_exact_versions_verbs_credentials_and_lossless_outputs() {
    let server = MockServer::start().await;
    let client = publisher(&server, 2);
    assert_eq!(
        client.micro_txn_sandbox().environment(),
        MicroTxnEnvironment::Sandbox
    );
    assert_eq!(
        client.micro_txn_production().environment(),
        MicroTxnEnvironment::Production
    );
    for sandbox in [true, false] {
        for (name, verb, version) in METHODS {
            Mock::given(method(verb))
                .and(path(format!("/{}/{name}/v{version}/", interface(sandbox))))
                .and(header("x-webapi-key", "billing-key"))
                .respond_with(ResponseTemplate::new(200).set_body_json(ok_response()))
                .expect(1)
                .mount(&server)
                .await;
            let response = call(&client, sandbox, name).await.unwrap();
            assert_eq!(
                response.as_json()["response"]["params"]["orderid"],
                u64::MAX.to_string()
            );
            assert_eq!(
                response.as_json()["response"]["params"]["transid"],
                u64::MAX.to_string()
            );
            assert_eq!(response.as_json()["response"]["params"]["status"], "Init");
            assert!(!format!("{response:?}").contains(PRIVATE));
        }
    }
    for request in server.received_requests().await.unwrap() {
        let fields = form(&request);
        assert_eq!(fields["format"], "json");
        assert_eq!(fields["appid"], "480");
        assert!(!fields.contains_key("input_json"));
        assert!(!fields.contains_key("key"));
        let name = request.url.path().split('/').nth(2).unwrap();
        match name {
            "AdjustAgreement" => {
                assert_eq!(fields["steamid"], u64::MAX.to_string());
                assert_eq!(fields["agreementid"], u64::MAX.to_string());
                assert_eq!(fields["nextprocessdate"], "20280229");
                assert_eq!(fields["oldnextprocessdate"], "20270228");
            }
            "CancelAgreement" => assert_eq!(fields["agreementid"], u64::MAX.to_string()),
            "FinalizeTxn" | "RefundTxn" => assert_eq!(fields["orderid"], u64::MAX.to_string()),
            "GetReport" => {
                assert_eq!(fields["type"], "SUBSCRIPTION");
                assert_eq!(fields["time"], "2026-10-21T12:34:56.123456Z");
                assert_eq!(fields["maxresults"], "50000");
            }
            "GetUserAgreementInfo" => assert_eq!(fields["steamid"], u64::MAX.to_string()),
            "GetUserInfo" => assert_eq!(fields["ipaddress"], "203.0.113.42"),
            "InitTxn" => {
                assert_eq!(fields["orderid"], u64::MAX.to_string());
                assert_eq!(fields["steamid"], u64::MAX.to_string());
                assert_eq!(fields["itemcount"], "2");
                assert_eq!(fields["qty[0]"], i16::MAX.to_string());
                assert_eq!(fields["amount[0]"], i64::MAX.to_string());
                assert_eq!(fields["amount[1]"], i64::MIN.to_string());
                assert_eq!(fields["description[0]"], "Sword & 中文=+");
                assert_eq!(fields["category[0]"], "gear&=+");
                assert_eq!(fields["associated_bundle[0]"], "7");
                assert_eq!(fields["billingtype[0]"], "Steam");
                assert_eq!(fields["period[0]"], "Month");
                assert_eq!(fields["frequency[0]"], "255");
                assert_eq!(fields["recurringamt[0]"], i64::MIN.to_string());
                assert_eq!(fields["bundlecount"], "1");
                assert_eq!(fields["bundle_qty[0]"], u32::MAX.to_string());
                assert_eq!(fields["bundle_desc[0]"], "Bundle & 中文");
                assert_eq!(fields["bundle_category[0]"], "bundle-category");
                assert_eq!(fields["usersession"], "web");
                assert_eq!(fields["currency"], "JPY");
                assert_eq!(fields["language"], "zh-CN");
                for name in [
                    "category[1]",
                    "billingtype[1]",
                    "associated_bundle[1]",
                    "bundleid[1]",
                ] {
                    assert!(!fields.contains_key(name));
                }
            }
            "ProcessAgreement" => {
                assert_eq!(fields["orderid"], "0");
                assert_eq!(fields["amount"], i32::MIN.to_string());
                assert_eq!(fields["currency"], "USD");
                assert_eq!(fields["agreementid"], u64::MAX.to_string());
            }
            "QueryTxn" => {
                assert_eq!(fields["orderid"], u64::MAX.to_string());
                assert_eq!(fields["transid"], u64::MAX.to_string());
            }
            _ => unreachable!(),
        }
    }
}

#[tokio::test]
async fn every_write_in_both_environments_is_never_replayed_or_redirected_to_other_environment() {
    let server = MockServer::start().await;
    let client = publisher(&server, 3);
    for sandbox in [true, false] {
        for (name, verb, version) in METHODS.into_iter().filter(|(_, verb, _)| *verb == "POST") {
            Mock::given(method(verb))
                .and(path(format!("/{}/{name}/v{version}/", interface(sandbox))))
                .respond_with(ResponseTemplate::new(503))
                .expect(1)
                .mount(&server)
                .await;
            assert!(matches!(
                call(&client, sandbox, name).await,
                Err(Error::Http { status: 503, .. })
            ));
        }
    }
    assert_eq!(client.metrics().attempts, 12);
    assert_eq!(client.metrics().retries, 0);
}

#[tokio::test]
async fn sandbox_failure_never_falls_back_to_available_production_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ISteamMicroTxnSandbox/FinalizeTxn/v2/"))
        .respond_with(ResponseTemplate::new(503))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/ISteamMicroTxn/FinalizeTxn/v2/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_response()))
        .expect(0)
        .mount(&server)
        .await;
    let client = publisher(&server, 3);
    assert!(matches!(
        client.micro_txn_sandbox().finalize_txn(order()).await,
        Err(Error::Http { status: 503, .. })
    ));
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn all_eight_read_routes_use_bounded_read_retries() {
    let server = MockServer::start().await;
    let client = publisher(&server, 2);
    for sandbox in [true, false] {
        for (name, verb, version) in METHODS.into_iter().filter(|(_, verb, _)| *verb == "GET") {
            Mock::given(method(verb))
                .and(path(format!("/{}/{name}/v{version}/", interface(sandbox))))
                .respond_with(ResponseTemplate::new(503))
                .expect(3)
                .mount(&server)
                .await;
            assert!(matches!(
                call(&client, sandbox, name).await,
                Err(Error::Http { status: 503, .. })
            ));
        }
    }
    assert_eq!(client.metrics().attempts, 24);
    assert_eq!(client.metrics().retries, 16);
}

#[tokio::test]
async fn all_routes_reject_user_and_anonymous_credentials_before_network() {
    let server = MockServer::start().await;
    for key in [None, Some(ApiKey::user("user-key").unwrap())] {
        let client = client(&server, key, 0);
        for sandbox in [true, false] {
            for (name, _, _) in METHODS {
                assert!(
                    matches!(
                        call(&client, sandbox, name).await,
                        Err(Error::MissingApiKey("publisher"))
                    ),
                    "{name}"
                );
            }
        }
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn every_route_requires_exact_ok_and_rejects_missing_numeric_and_failure_results() {
    for body in [
        json!({}),
        json!({"response":{}}),
        json!({"response":{"result":1}}),
        json!({"response":{"result":"Failure","error":{"errorcode":5,"errordesc":PRIVATE}}}),
        json!({"response":{"result":"OK","error":{"errorcode":6,"errordesc":PRIVATE}}}),
    ] {
        let server = MockServer::start().await;
        let client = publisher(&server, 3);
        for sandbox in [true, false] {
            for (name, verb, version) in METHODS {
                Mock::given(method(verb))
                    .and(path(format!("/{}/{name}/v{version}/", interface(sandbox))))
                    .respond_with(ResponseTemplate::new(200).set_body_json(&body))
                    .expect(1)
                    .mount(&server)
                    .await;
                let error = call(&client, sandbox, name).await.unwrap_err();
                assert!(
                    if body["response"].get("error").is_some() {
                        matches!(error, Error::Api { .. })
                    } else {
                        matches!(error, Error::Decode { .. })
                    },
                    "{name}: {error:?}"
                );
                assert!(!format!("{error:?} {error}").contains(PRIVATE));
            }
        }
        assert_eq!(client.metrics().succeeded, 0);
        assert_eq!(client.metrics().failed, 20);
        assert_eq!(client.metrics().retries, 0);
    }
}

#[tokio::test]
async fn optional_defaults_are_omitted_and_web_session_pairing_is_explicit() {
    let server = MockServer::start().await;
    let client = publisher(&server, 0);
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_response()))
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_response()))
        .expect(2)
        .mount(&server)
        .await;
    let items = [MicroTxnItem {
        itemid: 1,
        qty: 1,
        amount: 0,
        description: "Trial",
        category: None,
        associated_bundle: None,
        recurring: None,
    }];
    let mut request = init(&items, &[]);
    request.currency = "KWD";
    request.usersession = None;
    request.ipaddress = None;
    client.micro_txn_sandbox().init_txn(request).await.unwrap();
    client
        .micro_txn_sandbox()
        .adjust_agreement(AdjustAgreementRequest {
            agreement: agreement(),
            nextprocessdate: "20280229",
            oldnextprocessdate: None,
        })
        .await
        .unwrap();
    client
        .micro_txn_sandbox()
        .get_report(MicroTxnReportRequest {
            appid: 480,
            time: "2026-10-21T00:00:00+00:00",
            report_type: None,
            maxresults: None,
        })
        .await
        .unwrap();
    client
        .micro_txn_sandbox()
        .get_user_info(PurchasingUserRequest {
            user: user(),
            ipaddress: None,
        })
        .await
        .unwrap();
    let requests = server.received_requests().await.unwrap();
    let first = form(&requests[0]);
    for key in [
        "usersession",
        "ipaddress",
        "bundlecount",
        "category[0]",
        "billingtype[0]",
        "period[0]",
        "frequency[0]",
        "recurringamt[0]",
    ] {
        assert!(!first.contains_key(key));
    }
    assert_eq!(first["amount[0]"], "0");
    assert_eq!(first["currency"], "KWD");
    assert!(!form(&requests[1]).contains_key("oldnextprocessdate"));
    assert!(!form(&requests[2]).contains_key("type"));
    assert!(!form(&requests[2]).contains_key("maxresults"));
    assert!(!form(&requests[3]).contains_key("ipaddress"));
}

#[tokio::test]
async fn report_cursor_is_caller_owned_after_empty_and_short_batches() {
    let server = MockServer::start().await;
    let client = publisher(&server, 0);
    let current = "2026-10-20T01:02:03Z";
    let next = "2026-10-20T01:02:04Z";
    Mock::given(path("/ISteamMicroTxnSandbox/GetReport/v5/")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"response":{"result":"OK","params":{"count":1,"orders":[{"orderid":"0","transid":u64::MAX.to_string(),"time":next,"status":"RefundedFriendlyFraud"}]}}}))).expect(1).mount(&server).await;
    let response = client
        .micro_txn_sandbox()
        .get_report(MicroTxnReportRequest {
            appid: 480,
            time: current,
            report_type: Some(MicroTxnReportType::Settlement),
            maxresults: Some(1000),
        })
        .await
        .unwrap();
    assert_eq!(
        response.as_json()["response"]["params"]["orders"][0]["status"],
        "RefundedFriendlyFraud"
    );
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        1,
        "A short report must not trigger hidden paging"
    );
    server.reset().await;
    Mock::given(path("/ISteamMicroTxnSandbox/GetReport/v5/"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(
                json!({"response":{"result":"OK","params":{"count":0,"orders":[]}}}),
            ),
        )
        .expect(2)
        .mount(&server)
        .await;
    for _ in 0..2 {
        client
            .micro_txn_sandbox()
            .get_report(MicroTxnReportRequest {
                appid: 480,
                time: next,
                report_type: None,
                maxresults: Some(1000),
            })
            .await
            .unwrap();
    }
    for request in server.received_requests().await.unwrap() {
        assert_eq!(form(&request)["time"], next);
    }
}

#[tokio::test]
async fn query_supports_either_identifier_and_store_renewal_zero_order_ids() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ISteamMicroTxnSandbox/QueryTxn/v3/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_response()))
        .expect(3)
        .mount(&server)
        .await;
    let client = publisher(&server, 0);
    for request in [
        QueryTxnRequest {
            appid: 480,
            orderid: Some(u64::MAX),
            transid: None,
        },
        QueryTxnRequest {
            appid: 480,
            orderid: None,
            transid: Some(u64::MAX),
        },
        QueryTxnRequest {
            appid: 480,
            orderid: Some(0),
            transid: Some(u64::MAX),
        },
    ] {
        client.micro_txn_sandbox().query_txn(request).await.unwrap();
    }
    let requests = server.received_requests().await.unwrap();
    assert!(!form(&requests[0]).contains_key("transid"));
    assert!(!form(&requests[1]).contains_key("orderid"));
    assert_eq!(form(&requests[2])["orderid"], "0");
}

#[tokio::test]
async fn unicode_character_limits_and_all_recurring_values_preserve_wire_precision() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ISteamMicroTxnSandbox/InitTxn/v3/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_response()))
        .expect(4)
        .mount(&server)
        .await;
    let client = publisher(&server, 0);
    let description = "中".repeat(128);
    let category = "文".repeat(64);
    for (billingtype, period) in [
        (RecurringBillingType::Steam, RecurringPeriod::Day),
        (RecurringBillingType::Game, RecurringPeriod::Week),
        (RecurringBillingType::Steam, RecurringPeriod::Month),
        (RecurringBillingType::Game, RecurringPeriod::Year),
    ] {
        let items = [MicroTxnItem {
            itemid: 1,
            qty: 2,
            amount: 199,
            description: &description,
            category: Some(&category),
            associated_bundle: None,
            recurring: Some(RecurringBilling {
                billingtype,
                period,
                frequency: 1,
                recurringamt: 299,
            }),
        }];
        let mut request = init(&items, &[]);
        request.usersession = Some(MicroTxnUserSession::Client);
        request.ipaddress = None;
        request.language = match period {
            RecurringPeriod::Day => "zh-CN",
            RecurringPeriod::Week => "pt-BR",
            RecurringPeriod::Month => "es-419",
            RecurringPeriod::Year => "zh-TW",
        };
        client.micro_txn_sandbox().init_txn(request).await.unwrap();
    }
    for (index, request) in server.received_requests().await.unwrap().iter().enumerate() {
        let fields = form(request);
        assert_eq!(
            fields["amount[0]"], "199",
            "Line totals must not be multiplied by qty"
        );
        assert_eq!(fields["description[0]"], description);
        assert_eq!(fields["category[0]"], category);
        assert_eq!(fields["usersession"], "client");
        assert_eq!(fields["period[0]"], ["Day", "Week", "Month", "Year"][index]);
        assert_eq!(
            fields["billingtype[0]"],
            if index % 2 == 0 { "Steam" } else { "Game" }
        );
        assert_eq!(
            fields["language"],
            ["zh-CN", "pt-BR", "es-419", "zh-TW"][index]
        );
    }
}

#[tokio::test]
async fn malformed_dates_cursors_and_count_bounds_fail_before_network() {
    let server = MockServer::start().await;
    let client = publisher(&server, 0);
    for date in [
        "",
        "20270229",
        "19000229",
        "20281301",
        "20280431",
        "00000101",
        "2028-02-29",
        "20280229\0",
    ] {
        assert!(matches!(
            client
                .micro_txn_sandbox()
                .adjust_agreement(AdjustAgreementRequest {
                    agreement: agreement(),
                    nextprocessdate: date,
                    oldnextprocessdate: None
                })
                .await,
            Err(Error::InvalidParameter(_))
        ));
    }
    assert!(
        client
            .micro_txn_sandbox()
            .adjust_agreement(AdjustAgreementRequest {
                agreement: agreement(),
                nextprocessdate: "20280229",
                oldnextprocessdate: Some("20280301")
            })
            .await
            .is_err()
    );
    for time in [
        "",
        "2026-02-29T00:00:00Z",
        "2026-10-21T24:00:00Z",
        "2026-10-21T00:60:00Z",
        "2026-10-21T00:00:61Z",
        "2026-10-21T00:00:00",
        "2026-10-21T00:00:00+01:00",
        "2026-10-21T00:00:00.Z",
        "2026-10-21T00:00:00.xyzZ",
        "２０２６-10-21T00:00:00Z",
    ] {
        assert!(
            matches!(
                client
                    .micro_txn_sandbox()
                    .get_report(MicroTxnReportRequest {
                        appid: 480,
                        time,
                        report_type: None,
                        maxresults: None
                    })
                    .await,
                Err(Error::InvalidParameter(_))
            ),
            "{time}"
        );
    }
    for count in [0, 50_001] {
        assert!(
            client
                .micro_txn_sandbox()
                .get_report(MicroTxnReportRequest {
                    appid: 480,
                    time: "2026-10-21T00:00:00Z",
                    report_type: None,
                    maxresults: Some(count)
                })
                .await
                .is_err()
        );
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn invalid_cart_sizes_pairing_ids_currency_and_descriptions_fail_before_network() {
    let server = MockServer::start().await;
    let client = publisher(&server, 0);
    let items = items();
    for request in [
        InitTxnRequest {
            appid: 0,
            ..init(&items, &[])
        },
        InitTxnRequest {
            orderid: 0,
            ..init(&items, &[])
        },
        InitTxnRequest {
            steamid: 0.into(),
            ..init(&items, &[])
        },
        InitTxnRequest {
            language: "",
            ..init(&items, &[])
        },
        InitTxnRequest {
            currency: "usd",
            ..init(&items, &[])
        },
        InitTxnRequest {
            ipaddress: None,
            ..init(&items, &[])
        },
        InitTxnRequest {
            ipaddress: Some("203.0.113.1:80"),
            ..init(&items, &[])
        },
        InitTxnRequest {
            items: &[],
            ..init(&items, &[])
        },
    ] {
        assert!(matches!(
            client.micro_txn_sandbox().init_txn(request).await,
            Err(Error::InvalidParameter(_))
        ));
    }
    let long_description = "中".repeat(129);
    let long_category = "文".repeat(65);
    for item in [
        MicroTxnItem { qty: 0, ..items[0] },
        MicroTxnItem {
            qty: -1,
            ..items[0]
        },
        MicroTxnItem {
            description: "",
            ..items[0]
        },
        MicroTxnItem {
            description: &long_description,
            ..items[0]
        },
        MicroTxnItem {
            category: Some(&long_category),
            ..items[0]
        },
        MicroTxnItem {
            recurring: Some(RecurringBilling {
                frequency: 0,
                ..items[0].recurring.unwrap()
            }),
            ..items[0]
        },
    ] {
        assert!(matches!(
            client
                .micro_txn_sandbox()
                .init_txn(init(&[item], &[]))
                .await,
            Err(Error::InvalidParameter(_))
        ));
    }
    for request in [
        QueryTxnRequest {
            appid: 480,
            orderid: None,
            transid: None,
        },
        QueryTxnRequest {
            appid: 0,
            orderid: Some(1),
            transid: None,
        },
        QueryTxnRequest {
            appid: 480,
            orderid: Some(0),
            transid: None,
        },
        QueryTxnRequest {
            appid: 480,
            orderid: None,
            transid: Some(0),
        },
    ] {
        assert!(client.micro_txn_sandbox().query_txn(request).await.is_err());
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn agreement_order_user_and_bundle_validation_rejects_invalid_requests_before_io() {
    let server = MockServer::start().await;
    let client = publisher(&server, 0);
    let api = client.micro_txn_sandbox();
    for agreement in [
        AgreementRequest {
            appid: 0,
            ..agreement()
        },
        AgreementRequest {
            steamid: 0.into(),
            ..agreement()
        },
        AgreementRequest {
            agreementid: 0,
            ..agreement()
        },
    ] {
        assert!(api.cancel_agreement(agreement).await.is_err());
        assert!(
            api.adjust_agreement(AdjustAgreementRequest {
                agreement,
                nextprocessdate: "20280229",
                oldnextprocessdate: None
            })
            .await
            .is_err()
        );
        assert!(
            api.process_agreement(ProcessAgreementRequest {
                agreement,
                orderid: 0,
                amount: 0,
                currency: "USD"
            })
            .await
            .is_err()
        );
    }
    for order in [
        OrderRequest {
            appid: 0,
            ..order()
        },
        OrderRequest {
            orderid: 0,
            ..order()
        },
    ] {
        assert!(api.finalize_txn(order).await.is_err());
        assert!(api.refund_txn(order).await.is_err());
    }
    for user in [
        MicroTxnUserRequest { appid: 0, ..user() },
        MicroTxnUserRequest {
            steamid: 0.into(),
            ..user()
        },
    ] {
        assert!(api.get_user_agreement_info(user).await.is_err());
        assert!(
            api.get_user_info(PurchasingUserRequest {
                user,
                ipaddress: None
            })
            .await
            .is_err()
        );
    }
    assert!(
        api.get_user_info(PurchasingUserRequest {
            user: user(),
            ipaddress: Some("not-an-ip")
        })
        .await
        .is_err()
    );
    assert!(
        api.process_agreement(ProcessAgreementRequest {
            agreement: agreement(),
            orderid: 0,
            amount: 0,
            currency: "US"
        })
        .await
        .is_err()
    );
    let long_description = "中".repeat(129);
    let long_category = "文".repeat(65);
    for bundle in [
        MicroTxnBundle {
            bundleid: 7,
            bundle_qty: 0,
            bundle_desc: "Bundle",
            bundle_category: None,
        },
        MicroTxnBundle {
            bundleid: 7,
            bundle_qty: 1,
            bundle_desc: &long_description,
            bundle_category: None,
        },
        MicroTxnBundle {
            bundleid: 7,
            bundle_qty: 1,
            bundle_desc: "Bundle",
            bundle_category: Some(&long_category),
        },
    ] {
        assert!(api.init_txn(init(&items(), &[bundle])).await.is_err());
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn all_report_categories_and_initiated_billing_state_remain_explicit() {
    let server = MockServer::start().await;
    let client = publisher(&server, 0);
    Mock::given(method("GET"))
        .and(path("/ISteamMicroTxnSandbox/GetReport/v5/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_response()))
        .expect(5)
        .mount(&server)
        .await;
    for report_type in [
        MicroTxnReportType::GameSales,
        MicroTxnReportType::SteamStoreSales,
        MicroTxnReportType::Settlement,
        MicroTxnReportType::Chargeback,
        MicroTxnReportType::Subscription,
    ] {
        client
            .micro_txn_sandbox()
            .get_report(MicroTxnReportRequest {
                appid: 480,
                time: "2026-10-21T00:00:00Z",
                report_type: Some(report_type),
                maxresults: Some(1),
            })
            .await
            .unwrap();
    }
    for (request, expected) in server.received_requests().await.unwrap().iter().zip([
        "GAMESALES",
        "STEAMSTORESALES",
        "SETTLEMENT",
        "CHARGEBACK",
        "SUBSCRIPTION",
    ]) {
        assert_eq!(form(request)["type"], expected);
    }
    Mock::given(method("POST"))
        .and(path("/ISteamMicroTxnSandbox/ProcessAgreement/v1/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response":{"result":"OK","params":{"status":"processing","agreementid":u64::MAX.to_string(),"orderid":"0"}}})))
        .expect(1).mount(&server).await;
    let response = client
        .micro_txn_sandbox()
        .process_agreement(ProcessAgreementRequest {
            agreement: agreement(),
            orderid: 0,
            amount: 199,
            currency: "USD",
        })
        .await
        .unwrap();
    assert_eq!(
        response.as_json()["response"]["params"]["status"],
        "processing"
    );
    assert_eq!(response.as_json()["response"]["params"]["orderid"], "0");
}
