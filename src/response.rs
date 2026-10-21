// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

use crate::{Error, Result, user_auth::OpaqueResponse};
use reqwest::header::HeaderMap;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Complete endpoint response contracts, kept together with their decoder.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResponsePolicy {
    Json,
    BuildConfirmation,
    CloudJson,
    CloudAcknowledgement,
    MicroTransaction,
    VacStatus,
}

impl ResponsePolicy {
    fn allows_empty(self, status: u16) -> bool {
        matches!(
            (self, status),
            (Self::BuildConfirmation, 201) | (Self::CloudAcknowledgement, 200)
        )
    }

    pub fn parse(self, endpoint: &'static str, status: u16, bytes: &[u8]) -> Result<Response> {
        if self == Self::CloudAcknowledgement && (!self.allows_empty(status) || !bytes.is_empty()) {
            return Err(Error::Decode {
                endpoint,
                line: 0,
                column: 0,
            });
        }
        let payload = if bytes.is_empty() && self.allows_empty(status) {
            Value::Null
        } else {
            let value: Value =
                serde_json::from_slice(bytes).map_err(|error| Error::decode(endpoint, error))?;
            if !value.is_object() {
                return Err(Error::Decode {
                    endpoint,
                    line: 0,
                    column: 0,
                });
            }
            value
        };
        Ok(Response {
            status,
            payload,
            policy: self,
        })
    }

    pub fn validate_headers(self, endpoint: &'static str, headers: &HeaderMap) -> Result<()> {
        let invalid = || Error::Decode {
            endpoint,
            line: 0,
            column: 0,
        };
        let mut values = headers.get_all("x-eresult").iter();
        let Some(value) = values.next() else {
            return if matches!(self, Self::CloudJson | Self::CloudAcknowledgement) {
                Err(invalid())
            } else {
                Ok(())
            };
        };
        if values.next().is_some() {
            return Err(invalid());
        }
        let code = value
            .to_str()
            .ok()
            .and_then(|text| text.trim().parse::<i64>().ok())
            .ok_or_else(invalid)?;
        if code == 1 {
            Ok(())
        } else {
            Err(Error::Api {
                endpoint,
                code: Some(code),
            })
        }
    }

    fn validate_business(self, endpoint: &'static str, value: &Value) -> Result<()> {
        if self == Self::VacStatus {
            check_local(endpoint, value, Self::Json)?;
            let body = value
                .get("response")
                .filter(|body| body.is_object())
                .ok_or(Error::Decode {
                    endpoint,
                    line: 0,
                    column: 0,
                })?;
            if body.get("success").and_then(Value::as_bool).is_none() {
                return Err(Error::Decode {
                    endpoint,
                    line: 0,
                    column: 0,
                });
            }
            check_local(endpoint, body, Self::VacStatus)?;
            check_envelopes(endpoint, body)?;
            // Only the canonical VAC response may use false as pending. Root
            // markers, sibling envelopes and nested failures remain strict.
            for key in ERROR_ENVELOPES {
                if key != "response"
                    && let Some(child) = value.get(key)
                {
                    check_api(endpoint, child)?;
                }
            }
        } else {
            check_api(endpoint, value)?;
        }
        if self == Self::MicroTransaction
            && value
                .get("response")
                .and_then(|body| body.get("result"))
                .and_then(Value::as_str)
                != Some("OK")
        {
            return Err(Error::Decode {
                endpoint,
                line: 0,
                column: 0,
            });
        }
        Ok(())
    }
}

const ERROR_ENVELOPES: [&str; 4] = ["response", "params", "playerstats", "result"];

pub(crate) struct Response {
    pub status: u16,
    pub payload: Value,
    pub policy: ResponsePolicy,
}

impl Response {
    pub fn opaque(self, endpoint: &'static str) -> Result<(u16, OpaqueResponse)> {
        self.policy.validate_business(endpoint, &self.payload)?;
        Ok((self.status, OpaqueResponse::from_json(self.payload)))
    }

    pub fn decode<T: DeserializeOwned>(
        self,
        endpoint: &'static str,
        envelope: &[&str],
    ) -> Result<(u16, T)> {
        let mut payload = self.payload;
        self.policy.validate_business(endpoint, &payload)?;
        for key in envelope {
            payload = payload
                .as_object_mut()
                .and_then(|object| object.remove(*key))
                .ok_or(Error::Decode {
                    endpoint,
                    line: 0,
                    column: 0,
                })?;
            // Standard error envelopes were already checked recursively. Other
            // selected envelopes need checking once when entering their subtree.
            if !ERROR_ENVELOPES.contains(key) {
                check_api(endpoint, &payload)?;
            }
        }
        // Owned deserialization can move strings into typed fields.
        // Only coordinates from an untrusted serde error may leave this layer.
        serde_json::from_value(payload)
            .map(|response| (self.status, response))
            .map_err(|error| Error::decode(endpoint, error))
    }
}

fn check_api(endpoint: &'static str, value: &Value) -> Result<()> {
    check_local(endpoint, value, ResponsePolicy::Json)?;
    check_envelopes(endpoint, value)
}

fn check_local(endpoint: &'static str, value: &Value, policy: ResponsePolicy) -> Result<()> {
    if value.get("error").is_some_and(|error| !error.is_null()) {
        let code = value
            .get("error")
            .and_then(|e| e.get("errorcode"))
            .and_then(as_i64);
        return Err(Error::Api { endpoint, code });
    }
    if let Some(success) = value.get("success") {
        match (success.as_bool(), as_i64(success)) {
            (Some(true), _) | (_, Some(1)) => (),
            (Some(false), _) if policy == ResponsePolicy::VacStatus => (),
            (Some(false), _) => {
                return Err(Error::Api {
                    endpoint,
                    code: None,
                });
            }
            (_, Some(code)) => {
                return Err(Error::Api {
                    endpoint,
                    code: Some(code),
                });
            }
            _ => {
                return Err(Error::Decode {
                    endpoint,
                    line: 0,
                    column: 0,
                });
            }
        }
    }
    for key in ["result", "eresult"] {
        if let Some(result) = value.get(key) {
            // UpdateItemDefs documents an object-valued result envelope. It is
            // checked recursively below; scalar result/eresult remain strict.
            if key == "result" && result.is_object() {
                continue;
            }
            match (as_i64(result), result.as_str()) {
                (Some(1), _) | (_, Some("OK")) => (),
                (Some(code), _) => {
                    return Err(Error::Api {
                        endpoint,
                        code: Some(code),
                    });
                }
                (_, Some(_)) => {
                    return Err(Error::Api {
                        endpoint,
                        code: None,
                    });
                }
                _ => {
                    return Err(Error::Decode {
                        endpoint,
                        line: 0,
                        column: 0,
                    });
                }
            }
        }
    }
    Ok(())
}

fn check_envelopes(endpoint: &'static str, value: &Value) -> Result<()> {
    // Some methods with undocumented response schemas return opaque JSON.
    // Still inspect their standard error envelopes before handing it to callers.
    for key in ERROR_ENVELOPES {
        if let Some(child) = value.get(key) {
            check_api(endpoint, child)?;
        }
    }
    Ok(())
}
fn as_i64(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| value.as_str()?.parse().ok())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use reqwest::header::HeaderValue;
    use serde::Deserialize;
    use serde_json::json;

    const ENDPOINT: &str = "test/response/v1/";

    #[test]
    fn microtransactions_require_explicit_acknowledgement_without_inventing_settlement() {
        let policy = ResponsePolicy::MicroTransaction;
        for payload in [
            json!({}),
            json!({"response": {}}),
            json!({"response": {"result": 1}}),
            json!({"response": {"result": true}}),
        ] {
            let response = policy
                .parse(ENDPOINT, 200, &serde_json::to_vec(&payload).unwrap())
                .unwrap();
            assert!(matches!(
                response.opaque(ENDPOINT),
                Err(Error::Decode { .. })
            ));
        }
        let payload =
            json!({"response": {"result": "OK", "params": {"status": "Failed", "orderid": "0"}}});
        let (_, response) = policy
            .parse(ENDPOINT, 200, &serde_json::to_vec(&payload).unwrap())
            .unwrap()
            .opaque(ENDPOINT)
            .unwrap();
        assert_eq!(response.as_json()["response"]["params"]["status"], "Failed");
        let payload = json!({"response": {"result": "Failure", "error": {"errorcode": 56, "errordesc": "private payment"}}});
        assert!(matches!(
            policy
                .parse(ENDPOINT, 200, &serde_json::to_vec(&payload).unwrap())
                .unwrap()
                .opaque(ENDPOINT),
            Err(Error::Api { code: Some(56), .. })
        ));
    }

    #[test]
    fn vac_pending_is_scoped_to_its_canonical_body_and_never_hides_errors() {
        let policy = ResponsePolicy::VacStatus;
        let payload = json!({"response": {"success": false}});
        let bytes = serde_json::to_vec(&payload).unwrap();
        assert!(
            policy
                .parse(ENDPOINT, 200, &bytes)
                .unwrap()
                .opaque(ENDPOINT)
                .is_ok()
        );
        assert!(matches!(
            ResponsePolicy::Json
                .parse(ENDPOINT, 200, &bytes)
                .unwrap()
                .opaque(ENDPOINT),
            Err(Error::Api { .. })
        ));
        for payload in [
            json!({"success": false, "response": {"success": false}}),
            json!({"response": {"success": false, "error": "private"}}),
            json!({"response": {"success": false, "params": {"success": false}}}),
            json!({"response": {"success": false}, "playerstats": {"success": false}}),
        ] {
            assert!(matches!(
                policy
                    .parse(ENDPOINT, 200, &serde_json::to_vec(&payload).unwrap())
                    .unwrap()
                    .opaque(ENDPOINT),
                Err(Error::Api { .. })
            ));
        }
        for payload in [
            json!({}),
            json!({"response": {}}),
            json!({"response": {"success": 0}}),
        ] {
            assert!(matches!(
                policy
                    .parse(ENDPOINT, 200, &serde_json::to_vec(&payload).unwrap())
                    .unwrap()
                    .opaque(ENDPOINT),
                Err(Error::Decode { .. })
            ));
        }
    }

    #[test]
    fn response_contracts_distinguish_cloud_acknowledgements_and_build_confirmation() {
        for policy in [
            ResponsePolicy::Json,
            ResponsePolicy::BuildConfirmation,
            ResponsePolicy::CloudJson,
            ResponsePolicy::CloudAcknowledgement,
        ] {
            for status in [200, 201, 204] {
                let expected = (policy == ResponsePolicy::BuildConfirmation && status == 201)
                    || (policy == ResponsePolicy::CloudAcknowledgement && status == 200);
                assert_eq!(policy.parse(ENDPOINT, status, b"").is_ok(), expected);
            }
            assert_eq!(
                policy.parse(ENDPOINT, 200, b"{}").is_ok(),
                policy != ResponsePolicy::CloudAcknowledgement
            );
            for bytes in [&b"null"[..], &b"[]"[..], &b"private invalid JSON"[..]] {
                assert!(matches!(
                    policy.parse(ENDPOINT, 200, bytes),
                    Err(Error::Decode { .. })
                ));
            }
        }
    }

    #[test]
    fn steam_result_headers_are_checked_before_cloud_body_policies() {
        let optional = ResponsePolicy::Json;
        let required = ResponsePolicy::CloudAcknowledgement;
        let mut headers = HeaderMap::new();
        assert!(optional.validate_headers(ENDPOINT, &headers).is_ok());
        assert!(matches!(
            required.validate_headers(ENDPOINT, &headers),
            Err(Error::Decode { .. })
        ));
        for (value, success) in [("1", true), (" 1 ", true), ("2", false), ("OK", false)] {
            headers.insert("x-eresult", HeaderValue::from_static(value));
            assert_eq!(
                required.validate_headers(ENDPOINT, &headers).is_ok(),
                success
            );
        }
        headers.insert("x-eresult", HeaderValue::from_static("2"));
        assert!(matches!(
            optional.validate_headers(ENDPOINT, &headers),
            Err(Error::Api { code: Some(2), .. })
        ));
        headers.append("x-eresult", HeaderValue::from_static("1"));
        assert!(matches!(
            required.validate_headers(ENDPOINT, &headers),
            Err(Error::Decode { .. })
        ));
        assert!(required.allows_empty(200));
        assert!(!required.allows_empty(201));
        assert!(!ResponsePolicy::CloudJson.allows_empty(200));
    }

    #[test]
    fn object_result_envelopes_preserve_success_and_reject_nested_failures() {
        let payload = json!({"result": {"updated": ["418"], "failed": [], "success": true}});
        let (_, response) = Response {
            policy: ResponsePolicy::Json,
            status: 200,
            payload,
        }
        .opaque(ENDPOINT)
        .unwrap();
        assert_eq!(response.as_json()["result"]["updated"][0], "418");
        for payload in [
            json!({"result": {"success": false}}),
            json!({"result": {"error": "private"}}),
            json!({"result": {"params": {"eresult": 2}}}),
            json!({"result": 2}),
        ] {
            assert!(matches!(
                Response {
                    policy: ResponsePolicy::Json,
                    status: 200,
                    payload
                }
                .opaque(ENDPOINT),
                Err(Error::Api { .. })
            ));
        }
        assert!(matches!(
            Response {
                policy: ResponsePolicy::Json,
                status: 200,
                payload: json!({"result": []})
            }
            .opaque(ENDPOINT),
            Err(Error::Decode { .. })
        ));
    }

    #[test]
    fn typed_strings_are_moved_from_the_selected_envelope() {
        #[derive(Deserialize)]
        struct Payload {
            text: String,
        }
        let payload = json!({"response": {"params": {"text": "owned payload"}}});
        let original = payload["response"]["params"]["text"]
            .as_str()
            .unwrap()
            .as_ptr();
        let (_, decoded): (_, Payload) = Response {
            policy: ResponsePolicy::Json,
            status: 200,
            payload,
        }
        .decode(ENDPOINT, &["response", "params"])
        .unwrap();
        assert_eq!(decoded.text.as_ptr(), original);
    }

    #[test]
    fn opaque_payload_retains_its_original_array_storage_and_root() {
        let payload = json!({"response": {"games": [1, 2, 3]}, "cursor": "next"});
        let original = payload["response"]["games"].as_array().unwrap().as_ptr();
        let (status, decoded) = Response {
            policy: ResponsePolicy::Json,
            status: 200,
            payload,
        }
        .opaque(ENDPOINT)
        .unwrap();
        assert_eq!(status, 200);
        assert_eq!(decoded.as_json()["cursor"], "next");
        assert_eq!(
            decoded.as_json()["response"]["games"]
                .as_array()
                .unwrap()
                .as_ptr(),
            original
        );
    }

    #[test]
    fn errors_in_selected_and_sibling_envelopes_still_fail_closed() {
        for (payload, envelope) in [
            (
                json!({"response": {"params": {"success": false}}}),
                vec!["response", "params"],
            ),
            (
                json!({"applist": {"apps": {"app": {"eresult": 2}}}}),
                vec!["applist", "apps", "app"],
            ),
            (
                json!({"response": {"ok": true}, "playerstats": {"success": false}}),
                vec!["response"],
            ),
        ] {
            let result: Result<(u16, Value)> = Response {
                policy: ResponsePolicy::Json,
                status: 200,
                payload,
            }
            .decode(ENDPOINT, &envelope);
            assert!(matches!(result, Err(Error::Api { .. })));
        }
        let result: Result<(u16, Value)> = Response {
            policy: ResponsePolicy::Json,
            status: 200,
            payload: json!({"response": {}}),
        }
        .decode(ENDPOINT, &["response", "params"]);
        assert!(matches!(result, Err(Error::Decode { .. })));
    }
}
