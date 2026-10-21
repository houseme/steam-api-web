// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

use crate::{Error, Result, SteamId, response::ResponsePolicy};
use reqwest::{Client, RequestBuilder, header::CONTENT_TYPE};
use serde::Serialize;
use std::io;
use url::Url;

#[derive(Clone, Copy)]
pub(crate) enum Access {
    Anonymous,
    User,
    Publisher,
    Financial,
}

/// The method and replay policy cannot be configured inconsistently.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Operation {
    Read,
    SingleGet,
    Post,
}

/// Only fixed internal paths may reach the transport.
#[derive(Clone, Copy)]
pub(crate) struct Endpoint {
    pub path: &'static str,
    pub access: Access,
    pub operation: Operation,
    pub response: ResponsePolicy,
}

impl Endpoint {
    pub const fn get(path: &'static str, access: Access) -> Self {
        Self::new(path, access, Operation::Read)
    }

    pub const fn get_once(path: &'static str, access: Access) -> Self {
        Self::new(path, access, Operation::SingleGet)
    }

    pub const fn post(path: &'static str, access: Access) -> Self {
        Self::new(path, access, Operation::Post)
    }

    const fn new(path: &'static str, access: Access, operation: Operation) -> Self {
        Self {
            path,
            access,
            operation,
            response: ResponsePolicy::Json,
        }
    }

    /// Prepare a single send by moving the URL and form into reqwest. The
    /// transport uses this path for writes, single GETs and reads with no retries.
    pub fn request(self, client: &Client, mut url: Url, encoded: String) -> RequestBuilder {
        if self.operation == Operation::Post {
            client
                .post(url)
                .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(encoded)
        } else {
            url.set_query(Some(&encoded));
            client.get(url)
        }
    }

    /// Only HTTP 201 may omit JSON when mobile confirmation is pending.
    pub const fn allow_empty_created(mut self) -> Self {
        self.response = ResponsePolicy::BuildConfirmation;
        self
    }

    /// Cloud calls require an explicit successful Steam result header and JSON.
    pub const fn cloud_json(mut self) -> Self {
        self.response = ResponsePolicy::CloudJson;
        self
    }

    /// Bodyless Cloud methods require a successful result header before HTTP 200
    /// can represent success without JSON.
    pub const fn cloud_acknowledgement(mut self) -> Self {
        self.response = ResponsePolicy::CloudAcknowledgement;
        self
    }

    /// Payment replies must explicitly acknowledge the request with result=OK.
    pub const fn micro_transaction(mut self) -> Self {
        self.response = ResponsePolicy::MicroTransaction;
        self
    }

    /// VAC can report success=false while session readiness is still pending.
    pub const fn vac_status(mut self) -> Self {
        self.response = ResponsePolicy::VacStatus;
        self
    }
}

/// Encode directly into the final form, without owning each key/value pair.
pub(crate) struct Params(String);

impl Params {
    pub fn new() -> Self {
        Self(String::with_capacity(128))
    }

    pub fn with(mut self, name: &str, value: impl FormValue) -> Self {
        self.push(name, value);
        self
    }

    pub fn push(&mut self, name: &str, value: impl FormValue) {
        self.key(name);
        value.encode(&mut self.0);
    }

    /// Append Valve's documented `name[index]` array key. Brackets travel
    /// percent-encoded (`name%5B0%5D`), byte-for-byte like a preformatted key,
    /// while the index is formatted on the stack without a temporary String.
    pub fn push_indexed(&mut self, name: &str, index: usize, value: impl FormValue) {
        self.prefixed_key(name, |key| {
            key.push_str("%5B");
            key.push_str(itoa::Buffer::new().format(index));
            key.push_str("%5D");
        });
        value.encode(&mut self.0);
    }

    /// Append `name{index}=value`, the legacy flat indexed spelling such as
    /// `classid0`, without allocating the key.
    pub fn push_suffixed(&mut self, name: &str, index: usize, value: impl FormValue) {
        self.prefixed_key(name, |key| {
            key.push_str(itoa::Buffer::new().format(index));
        });
        value.encode(&mut self.0);
    }

    pub fn push_bytes(&mut self, name: &str, value: &[u8]) {
        self.key(name);
        component(&mut self.0, value);
    }

    pub fn json(value: &impl Serialize) -> Result<Self> {
        let mut params = Self::new();
        params.push_json("input_json", value)?;
        Ok(params)
    }

    pub fn push_json<T: Serialize + ?Sized>(&mut self, name: &str, value: &T) -> Result<()> {
        self.key(name);
        // Serialize into the percent encoder, avoiding an intermediate JSON DOM
        // or serialized String and preserving borrowed request fields.
        serde_json::to_writer(Encoded(&mut self.0), value)
            .map_err(|_| Error::InvalidParameter("request field must serialize as JSON"))
    }

    fn key(&mut self, name: &str) {
        if !self.0.is_empty() {
            self.0.push('&');
        }
        // Names are ordinary text; borrow their unescaped ASCII prefix
        // instead of visiting every byte through the escaping encoder.
        FormValue::encode(name, &mut self.0);
        self.0.push('=');
    }

    /// Write `&name` once, then let the caller append its index spelling
    /// directly into the final form buffer.
    fn prefixed_key(&mut self, name: &str, finish: impl FnOnce(&mut String)) {
        if !self.0.is_empty() {
            self.0.push('&');
        }
        FormValue::encode(name, &mut self.0);
        finish(&mut self.0);
        self.0.push('=');
    }

    pub fn finish(self) -> String {
        self.0
    }
}

struct Encoded<'a>(&'a mut String);

impl io::Write for Encoded<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        component(self.0, bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn component(output: &mut String, bytes: &[u8]) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for &byte in bytes {
        match byte {
            byte if unescaped(byte) => output.push(char::from(byte)),
            b' ' => output.push('+'),
            _ => {
                output.push('%');
                output.push(char::from(HEX[(byte >> 4) as usize]));
                output.push(char::from(HEX[(byte & 15) as usize]));
            }
        }
    }
}

fn unescaped(byte: u8) -> bool {
    matches!(byte, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'*')
}

/// Only internal wire scalars implement this trait. Text is borrowed and integer
/// formatting uses a stack buffer, so values need no temporary String.
pub(crate) trait FormValue {
    fn encode(&self, output: &mut String);
}

impl FormValue for str {
    fn encode(&self, output: &mut String) {
        if let Some(index) = self.bytes().position(|byte| !unescaped(byte)) {
            // This prefix contains only ASCII, so index is a UTF-8 boundary.
            output.push_str(&self[..index]);
            component(output, &self.as_bytes()[index..]);
        } else {
            output.push_str(self);
        }
    }
}

impl FormValue for String {
    fn encode(&self, output: &mut String) {
        self.as_str().encode(output);
    }
}

impl<T: FormValue + ?Sized> FormValue for &T {
    fn encode(&self, output: &mut String) {
        T::encode(self, output);
    }
}

impl FormValue for bool {
    fn encode(&self, output: &mut String) {
        output.push_str(if *self { "true" } else { "false" });
    }
}

impl FormValue for SteamId {
    fn encode(&self, output: &mut String) {
        self.raw().encode(output);
    }
}

macro_rules! integer_values {
    ($($integer:ty),+ $(,)?) => {$(
        impl FormValue for $integer {
            fn encode(&self, output: &mut String) {
                output.push_str(itoa::Buffer::new().format(*self));
            }
        }
    )+};
}

integer_values!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn scalar_wire_values_match_the_standard_encoder_at_integer_extremes() {
        let owned = "owned +&=%/中文\0".to_owned();
        let id = SteamId(u64::MAX);
        let actual = Params::new()
            .with("text", "borrowed +&=%/中文\0")
            .with("owned", &owned)
            .with("id", id)
            .with("zero", 0u8)
            .with("unsigned", u128::MAX)
            .with("signed", i128::MIN)
            .with("isize", isize::MIN)
            .with("usize", usize::MAX)
            .with("true", true)
            .with("false", false)
            .finish();
        let expected = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("text", "borrowed +&=%/中文\0")
            .append_pair("owned", &owned)
            .append_pair("id", &id.to_string())
            .append_pair("zero", "0")
            .append_pair("unsigned", &u128::MAX.to_string())
            .append_pair("signed", &i128::MIN.to_string())
            .append_pair("isize", &isize::MIN.to_string())
            .append_pair("usize", &usize::MAX.to_string())
            .append_pair("true", "true")
            .append_pair("false", "false")
            .finish();
        assert_eq!(actual, expected);
    }

    #[test]
    fn indexed_keys_match_formatted_spelling_without_temporary_strings() {
        let mut indexed = Params::new().with("count", 2);
        indexed.push_indexed("name", 0, "first +");
        indexed.push_indexed("name", 1, u64::MAX);
        let mut suffixed = Params::new().with("class_count", 2);
        suffixed.push_suffixed("classid", 0, 1);
        suffixed.push_suffixed("classid", 1, u64::MAX);
        let expected = |pairs: &[(&str, String)]| {
            let mut serializer = url::form_urlencoded::Serializer::new(String::new());
            for (key, value) in pairs {
                serializer.append_pair(key, value);
            }
            serializer.finish()
        };
        assert_eq!(
            indexed.finish(),
            expected(&[
                ("count", "2".to_owned()),
                ("name[0]", "first +".to_owned()),
                ("name[1]", u64::MAX.to_string()),
            ])
        );
        assert_eq!(
            suffixed.finish(),
            expected(&[
                ("class_count", "2".to_owned()),
                ("classid0", "1".to_owned()),
                ("classid1", u64::MAX.to_string()),
            ])
        );
    }

    #[test]
    fn service_json_preserves_escaping_unsigned_ids_and_field_types() {
        #[derive(Serialize)]
        struct Request<'a> {
            steamid: u64,
            memo: &'a str,
            enabled: bool,
        }
        let form = Params::json(&Request {
            steamid: u64::MAX,
            memo: "\"\\\n\0+&%=中文",
            enabled: false,
        })
        .unwrap()
        .with("format", "json")
        .finish();
        let pairs: Vec<_> = url::form_urlencoded::parse(form.as_bytes()).collect();
        let value: serde_json::Value = serde_json::from_str(&pairs[0].1).unwrap();
        assert_eq!(value["steamid"].as_u64(), Some(u64::MAX));
        assert_eq!(value["memo"], "\"\\\n\0+&%=中文");
        assert_eq!(value["enabled"], false);
        assert_eq!(pairs[1], ("format".into(), "json".into()));
    }

    #[test]
    fn service_serialization_errors_do_not_expose_untrusted_text() {
        struct Fails;
        impl Serialize for Fails {
            fn serialize<S: serde::Serializer>(
                &self,
                _: S,
            ) -> std::result::Result<S::Ok, S::Error> {
                Err(serde::ser::Error::custom("secret-login-token"))
            }
        }
        match Params::json(&Fails) {
            Err(error @ Error::InvalidParameter(_)) => {
                assert!(!error.to_string().contains("secret-login-token"));
            }
            _ => panic!("serialization must fail closed"),
        }
    }
}
