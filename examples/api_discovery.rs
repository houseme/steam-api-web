// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Read public server metadata and discover currently visible API methods.
//!
//! Run with `cargo run --example api_discovery -- `.

use steam_api_web::{Host, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = SteamClient::builder().host(Host::Public).build()?;
    let info = client.web_api_util().get_server_info().await?;
    let methods = client.web_api_util().get_supported_api_list().await?;
    // Discovery is visibility-dependent and does not grant access to listed methods.
    println!("Server metadata: {info:?}; visible API catalogue: {methods:?}");
    Ok(())
}
