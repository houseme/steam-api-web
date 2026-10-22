// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Read pending Workshop approval requests without submitting a moderation decision.
//!
//! Run with `cargo run --example workshop_approvals -- <appid>`.

use steam_api_web::published_file_moderation::ApprovalRequestsOptions;
use steam_api_web::{ApiKey, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let appid: u32 = args
        .next()
        .ok_or("usage: workshop_approvals <appid>")?
        .parse()?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let key = ApiKey::publisher(std::env::var("STEAM_PUBLISHER_KEY")?)?;
    let client = SteamClient::builder().api_key(key).build()?;
    let response = client
        .published_file_moderation()
        .get_all_approval_requests(
            appid,
            ApprovalRequestsOptions {
                revision: None,
                start: Some(0),
                count: Some(20),
            },
        )
        .await?;
    println!("Approval requests: {response:?}");
    Ok(())
}
