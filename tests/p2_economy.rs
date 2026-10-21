// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Economy wire contracts against local HTTP servers; no Steam keys needed.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use steam_api_web::{
    ApiKey, Error, RetryPolicy, SteamClient, SteamId,
    econ::{TradeHistoryOptions, TradeOffersOptions},
    econ_market::PopularMarketOptions,
    economy::{AssetClass, AssetPriceOptions, AssetPurchase, StartAssetTransactionRequest},
    user_auth::OpaqueResponse,
};
use wiremock::{
    Mock, MockServer, Request, Respond, ResponseTemplate,
    matchers::{header, method, path},
};

const ID: u64 = u64::MAX;
const TEXT: &str = "opaque+&=%/中文";
const METHODS: [(&str, &str, &str, bool); 19] = [
    ("ISteamEconomy", "CanTrade", "GET", true),
    ("ISteamEconomy", "FinalizeAssetTransaction", "POST", true),
    ("ISteamEconomy", "GetAssetClassInfo", "GET", false),
    ("ISteamEconomy", "GetAssetPrices", "GET", false),
    ("ISteamEconomy", "GetExportedAssetsForUser", "GET", true),
    ("ISteamEconomy", "GetMarketPrices", "GET", true),
    ("ISteamEconomy", "StartAssetTransaction", "POST", true),
    ("ISteamEconomy", "StartTrade", "GET", true),
    ("IEconService", "GetTradeHistory", "GET", false),
    ("IEconService", "FlushInventoryCache", "POST", true),
    ("IEconService", "FlushAssetAppearanceCache", "POST", true),
    ("IEconService", "FlushContextCache", "POST", true),
    ("IEconService", "GetTradeOffers", "GET", false),
    ("IEconService", "GetTradeOffer", "GET", false),
    ("IEconService", "GetTradeOffersSummary", "GET", false),
    ("IEconMarketService", "GetMarketEligibility", "GET", true),
    (
        "IEconMarketService",
        "CancelAppListingsForUser",
        "POST",
        true,
    ),
    ("IEconMarketService", "GetAssetID", "GET", true),
    ("IEconMarketService", "GetPopular", "GET", true),
];

fn client(server: &MockServer, publisher: bool) -> SteamClient {
    let key = if publisher {
        ApiKey::publisher("economy-key").unwrap()
    } else {
        ApiKey::user("economy-key").unwrap()
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

fn history() -> TradeHistoryOptions<'static> {
    TradeHistoryOptions {
        max_trades: 50,
        start_after_time: u32::MAX,
        start_after_tradeid: ID,
        navigating_back: true,
        get_descriptions: true,
        language: TEXT,
        include_failed: true,
        include_total: false,
    }
}

fn offers() -> TradeOffersOptions<'static> {
    TradeOffersOptions {
        get_sent_offers: true,
        get_received_offers: true,
        get_descriptions: true,
        language: TEXT,
        active_only: true,
        historical_only: false,
        time_historical_cutoff: u32::MAX,
    }
}

fn popular() -> PopularMarketOptions<'static> {
    PopularMarketOptions {
        language: TEXT,
        rows: Some(30),
        start: u32::MAX,
        filter_appid: Some(480),
        ecurrency: Some(23),
    }
}

fn purchase<'a>(assets: &'a [AssetPurchase<'a>]) -> StartAssetTransactionRequest<'a> {
    StartAssetTransactionRequest {
        appid: 480,
        steamid: SteamId(ID),
        assets,
        currency: "CNY",
        language: TEXT,
        ipaddress: "2001:db8::1",
        referrer: Some("https://example.com/?source=+&key=中文"),
        clientauth: Some(false),
    }
}

async fn call(client: &SteamClient, name: &str) -> steam_api_web::Result<OpaqueResponse> {
    match name {
        "CanTrade" => client.economy().can_trade(480, ID.into(), 42.into()).await,
        "FinalizeAssetTransaction" => {
            client
                .economy()
                .finalize_asset_transaction(480, ID.into(), TEXT, "english")
                .await
        }
        "GetAssetClassInfo" => {
            client
                .economy()
                .get_asset_class_info(
                    480,
                    &[
                        AssetClass {
                            classid: ID,
                            instanceid: Some(0),
                        },
                        AssetClass {
                            classid: ID - 1,
                            instanceid: Some(ID),
                        },
                        AssetClass {
                            classid: 1,
                            instanceid: None,
                        },
                    ],
                    Some(TEXT),
                )
                .await
        }
        "GetAssetPrices" => {
            client
                .economy()
                .get_asset_prices(
                    480,
                    AssetPriceOptions {
                        currency: Some("CNY"),
                        language: Some(TEXT),
                    },
                )
                .await
        }
        "GetExportedAssetsForUser" => {
            client
                .economy()
                .get_exported_assets_for_user(480, ID.into(), ID)
                .await
        }
        "GetMarketPrices" => client.economy().get_market_prices(480).await,
        "StartAssetTransaction" => {
            client
                .economy()
                .start_asset_transaction(purchase(&[
                    AssetPurchase {
                        assetid: TEXT,
                        quantity: 1,
                    },
                    AssetPurchase {
                        assetid: "00018446744073709551615",
                        quantity: u32::MAX,
                    },
                ]))
                .await
        }
        "StartTrade" => {
            client
                .economy()
                .start_trade(480, ID.into(), 42.into())
                .await
        }
        "GetTradeHistory" => client.econ().get_trade_history(history()).await,
        "FlushInventoryCache" => {
            client
                .econ()
                .flush_inventory_cache(ID.into(), 480, ID)
                .await
        }
        "FlushAssetAppearanceCache" => client.econ().flush_asset_appearance_cache(480).await,
        "FlushContextCache" => client.econ().flush_context_cache(480).await,
        "GetTradeOffers" => client.econ().get_trade_offers(offers()).await,
        "GetTradeOffer" => client.econ().get_trade_offer(ID, TEXT).await,
        "GetTradeOffersSummary" => client.econ().get_trade_offers_summary(Some(u32::MAX)).await,
        "GetMarketEligibility" => client.econ_market().get_market_eligibility(ID.into()).await,
        "CancelAppListingsForUser" => {
            client
                .econ_market()
                .cancel_app_listings_for_user(480, ID.into(), false)
                .await
        }
        "GetAssetID" => client.econ_market().get_asset_id(480, ID).await,
        "GetPopular" => client.econ_market().get_popular(popular()).await,
        _ => panic!("unknown economy test method"),
    }
}

fn form(request: &Request) -> BTreeMap<String, String> {
    if request.method.as_str() == "POST" {
        url::form_urlencoded::parse(&request.body)
            .into_owned()
            .collect()
    } else {
        assert!(request.body.is_empty());
        request.url.query_pairs().into_owned().collect()
    }
}

fn service_input(request: &Request) -> Value {
    let params = form(request);
    assert_eq!(params.len(), 2);
    assert_eq!(params["format"], "json");
    serde_json::from_str(&params["input_json"]).unwrap()
}

#[tokio::test]
async fn every_economy_endpoint_uses_exact_auth_method_version_fields_and_lossless_results() {
    let server = MockServer::start().await;
    let publisher = client(&server, true);
    let user = client(&server, false);
    for (interface, name, verb, needs_publisher) in METHODS {
        Mock::given(method(verb))
            .and(path(format!("/{interface}/{name}/v1/")))
            .and(header("x-webapi-key", "economy-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response": {
                "opaque": TEXT, "integer_price": ID, "decimal_price": "12345678901234567890.000000001"
            }})))
            .expect(1).mount(&server).await;
        let response = call(if needs_publisher { &publisher } else { &user }, name)
            .await
            .unwrap();
        assert_eq!(
            response.as_json()["response"]["integer_price"].as_u64(),
            Some(ID)
        );
        assert_eq!(
            response.as_json()["response"]["decimal_price"],
            "12345678901234567890.000000001"
        );
        assert!(!format!("{response:?}").contains(TEXT));
    }
    for request in server.received_requests().await.unwrap() {
        let params = form(&request);
        assert_eq!(params["format"], "json");
        assert!(!params.contains_key("key"));
        let name = request.url.path().split('/').nth(2).unwrap();
        if request.method.as_str() == "POST" {
            assert!(request.url.query().is_none());
            assert_eq!(
                request.headers.get("content-type").unwrap(),
                "application/x-www-form-urlencoded"
            );
        }
        if request.url.path().starts_with("/ISteamEconomy/") {
            assert!(!params.contains_key("input_json"));
            assert_eq!(params["appid"], "480");
            match name {
                "CanTrade" => {
                    assert_eq!(params["steamid"], ID.to_string());
                    assert_eq!(params["targetid"], "42");
                }
                "FinalizeAssetTransaction" => {
                    assert_eq!(params["steamid"], ID.to_string());
                    assert_eq!(params["txnid"], TEXT);
                    assert_eq!(params["language"], "english");
                }
                "GetAssetClassInfo" => {
                    assert_eq!(params["class_count"], "3");
                    assert_eq!(params["classid0"], ID.to_string());
                    assert_eq!(params["instanceid0"], "0");
                    assert_eq!(params["classid1"], (ID - 1).to_string());
                    assert_eq!(params["instanceid1"], ID.to_string());
                    assert_eq!(params["classid2"], "1");
                    assert!(!params.contains_key("instanceid2"));
                    assert!(!params.keys().any(|key| key.contains('[')));
                }
                "GetAssetPrices" => {
                    assert_eq!(params["currency"], "CNY");
                    assert_eq!(params["language"], TEXT);
                }
                "GetExportedAssetsForUser" => {
                    assert_eq!(params["contextid"], ID.to_string());
                    assert_eq!(params["steamid"], ID.to_string());
                }
                "GetMarketPrices" => assert_eq!(params.len(), 2),
                "StartAssetTransaction" => {
                    assert_eq!(params["assetid0"], TEXT);
                    assert_eq!(params["assetquantity0"], "1");
                    assert_eq!(params["assetid1"], "00018446744073709551615");
                    assert_eq!(params["assetquantity1"], u32::MAX.to_string());
                    assert_eq!(params["currency"], "CNY");
                    assert_eq!(params["ipaddress"], "2001:db8::1");
                    assert_eq!(params["clientauth"], "false");
                    assert_eq!(params["referrer"], "https://example.com/?source=+&key=中文");
                    assert!(!params.contains_key("count"));
                    assert!(!params.contains_key("asset_count"));
                }
                "StartTrade" => {
                    assert_eq!(params["partya"], ID.to_string());
                    assert_eq!(params["partyb"], "42");
                }
                _ => panic!("unknown ordinary form method"),
            }
        } else {
            let input = service_input(&request);
            match name {
                "GetTradeHistory" => {
                    assert_eq!(input["start_after_tradeid"].as_u64(), Some(ID));
                    assert_eq!(input["start_after_time"], u32::MAX);
                    assert_eq!(input["max_trades"], 50);
                    assert_eq!(input["language"], TEXT);
                    assert_eq!(input["navigating_back"], true);
                    assert_eq!(input["get_descriptions"], true);
                    assert_eq!(input["include_failed"], true);
                    assert_eq!(input["include_total"], false);
                }
                "FlushInventoryCache" => assert_eq!(
                    input,
                    json!({"steamid": ID.to_string(), "appid":480, "contextid":ID})
                ),
                "FlushAssetAppearanceCache" | "FlushContextCache" => {
                    assert_eq!(input, json!({"appid":480}))
                }
                "GetTradeOffers" => {
                    assert_eq!(input["get_sent_offers"], true);
                    assert_eq!(input["get_received_offers"], true);
                    assert_eq!(input["get_descriptions"], true);
                    assert_eq!(input["language"], TEXT);
                    assert_eq!(input["active_only"], true);
                    assert_eq!(input["historical_only"], false);
                    assert_eq!(input["time_historical_cutoff"], u32::MAX);
                }
                "GetTradeOffer" => assert_eq!(input, json!({"tradeofferid":ID, "language":TEXT})),
                "GetTradeOffersSummary" => assert_eq!(input, json!({"time_last_visit":u32::MAX})),
                "GetMarketEligibility" => assert_eq!(input, json!({"steamid":ID.to_string()})),
                "CancelAppListingsForUser" => assert_eq!(
                    input,
                    json!({"appid":480, "steamid":ID.to_string(), "synchronous":false})
                ),
                "GetAssetID" => assert_eq!(input, json!({"appid":480, "listingid":ID})),
                "GetPopular" => assert_eq!(
                    input,
                    json!({"language":TEXT, "rows":30, "start":u32::MAX, "filter_appid":480, "ecurrency":23})
                ),
                _ => panic!("unknown service method"),
            }
        }
    }
}

#[tokio::test]
async fn seven_mutations_including_get_start_trade_are_never_automatically_replayed() {
    let server = MockServer::start().await;
    let client = client(&server, true);
    for (interface, name, verb, _) in METHODS
        .into_iter()
        .filter(|(_, name, verb, _)| *verb == "POST" || *name == "StartTrade")
    {
        Mock::given(method(verb))
            .and(path(format!("/{interface}/{name}/v1/")))
            .respond_with(ResponseTemplate::new(503).insert_header("retry-after", "0"))
            .expect(1)
            .mount(&server)
            .await;
        assert!(matches!(
            call(&client, name).await,
            Err(Error::Http { status: 503, .. })
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
            ResponseTemplate::new(200).set_body_json(json!({"response": {}}))
        }
    }
}

#[tokio::test]
async fn twelve_observational_reads_can_retry_temporary_failures() {
    let server = MockServer::start().await;
    let client = client(&server, true);
    for (interface, name, verb, _) in METHODS
        .into_iter()
        .filter(|(_, name, verb, _)| *verb == "GET" && *name != "StartTrade")
    {
        Mock::given(method(verb))
            .and(path(format!("/{interface}/{name}/v1/")))
            .respond_with(TemporaryFailure(AtomicUsize::new(0)))
            .expect(2)
            .mount(&server)
            .await;
        call(&client, name).await.unwrap();
    }
    assert_eq!(client.metrics().attempts, 24);
    assert_eq!(client.metrics().retries, 12);
}

#[tokio::test]
async fn anonymous_and_user_keys_cannot_reach_protected_methods() {
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
async fn all_nineteen_opaque_methods_reject_business_errors_without_leaking_response_text() {
    let server = MockServer::start().await;
    let client = client(&server, true);
    for (interface, name, verb, _) in METHODS {
        Mock::given(method(verb))
            .and(path(format!("/{interface}/{name}/v1/")))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"response":{"params":{"result":15, "secret":TEXT}}})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let error = call(&client, name).await.unwrap_err();
        assert!(matches!(error, Error::Api { code: Some(15), .. }));
        assert!(!format!("{error:?} {error}").contains(TEXT));
    }
    assert_eq!(client.metrics().retries, 0);
}

#[tokio::test]
async fn all_nineteen_methods_reject_malformed_json_without_leaks_or_retries() {
    let server = MockServer::start().await;
    let client = client(&server, true);
    for (interface, name, verb, _) in METHODS {
        Mock::given(method(verb))
            .and(path(format!("/{interface}/{name}/v1/")))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(format!("{{\"response\":\"{TEXT}")),
            )
            .expect(1)
            .mount(&server)
            .await;
        let error = call(&client, name).await.unwrap_err();
        assert!(matches!(error, Error::Decode { .. }));
        assert!(!format!("{error:?} {error}").contains(TEXT));
    }
    assert_eq!(client.metrics().attempts, 19);
    assert_eq!(client.metrics().retries, 0);
}

#[tokio::test]
async fn optional_defaults_and_initial_and_reverse_history_cursors_are_preserved() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response":{"cursor":TEXT}})))
        .expect(8)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response":{}})))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server, true);
    let mut options = history();
    options.start_after_time = 0;
    options.start_after_tradeid = 0;
    options.navigating_back = false;
    let first = client.econ().get_trade_history(options).await.unwrap();
    assert_eq!(first.as_json()["response"]["cursor"], TEXT);
    client.econ().get_trade_history(history()).await.unwrap();
    client.econ().get_trade_offers_summary(None).await.unwrap();
    client
        .econ()
        .get_trade_offers_summary(Some(0))
        .await
        .unwrap();
    let mut popular = popular();
    popular.start = 0;
    popular.rows = None;
    popular.filter_appid = None;
    popular.ecurrency = None;
    client.econ_market().get_popular(popular).await.unwrap();
    client
        .economy()
        .get_asset_prices(480, AssetPriceOptions::default())
        .await
        .unwrap();
    client
        .economy()
        .get_asset_class_info(
            480,
            &[AssetClass {
                classid: 1,
                instanceid: None,
            }],
            None,
        )
        .await
        .unwrap();
    options.start_after_time = 1;
    options.start_after_tradeid = ID;
    client.econ().get_trade_history(options).await.unwrap();
    let assets = [AssetPurchase {
        assetid: "0001",
        quantity: 1,
    }];
    let mut request = purchase(&assets);
    request.referrer = None;
    request.clientauth = None;
    client
        .economy()
        .start_asset_transaction(request)
        .await
        .unwrap();
    let requests = server.received_requests().await.unwrap();
    assert_eq!(service_input(&requests[0])["start_after_tradeid"], 0);
    assert_eq!(service_input(&requests[0])["start_after_time"], 0);
    assert_eq!(service_input(&requests[1])["navigating_back"], true);
    assert_eq!(
        service_input(&requests[1])["start_after_tradeid"].as_u64(),
        Some(ID)
    );
    assert_eq!(service_input(&requests[2]), json!({}));
    assert_eq!(service_input(&requests[3]), json!({"time_last_visit":0}));
    assert_eq!(
        service_input(&requests[4]),
        json!({"language":TEXT, "start":0})
    );
    assert_eq!(form(&requests[5]).len(), 2);
    assert!(!form(&requests[6]).contains_key("language"));
    assert!(!form(&requests[6]).contains_key("instanceid0"));
    assert_eq!(
        service_input(&requests[7])["start_after_tradeid"].as_u64(),
        Some(ID)
    );
    assert!(!form(&requests[8]).contains_key("referrer"));
    assert!(!form(&requests[8]).contains_key("clientauth"));
}

#[tokio::test]
async fn invalid_ids_arrays_quantities_counts_filters_and_languages_fail_before_network() {
    let server = MockServer::start().await;
    let client = client(&server, true);
    let economy = client.economy();
    assert!(economy.can_trade(0, ID.into(), 1.into()).await.is_err());
    assert!(economy.can_trade(480, ID.into(), 0.into()).await.is_err());
    assert!(
        economy
            .finalize_asset_transaction(480, ID.into(), "", "english")
            .await
            .is_err()
    );
    assert!(
        economy
            .finalize_asset_transaction(480, 0.into(), TEXT, "english")
            .await
            .is_err()
    );
    assert!(economy.get_asset_class_info(480, &[], None).await.is_err());
    assert!(
        economy
            .get_asset_class_info(
                480,
                &[AssetClass {
                    classid: 0,
                    instanceid: None
                }],
                None
            )
            .await
            .is_err()
    );
    assert!(
        economy
            .get_asset_prices(
                480,
                AssetPriceOptions {
                    currency: Some("USD\0"),
                    language: None
                }
            )
            .await
            .is_err()
    );
    assert!(
        economy
            .get_exported_assets_for_user(480, ID.into(), 0)
            .await
            .is_err()
    );
    assert!(economy.get_market_prices(0).await.is_err());
    assert!(
        economy
            .start_trade(480, ID.into(), ID.into())
            .await
            .is_err()
    );
    assert!(
        economy
            .start_asset_transaction(purchase(&[]))
            .await
            .is_err()
    );
    assert!(
        economy
            .start_asset_transaction(purchase(&[AssetPurchase {
                assetid: TEXT,
                quantity: 0
            }]))
            .await
            .is_err()
    );
    assert!(
        economy
            .start_asset_transaction(purchase(&[AssetPurchase {
                assetid: "\0",
                quantity: 1
            }]))
            .await
            .is_err()
    );
    let assets = [AssetPurchase {
        assetid: TEXT,
        quantity: 1,
    }];
    let mut request = purchase(&assets);
    request.ipaddress = "example.com:80";
    assert!(economy.start_asset_transaction(request).await.is_err());
    request = purchase(&assets);
    request.currency = " ";
    assert!(economy.start_asset_transaction(request).await.is_err());
    let econ = client.econ();
    let mut h = history();
    h.max_trades = 0;
    assert!(econ.get_trade_history(h).await.is_err());
    h = history();
    h.language = "\0";
    assert!(econ.get_trade_history(h).await.is_err());
    assert!(econ.flush_inventory_cache(ID.into(), 480, 0).await.is_err());
    assert!(econ.flush_asset_appearance_cache(0).await.is_err());
    assert!(econ.flush_context_cache(0).await.is_err());
    assert!(econ.get_trade_offer(0, "english").await.is_err());
    let market = client.econ_market();
    assert!(market.get_market_eligibility(0.into()).await.is_err());
    assert!(
        market
            .cancel_app_listings_for_user(0, ID.into(), true)
            .await
            .is_err()
    );
    assert!(market.get_asset_id(480, 0).await.is_err());
    let mut p = popular();
    p.rows = Some(0);
    assert!(market.get_popular(p).await.is_err());
    p = popular();
    p.filter_appid = Some(0);
    assert!(market.get_popular(p).await.is_err());
    p = popular();
    p.ecurrency = Some(0);
    assert!(market.get_popular(p).await.is_err());
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn trade_offer_filters_preserve_combinations_without_invented_exclusions() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/IEconService/GetTradeOffers/v1/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response": {}})))
        .expect(3)
        .mount(&server)
        .await;
    let client = client(&server, false);
    let combinations = [
        (true, true, true, true),
        (true, false, false, false),
        (false, false, false, false),
    ];
    for (active, historical, sent, received) in combinations {
        let options = TradeOffersOptions {
            active_only: active,
            historical_only: historical,
            get_sent_offers: sent,
            get_received_offers: received,
            ..offers()
        };
        client.econ().get_trade_offers(options).await.unwrap();
    }
    for (request, (active, historical, sent, received)) in server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .zip(combinations)
    {
        let input = service_input(request);
        assert_eq!(input["active_only"], active);
        assert_eq!(input["historical_only"], historical);
        assert_eq!(input["get_sent_offers"], sent);
        assert_eq!(input["get_received_offers"], received);
        assert_eq!(input["time_historical_cutoff"], u32::MAX);
    }
}
