// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Read one review page, preserving Steam's opaque continuation cursor.
//!
//! Run with `cargo run --example app_reviews -- <appid>`.

use steam_api_web::reviews::AppReviewsOptions;
use steam_api_web::{Host, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let appid: u32 = args.next().ok_or("usage: app_reviews <appid>")?.parse()?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let client = SteamClient::builder().host(Host::Public).build()?;
    let response = client
        .reviews()
        .get_app_reviews(
            appid,
            AppReviewsOptions {
                num_per_page: Some(20),
                ..Default::default()
            },
        )
        .await?;
    // Read the returned cursor for a subsequent page; do not log review authors or text here.
    println!("Reviews in this page: {}", response.reviews.len());
    Ok(())
}
