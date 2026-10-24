// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Offline CSV and legacy list-encoding comparisons against reviewed
//! implementations. Kept in its own binary so its code cannot shift the
//! codegen layout of the core request-encoding benchmarks.

#![allow(clippy::unwrap_used)]

#[path = "support/harness.rs"]
mod harness;

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

use harness::compare;
use std::hint::black_box;
use steam_api_web::{Error, Result, SteamId};

#[expect(
    clippy::iter_cloned_collect,
    reason = "Preserve the reviewed collect-and-join publisher filter baseline"
)]
fn main() {
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
