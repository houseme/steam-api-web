// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

// Reference algorithms copied from commit 8b7613a for reproducible comparisons.
use crate::{Error, Result};
use serde::de::DeserializeOwned;
use serde_json::Value;

pub type Params = Vec<(String, Vec<u8>)>;
pub fn param(name: &str, value: impl ToString) -> (String, Vec<u8>) {
    (name.to_owned(), value.to_string().into_bytes())
}

pub fn decode<T: DeserializeOwned>(bytes: &[u8], envelope: &[&str]) -> Result<T> {
    let value: Value = serde_json::from_slice(bytes).map_err(|error| Error::Decode {
        endpoint: "bench/v1/",
        line: error.line(),
        column: error.column(),
    })?;
    let mut payload = &value;
    check_api("bench/v1/", payload)?;
    for key in envelope {
        payload = &payload[*key];
        check_api("bench/v1/", payload)?;
    }
    T::deserialize(payload).map_err(|error| Error::Decode {
        endpoint: "bench/v1/",
        line: error.line(),
        column: error.column(),
    })
}

pub fn check_api(endpoint: &'static str, value: &Value) -> Result<()> {
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
    for key in ["response", "params", "playerstats"] {
        if let Some(child) = value.get(key) {
            check_api(endpoint, child)?;
        }
    }
    Ok(())
}
fn as_i64(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| value.as_str()?.parse().ok())
}
/// Byte-preserving form encoding, including rawbinary (not UTF-8 conversion).
pub fn encode_form(params: &Params) -> String {
    fn component(output: &mut String, bytes: &[u8]) {
        const HEX: &[u8; 16] = b"0123456789ABCDEF";
        for &byte in bytes {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'*' => {
                    output.push(char::from(byte))
                }
                b' ' => output.push('+'),
                _ => {
                    output.push('%');
                    output.push(char::from(HEX[(byte >> 4) as usize]));
                    output.push(char::from(HEX[(byte & 15) as usize]));
                }
            }
        }
    }
    let mut output = String::new();
    for (index, (key, value)) in params.iter().enumerate() {
        if index != 0 {
            output.push('&');
        }
        component(&mut output, key.as_bytes());
        output.push('=');
        component(&mut output, value);
    }
    output
}
