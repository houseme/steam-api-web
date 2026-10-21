// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Publisher inventory wire, mutation, and error contracts against local servers.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::{Value, json};
use std::{collections::BTreeMap, time::Duration};
use steam_api_web::{
    ApiKey, Error, RetryPolicy, SteamClient, SteamId,
    game_inventory::{
        AssetHistoryRequest, HistoryCommandDetailsRequest, HistoryExecuteCommandsRequest,
        UpdateItemDefsRequest, UserHistoryRequest,
    },
    inventory::{
        AddItemRequest, AddPromoItemRequest, ConsumeItemRequest, DynamicPropertyUpdate,
        DynamicPropertyValue, ExchangeItemRequest, ItemDefsOptions, ItemTypesRequest,
        ModifyItemsRequest,
    },
    user_auth::OpaqueResponse,
};
use wiremock::{
    Mock, MockServer, Request, ResponseTemplate,
    matchers::{header, method, path},
};

const ID: SteamId = SteamId(u64::MAX);
const METHODS: [(&str, &str, &str); 15] = [
    ("IInventoryService", "AddItem", "POST"),
    ("IInventoryService", "AddPromoItem", "POST"),
    ("IInventoryService", "ConsumeItem", "POST"),
    ("IInventoryService", "ExchangeItem", "POST"),
    ("IInventoryService", "GetInventory", "GET"),
    ("IInventoryService", "GetItemDefs", "GET"),
    ("IInventoryService", "GetPriceSheet", "GET"),
    ("IInventoryService", "Consolidate", "POST"),
    ("IInventoryService", "GetQuantity", "GET"),
    ("IInventoryService", "ModifyItems", "POST"),
    ("IGameInventory", "GetHistoryCommandDetails", "GET"),
    ("IGameInventory", "GetUserHistory", "GET"),
    ("IGameInventory", "HistoryExecuteCommands", "POST"),
    ("IGameInventory", "SupportGetAssetHistory", "GET"),
    ("IGameInventory", "UpdateItemDefs", "POST"),
];

fn endpoint(interface: &str, name: &str) -> String {
    if name == "UpdateItemDefs" {
        format!("/{interface}/{name}/v0001")
    } else {
        format!("/{interface}/{name}/v1/")
    }
}

fn client(server: &MockServer, publisher: bool) -> SteamClient {
    let key = if publisher {
        ApiKey::publisher("inventory-key")
    } else {
        ApiKey::user("inventory-key")
    };
    SteamClient::builder()
        .base_url(server.uri())
        .api_key(key.unwrap())
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
        assert!(request.url.query().is_none());
        url::form_urlencoded::parse(&request.body)
            .into_owned()
            .collect()
    } else {
        assert!(request.body.is_empty());
        request.url.query_pairs().into_owned().collect()
    }
}

fn input(request: &Request) -> Value {
    serde_json::from_str(&form(request)["input_json"]).unwrap()
}

fn add() -> AddItemRequest<'static> {
    AddItemRequest {
        appid: 480,
        steamid: ID,
        itemdefid: &[u64::MAX, 2],
        itempropsjson: Some("{\"label\":\"中文 & + =\"}"),
        notify: Some(false),
        requestid: Some(u64::MAX),
        trade_restriction: Some(true),
    }
}

fn types() -> ItemTypesRequest<'static> {
    ItemTypesRequest {
        appid: 480,
        steamid: ID,
        itemdefid: &[u64::MAX, 2],
        force: Some(false),
    }
}

async fn call(client: &SteamClient, name: &str) -> steam_api_web::Result<OpaqueResponse> {
    match name {
        "AddItem" => client.inventory().add_item(&add()).await,
        "AddPromoItem" => client.inventory().add_promo_item(&AddPromoItemRequest {appid:480, steamid:ID, itemdefid:Some(u64::MAX), notify:Some(true), requestid:Some(u64::MAX)}).await,
        "ConsumeItem" => client.inventory().consume_item(&ConsumeItemRequest {appid:480, steamid:ID, itemid:u64::MAX, quantity:u64::MAX, requestid:u64::MAX}).await,
        "ExchangeItem" => client.inventory().exchange_item(&ExchangeItemRequest {appid:480, steamid:ID, materialsitemid:&[u64::MAX,2], materialsquantity:&[1,u32::MAX], outputitemdefid:u64::MAX}).await,
        "GetInventory" => client.inventory().get_inventory(480,ID).await,
        "GetItemDefs" => client.inventory().get_item_defs(480,ItemDefsOptions {modifiedsince:Some("20240229T235959Z"), itemdefids:Some(u64::MAX), workshopids:Some(u64::MAX), cache_max_age_seconds:Some(0)}).await,
        "GetPriceSheet" => client.inventory().get_price_sheet(i32::MIN).await,
        "Consolidate" => client.inventory().consolidate(&types()).await,
        "GetQuantity" => client.inventory().get_quantity(&types()).await,
        "ModifyItems" => client.inventory().modify_items(&ModifyItemsRequest {appid:480, steamid:ID, timestamp:u32::MAX, updates:&[
            DynamicPropertyUpdate {itemid:u64::MAX, property_name:"score", value:DynamicPropertyValue::Integer(i64::MIN)},
            DynamicPropertyUpdate {itemid:u64::MAX, property_name:"clear", value:DynamicPropertyValue::Remove},
            DynamicPropertyUpdate {itemid:u64::MAX, property_name:"text", value:DynamicPropertyValue::String("中文\0&+=")},
            DynamicPropertyUpdate {itemid:u64::MAX, property_name:"boolean", value:DynamicPropertyValue::Boolean(false)},
            DynamicPropertyUpdate {itemid:u64::MAX, property_name:"fraction", value:DynamicPropertyValue::Float(-1.25)},
        ]}).await,
        "GetHistoryCommandDetails" => client.game_inventory().get_history_command_details(&HistoryCommandDetailsRequest {appid:480, steamid:ID, command:"grant &+=", contextid:u64::MAX, arguments:""}).await,
        "GetUserHistory" => client.game_inventory().get_user_history(&UserHistoryRequest {appid:480, steamid:ID, contextid:u64::MAX, starttime:0, endtime:u32::MAX}).await,
        "HistoryExecuteCommands" => client.game_inventory().history_execute_commands(&HistoryExecuteCommandsRequest {appid:480, steamid:ID, contextid:u64::MAX, actorid:u32::MAX}).await,
        "SupportGetAssetHistory" => client.game_inventory().support_get_asset_history(&AssetHistoryRequest {appid:480, assetid:u64::MAX, contextid:u64::MAX}).await,
        "UpdateItemDefs" => client.game_inventory().update_item_defs(&UpdateItemDefsRequest {appid:480,itemdefs:&[json!({"appid":"480","itemdefid":u64::MAX.to_string(),"name":"中文 &+=","tradable":true})]}).await,
        _ => panic!("unknown inventory test method"),
    }
}

#[tokio::test]
async fn all_fifteen_methods_use_exact_paths_verbs_and_publisher_credentials() {
    let server = MockServer::start().await;
    for (interface, name, verb) in METHODS {
        let mock = Mock::given(method(verb))
            .and(path(endpoint(interface, name)))
            .and(header("x-webapi-key", "inventory-key"));
        let mock = if verb == "POST" {
            mock.and(header("content-type", "application/x-www-form-urlencoded"))
        } else {
            mock
        };
        mock.respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"response":{"future":"preserved","item_json":"[]"}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    }
    let client = client(&server, true);
    for (_, name, _) in METHODS {
        let result = call(&client, name).await.unwrap();
        assert_eq!(result.as_json()["response"]["future"], "preserved");
        assert_eq!(format!("{result:?}"), "OpaqueResponse([REDACTED])");
    }
    for request in server.received_requests().await.unwrap() {
        let form = form(&request);
        assert_eq!(form["format"], "json");
        assert!(!form.contains_key("key"));
        let name = request.url.path().split('/').nth(2).unwrap();
        if request.url.path().contains("IInventoryService") {
            assert!(form.contains_key("input_json"));
            if name == "ModifyItems" {
                assert_eq!(form["appid"], "480");
                assert!(input(&request).get("appid").is_none());
            } else {
                assert_eq!(form.len(), 2);
                if name == "GetPriceSheet" {
                    assert!(input(&request).get("appid").is_none());
                } else {
                    assert_eq!(input(&request)["appid"], 480);
                }
            }
        } else {
            assert!(!form.contains_key("input_json"));
            assert_eq!(form["appid"], "480");
        }
    }
}

#[tokio::test]
async fn service_json_preserves_ids_deduplication_flags_arrays_and_property_operations() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response":{}})))
        .expect(6)
        .mount(&server)
        .await;
    let client = client(&server, true);
    for name in [
        "AddItem",
        "AddPromoItem",
        "ConsumeItem",
        "ExchangeItem",
        "Consolidate",
        "ModifyItems",
    ] {
        call(&client, name).await.unwrap();
    }
    let requests = server.received_requests().await.unwrap();
    let add = input(&requests[0]);
    assert_eq!(add["steamid"], u64::MAX.to_string());
    assert_eq!(add["itemdefid"][0].as_u64(), Some(u64::MAX));
    assert_eq!(add["requestid"].as_u64(), Some(u64::MAX));
    assert_eq!(add["itempropsjson"], "{\"label\":\"中文 & + =\"}");
    assert_eq!(add["notify"], false);
    assert_eq!(add["trade_restriction"], true);
    assert_eq!(input(&requests[1])["requestid"].as_u64(), Some(u64::MAX));
    let consume = input(&requests[2]);
    assert_eq!(consume["quantity"], u64::MAX.to_string());
    assert_eq!(consume["itemid"].as_u64(), Some(u64::MAX));
    assert_eq!(consume["requestid"].as_u64(), Some(u64::MAX));
    let exchange = input(&requests[3]);
    assert_eq!(exchange["materialsitemid"], json!([u64::MAX, 2]));
    assert_eq!(exchange["materialsquantity"], json!([1, u32::MAX]));
    assert_eq!(exchange["outputitemdefid"].as_u64(), Some(u64::MAX));
    assert_eq!(input(&requests[4])["force"], false);
    let modify = input(&requests[5]);
    assert_eq!(modify["timestamp"].as_u64(), Some(u64::from(u32::MAX)));
    assert_eq!(
        modify["updates"][0]["property_value_int"].as_i64(),
        Some(i64::MIN)
    );
    assert_eq!(modify["updates"][1]["remove_property"], true);
    assert_eq!(modify["updates"][2]["property_value_string"], "中文\0&+=");
    assert_eq!(modify["updates"][3]["property_value_bool"], false);
    assert_eq!(modify["updates"][4]["property_value_float"], "-1.25");
    for update in modify["updates"].as_array().unwrap() {
        assert_eq!(update.as_object().unwrap().len(), 3);
    }
}

#[tokio::test]
async fn read_filters_and_game_inventory_arguments_retain_their_documented_shapes() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response":{}})))
        .expect(7)
        .mount(&server)
        .await;
    let client = client(&server, true);
    for name in [
        "GetInventory",
        "GetItemDefs",
        "GetPriceSheet",
        "GetQuantity",
        "GetHistoryCommandDetails",
        "GetUserHistory",
        "SupportGetAssetHistory",
    ] {
        call(&client, name).await.unwrap();
    }
    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        input(&requests[0]),
        json!({"appid":480,"steamid":u64::MAX.to_string()})
    );
    assert_eq!(
        input(&requests[1]),
        json!({"appid":480,"modifiedsince":"20240229T235959Z","itemdefids":u64::MAX,"workshopids":u64::MAX,"cache_max_age_seconds":0})
    );
    assert_eq!(input(&requests[2]), json!({"ecurrency":i32::MIN}));
    assert_eq!(input(&requests[3])["itemdefid"], json!([u64::MAX, 2]));
    let history = form(&requests[4]);
    assert_eq!(history["contextid"], u64::MAX.to_string());
    assert_eq!(history["steamid"], u64::MAX.to_string());
    assert_eq!(history["arguments"], "");
    assert_eq!(history["command"], "grant &+=");
    let user = form(&requests[5]);
    assert_eq!(user["starttime"], "0");
    assert_eq!(user["endtime"], u32::MAX.to_string());
    assert_eq!(form(&requests[6])["assetid"], u64::MAX.to_string());
}

#[tokio::test]
async fn optional_flags_are_omitted_without_inventing_idempotency_identifiers() {
    let server = MockServer::start().await;
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
    let client = client(&server, true);
    client
        .inventory()
        .add_item(&AddItemRequest {
            itempropsjson: None,
            notify: None,
            requestid: None,
            trade_restriction: None,
            ..add()
        })
        .await
        .unwrap();
    client
        .inventory()
        .add_promo_item(&AddPromoItemRequest {
            appid: 480,
            steamid: ID,
            itemdefid: None,
            notify: None,
            requestid: None,
        })
        .await
        .unwrap();
    client
        .inventory()
        .consolidate(&ItemTypesRequest {
            force: None,
            ..types()
        })
        .await
        .unwrap();
    client
        .inventory()
        .get_item_defs(480, ItemDefsOptions::default())
        .await
        .unwrap();
    let requests = server.received_requests().await.unwrap();
    assert_eq!(input(&requests[0]).as_object().unwrap().len(), 3);
    assert_eq!(
        input(&requests[1]),
        json!({"appid":480,"steamid":u64::MAX.to_string()})
    );
    assert!(input(&requests[2]).get("force").is_none());
    assert_eq!(input(&requests[3]), json!({"appid":480}));
}

#[tokio::test]
async fn every_write_is_sent_once_while_reads_use_the_shared_bounded_retry_policy() {
    for (interface, name, verb) in METHODS {
        let server = MockServer::start().await;
        let attempts = if verb == "GET" { 3 } else { 1 };
        Mock::given(method(verb))
            .and(path(endpoint(interface, name)))
            .respond_with(ResponseTemplate::new(503))
            .expect(attempts)
            .mount(&server)
            .await;
        let client = client(&server, true);
        assert!(
            matches!(
                call(&client, name).await,
                Err(Error::Http { status: 503, .. })
            ),
            "{name}"
        );
        assert_eq!(client.metrics().attempts, attempts);
    }
}

#[tokio::test]
async fn every_method_requires_a_publisher_key_before_network_io() {
    let server = MockServer::start().await;
    let client = client(&server, false);
    for (_, name, _) in METHODS {
        assert!(
            matches!(
                call(&client, name).await,
                Err(Error::MissingApiKey("publisher"))
            ),
            "{name}"
        );
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn every_method_rejects_nested_steam_business_errors_without_replaying_or_leaking_text() {
    for (interface, name, verb) in METHODS {
        let server = MockServer::start().await;
        let body = if name == "UpdateItemDefs" {
            json!({"result":{"success":false,"error":"private-provider-text"}})
        } else {
            json!({"response":{"success":false,"error":"private-provider-text"}})
        };
        Mock::given(method(verb))
            .and(path(endpoint(interface, name)))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .expect(1)
            .mount(&server)
            .await;
        let error = call(&client(&server, true), name).await.unwrap_err();
        assert!(matches!(error, Error::Api { .. }), "{name}: {error:?}");
        let printed = format!("{error} {error:?}");
        for secret in ["private-provider-text", "inventory-key"] {
            assert!(!printed.contains(secret));
        }
    }
}

#[tokio::test]
async fn definition_updates_preserve_object_result_envelope_and_partial_failure_ids() {
    let server = MockServer::start().await;
    let body = json!({"result":{"updated":[u64::MAX.to_string()],"failed":["2"],"success":true}});
    Mock::given(path("/IGameInventory/UpdateItemDefs/v0001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .expect(1)
        .mount(&server)
        .await;
    let response = call(&client(&server, true), "UpdateItemDefs")
        .await
        .unwrap();
    assert_eq!(response.as_json(), &body);
    assert_eq!(response.as_json()["result"]["failed"], json!(["2"]));
    let requests = server.received_requests().await.unwrap();
    let wire = form(&requests[0]);
    let definitions: Value = serde_json::from_str(&wire["itemdefs"]).unwrap();
    assert_eq!(definitions[0]["itemdefid"], u64::MAX.to_string());
    assert_eq!(definitions[0]["name"], "中文 &+=");
    assert_eq!(definitions[0]["tradable"], true);
}

#[tokio::test]
async fn malformed_response_roots_fail_and_encoded_empty_item_arrays_remain_explicit() {
    for body in ["{broken", "null", "[]"] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .expect(1)
            .mount(&server)
            .await;
        assert!(matches!(
            call(&client(&server, true), "ExchangeItem").await,
            Err(Error::Decode { .. })
        ));
    }
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"response":{"item_json":"[]"}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let result = call(&client(&server, true), "ExchangeItem").await.unwrap();
    assert_eq!(result.as_json()["response"]["item_json"], "[]");
}

#[tokio::test]
async fn invalid_inventory_requests_fail_before_network() {
    let server = MockServer::start().await;
    let client = client(&server, true);
    for request in [
        AddItemRequest {
            itemdefid: &[],
            ..add()
        },
        AddItemRequest {
            itemdefid: &[0],
            ..add()
        },
        AddItemRequest {
            itempropsjson: Some("secret-invalid-json"),
            ..add()
        },
    ] {
        assert!(client.inventory().add_item(&request).await.is_err());
    }
    assert!(
        client
            .inventory()
            .consume_item(&ConsumeItemRequest {
                appid: 480,
                steamid: ID,
                itemid: 1,
                quantity: 0,
                requestid: 1
            })
            .await
            .is_err()
    );
    for (ids, quantities) in [
        (&[1_u64, 2][..], &[1_u32][..]),
        (&[1, 1][..], &[1, 1][..]),
        (&[1][..], &[0][..]),
    ] {
        assert!(
            client
                .inventory()
                .exchange_item(&ExchangeItemRequest {
                    appid: 480,
                    steamid: ID,
                    materialsitemid: ids,
                    materialsquantity: quantities,
                    outputitemdefid: 1
                })
                .await
                .is_err()
        );
    }
    for value in [
        "20230229T000000Z",
        "20240101T240000Z",
        "20240101T000060Z",
        "20241301T000000Z",
        "00000101T000000Z",
        "2024-01-01",
    ] {
        assert!(
            client
                .inventory()
                .get_item_defs(
                    480,
                    ItemDefsOptions {
                        modifiedsince: Some(value),
                        ..Default::default()
                    }
                )
                .await
                .is_err()
        );
    }
    assert!(
        client
            .inventory()
            .get_quantity(&ItemTypesRequest {
                itemdefid: &[],
                ..types()
            })
            .await
            .is_err()
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn property_updates_validate_conflicts_finite_floats_and_distinct_item_limit() {
    let server = MockServer::start().await;
    let client = client(&server, true);
    let update = DynamicPropertyUpdate {
        itemid: 1,
        property_name: "score",
        value: DynamicPropertyValue::Integer(1),
    };
    for updates in [
        vec![],
        vec![update, update],
        vec![DynamicPropertyUpdate {
            value: DynamicPropertyValue::Float(f32::NAN),
            ..update
        }],
        vec![DynamicPropertyUpdate {
            itemid: 0,
            ..update
        }],
        vec![DynamicPropertyUpdate {
            property_name: "",
            ..update
        }],
    ] {
        assert!(
            client
                .inventory()
                .modify_items(&ModifyItemsRequest {
                    appid: 480,
                    steamid: ID,
                    timestamp: 1,
                    updates: &updates
                })
                .await
                .is_err()
        );
    }
    let too_many: Vec<_> = (1..=101)
        .map(|itemid| DynamicPropertyUpdate { itemid, ..update })
        .collect();
    assert!(
        client
            .inventory()
            .modify_items(&ModifyItemsRequest {
                appid: 480,
                steamid: ID,
                timestamp: 1,
                updates: &too_many
            })
            .await
            .is_err()
    );
    assert!(server.received_requests().await.unwrap().is_empty());
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response":{}})))
        .expect(2)
        .mount(&server)
        .await;
    client
        .inventory()
        .modify_items(&ModifyItemsRequest {
            appid: 480,
            steamid: ID,
            timestamp: 1,
            updates: &too_many[..100],
        })
        .await
        .unwrap();
    let names: Vec<_> = (0..101).map(|index| format!("p{index}")).collect();
    let many_properties: Vec<_> = names
        .iter()
        .map(|name| DynamicPropertyUpdate {
            property_name: name,
            ..update
        })
        .collect();
    client
        .inventory()
        .modify_items(&ModifyItemsRequest {
            appid: 480,
            steamid: ID,
            timestamp: 1,
            updates: &many_properties,
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn history_ranges_and_definition_json_are_validated_before_mutation() {
    let server = MockServer::start().await;
    let client = client(&server, true);
    assert!(
        client
            .game_inventory()
            .get_user_history(&UserHistoryRequest {
                appid: 480,
                steamid: ID,
                contextid: 2,
                starttime: 2,
                endtime: 1
            })
            .await
            .is_err()
    );
    for definitions in [
        vec![],
        vec![Value::Null],
        vec![json!({"appid":"481"})],
        vec![json!({"appid":1.5})],
    ] {
        assert!(
            client
                .game_inventory()
                .update_item_defs(&UpdateItemDefsRequest {
                    appid: 480,
                    itemdefs: &definitions
                })
                .await
                .is_err()
        );
    }
    assert!(
        client
            .game_inventory()
            .get_history_command_details(&HistoryCommandDetailsRequest {
                appid: 480,
                steamid: ID,
                contextid: 2,
                command: "",
                arguments: ""
            })
            .await
            .is_err()
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}
