// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Offline scalar decoder comparison against 0a4617b; no network or full reports.
#![allow(clippy::unwrap_used)]

#[path = "support/integer_v050.rs"]
mod baseline;
// Include the actual shared decoder; the harness does not run its unit tests.
#[allow(dead_code, unused_imports)]
#[path = "../src/types.rs"]
mod types;

use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{hint::black_box, str::FromStr, time::Instant};
use steam_api_web::{Error, Result};

#[derive(Serialize)]
#[serde(transparent)]
struct Previous<T>(T);
impl<'de, T: Deserialize<'de> + FromStr> Deserialize<'de> for Previous<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        baseline::integer(deserializer).map(Self)
    }
}

#[derive(Serialize)]
#[serde(transparent)]
struct Current<T>(T);
impl<'de, T: FromStr + TryFrom<u64> + TryFrom<i64>> Deserialize<'de> for Current<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        types::integer(deserializer).map(Self)
    }
}

fn elapsed<T>(iterations: usize, operation: &mut impl FnMut() -> T) -> f64 {
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(operation());
    }
    start.elapsed().as_nanos() as f64 / iterations as f64
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    (values[(values.len() - 1) / 2] + values[values.len() / 2]) / 2.
}

fn compare<A: Serialize, B: Serialize>(
    name: &str,
    mut previous: impl FnMut() -> A,
    mut current: impl FnMut() -> B,
) {
    assert_eq!(
        serde_json::to_value(previous()).unwrap(),
        serde_json::to_value(current()).unwrap()
    );
    for _ in 0..100 {
        black_box(previous());
        black_box(current());
    }
    let mut a1 = Vec::new();
    let mut a2 = Vec::new();
    let mut b = Vec::new();
    for _ in 0..9 {
        a1.push(elapsed(2_000, &mut previous));
        b.push(elapsed(2_000, &mut current));
        b.push(elapsed(2_000, &mut current));
        a2.push(elapsed(2_000, &mut previous));
    }
    let drift = (median(&mut a2) / median(&mut a1) - 1.) * 100.;
    assert!(
        drift.abs() <= 10.,
        "baseline drift {drift:+.1}% exceeds 10%; rerun {name}"
    );
    a1.extend(a2);
    let previous = median(&mut a1);
    let current = median(&mut b);
    println!(
        "{name}: baseline {previous:.0} ns/op, current {current:.0} ns/op, {:.1}% less time, baseline drift {drift:+.1}%",
        (1. - current / previous) * 100.
    );
}

fn decode<T: DeserializeOwned>(wire: &[u8], via_value: bool) -> Vec<T> {
    if via_value {
        // Match the client's parse-to-Value then consume-to-typed stages.
        let value: Value = serde_json::from_slice(black_box(wire)).unwrap();
        serde_json::from_value(value).unwrap()
    } else {
        serde_json::from_slice(black_box(wire)).unwrap()
    }
}

fn cases<T>(name: &str, values: &[T])
where
    T: Serialize + DeserializeOwned + FromStr + TryFrom<u64> + TryFrom<i64> + ToString,
{
    for quoted in [false, true] {
        let wire = if quoted {
            serde_json::to_vec(&values.iter().map(ToString::to_string).collect::<Vec<_>>()).unwrap()
        } else {
            serde_json::to_vec(values).unwrap()
        };
        for via_value in [false, true] {
            let case = format!(
                "{name} / {} / {}",
                if quoted { "strings" } else { "numbers" },
                if via_value {
                    "JSON -> Value -> typed"
                } else {
                    "JSON -> typed"
                }
            );
            compare(
                &case,
                || decode::<Previous<T>>(&wire, via_value),
                || decode::<Current<T>>(&wire, via_value),
            );
        }
    }
}

fn main() {
    println!(
        "100 exact integers per operation; reference 0a4617b; optimized profile, 9 ABBA rounds, 10% drift gate, parsing and output drop included"
    );
    cases(
        "signed",
        &(0..100)
            .map(|index| [i64::MIN, -1, 0, i64::MAX][index % 4])
            .collect::<Vec<_>>(),
    );
    cases(
        "unsigned",
        &(0..100)
            .map(|index| [0, 480, 76561198000000000, u64::MAX][index % 4])
            .collect::<Vec<_>>(),
    );
}
