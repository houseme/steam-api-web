// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Exact financial integer decoder from 0a4617b, before the 0.5 review.

use serde::{Deserialize, Deserializer, de};
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
