// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Read Cloud metadata with user-granted read_cloud OAuth access.

use steam_api_web::{
    Host, SteamClient,
    cloud::{EnumerateUserFilesRequest, OAuthAccessToken},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let appid: u32 = std::env::args()
        .nth(1)
        .ok_or("usage: cloud_files <appid>; set STEAM_OAUTH_ACCESS_TOKEN")?
        .parse()?;
    let token = OAuthAccessToken::new(std::env::var("STEAM_OAUTH_ACCESS_TOKEN")?)?;
    let client = SteamClient::builder().host(Host::Public).build()?;
    let page = client
        .cloud()
        .enumerate_user_files(
            &token,
            &EnumerateUserFilesRequest {
                appid,
                extended_details: Some(false),
                count: Some(100),
                start_index: Some(0),
            },
        )
        .await?;
    // Do not print OAuth tokens, private file names, or signed download URLs.
    println!(
        "Cloud files: {} in this page, {} total",
        page.files.len(),
        page.total_files
    );
    Ok(())
}
