// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Lossless wire types shared by the three Steam interfaces.

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use serde_json::Number;
use std::{fmt, str::FromStr};

/// A 64-bit Steam ID accepting JSON numbers and decimal strings.
/// Zero is preserved in responses (e.g. a canceled license's owner). Request
/// methods reject zero user IDs. Serialization avoids JavaScript rounding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SteamId(pub u64);

impl SteamId {
    /// Construct from the native SDK's raw 64-bit representation.
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }
    /// Return the raw value for SDK or network interoperability.
    pub const fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for SteamId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl From<u64> for SteamId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}
impl FromStr for SteamId {
    type Err = std::num::ParseIntError;
    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        value.parse().map(Self)
    }
}
impl Serialize for SteamId {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}
impl<'de> Deserialize<'de> for SteamId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        u64_string(deserializer).map(Self)
    }
}
pub(crate) fn u64_string<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<u64, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Wire {
        Integer(u64),
        Text(String),
    }
    match Wire::deserialize(deserializer)? {
        Wire::Integer(value) => Ok(value),
        Wire::Text(value) => value
            .parse()
            .map_err(|_| de::Error::custom("expected unsigned 64-bit decimal")),
    }
}

/// A JSON number or numeric string, preserving 64-bit signed/unsigned integers.
/// Non-finite floats and non-numeric strings are rejected.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(transparent)]
pub struct StatValue(pub Number);

impl<'de> Deserialize<'de> for StatValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Number(Number),
            Text(String),
        }
        match Wire::deserialize(deserializer)? {
            Wire::Number(value) => Ok(Self(value)),
            Wire::Text(value) => value
                .parse()
                .map(Self)
                .map_err(|_| de::Error::custom("expected numeric statistic")),
        }
    }
}
pub(crate) fn nonzero(value: u64, name: &'static str) -> crate::Result<()> {
    if value == 0 {
        Err(crate::Error::InvalidParameter(name))
    } else {
        Ok(())
    }
}
pub(crate) fn text(value: &str, name: &'static str) -> crate::Result<()> {
    if value.trim().is_empty() || value.contains('\0') {
        Err(crate::Error::InvalidParameter(name))
    } else {
        Ok(())
    }
}
