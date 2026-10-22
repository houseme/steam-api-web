// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Read public Workshop file metadata without subscribing to files.
//!
//! Run with `cargo run --example remote_file_details -- <publishedfileid>`.

use steam_api_web::remote_storage::PublishedFileIdsRequest;
use steam_api_web::{Host, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let publishedfileid: u64 = args
        .next()
        .ok_or("usage: remote_file_details <publishedfileid>")?
        .parse()?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let client = SteamClient::builder().host(Host::Public).build()?;
    let response = client
        .remote_storage()
        .get_published_file_details(PublishedFileIdsRequest {
            publishedfileids: &[publishedfileid],
        })
        .await?;
    println!("Published file details: {response:?}");
    Ok(())
}
