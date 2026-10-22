// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Transport prepared legacy ciphertext; prefer verify_ticket for new integrations.
//!
//! Run with `cargo run --example legacy_login -- <steamid> <sessionkey_file> <loginkey_file>`.

use steam_api_web::user_auth::AuthenticateUserRequest;
use steam_api_web::{ApiKey, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let steamid: u64 = args
        .next()
        .ok_or("usage: legacy_login <steamid> <sessionkey_file> <loginkey_file>")?
        .parse()?;
    let sessionkey_file: String = args
        .next()
        .ok_or("usage: legacy_login <steamid> <sessionkey_file> <loginkey_file>")?;
    let loginkey_file: String = args
        .next()
        .ok_or("usage: legacy_login <steamid> <sessionkey_file> <loginkey_file>")?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let key = ApiKey::publisher(std::env::var("STEAM_PUBLISHER_KEY")?)?;
    let client = SteamClient::builder().api_key(key).build()?;
    let sessionkey = zeroize::Zeroizing::new(std::fs::read(sessionkey_file)?);
    let encrypted_loginkey = zeroize::Zeroizing::new(std::fs::read(loginkey_file)?);
    let response = client
        .user_auth()
        .authenticate_user(&AuthenticateUserRequest {
            steamid: steamid.into(),
            sessionkey: &sessionkey,
            encrypted_loginkey: &encrypted_loginkey,
        })
        .await?;
    // Files must contain prepared RSA/AES ciphertext, not plaintext keys or hexadecimal text.
    println!("Legacy login response: {response:?}");
    Ok(())
}
