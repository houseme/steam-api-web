// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Offline request-encoding comparisons against the reviewed 0.3 implementation.
#![allow(clippy::unwrap_used)]

use serde::Serialize;
use std::{fmt::Debug, hint::black_box, time::Instant};
use steam_api_web::{Error, Result, SteamId};

#[allow(dead_code)]
#[path = "support/request_v030.rs"]
mod baseline;
#[path = "support/preparation_v040.rs"]
mod preparation_baseline;
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
        MicroTransaction,
        VacStatus,
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
macro_rules! cart_form {
    ($params:path) => {{
        let mut params = <$params>::new().with("appid", 480).with("itemcount", 100);
        for index in 0..100 {
            params.push(&format!("itemid[{index}]"), 1000 + index as u32);
            params.push(&format!("qty[{index}]"), 1_i16);
            params.push(&format!("amount[{index}]"), 499_i64);
            params.push(&format!("description[{index}]"), "Item + & name");
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
fn compare<T: PartialEq + Debug>(
    name: &str,
    iterations: usize,
    mut old: impl FnMut() -> T,
    mut new: impl FnMut() -> T,
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

#[derive(Debug)]
struct WireRequest(reqwest::Request);

impl PartialEq for WireRequest {
    fn eq(&self, other: &Self) -> bool {
        self.0.method() == other.0.method()
            && self.0.url() == other.0.url()
            && self.0.headers() == other.0.headers()
            && self.0.body().and_then(reqwest::Body::as_bytes)
                == other.0.body().and_then(reqwest::Body::as_bytes)
    }
}

fn preparation() {
    // Requests are only built, never sent. No platform roots or global crypto
    // provider are needed for the explicit AWS-LC configuration in this probe.
    let tls = rustls::ClientConfig::builder_with_provider(std::sync::Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_root_certificates(rustls::RootCertStore::empty())
    .with_no_client_auth();
    let http = reqwest::Client::builder()
        .tls_backend_preconfigured(tls)
        .build()
        .unwrap();
    println!(
        "Request preparation reference ee0e71b; includes equal input/URL clones, request construction and output drop; no network or cache lookup"
    );
    for (name, post, form) in [
        (
            "single-send GET preparation",
            false,
            "appid=480&steamid=18446744073709551615&format=json".to_owned(),
        ),
        (
            "4 KiB single-send GET preparation",
            false,
            format!("access_token={}&format=json", "token_".repeat(683)),
        ),
        (
            "POST request preparation",
            true,
            "appid=480&orderid=18446744073709551615&format=json".to_owned(),
        ),
    ] {
        let endpoint = if post {
            request::Endpoint::post("ISteamMicroTxn/FinalizeTxn/v2/", request::Access::Publisher)
        } else {
            request::Endpoint::get_once("ISteamMicroTxn/QueryTxn/v3/", request::Access::Publisher)
        };
        let url =
            url::Url::parse(&format!("https://partner.steam-api.com/{}", endpoint.path)).unwrap();
        compare(
            name,
            10_000,
            || {
                WireRequest(preparation_baseline::prepare(
                    &http,
                    endpoint.operation,
                    url.clone(),
                    form.clone(),
                ))
            },
            || {
                WireRequest(
                    endpoint
                        .request(&http, url.clone(), form.clone())
                        .build()
                        .unwrap(),
                )
            },
        );
    }
}

#[expect(
    clippy::iter_cloned_collect,
    reason = "Preserve the reviewed collect-and-join publisher filter baseline"
)]
fn lists() {
    println!("List encoding reference 9d2cab1; exact wire equality and output drop; no network");
    for count in [1, 100] {
        let ids: Vec<_> = (0..count).map(|index| SteamId(u64::MAX - index)).collect();
        compare(
            &format!("{count} CSV Steam IDs"),
            20_000,
            || {
                let joined = black_box(&ids)
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(",");
                request::Params::new()
                    .with("steamids", joined)
                    .with("format", "json")
                    .finish()
            },
            || {
                request::Params::new()
                    .with(
                        "steamids",
                        request::CommaSeparated(black_box(ids.as_slice())),
                    )
                    .with("format", "json")
                    .finish()
            },
        );
    }
    let types = ["game", "application", "tool", "demo", "dlc", "music"];
    compare(
        "six CSV app types",
        20_000,
        || {
            let joined = black_box(&types)
                .iter()
                .copied()
                .collect::<Vec<_>>()
                .join(",");
            request::Params::new()
                .with("type_filter", joined)
                .with("format", "json")
                .finish()
        },
        || {
            request::Params::new()
                .with("type_filter", request::CommaSeparated(black_box(&types)))
                .with("format", "json")
                .finish()
        },
    );
    let assets: Vec<_> = (0..100).map(|index| format!("asset-{index}")).collect();
    compare(
        "100 legacy asset lines",
        2_000,
        || {
            let mut params = request::Params::new()
                .with("appid", 480)
                .with("steamid", SteamId(u64::MAX));
            for (index, asset) in black_box(&assets).iter().enumerate() {
                params.push(&format!("assetid{index}"), asset);
                params.push(&format!("assetquantity{index}"), 1_u32);
            }
            params.with("format", "json").finish()
        },
        || {
            let mut params = request::Params::new()
                .with("appid", 480)
                .with("steamid", SteamId(u64::MAX));
            for (index, asset) in black_box(&assets).iter().enumerate() {
                params.push_suffixed("assetid", index, asset);
                params.push_suffixed("assetquantity", index, 1_u32);
            }
            params.with("format", "json").finish()
        },
    );
}

fn main() {
    if std::env::args().any(|arg| arg == "--lists-only") {
        lists();
        return;
    }
    if std::env::args().any(|arg| arg == "--preparation-only") {
        preparation();
        return;
    }
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
        || {
            let mut params = request::Params::new()
                .with("appid", 480)
                .with("class_count", 100);
            for index in 0..100 {
                params.push_suffixed("classid", index, u64::MAX - index as u64);
            }
            params.with("format", "json").finish()
        },
    );
    compare(
        "100 cart lines",
        500,
        || cart_form!(baseline::Params),
        || {
            let mut params = request::Params::new()
                .with("appid", 480)
                .with("itemcount", 100);
            for index in 0..100 {
                params.push_indexed("itemid", index, 1000 + index as u32);
                params.push_indexed("qty", index, 1_i16);
                params.push_indexed("amount", index, 499_i64);
                params.push_indexed("description", index, "Item + & name");
            }
            params.with("format", "json").finish()
        },
    );
    compare(
        "100 JSON definitions",
        50,
        || definition_form!(baseline::Params, &definitions),
        || definition_form!(request::Params, &definitions),
    );
    preparation();
}
