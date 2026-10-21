// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

use crate::{Error, Result, user_auth::OpaqueResponse};
use serde::de::DeserializeOwned;
use serde_json::Value;

const ERROR_ENVELOPES: [&str; 4] = ["response", "params", "playerstats", "result"];

pub(crate) struct Response {
    pub status: u16,
    pub payload: Value,
}

impl Response {
    pub fn opaque(self, endpoint: &'static str) -> Result<(u16, OpaqueResponse)> {
        check_api(endpoint, &self.payload)?;
        Ok((self.status, OpaqueResponse::from_json(self.payload)))
    }

    pub fn decode<T: DeserializeOwned>(
        self,
        endpoint: &'static str,
        envelope: &[&str],
    ) -> Result<(u16, T)> {
        let mut payload = self.payload;
        check_api(endpoint, &payload)?;
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
        // Owned deserialization moves strings and arrays instead of cloning them.
        // Only coordinates from an untrusted serde error may leave this layer.
        serde_json::from_value(payload)
            .map(|response| (self.status, response))
            .map_err(|error| Error::decode(endpoint, error))
    }
}

fn check_api(endpoint: &'static str, value: &Value) -> Result<()> {
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
    use serde::Deserialize;
    use serde_json::json;

    const ENDPOINT: &str = "test/response/v1/";

    #[test]
    fn object_result_envelopes_preserve_success_and_reject_nested_failures() {
        let payload = json!({"result": {"updated": ["418"], "failed": [], "success": true}});
        let (_, response) = Response {
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
                    status: 200,
                    payload
                }
                .opaque(ENDPOINT),
                Err(Error::Api { .. })
            ));
        }
        assert!(matches!(
            Response {
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
                status: 200,
                payload,
            }
            .decode(ENDPOINT, &envelope);
            assert!(matches!(result, Err(Error::Api { .. })));
        }
        let result: Result<(u16, Value)> = Response {
            status: 200,
            payload: json!({"response": {}}),
        }
        .decode(ENDPOINT, &["response", "params"]);
        assert!(matches!(result, Err(Error::Decode { .. })));
    }
}
