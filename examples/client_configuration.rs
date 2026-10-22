// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Configure a reusable bounded client and inspect redacted aggregate metrics.
//!
//! Run with `cargo run --example client_configuration -- <appid>`.

use std::time::Duration;
use steam_api_web::RetryPolicy;
use steam_api_web::{Host, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let appid: u32 = args
        .next()
        .ok_or("usage: client_configuration <appid>")?
        .parse()?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let client = SteamClient::builder()
        .host(Host::Public)
        .connect_timeout(Duration::from_secs(3))
        .request_timeout(Duration::from_secs(10))
        .operation_timeout(Duration::from_secs(30))
        .max_in_flight(8)
        .max_response_bytes(1024 * 1024)
        .min_request_interval(Duration::from_millis(100))
        .retry_policy(RetryPolicy {
            max_retries: 2,
            initial_backoff: Duration::from_millis(200),
            max_backoff: Duration::from_secs(2),
        })
        .build()?;
    let shared = client.clone();
    let count = shared
        .user_stats()
        .get_number_of_current_players(appid)
        .await?;
    println!(
        "Online players: {}; shared metrics: {:?}",
        count.player_count,
        client.metrics()
    );
    Ok(())
}
