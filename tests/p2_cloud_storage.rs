// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Cloud OAuth stages and legacy Workshop wire contracts against local servers.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::{Value, json};
use std::{collections::BTreeMap, time::Duration};
use steam_api_web::{
    ApiKey, Error, Result, RetryPolicy, SteamClient,
    cloud::{
        BeginAppUploadBatchRequest, BeginHttpUploadRequest, CloudBatchResult, CloudPlatform,
        CommitHttpUploadRequest, CompleteAppUploadBatchRequest, DeleteCloudFileRequest,
        EnumerateUserFilesRequest, OAuthAccessToken,
    },
    remote_storage::{
        EnumerateSubscribedFilesRequest, PublishedFileIdsRequest, PublishedFileSubscriptionRequest,
        SetUgcUsedByGcRequest, UgcFileDetailsRequest,
    },
    user_auth::OpaqueResponse,
};
use wiremock::{
    Mock, MockServer, Request, ResponseTemplate,
    matchers::{header, method, path},
};

const TOKEN: &str = "secret-token&=+%中文";
const FILE: &str = "%WinAppDataLocal%/game/save 中文&=+.sav";
const SHA: &str = "0123456789abcdef0123456789abcdefABCDEF01";
const CLOUD_METHODS: [(&str, &str); 6] = [
    ("EnumerateUserFiles", "GET"),
    ("BeginAppUploadBatch", "POST"),
    ("BeginHTTPUpload", "POST"),
    ("CommitHTTPUpload", "POST"),
    ("Delete", "POST"),
    ("CompleteAppUploadBatch", "POST"),
];
const STORAGE_METHODS: [(&str, &str); 7] = [
    ("EnumerateUserSubscribedFiles", "POST"),
    ("GetCollectionDetails", "POST"),
    ("GetPublishedFileDetails", "POST"),
    ("GetUGCFileDetails", "GET"),
    ("SetUGCUsedByGC", "POST"),
    ("SubscribePublishedFile", "POST"),
    ("UnsubscribePublishedFile", "POST"),
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

fn input(request: &Request) -> Value {
    serde_json::from_str(form(request).get("input_json").unwrap()).unwrap()
}

fn enumeration() -> EnumerateUserFilesRequest {
    EnumerateUserFilesRequest {
        appid: 480,
        extended_details: Some(true),
        count: Some(500),
        start_index: Some(0),
    }
}

fn begin_upload() -> BeginHttpUploadRequest<'static> {
    BeginHttpUploadRequest {
        appid: 480,
        file_size: 0,
        filename: FILE,
        file_sha: SHA,
        platforms_to_sync: &[CloudPlatform::Windows, CloudPlatform::MacOS],
        upload_batch_id: u64::MAX,
        is_public: Some(false),
    }
}

fn subscription() -> PublishedFileSubscriptionRequest {
    PublishedFileSubscriptionRequest {
        steamid: u64::MAX.into(),
        appid: 480,
        publishedfileid: u64::MAX,
    }
}

async fn cloud_call(client: &SteamClient, name: &str) -> Result<()> {
    let token = OAuthAccessToken::new(TOKEN)?;
    match name {
        "EnumerateUserFiles" => client
            .cloud()
            .enumerate_user_files(&token, &enumeration())
            .await
            .map(|_| ()),
        "BeginAppUploadBatch" => client
            .cloud()
            .begin_app_upload_batch(
                &token,
                &BeginAppUploadBatchRequest {
                    appid: 480,
                    machine_name: "Handheld",
                    files_to_upload: &[FILE],
                    files_to_delete: &["old.sav"],
                },
            )
            .await
            .map(|_| ()),
        "BeginHTTPUpload" => client
            .cloud()
            .begin_http_upload(&token, &begin_upload())
            .await
            .map(|_| ()),
        "CommitHTTPUpload" => client
            .cloud()
            .commit_http_upload(
                &token,
                &CommitHttpUploadRequest {
                    appid: 480,
                    transfer_succeeded: false,
                    filename: FILE,
                    file_sha: SHA,
                },
            )
            .await
            .map(|_| ()),
        "Delete" => {
            client
                .cloud()
                .delete(
                    &token,
                    &DeleteCloudFileRequest {
                        appid: 480,
                        filename: "old.sav",
                    },
                )
                .await
        }
        "CompleteAppUploadBatch" => {
            client
                .cloud()
                .complete_app_upload_batch(
                    &token,
                    &CompleteAppUploadBatchRequest {
                        appid: 480,
                        batch_id: u64::MAX,
                        batch_eresult: CloudBatchResult::Failure,
                    },
                )
                .await
        }
        _ => unreachable!(),
    }
}

async fn storage_call(client: &SteamClient, name: &str) -> Result<OpaqueResponse> {
    match name {
        "EnumerateUserSubscribedFiles" => {
            client
                .remote_storage()
                .enumerate_user_subscribed_files(&EnumerateSubscribedFilesRequest {
                    steamid: u64::MAX.into(),
                    appid: 480,
                    listtype: Some(u32::MAX),
                })
                .await
        }
        "GetCollectionDetails" => {
            client
                .remote_storage()
                .get_collection_details(PublishedFileIdsRequest {
                    publishedfileids: &[u64::MAX, 42],
                })
                .await
        }
        "GetPublishedFileDetails" => {
            client
                .remote_storage()
                .get_published_file_details(PublishedFileIdsRequest {
                    publishedfileids: &[u64::MAX, 42],
                })
                .await
        }
        "GetUGCFileDetails" => {
            client
                .remote_storage()
                .get_ugc_file_details(&UgcFileDetailsRequest {
                    ugcid: u64::MAX,
                    appid: 480,
                    steamid: Some(u64::MAX.into()),
                })
                .await
        }
        "SetUGCUsedByGC" => {
            client
                .remote_storage()
                .set_ugc_used_by_gc(&SetUgcUsedByGcRequest {
                    steamid: u64::MAX.into(),
                    ugcid: u64::MAX,
                    appid: 480,
                    used: false,
                })
                .await
        }
        "SubscribePublishedFile" => {
            client
                .remote_storage()
                .subscribe_published_file(&subscription())
                .await
        }
        "UnsubscribePublishedFile" => {
            client
                .remote_storage()
                .unsubscribe_published_file(&subscription())
                .await
        }
        _ => unreachable!(),
    }
}

#[tokio::test]
async fn cloud_all_six_stages_preserve_oauth_metadata_and_file_outcomes() {
    let server = MockServer::start().await;
    let storage = MockServer::start().await;
    let payloads = [
        (
            "EnumerateUserFiles",
            "GET",
            Some(
                json!({"response":{"files":[{"appid":480,"ugcid":u64::MAX.to_string(),"filename":FILE,"timestamp":u64::MAX,"file_size":0,"url":format!("{}/download?signature=secret", storage.uri()),"steamid_creator":u64::MAX.to_string(),"flags":0,"platforms_to_sync":"windows,macos","file_sha":SHA}],"total_files":1}}),
            ),
        ),
        (
            "BeginAppUploadBatch",
            "POST",
            Some(
                json!({"response":{"batch_id":u64::MAX.to_string(),"app_change_number":u64::MAX}}),
            ),
        ),
        (
            "BeginHTTPUpload",
            "POST",
            Some(
                json!({"response":{"ugcid":u64::MAX.to_string(),"timestamp":123,"url_host":storage.address().to_string(),"url_path":"/upload?signature=private+%2B","use_https":true,"request_headers":[{"name":"x-storage-signature","value":"private-header"}]}}),
            ),
        ),
        (
            "CommitHTTPUpload",
            "POST",
            Some(json!({"response":{"file_committed":false}})),
        ),
        ("Delete", "POST", None),
        ("CompleteAppUploadBatch", "POST", None),
    ];
    for (name, verb, payload) in payloads {
        let mut response = ResponseTemplate::new(200).insert_header("x-eresult", "1");
        if let Some(payload) = payload {
            response = response.set_body_json(payload);
        }
        Mock::given(method(verb))
            .and(path(format!("/ICloudService/{name}/v1/")))
            .respond_with(response)
            .expect(1)
            .mount(&server)
            .await;
    }
    let client = client(&server, None, 2);
    let token = OAuthAccessToken::new(TOKEN).unwrap();
    let files = client
        .cloud()
        .enumerate_user_files(&token, &enumeration())
        .await
        .unwrap();
    assert_eq!(files.total_files, 1);
    assert_eq!(files.files[0].ugcid, Some(u64::MAX));
    assert_eq!(files.files[0].timestamp, Some(u64::MAX));
    assert_eq!(files.files[0].steamid_creator.unwrap().raw(), u64::MAX);
    assert_eq!(files.files[0].filename, FILE);
    assert!(!format!("{files:?}").contains("signature"));
    let batch = client
        .cloud()
        .begin_app_upload_batch(
            &token,
            &BeginAppUploadBatchRequest {
                appid: 480,
                machine_name: "Device & 中文",
                files_to_upload: &[FILE],
                files_to_delete: &["old.sav"],
            },
        )
        .await
        .unwrap();
    assert_eq!(batch.batch_id, u64::MAX);
    assert_eq!(batch.app_change_number, u64::MAX);
    let plan = client
        .cloud()
        .begin_http_upload(&token, &begin_upload())
        .await
        .unwrap();
    assert_eq!(plan.ugcid, u64::MAX);
    assert_eq!(plan.url_path, "/upload?signature=private+%2B");
    assert_eq!(plan.request_headers[0].value, "private-header");
    assert!(plan.use_https);
    assert!(!format!("{plan:?}").contains("private"));
    assert!(!format!("{:?}", plan.request_headers[0]).contains("private-header"));
    assert!(
        storage.received_requests().await.unwrap().is_empty(),
        "Staging must never perform the separate PUT"
    );
    let committed = client
        .cloud()
        .commit_http_upload(
            &token,
            &CommitHttpUploadRequest {
                appid: 480,
                transfer_succeeded: false,
                filename: FILE,
                file_sha: SHA,
            },
        )
        .await
        .unwrap();
    assert!(
        !committed.file_committed,
        "HTTP success must not invent file persistence"
    );
    client
        .cloud()
        .delete(
            &token,
            &DeleteCloudFileRequest {
                appid: 480,
                filename: "old.sav",
            },
        )
        .await
        .unwrap();
    client
        .cloud()
        .complete_app_upload_batch(
            &token,
            &CompleteAppUploadBatchRequest {
                appid: 480,
                batch_id: batch.batch_id,
                batch_eresult: CloudBatchResult::Failure,
            },
        )
        .await
        .unwrap();
    let requests = server.received_requests().await.unwrap();
    for request in &requests {
        let fields = form(request);
        assert_eq!(fields.get("access_token").unwrap(), TOKEN);
        assert_eq!(fields.get("format").unwrap(), "json");
        assert!(!fields.contains_key("key"));
        assert!(!request.headers.contains_key("x-webapi-key"));
        assert!(input(request).get("access_token").is_none());
    }
    assert_eq!(
        input(&requests[0]),
        json!({"appid":480,"extended_details":true,"count":500,"start_index":0})
    );
    assert_eq!(
        input(&requests[1]),
        json!({"appid":480,"machine_name":"Device & 中文","files_to_upload":[FILE],"files_to_delete":["old.sav"]})
    );
    assert_eq!(
        input(&requests[2]),
        json!({"appid":480,"file_size":0,"filename":FILE,"file_sha":SHA,"platforms_to_sync":["windows","macos"],"upload_batch_id":u64::MAX,"is_public":false})
    );
    assert_eq!(input(&requests[3])["transfer_succeeded"], false);
    assert_eq!(input(&requests[5])["batch_eresult"], 2);
    assert_eq!(input(&requests[5])["batch_id"].as_u64(), Some(u64::MAX));
}

#[tokio::test]
async fn cloud_all_methods_require_a_valid_success_header_even_with_http_200() {
    for header_value in [None, Some("2"), Some("private-malformed-code"), Some("1.0")] {
        let server = MockServer::start().await;
        for (name, verb) in CLOUD_METHODS {
            let mut response = ResponseTemplate::new(200).set_body_json(json!({"response":{}}));
            if let Some(value) = header_value {
                response = response.insert_header("x-eresult", value);
            }
            Mock::given(method(verb))
                .and(path(format!("/ICloudService/{name}/v1/")))
                .respond_with(response)
                .expect(1)
                .mount(&server)
                .await;
        }
        let client = client(&server, None, 3);
        for (name, _) in CLOUD_METHODS {
            let error = cloud_call(&client, name).await.unwrap_err();
            if header_value == Some("2") {
                assert!(
                    matches!(error, Error::Api { code: Some(2), .. }),
                    "{name}: {error:?}"
                );
            } else {
                assert!(matches!(error, Error::Decode { .. }), "{name}: {error:?}");
            }
            assert!(!error.to_string().contains("private-malformed-code"));
            assert!(!error.to_string().contains(TOKEN));
        }
        assert_eq!(client.metrics().attempts, 6);
        assert_eq!(client.metrics().retries, 0);
    }
}

#[tokio::test]
async fn cloud_bodyless_acknowledgements_reject_missing_or_failed_header() {
    for result in [None, Some("15")] {
        let server = MockServer::start().await;
        for name in ["Delete", "CompleteAppUploadBatch"] {
            let mut response = ResponseTemplate::new(200);
            if let Some(value) = result {
                response = response.insert_header("x-eresult", value);
            }
            Mock::given(method("POST"))
                .and(path(format!("/ICloudService/{name}/v1/")))
                .respond_with(response)
                .expect(1)
                .mount(&server)
                .await;
        }
        let client = client(&server, None, 3);
        for name in ["Delete", "CompleteAppUploadBatch"] {
            let error = cloud_call(&client, name).await.unwrap_err();
            assert!(match result {
                None => matches!(error, Error::Decode { .. }),
                _ => matches!(error, Error::Api { code: Some(15), .. }),
            });
        }
    }
}

#[tokio::test]
async fn cloud_json_stages_reject_empty_bodies_and_nested_business_errors() {
    for body in [
        None,
        Some(json!({"response":{"error":{"errorcode":15,"errordesc":"private error"}}})),
    ] {
        let server = MockServer::start().await;
        for (name, verb) in CLOUD_METHODS.into_iter().take(4) {
            let mut response = ResponseTemplate::new(200).insert_header("x-eresult", "1");
            if let Some(body) = &body {
                response = response.set_body_json(body);
            }
            Mock::given(method(verb))
                .and(path(format!("/ICloudService/{name}/v1/")))
                .respond_with(response)
                .expect(1)
                .mount(&server)
                .await;
        }
        let client = client(&server, None, 2);
        for (name, _) in CLOUD_METHODS.into_iter().take(4) {
            let error = cloud_call(&client, name).await.unwrap_err();
            assert!(
                if body.is_some() {
                    matches!(error, Error::Api { code: Some(15), .. })
                } else {
                    matches!(error, Error::Decode { .. })
                },
                "{name}: {error:?}"
            );
        }
    }
}

#[tokio::test]
async fn cloud_mutation_stages_are_never_replayed() {
    let server = MockServer::start().await;
    for (name, verb) in CLOUD_METHODS.into_iter().skip(1) {
        Mock::given(method(verb))
            .and(path(format!("/ICloudService/{name}/v1/")))
            .respond_with(ResponseTemplate::new(503))
            .expect(1)
            .mount(&server)
            .await;
    }
    let client = client(&server, None, 3);
    for (name, _) in CLOUD_METHODS.into_iter().skip(1) {
        assert!(matches!(
            cloud_call(&client, name).await,
            Err(Error::Http { status: 503, .. })
        ));
    }
    assert_eq!(client.metrics().attempts, 5);
    assert_eq!(client.metrics().retries, 0);
}

#[tokio::test]
async fn cloud_file_enumeration_uses_bounded_safe_read_retries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ICloudService/EnumerateUserFiles/v1/"))
        .respond_with(ResponseTemplate::new(503))
        .expect(3)
        .mount(&server)
        .await;
    let client = client(&server, None, 2);
    assert!(matches!(
        cloud_call(&client, "EnumerateUserFiles").await,
        Err(Error::Http { status: 503, .. })
    ));
    assert_eq!(client.metrics().attempts, 3);
    assert_eq!(client.metrics().retries, 2);
}

#[tokio::test]
async fn cloud_optional_metadata_and_request_fields_remain_absent() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ICloudService/EnumerateUserFiles/v1/"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-eresult", "1")
                .set_body_json(
                    json!({"response":{"files":[{"filename":"private.sav"}],"total_files":1}}),
                ),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server, None, 0);
    let token = OAuthAccessToken::new(TOKEN).unwrap();
    let result = client
        .cloud()
        .enumerate_user_files(
            &token,
            &EnumerateUserFilesRequest {
                appid: 480,
                extended_details: None,
                count: None,
                start_index: None,
            },
        )
        .await
        .unwrap();
    assert!(result.files[0].ugcid.is_none());
    assert!(result.files[0].url.is_none());
    assert!(result.files[0].file_sha.is_none());
    assert_eq!(
        input(&server.received_requests().await.unwrap()[0]),
        json!({"appid":480})
    );
    assert!(!format!("{:?}", result.files[0]).contains("private.sav"));
}

#[tokio::test]
async fn remote_storage_all_seven_methods_use_exact_versions_verbs_and_counts() {
    let server = MockServer::start().await;
    for (name, verb) in STORAGE_METHODS {
        Mock::given(method(verb))
            .and(path(format!("/ISteamRemoteStorage/{name}/v1/")))
            .and(header("x-webapi-key", "wire-key"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"response":{"method":name,"private_payload":"opaque"}})),
            )
            .expect(1)
            .mount(&server)
            .await;
    }
    let client = client(&server, Some(ApiKey::publisher("wire-key").unwrap()), 2);
    for (name, _) in STORAGE_METHODS {
        let response = storage_call(&client, name).await.unwrap();
        assert_eq!(response.as_json()["response"]["method"], name);
        assert!(!format!("{response:?}").contains("private_payload"));
    }
    let requests = server.received_requests().await.unwrap();
    for request in &requests {
        let fields = form(request);
        assert_eq!(fields.get("format").unwrap(), "json");
        assert!(!fields.contains_key("input_json"));
        assert!(!fields.contains_key("key"));
    }
    let subscribed = form(&requests[0]);
    assert_eq!(subscribed.get("steamid").unwrap(), &u64::MAX.to_string());
    assert_eq!(subscribed.get("listtype").unwrap(), &u32::MAX.to_string());
    for (index, count_name) in [(1, "collectioncount"), (2, "itemcount")] {
        let fields = form(&requests[index]);
        assert_eq!(fields.get(count_name).unwrap(), "2");
        assert_eq!(
            fields.get("publishedfileids[0]").unwrap(),
            &u64::MAX.to_string()
        );
        assert_eq!(fields.get("publishedfileids[1]").unwrap(), "42");
    }
    assert_eq!(
        form(&requests[3]).get("ugcid").unwrap(),
        &u64::MAX.to_string()
    );
    assert_eq!(form(&requests[4]).get("used").unwrap(), "false");
    assert_eq!(
        form(&requests[5]).get("publishedfileid").unwrap(),
        &u64::MAX.to_string()
    );
    assert_eq!(
        form(&requests[6]).get("publishedfileid").unwrap(),
        &u64::MAX.to_string()
    );
}

#[tokio::test]
async fn remote_storage_public_metadata_needs_no_key_and_ugc_accepts_a_user_key() {
    let server = MockServer::start().await;
    for name in ["GetCollectionDetails", "GetPublishedFileDetails"] {
        Mock::given(method("POST"))
            .and(path(format!("/ISteamRemoteStorage/{name}/v1/")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response":{}})))
            .expect(1)
            .mount(&server)
            .await;
    }
    let anonymous = client(&server, None, 0);
    storage_call(&anonymous, "GetCollectionDetails")
        .await
        .unwrap();
    storage_call(&anonymous, "GetPublishedFileDetails")
        .await
        .unwrap();
    for request in server.received_requests().await.unwrap() {
        assert!(!request.headers.contains_key("x-webapi-key"));
    }
    Mock::given(method("GET"))
        .and(path("/ISteamRemoteStorage/GetUGCFileDetails/v1/"))
        .and(header("x-webapi-key", "user-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"response":{}})))
        .expect(1)
        .mount(&server)
        .await;
    let user = client(&server, Some(ApiKey::user("user-key").unwrap()), 0);
    user.remote_storage()
        .get_ugc_file_details(&UgcFileDetailsRequest {
            ugcid: 42,
            appid: 480,
            steamid: None,
        })
        .await
        .unwrap();
    assert!(
        !form(server.received_requests().await.unwrap().last().unwrap()).contains_key("steamid")
    );
}

#[tokio::test]
async fn remote_storage_all_methods_reject_nested_business_errors() {
    let server = MockServer::start().await;
    for (name, verb) in STORAGE_METHODS {
        Mock::given(method(verb))
            .and(path(format!("/ISteamRemoteStorage/{name}/v1/")))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                json!({"response":{"error":{"errorcode":15,"errordesc":"private cloud error"}}}),
            ))
            .expect(1)
            .mount(&server)
            .await;
    }
    let client = client(&server, Some(ApiKey::publisher("wire-key").unwrap()), 2);
    for (name, _) in STORAGE_METHODS {
        let error = storage_call(&client, name).await.unwrap_err();
        assert!(
            matches!(error, Error::Api { code: Some(15), .. }),
            "{name}: {error:?}"
        );
        assert!(!error.to_string().contains("private cloud error"));
    }
}

#[tokio::test]
async fn remote_storage_post_reads_and_mutations_are_single_send() {
    let server = MockServer::start().await;
    for (name, verb) in STORAGE_METHODS
        .into_iter()
        .filter(|(_, verb)| *verb == "POST")
    {
        Mock::given(method(verb))
            .and(path(format!("/ISteamRemoteStorage/{name}/v1/")))
            .respond_with(ResponseTemplate::new(503))
            .expect(1)
            .mount(&server)
            .await;
    }
    let client = client(&server, Some(ApiKey::publisher("wire-key").unwrap()), 3);
    for (name, _) in STORAGE_METHODS
        .into_iter()
        .filter(|(_, verb)| *verb == "POST")
    {
        assert!(matches!(
            storage_call(&client, name).await,
            Err(Error::Http { status: 503, .. })
        ));
    }
    assert_eq!(client.metrics().attempts, 6);
    assert_eq!(client.metrics().retries, 0);
}

#[tokio::test]
async fn remote_storage_publisher_and_user_key_requirements_fail_before_network() {
    let server = MockServer::start().await;
    for key in [None, Some(ApiKey::user("user-key").unwrap())] {
        let client = client(&server, key, 0);
        for name in [
            "EnumerateUserSubscribedFiles",
            "SetUGCUsedByGC",
            "SubscribePublishedFile",
            "UnsubscribePublishedFile",
        ] {
            assert!(matches!(
                storage_call(&client, name).await,
                Err(Error::MissingApiKey("publisher"))
            ));
        }
    }
    let anonymous = client(&server, None, 0);
    assert!(matches!(
        storage_call(&anonymous, "GetUGCFileDetails").await,
        Err(Error::MissingApiKey("user or publisher"))
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn cloud_validation_and_token_debug_fail_closed_before_network() {
    let server = MockServer::start().await;
    let client = client(&server, None, 0);
    for value in ["", " ", "bad\0token"] {
        assert!(OAuthAccessToken::new(value).is_err());
    }
    let token = OAuthAccessToken::new(TOKEN).unwrap();
    assert!(!format!("{token:?}").contains(TOKEN));
    assert_eq!(token.expose_secret(), TOKEN);
    for request in [
        EnumerateUserFilesRequest {
            appid: 0,
            ..enumeration()
        },
        EnumerateUserFilesRequest {
            count: Some(0),
            ..enumeration()
        },
    ] {
        assert!(matches!(
            client.cloud().enumerate_user_files(&token, &request).await,
            Err(Error::InvalidParameter(_))
        ));
    }
    for request in [
        BeginAppUploadBatchRequest {
            appid: 0,
            machine_name: "Device",
            files_to_upload: &[],
            files_to_delete: &[],
        },
        BeginAppUploadBatchRequest {
            appid: 480,
            machine_name: "",
            files_to_upload: &[],
            files_to_delete: &[],
        },
        BeginAppUploadBatchRequest {
            appid: 480,
            machine_name: "Device",
            files_to_upload: &[""],
            files_to_delete: &[],
        },
        BeginAppUploadBatchRequest {
            appid: 480,
            machine_name: "Device",
            files_to_upload: &[FILE],
            files_to_delete: &[FILE],
        },
    ] {
        assert!(matches!(
            client
                .cloud()
                .begin_app_upload_batch(&token, &request)
                .await,
            Err(Error::InvalidParameter(_))
        ));
    }
    for request in [
        BeginHttpUploadRequest {
            appid: 0,
            ..begin_upload()
        },
        BeginHttpUploadRequest {
            filename: "",
            ..begin_upload()
        },
        BeginHttpUploadRequest {
            filename: "bad\0file",
            ..begin_upload()
        },
        BeginHttpUploadRequest {
            file_sha: "short",
            ..begin_upload()
        },
        BeginHttpUploadRequest {
            file_sha: "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
            ..begin_upload()
        },
        BeginHttpUploadRequest {
            upload_batch_id: 0,
            ..begin_upload()
        },
        BeginHttpUploadRequest {
            platforms_to_sync: &[],
            ..begin_upload()
        },
        BeginHttpUploadRequest {
            platforms_to_sync: &[CloudPlatform::Windows, CloudPlatform::Windows],
            ..begin_upload()
        },
    ] {
        assert!(matches!(
            client.cloud().begin_http_upload(&token, &request).await,
            Err(Error::InvalidParameter(_))
        ));
    }
    for request in [
        CommitHttpUploadRequest {
            appid: 0,
            transfer_succeeded: true,
            filename: FILE,
            file_sha: SHA,
        },
        CommitHttpUploadRequest {
            appid: 480,
            transfer_succeeded: true,
            filename: "",
            file_sha: SHA,
        },
        CommitHttpUploadRequest {
            appid: 480,
            transfer_succeeded: true,
            filename: FILE,
            file_sha: "bad",
        },
    ] {
        assert!(matches!(
            client.cloud().commit_http_upload(&token, &request).await,
            Err(Error::InvalidParameter(_))
        ));
    }
    for request in [
        CompleteAppUploadBatchRequest {
            appid: 0,
            batch_id: 1,
            batch_eresult: CloudBatchResult::Failure,
        },
        CompleteAppUploadBatchRequest {
            appid: 480,
            batch_id: 0,
            batch_eresult: CloudBatchResult::Success,
        },
    ] {
        assert!(matches!(
            client
                .cloud()
                .complete_app_upload_batch(&token, &request)
                .await,
            Err(Error::InvalidParameter(_))
        ));
    }
    for request in [
        DeleteCloudFileRequest {
            appid: 0,
            filename: FILE,
        },
        DeleteCloudFileRequest {
            appid: 480,
            filename: "",
        },
    ] {
        assert!(matches!(
            client.cloud().delete(&token, &request).await,
            Err(Error::InvalidParameter(_))
        ));
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn remote_storage_ids_and_lists_are_validated_before_network() {
    let server = MockServer::start().await;
    let client = client(&server, Some(ApiKey::publisher("wire-key").unwrap()), 0);
    for ids in [&[][..], &[0][..], &[1, 0][..]] {
        assert!(matches!(
            client
                .remote_storage()
                .get_collection_details(PublishedFileIdsRequest {
                    publishedfileids: ids
                })
                .await,
            Err(Error::InvalidParameter(_))
        ));
        assert!(matches!(
            client
                .remote_storage()
                .get_published_file_details(PublishedFileIdsRequest {
                    publishedfileids: ids
                })
                .await,
            Err(Error::InvalidParameter(_))
        ));
    }
    for request in [
        EnumerateSubscribedFilesRequest {
            steamid: 0.into(),
            appid: 480,
            listtype: None,
        },
        EnumerateSubscribedFilesRequest {
            steamid: 1.into(),
            appid: 0,
            listtype: None,
        },
    ] {
        assert!(matches!(
            client
                .remote_storage()
                .enumerate_user_subscribed_files(&request)
                .await,
            Err(Error::InvalidParameter(_))
        ));
    }
    for request in [
        UgcFileDetailsRequest {
            ugcid: 0,
            appid: 480,
            steamid: None,
        },
        UgcFileDetailsRequest {
            ugcid: 1,
            appid: 0,
            steamid: None,
        },
        UgcFileDetailsRequest {
            ugcid: 1,
            appid: 480,
            steamid: Some(0.into()),
        },
    ] {
        assert!(matches!(
            client.remote_storage().get_ugc_file_details(&request).await,
            Err(Error::InvalidParameter(_))
        ));
    }
    for request in [
        SetUgcUsedByGcRequest {
            steamid: 0.into(),
            ugcid: 1,
            appid: 480,
            used: true,
        },
        SetUgcUsedByGcRequest {
            steamid: 1.into(),
            ugcid: 0,
            appid: 480,
            used: true,
        },
        SetUgcUsedByGcRequest {
            steamid: 1.into(),
            ugcid: 1,
            appid: 0,
            used: true,
        },
    ] {
        assert!(matches!(
            client.remote_storage().set_ugc_used_by_gc(&request).await,
            Err(Error::InvalidParameter(_))
        ));
    }
    for request in [
        PublishedFileSubscriptionRequest {
            steamid: 0.into(),
            ..subscription()
        },
        PublishedFileSubscriptionRequest {
            appid: 0,
            ..subscription()
        },
        PublishedFileSubscriptionRequest {
            publishedfileid: 0,
            ..subscription()
        },
    ] {
        assert!(matches!(
            client
                .remote_storage()
                .subscribe_published_file(&request)
                .await,
            Err(Error::InvalidParameter(_))
        ));
        assert!(matches!(
            client
                .remote_storage()
                .unsubscribe_published_file(&request)
                .await,
            Err(Error::InvalidParameter(_))
        ));
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[test]
fn cloud_platforms_and_batch_results_match_documented_wire_values() {
    assert_eq!(
        serde_json::to_value([
            CloudPlatform::All,
            CloudPlatform::Windows,
            CloudPlatform::MacOS,
            CloudPlatform::Linux,
            CloudPlatform::Android,
            CloudPlatform::IPhoneOS,
            CloudPlatform::Switch
        ])
        .unwrap(),
        json!([
            "all", "windows", "macos", "linux", "android", "iphoneos", "switch"
        ])
    );
    assert_eq!(serde_json::to_value(CloudBatchResult::Success).unwrap(), 1);
    assert_eq!(serde_json::to_value(CloudBatchResult::Failure).unwrap(), 2);
}
