// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Offline codec comparisons; run with `cargo bench --bench codec --locked`.

#![allow(clippy::unwrap_used)]

#[path = "support/baseline.rs"]
mod baseline;
#[allow(dead_code)]
#[path = "../src/error.rs"]
mod error;
// A harness-free benchmark compiles with cfg(test), but does not run the
// included modules' unit tests. Their test-only imports are unused here.
#[allow(dead_code, unused_imports)]
#[path = "../src/request.rs"]
mod request;
#[allow(dead_code, unused_imports)]
#[path = "../src/response.rs"]
mod response;

use error::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{hint::black_box, time::Instant};
use steam_api_web::SteamId;

// Match the public wrapper's representation while allowing the included private
// decoder to construct it. This wrapper does no work beyond owning the Value.
mod user_auth {
    #[derive(serde::Deserialize)]
    #[serde(transparent)]
    pub struct OpaqueResponse(serde_json::Value);
    impl OpaqueResponse {
        pub fn from_json(value: serde_json::Value) -> Self {
            Self(value)
        }
        pub fn as_json(&self) -> &serde_json::Value {
            &self.0
        }
    }
}

#[derive(Serialize)]
struct ServiceRequest<'a> {
    steamid: steam_api_web::SteamId,
    appid: u32,
    memo: &'a str,
}
#[derive(Deserialize, Debug, PartialEq)]
struct Page {
    games: Vec<Game>,
}
#[derive(Deserialize, Debug, PartialEq)]
struct Game {
    appid: u32,
    name: String,
    description: String,
}

fn form_old() -> String {
    baseline::encode_form(&vec![
        baseline::param("appid", 480),
        baseline::param("steamid", u64::MAX),
        baseline::param("betakey", "beta + & release"),
        baseline::param("format", "json"),
    ])
}
fn form_new() -> String {
    request::Params::new()
        .with("appid", 480)
        .with("steamid", u64::MAX)
        .with("betakey", "beta + & release")
        .with("format", "json")
        .finish()
}
fn service_old() -> String {
    let payload = json!({"steamid": u64::MAX.to_string(), "appid": 480, "memo": "token + & memo"});
    baseline::encode_form(&vec![
        baseline::param("input_json", payload),
        baseline::param("format", "json"),
    ])
}
fn service_new() -> String {
    request::Params::json(&ServiceRequest {
        steamid: steam_api_web::SteamId(u64::MAX),
        appid: 480,
        memo: "token + & memo",
    })
    .unwrap()
    .with("format", "json")
    .finish()
}
fn parsed(bytes: &[u8]) -> response::Response {
    response::Response {
        policy: response::ResponsePolicy::Json,
        status: 200,
        payload: serde_json::from_slice(bytes).unwrap(),
    }
}

fn elapsed(iterations: usize, operation: &mut impl FnMut()) -> f64 {
    let start = Instant::now();
    for _ in 0..iterations {
        operation();
    }
    start.elapsed().as_nanos() as f64 / iterations as f64
}
fn compare(name: &str, iterations: usize, mut old: impl FnMut(), mut new: impl FnMut()) {
    for _ in 0..100 {
        old();
        new();
    }
    let mut old_samples = Vec::new();
    let mut new_samples = Vec::new();
    // Interleave A/B/B/A to reduce drift, then report medians from 18 samples.
    for _ in 0..9 {
        old_samples.push(elapsed(iterations, &mut old));
        new_samples.push(elapsed(iterations, &mut new));
        new_samples.push(elapsed(iterations, &mut new));
        old_samples.push(elapsed(iterations, &mut old));
    }
    old_samples.sort_by(f64::total_cmp);
    new_samples.sort_by(f64::total_cmp);
    let old = (old_samples[8] + old_samples[9]) / 2.;
    let new = (new_samples[8] + new_samples[9]) / 2.;
    println!(
        "{name}: baseline {old:.0} ns/op, current {new:.0} ns/op, {:.2}x, {:.1}% less time",
        old / new,
        (1. - new / old) * 100.
    );
}

fn main() {
    assert_eq!(form_old(), form_new());
    let service_value = |form: String| -> Value {
        let pairs: Vec<_> = url::form_urlencoded::parse(form.as_bytes())
            .into_owned()
            .collect();
        serde_json::from_str(&pairs.iter().find(|(key, _)| key == "input_json").unwrap().1).unwrap()
    };
    assert_eq!(service_value(service_old()), service_value(service_new()));
    let fixture = serde_json::to_vec(&json!({"response": {"games": (0..100).map(|appid| json!({
        "appid": appid, "name": format!("Game {appid}"), "description": "A representative game description. ".repeat(16)
    })).collect::<Vec<_>>()}})).unwrap();
    let expected: Page = baseline::decode(&fixture, &["response"]).unwrap();
    let actual: Page = parsed(&fixture)
        .decode("bench/v1/", &["response"])
        .unwrap()
        .1;
    assert_eq!(expected, actual);
    let expected: user_auth::OpaqueResponse = baseline::decode(&fixture, &[]).unwrap();
    let actual = parsed(&fixture).opaque("bench/v1/").unwrap().1;
    assert_eq!(expected.as_json(), actual.as_json());
    println!(
        "Optimized codec benchmarks; 100 games, {} response bytes; includes JSON parsing and output drop",
        fixture.len()
    );
    compare(
        "form",
        10_000,
        || {
            black_box(form_old());
        },
        || {
            black_box(form_new());
        },
    );
    compare(
        "service JSON",
        5_000,
        || {
            black_box(service_old());
        },
        || {
            black_box(service_new());
        },
    );
    compare(
        "typed response",
        200,
        || {
            black_box(baseline::decode::<Page>(black_box(&fixture), &["response"]).unwrap());
        },
        || {
            black_box(
                parsed(black_box(&fixture))
                    .decode::<Page>("bench/v1/", &["response"])
                    .unwrap(),
            );
        },
    );
    compare(
        "opaque response",
        200,
        || {
            black_box(
                baseline::decode::<user_auth::OpaqueResponse>(black_box(&fixture), &[]).unwrap(),
            );
        },
        || {
            black_box(parsed(black_box(&fixture)).opaque("bench/v1/").unwrap());
        },
    );
}
