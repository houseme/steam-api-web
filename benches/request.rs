// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Offline request-encoding comparisons against the reviewed 0.3 implementation.
#![allow(clippy::unwrap_used)]

use serde::Serialize;
use std::{hint::black_box, time::Instant};
use steam_api_web::{Error, Result, SteamId};

#[allow(dead_code)]
#[path = "support/request_v030.rs"]
mod baseline;
#[allow(dead_code, unused_imports)]
#[path = "../src/request.rs"]
mod request;

// Endpoint response policies are not involved in these encoding measurements.
// Supply the unused descriptor type required by the included production module.
#[allow(dead_code)]
mod response {
    #[derive(Clone, Copy)]
    pub enum ResponsePolicy {
        Json,
        BuildConfirmation,
        CloudJson,
        CloudAcknowledgement,
    }
}

#[derive(Serialize)]
struct CloudRequest<'a> {
    appid: u32,
    machine_name: &'a str,
    files_to_upload: &'a [&'a str],
    files_to_delete: &'a [&'a str],
}
#[derive(Serialize)]
struct Definition<'a> {
    appid: u32,
    itemdefid: u64,
    name: &'a str,
    description: &'a str,
}

macro_rules! scalar_form {
    ($params:path) => {
        <$params>::new()
            .with("appid", 480)
            .with("steamid", SteamId(u64::MAX))
            .with("betakey", "beta + & release")
            .with("format", "json")
            .finish()
    };
}
macro_rules! cloud_form {
    ($params:path, $payload:expr, $token:expr) => {
        <$params>::json($payload)
            .unwrap()
            .with("access_token", $token)
            .with("format", "json")
            .finish()
    };
}
macro_rules! indexed_form {
    ($params:path) => {{
        let mut params = <$params>::new().with("appid", 480).with("class_count", 100);
        for index in 0..100 {
            params.push(&format!("classid{index}"), u64::MAX - index as u64);
        }
        params.with("format", "json").finish()
    }};
}
macro_rules! definition_form {
    ($params:path, $definitions:expr) => {{
        let mut params = <$params>::new().with("appid", 480);
        params.push_json("itemdefs", $definitions).unwrap();
        params.with("format", "json").finish()
    }};
}
macro_rules! binary_form {
    ($params:path, $bytes:expr) => {{
        let mut params = <$params>::new();
        params.push_bytes("details", $bytes);
        params.finish()
    }};
}

fn elapsed(iterations: usize, operation: &mut impl FnMut()) -> f64 {
    let start = Instant::now();
    for _ in 0..iterations {
        operation();
    }
    start.elapsed().as_nanos() as f64 / iterations as f64
}
fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    (values[(values.len() - 1) / 2] + values[values.len() / 2]) / 2.
}
fn compare(
    name: &str,
    iterations: usize,
    mut old: impl FnMut() -> String,
    mut new: impl FnMut() -> String,
) {
    assert_eq!(old(), new(), "wire mismatch: {name}");
    for _ in 0..100 {
        black_box(old());
        black_box(new());
    }
    let mut a1 = Vec::new();
    let mut a2 = Vec::new();
    let mut b = Vec::new();
    for _ in 0..9 {
        a1.push(elapsed(iterations, &mut || {
            black_box(old());
        }));
        b.push(elapsed(iterations, &mut || {
            black_box(new());
        }));
        b.push(elapsed(iterations, &mut || {
            black_box(new());
        }));
        a2.push(elapsed(iterations, &mut || {
            black_box(old());
        }));
    }
    let drift = (median(&mut a2) / median(&mut a1) - 1.) * 100.;
    let mut a = a1;
    a.extend(a2);
    let old = median(&mut a);
    let new = median(&mut b);
    println!(
        "{name}: baseline {old:.0} ns/op, current {new:.0} ns/op, {:.1}% less time, baseline drift {drift:+.1}%",
        (1. - new / old) * 100.
    );
    assert!(
        drift.abs() <= 10.,
        "baseline drift exceeds 10%; rerun before interpreting {name}"
    );
}

fn main() {
    let token = "oauth_ascii_token_0123456789".repeat(160);
    let cloud = CloudRequest {
        appid: 480,
        machine_name: "Handheld",
        files_to_upload: &["%WinAppDataLocal%/game/save.sav"],
        files_to_delete: &[],
    };
    let description = "A representative definition description. ".repeat(32);
    let definitions: Vec<_> = (0..100)
        .map(|itemdefid| Definition {
            appid: 480,
            itemdefid,
            name: "Item + & name",
            description: &description,
        })
        .collect();
    let binary: Vec<_> = (0..=255).collect();
    assert_eq!(
        binary_form!(baseline::Params, &binary),
        binary_form!(request::Params, &binary)
    );
    println!(
        "Reviewed baseline 2077350; optimized build; wire-equivalence checks and A/B/B/A medians; timings include output drop"
    );
    compare(
        "scalar form",
        20_000,
        || scalar_form!(baseline::Params),
        || scalar_form!(request::Params),
    );
    compare(
        "Cloud form with 4 KiB OAuth token",
        500,
        || cloud_form!(baseline::Params, &cloud, black_box(token.as_str())),
        || cloud_form!(request::Params, &cloud, black_box(token.as_str())),
    );
    compare(
        "100 indexed IDs",
        500,
        || indexed_form!(baseline::Params),
        || indexed_form!(request::Params),
    );
    compare(
        "100 JSON definitions",
        50,
        || definition_form!(baseline::Params, &definitions),
        || definition_form!(request::Params, &definitions),
    );
}
