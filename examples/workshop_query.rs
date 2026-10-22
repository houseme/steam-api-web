// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Read the first cursor-based Workshop search page.
//!
//! Run with `cargo run --example workshop_query -- <creator_appid> <appid>`.

use steam_api_web::published_file::{QueryFilesOptions, QueryFilesPagination, QueryFilesRequest};
use steam_api_web::{ApiKey, Host, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let creator_appid: u32 = args
        .next()
        .ok_or("usage: workshop_query <creator_appid> <appid>")?
        .parse()?;
    let appid: u32 = args
        .next()
        .ok_or("usage: workshop_query <creator_appid> <appid>")?
        .parse()?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let key = ApiKey::user(std::env::var("STEAM_USER_KEY")?)?;
    let client = SteamClient::builder()
        .host(Host::Public)
        .api_key(key)
        .build()?;
    let response = client
        .published_file()
        .query_files(QueryFilesRequest {
            query_type: 1,
            creator_appid,
            appid,
            pagination: QueryFilesPagination::Cursor("*"),
            options: QueryFilesOptions {
                numperpage: Some(20),
                return_tags: true,
                ..Default::default()
            },
        })
        .await?;
    // For the next page, reuse Steam's next_cursor verbatim instead of deriving it from page size.
    println!("Workshop search: {response:?}");
    Ok(())
}
