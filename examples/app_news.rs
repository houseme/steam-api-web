// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Read a small public news page without exposing a publisher key.
//!
//! Run with `cargo run --example app_news -- <appid>`.

use steam_api_web::news::NewsOptions;
use steam_api_web::{Host, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let appid: u32 = args.next().ok_or("usage: app_news <appid>")?.parse()?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let client = SteamClient::builder().host(Host::Public).build()?;
    let response = client
        .news()
        .get_news_for_app(
            appid,
            NewsOptions {
                count: Some(5),
                maxlength: Some(256),
                ..Default::default()
            },
        )
        .await?;
    println!("App news: {response:?}");
    Ok(())
}
