// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

// Reference encoder from 2077350; retained only for reproducible benchmarks.
use crate::{Error, Result};
use serde::Serialize;
use std::io;

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
