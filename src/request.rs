// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

use crate::{Error, Result};
use serde::Serialize;
use std::io;

#[derive(Clone, Copy)]
pub(crate) enum Access {
    Anonymous,
    User,
    Publisher,
}

/// The method and replay policy cannot be configured inconsistently.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Operation {
    Read,
    SingleGet,
    Post,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum BodyPolicy {
    Json,
    EmptyCreated,
    SteamResultJson,
    SteamResultEmpty,
}

impl BodyPolicy {
    pub const fn requires_steam_result(self) -> bool {
        matches!(self, Self::SteamResultJson | Self::SteamResultEmpty)
    }

    pub const fn allows_empty(self, status: u16) -> bool {
        matches!(
            (self, status),
            (Self::EmptyCreated, 201) | (Self::SteamResultEmpty, 200)
        )
    }
}

/// Only fixed internal paths may reach the transport.
#[derive(Clone, Copy)]
pub(crate) struct Endpoint {
    pub path: &'static str,
    pub access: Access,
    pub operation: Operation,
    pub body: BodyPolicy,
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
            body: BodyPolicy::Json,
        }
    }

    /// Only HTTP 201 may omit JSON when mobile confirmation is pending.
    pub const fn allow_empty_created(mut self) -> Self {
        self.body = BodyPolicy::EmptyCreated;
        self
    }

    /// Cloud calls require an explicit successful Steam result header and JSON.
    pub const fn steam_result(mut self) -> Self {
        self.body = BodyPolicy::SteamResultJson;
        self
    }

    /// Bodyless Cloud methods require a successful result header before HTTP 200
    /// can represent success without JSON.
    pub const fn empty_steam_result(mut self) -> Self {
        self.body = BodyPolicy::SteamResultEmpty;
        self
    }
}

/// Encode directly into the final form, without owning each key/value pair.
pub(crate) struct Params(String);

impl Params {
    pub fn new() -> Self {
        Self(String::with_capacity(128))
    }

    pub fn with(mut self, name: &str, value: impl ToString) -> Self {
        self.push(name, value);
        self
    }

    pub fn push(&mut self, name: &str, value: impl ToString) {
        self.push_bytes(name, value.to_string().as_bytes());
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
        component(&mut self.0, name.as_bytes());
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

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
