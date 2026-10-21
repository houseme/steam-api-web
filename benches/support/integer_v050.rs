// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Exact financial integer decoder from 0a4617b, before the 0.5 review.

use serde::{Deserialize, Deserializer, de};
use serde_json::Number;
use std::str::FromStr;

#[derive(Deserialize)]
#[serde(untagged)]
enum Integer<T> {
    Number(T),
    Text(String),
}

pub fn integer<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + FromStr,
{
    match Integer::<T>::deserialize(deserializer)? {
        Integer::Number(value) => Ok(value),
        Integer::Text(value) => value
            .parse()
            .map_err(|_| de::Error::custom("expected exact decimal integer")),
    }
}

// StatValue's number/string decoder from the same reviewed revision.
#[derive(serde::Serialize)]
#[serde(transparent)]
pub struct PreviousStatistic(pub Number);
impl<'de> Deserialize<'de> for PreviousStatistic {
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
