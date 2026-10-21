// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Financial reporting contracts use dummy keys and local mock servers only.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::{Value, json};
use std::{collections::BTreeMap, time::Duration};
use steam_api_web::{
    ApiKey, Error, Result, RetryPolicy, SteamClient,
    partner_financials::{AppWishlistRequest, ChangedDatesRequest, DetailedSalesRequest},
};
use wiremock::{
    Mock, MockServer, Request, ResponseTemplate,
    matchers::{header, method, path},
};

const PRIVATE: &str = "private-financial-data&=+中文";
const METHODS: [&str; 3] = [
    "GetChangedDatesForPartner",
    "GetDetailedSales",
    "GetAppWishlistReporting",
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

fn financial(server: &MockServer, retries: u32) -> SteamClient {
    client(
        server,
        Some(ApiKey::financial("financial-group-key").unwrap()),
        retries,
    )
}

fn endpoint(name: &str) -> String {
    format!("/IPartnerFinancialsService/{name}/v001/")
}

fn fields(request: &Request) -> BTreeMap<String, String> {
    assert!(request.body.is_empty());
    request.url.query_pairs().into_owned().collect()
}

fn input(request: &Request) -> Value {
    let fields = fields(request);
    assert_eq!(fields.len(), 2);
    assert_eq!(fields["format"], "json");
    serde_json::from_str(&fields["input_json"]).unwrap()
}

fn response(name: &str) -> Value {
    match name {
        "GetChangedDatesForPartner" => json!({"dates": [], "result_highwatermark":"0"}),
        "GetDetailedSales" => json!({"results": [], "max_id":"0"}),
        "GetAppWishlistReporting" => json!({"app_min_date":"2010-01-01"}),
        _ => panic!("unknown financial test method"),
    }
}

async fn call(client: &SteamClient, name: &str) -> Result<()> {
    match name {
        "GetChangedDatesForPartner" => client
            .partner_financials()
            .get_changed_dates_for_partner(ChangedDatesRequest {
                highwatermark: u64::MAX,
                include_view_grants: Some(false),
            })
            .await
            .map(|_| ()),
        "GetDetailedSales" => client
            .partner_financials()
            .get_detailed_sales(DetailedSalesRequest {
                date: "2028/02/29",
                highwatermark_id: u64::MAX,
                include_view_grants: Some(true),
            })
            .await
            .map(|_| ()),
        "GetAppWishlistReporting" => client
            .partner_financials()
            .get_app_wishlist_reporting(AppWishlistRequest {
                appid: u32::MAX,
                date: "20280229",
            })
            .await
            .map(|_| ()),
        _ => panic!("unknown financial test method"),
    }
}

#[tokio::test]
async fn all_three_methods_use_financial_headers_exact_v001_paths_and_input_json() {
    let server = MockServer::start().await;
    for name in METHODS {
        Mock::given(method("GET"))
            .and(path(endpoint(name)))
            .and(header("x-webapi-key", "financial-group-key"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"response": response(name)})),
            )
            .expect(1)
            .mount(&server)
            .await;
    }
    let client = financial(&server, 0);
    for name in METHODS {
        call(&client, name).await.unwrap();
    }
    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        input(&requests[0]),
        json!({"highwatermark":u64::MAX,"include_view_grants":false})
    );
    assert_eq!(
        input(&requests[1]),
        json!({"date":"2028/02/29","highwatermark_id":u64::MAX,"include_view_grants":true})
    );
    assert_eq!(
        input(&requests[2]),
        json!({"appid":u32::MAX,"date":"20280229"})
    );
    for request in requests {
        assert!(!fields(&request).contains_key("key"));
        assert!(!request.url.as_str().contains("financial-group-key"));
    }
    assert_eq!(client.metrics().succeeded, 3);
}

#[tokio::test]
async fn anonymous_user_and_publisher_keys_cannot_send_financial_requests() {
    let server = MockServer::start().await;
    for key in [
        None,
        Some(ApiKey::user("ordinary-user").unwrap()),
        Some(ApiKey::publisher("ordinary-publisher").unwrap()),
    ] {
        let client = client(&server, key, 2);
        for name in METHODS {
            assert!(matches!(
                call(&client, name).await,
                Err(Error::MissingApiKey("financial"))
            ));
        }
        assert_eq!(client.metrics().attempts, 0);
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn omitted_view_grants_and_zero_cursors_are_not_replaced_by_defaults() {
    let server = MockServer::start().await;
    for name in &METHODS[..2] {
        Mock::given(path(endpoint(name)))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"response":response(name)})),
            )
            .expect(1)
            .mount(&server)
            .await;
    }
    let client = financial(&server, 0);
    client
        .partner_financials()
        .get_changed_dates_for_partner(ChangedDatesRequest {
            highwatermark: 0,
            include_view_grants: None,
        })
        .await
        .unwrap();
    client
        .partner_financials()
        .get_detailed_sales(DetailedSalesRequest {
            date: "2026-10-21",
            highwatermark_id: 0,
            include_view_grants: None,
        })
        .await
        .unwrap();
    let requests = server.received_requests().await.unwrap();
    assert_eq!(input(&requests[0]), json!({"highwatermark":0}));
    assert_eq!(
        input(&requests[1]),
        json!({"date":"2026-10-21","highwatermark_id":0})
    );
}

#[tokio::test]
async fn documented_date_formats_and_leap_centuries_are_preserved_without_offsets() {
    let server = MockServer::start().await;
    let dates = [
        "2026-10-21",
        "2026/10/21",
        "20261021",
        "2000-02-29",
        "2400/02/29",
    ];
    for name in &METHODS[1..] {
        Mock::given(path(endpoint(name)))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"response":response(name)})),
            )
            .expect(dates.len() as u64)
            .mount(&server)
            .await;
    }
    let client = financial(&server, 0);
    for date in dates {
        client
            .partner_financials()
            .get_detailed_sales(DetailedSalesRequest {
                date,
                highwatermark_id: 0,
                include_view_grants: None,
            })
            .await
            .unwrap();
        client
            .partner_financials()
            .get_app_wishlist_reporting(AppWishlistRequest { appid: 480, date })
            .await
            .unwrap();
    }
    for (requests, date) in server
        .received_requests()
        .await
        .unwrap()
        .as_chunks::<2>()
        .0
        .iter()
        .zip(dates)
    {
        assert_eq!(input(&requests[0])["date"], date);
        assert_eq!(input(&requests[1])["date"], date);
    }
}

#[tokio::test]
async fn invalid_dates_and_zero_app_are_rejected_before_io_without_echoing_inputs() {
    let server = MockServer::start().await;
    let client = financial(&server, 0);
    for date in [
        "",
        PRIVATE,
        "2026/10-21",
        "2026-2-01",
        "20260229",
        "1900-02-29",
        "2026-04-31",
        "2026-00-10",
        "2026-13-10",
        "2026-10-00",
        "2026-10-32",
        "2026-10-21T00:00:00Z",
        "２０２６１０２１",
        "2026102\0",
    ] {
        let errors = [
            client
                .partner_financials()
                .get_detailed_sales(DetailedSalesRequest {
                    date,
                    highwatermark_id: 0,
                    include_view_grants: None,
                })
                .await
                .unwrap_err(),
            client
                .partner_financials()
                .get_app_wishlist_reporting(AppWishlistRequest { appid: 480, date })
                .await
                .unwrap_err(),
        ];
        for error in errors {
            assert!(matches!(error, Error::InvalidParameter(_)));
            assert!(!format!("{error:?} {error}").contains(PRIVATE));
        }
    }
    assert!(matches!(
        client
            .partner_financials()
            .get_app_wishlist_reporting(AppWishlistRequest {
                appid: 0,
                date: "2026-10-21"
            })
            .await,
        Err(Error::InvalidParameter(_))
    ));
    assert_eq!(client.metrics().attempts, 0);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn changed_dates_keep_their_exact_strings_and_independent_outer_cursor() {
    let server = MockServer::start().await;
    Mock::given(path(endpoint(METHODS[0])))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"response": {
                "dates":["2025/05/03", "2025/05/04"],
                "result_highwatermark":"00018446744073709551615", "private":PRIVATE,
            }})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = financial(&server, 2);
    let result = client
        .partner_financials()
        .get_changed_dates_for_partner(ChangedDatesRequest {
            highwatermark: 0,
            include_view_grants: None,
        })
        .await
        .unwrap();
    assert_eq!(result.result_highwatermark, "00018446744073709551615");
    assert_eq!(result.dates, ["2025/05/03", "2025/05/04"]);
    assert_eq!(result.extra["private"], PRIVATE);
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    assert!(!format!("{result:?}").contains(PRIVATE));
}

#[tokio::test]
async fn sales_preserve_signed_counts_uint64_prices_decimal_strings_and_extensions() {
    let server = MockServer::start().await;
    Mock::given(path(endpoint(METHODS[1])))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response": {
            "results":[{
                "partnerid":i32::MIN.to_string(), "date":"2025/05/01", "line_item_type":"FutureCategory", "country_code":"US",
                "packageid":u32::MAX, "bundleid":u32::MAX.to_string(), "appid":u32::MAX,
                "game_item_id":i32::MIN, "package_sale_type":"FutureChannel", "key_request_id":u32::MAX,
                "platform":"FuturePlatform", "base_price":u64::MAX.to_string(), "sale_price":u64::MAX,
                "currency":"USD", "combined_discount_id":i32::MIN,
                "gross_units_sold":i32::MAX, "gross_units_returned":i32::MIN.to_string(),
                "gross_sales_usd":"18446744073709551615.1234", "gross_returns_usd":"-25.5322",
                "net_tax_usd":"0.0000", "gross_units_activated":i32::MAX,
                "view_grant_partnerid":i32::MAX.to_string(), "net_units_sold":i32::MIN,
                "net_sales_usd":"-18446744073709551615.9999", "avg_sale_price_usd":"4.9900",
                "primary_appid":0, "additional_revenue_share_tier":i32::MAX,
                "private_extension":PRIVATE, "total_discount_percentage":50,
            }],
            "max_id":i64::MAX.to_string(),
            "package_info":[{"packageid":u32::MAX,"package_name":PRIVATE}],
            "download_url":"http://127.0.0.1:1/not-automatically-followed",
        }}))).expect(1).mount(&server).await;
    let client = financial(&server, 2);
    let response = client
        .partner_financials()
        .get_detailed_sales(DetailedSalesRequest {
            date: "2025/05/01",
            highwatermark_id: 0,
            include_view_grants: Some(true),
        })
        .await
        .unwrap();
    assert_eq!(response.max_id, i64::MAX);
    let row = &response.results[0];
    assert_eq!(row.partnerid, i32::MIN);
    assert_eq!(row.packageid, Some(u32::MAX));
    assert_eq!(row.bundleid, Some(u32::MAX));
    assert_eq!(row.game_item_id, Some(i32::MIN));
    assert_eq!(row.base_price, Some(u64::MAX));
    assert_eq!(row.sale_price, Some(u64::MAX));
    assert_eq!(row.gross_units_returned, Some(i32::MIN));
    assert_eq!(
        row.gross_sales_usd.as_deref(),
        Some("18446744073709551615.1234")
    );
    assert_eq!(row.gross_returns_usd.as_deref(), Some("-25.5322"));
    assert_eq!(row.net_tax_usd.as_deref(), Some("0.0000"));
    assert_eq!(
        row.net_sales_usd.as_deref(),
        Some("-18446744073709551615.9999")
    );
    assert_eq!(row.primary_appid, Some(0));
    assert_eq!(row.additional_revenue_share_tier, Some(i32::MAX));
    assert_eq!(row.line_item_type, "FutureCategory");
    assert_eq!(row.extra["total_discount_percentage"], 50);
    assert_eq!(row.extra["private_extension"], PRIVATE);
    assert_eq!(response.extra["package_info"][0]["package_name"], PRIVATE);
    assert!(!format!("{response:?} {row:?}").contains(PRIVATE));
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn conditional_sales_fields_are_absent_for_retail_keys_and_new_categories() {
    let server = MockServer::start().await;
    Mock::given(path(endpoint(METHODS[1])))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response": {
            "results":[{"partnerid":10500,"date":"20260501","line_item_type":"Package","country_code":"AE","packageid":123,"package_sale_type":"Retail","key_request_id":7100,"gross_units_activated":10}],
            "max_id":0,
        }}))).expect(1).mount(&server).await;
    let response = financial(&server, 0)
        .partner_financials()
        .get_detailed_sales(DetailedSalesRequest {
            date: "20260501",
            highwatermark_id: 0,
            include_view_grants: None,
        })
        .await
        .unwrap();
    let row = &response.results[0];
    assert_eq!(row.gross_units_activated, Some(10));
    assert!(row.base_price.is_none());
    assert!(row.gross_sales_usd.is_none());
    assert!(row.appid.is_none());
}

#[tokio::test]
async fn sales_cursor_is_signed_and_empty_rows_never_trigger_automatic_pagination() {
    for cursor in [
        json!(i64::MIN),
        json!(i64::MAX.to_string()),
        json!("17"),
        json!(0),
    ] {
        let server = MockServer::start().await;
        Mock::given(path(endpoint(METHODS[1])))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"response":{"results":[],"max_id":cursor}})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let client = financial(&server, 2);
        let response = client
            .partner_financials()
            .get_detailed_sales(DetailedSalesRequest {
                date: "2026-10-21",
                highwatermark_id: 17,
                include_view_grants: None,
            })
            .await
            .unwrap();
        let expected: i64 = match cursor {
            Value::String(text) => text.parse().unwrap(),
            other => other.as_i64().unwrap(),
        };
        assert_eq!(response.max_id, expected);
        assert!(response.results.is_empty());
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

fn summary() -> Value {
    json!({"wishlist_adds":u32::MAX,"wishlist_deletes":0,"wishlist_purchases":1,"wishlist_gifts":2,"wishlist_adds_windows":3,"wishlist_adds_mac":4,"wishlist_adds_linux":5,"future_counter":u64::MAX})
}

#[tokio::test]
async fn wishlist_counts_country_languages_and_example_only_metadata_are_preserved() {
    let server = MockServer::start().await;
    Mock::given(path(endpoint(METHODS[2])))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response":{
            "appid":480,"date":"2026-10-21","app_min_date":"2010-01-01",
            "wishlist_summary":summary(),
            "country_summary":[{"country_code":"CA","country_name":PRIVATE,"region":"North America","summary_actions":summary(),"future_country":PRIVATE}],
            "language_summary":[{"language":u32::MAX,"language_name":PRIVATE,"summary_actions":summary()},{"language":0,"language_name":"english","summary_actions":summary()}],
            "time_generated":u64::MAX, "private":PRIVATE,
        }}))).expect(1).mount(&server).await;
    let response = financial(&server, 0)
        .partner_financials()
        .get_app_wishlist_reporting(AppWishlistRequest {
            appid: 480,
            date: "2026-10-21",
        })
        .await
        .unwrap();
    assert_eq!(response.appid, Some(480));
    assert_eq!(
        response.wishlist_summary.as_ref().unwrap().wishlist_adds,
        u32::MAX
    );
    assert_eq!(response.country_summary[0].country_name, PRIVATE);
    assert_eq!(response.country_summary[0].extra["future_country"], PRIVATE);
    assert_eq!(response.language_summary[0].language, u32::MAX);
    assert_eq!(response.language_summary[1].language, 0);
    assert_eq!(response.extra["time_generated"], u64::MAX);
    assert_eq!(
        response.wishlist_summary.as_ref().unwrap().extra["future_counter"],
        u64::MAX
    );
    let debug = format!(
        "{response:?} {:?} {:?} {:?}",
        response.wishlist_summary.as_ref().unwrap(),
        response.country_summary[0],
        response.language_summary[0]
    );
    assert!(!debug.contains(PRIVATE));
    assert!(debug.contains("REDACTED"));
}

#[tokio::test]
async fn out_of_range_wishlist_response_does_not_invent_activity_or_app_identity() {
    let server = MockServer::start().await;
    Mock::given(path(endpoint(METHODS[2])))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"response":{"app_min_date":"2010-01-01"}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let response = financial(&server, 0)
        .partner_financials()
        .get_app_wishlist_reporting(AppWishlistRequest {
            appid: 480,
            date: "2000-01-01",
        })
        .await
        .unwrap();
    assert_eq!(response.app_min_date, "2010-01-01");
    assert!(response.appid.is_none());
    assert!(response.date.is_none());
    assert!(response.wishlist_summary.is_none());
    assert!(response.country_summary.is_empty());
    assert!(response.language_summary.is_empty());
}

#[tokio::test]
async fn business_errors_are_checked_before_typed_financial_decoding_without_replay() {
    for name in METHODS {
        let server = MockServer::start().await;
        Mock::given(path(endpoint(name)))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                json!({"response":{"eresult":15,"error":{"errorcode":15,"message":PRIVATE}}}),
            ))
            .expect(1)
            .mount(&server)
            .await;
        let client = financial(&server, 2);
        let error = call(&client, name).await.unwrap_err();
        assert!(matches!(error, Error::Api { code: Some(15), .. }));
        assert!(!format!("{error:?} {error}").contains(PRIVATE));
        assert_eq!(client.metrics().attempts, 1);
        assert_eq!(client.metrics().retries, 0);
        assert_eq!(client.metrics().failed, 1);
        assert_eq!(client.metrics().succeeded, 0);
    }
}

#[tokio::test]
async fn every_financial_get_uses_bounded_retry_for_temporary_http_failure() {
    for name in METHODS {
        let server = MockServer::start().await;
        Mock::given(path(endpoint(name)))
            .respond_with(ResponseTemplate::new(503))
            .expect(3)
            .mount(&server)
            .await;
        let client = financial(&server, 2);
        assert!(matches!(
            call(&client, name).await,
            Err(Error::Http { status: 503, .. })
        ));
        assert_eq!(client.metrics().attempts, 3);
        assert_eq!(client.metrics().retries, 2);
        assert_eq!(client.metrics().failed, 1);
    }
}

#[tokio::test]
async fn malformed_financial_envelopes_and_lossy_numeric_forms_fail_before_success() {
    let mut invalid_row = json!({"partnerid":10500,"date":"2026-10-21","line_item_type":"Package","country_code":"US"});
    let mut rows = Vec::new();
    for (field, value) in [
        ("base_price", json!(18446744073709551616.0_f64)),
        ("sale_price", json!(-1)),
        ("gross_units_returned", json!(i64::MAX)),
        ("gross_units_sold", json!(1.5)),
        ("gross_sales_usd", json!(0.0001)),
        ("partnerid", json!(PRIVATE)),
    ] {
        let old = invalid_row[field].take();
        invalid_row[field] = value;
        rows.push(json!({"response":{"results":[invalid_row.clone()],"max_id":"1"}}));
        if old.is_null() {
            invalid_row.as_object_mut().unwrap().remove(field);
        } else {
            invalid_row[field] = old;
        }
    }
    rows.extend([
        json!({"response":{"results":[],"max_id":"18446744073709551615"}}),
        json!({"response":{"results":[],"max_id":1.25}}),
        json!({"response":{"results":[]}}),
        json!({"results":[],"max_id":"1"}),
    ]);
    for body in rows {
        let server = MockServer::start().await;
        Mock::given(path(endpoint(METHODS[1])))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .expect(1)
            .mount(&server)
            .await;
        let client = financial(&server, 2);
        let error = call(&client, METHODS[1]).await.unwrap_err();
        assert!(matches!(error, Error::Decode { .. }));
        assert!(!format!("{error:?} {error}").contains(PRIVATE));
        assert_eq!(client.metrics().failed, 1);
        assert_eq!(client.metrics().succeeded, 0);
        assert_eq!(client.metrics().retries, 0);
    }
}

#[test]
fn financial_request_debug_hides_dates_and_both_cursor_values() {
    let debug = format!(
        "{:?} {:?} {:?}",
        ChangedDatesRequest {
            highwatermark: u64::MAX,
            include_view_grants: Some(true)
        },
        DetailedSalesRequest {
            date: PRIVATE,
            highwatermark_id: u64::MAX,
            include_view_grants: None
        },
        AppWishlistRequest {
            appid: 480,
            date: PRIVATE
        }
    );
    assert!(!debug.contains(PRIVATE));
    assert!(!debug.contains(&u64::MAX.to_string()));
    assert_eq!(debug.matches("REDACTED").count(), 3);
}
