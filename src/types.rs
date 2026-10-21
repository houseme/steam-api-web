// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Lossless wire scalars and calendar validation shared by Steam interfaces.

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

/// Validate one four-digit Gregorian date without timezone conversion.
pub(crate) fn valid_calendar_date(year: u32, month: u32, day: u32) -> bool {
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=9999).contains(&year) && (1..=days).contains(&day)
}

/// Validate the UTC timestamp spelling accepted by the reporting endpoints.
/// Fractional precision and the original caller spelling remain unchanged.
pub(crate) fn valid_utc_time(value: &str) -> bool {
    utc_time_parts(value).is_some()
}

/// Compare UTC timestamps without floating point or lexically comparing their
/// different suffixes/fraction widths. None means either input is invalid.
pub(crate) fn compare_utc_times(left: &str, right: &str) -> Option<std::cmp::Ordering> {
    let (left, left_fraction) = utc_time_parts(left)?;
    let (right, right_fraction) = utc_time_parts(right)?;
    Some(
        left.cmp(&right)
            .then_with(|| left_fraction.cmp(right_fraction)),
    )
}

fn utc_time_parts(value: &str) -> Option<([u32; 6], &[u8])> {
    let value = value
        .strip_suffix('Z')
        .or_else(|| value.strip_suffix('z'))
        .or_else(|| value.strip_suffix("+00:00"))?;
    let bytes = value.as_bytes();
    if bytes.len() < 19
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !matches!(bytes[10], b'T' | b't')
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[..19]
            .iter()
            .enumerate()
            .any(|(index, byte)| ![4, 7, 10, 13, 16].contains(&index) && !byte.is_ascii_digit())
    {
        return None;
    }
    let number = |start: usize, end: usize| {
        bytes[start..end]
            .iter()
            .fold(0u32, |value, digit| value * 10 + u32::from(digit - b'0'))
    };
    let parts = [
        number(0, 4),
        number(5, 7),
        number(8, 10),
        number(11, 13),
        number(14, 16),
        number(17, 19),
    ];
    if !valid_calendar_date(parts[0], parts[1], parts[2])
        || parts[3] > 23
        || parts[4] > 59
        || parts[5] > 60
    {
        return None;
    }
    let fraction = if bytes.len() == 19 {
        &[][..]
    } else {
        if bytes[19] != b'.' || bytes.len() == 20 || !bytes[20..].iter().all(u8::is_ascii_digit) {
            return None;
        }
        &bytes[20..]
    };
    // Trailing zeros do not change a fraction. With those removed, byte order
    // also compares different decimal widths correctly, with no allocation.
    let fraction_end = fraction
        .iter()
        .rposition(|digit| *digit != b'0')
        .map_or(0, |index| index + 1);
    Some((parts, &fraction[..fraction_end]))
}

#[cfg(test)]
mod calendar_tests {
    use super::*;
    use std::cmp::Ordering::{Equal, Greater, Less};

    #[test]
    fn utc_comparison_preserves_decimal_precision_and_equivalent_spellings() {
        for (left, right, expected) in [
            (
                "2026-10-21T00:00:00.10Z",
                "2026-10-21t00:00:00.1+00:00",
                Equal,
            ),
            ("2026-10-21T00:00:00z", "2026-10-21T00:00:00.000Z", Equal),
            (
                "2026-10-21T00:00:00.0000000000000000001Z",
                "2026-10-21T00:00:00.0000000000000000002Z",
                Less,
            ),
            ("2026-10-21T00:00:00.1Z", "2026-10-21T00:00:00.01Z", Greater),
            ("2026-10-22T00:00:00Z", "2026-10-21T23:59:59.999Z", Greater),
        ] {
            assert_eq!(compare_utc_times(left, right), Some(expected));
        }
    }

    #[test]
    fn calendar_and_utc_validation_reject_invalid_wire_ranges() {
        assert!(valid_calendar_date(2000, 2, 29));
        assert!(!valid_calendar_date(2100, 2, 29));
        for value in [
            "0000-01-01T00:00:00Z",
            "2026-02-29T00:00:00Z",
            "2026-01-01T24:00:00Z",
            "2026-01-01T00:00:61Z",
            "2026-01-01T00:00:00.Z",
            "2026-01-01T00:00:00+01:00",
            "2026-01-01T00:00:00.中文Z",
        ] {
            assert!(!valid_utc_time(value), "{value}");
            assert_eq!(compare_utc_times(value, "2026-01-01T00:00:00Z"), None);
        }
        assert!(valid_utc_time("2024-02-29T23:59:59.12345678901234567890Z"));
    }
}
