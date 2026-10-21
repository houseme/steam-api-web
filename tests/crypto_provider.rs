// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Separate process: verify AWS-LC configuration ignores the global provider.
#![allow(clippy::unwrap_used)]

#[test]
fn client_does_not_adopt_a_host_installed_crypto_provider() {
    let mut unusable_global = rustls::crypto::aws_lc_rs::default_provider();
    unusable_global.cipher_suites.clear();
    unusable_global.install_default().unwrap();
    // A client that implicitly used that global provider could not build.
    let client = steam_api_web::SteamClient::builder().build().unwrap();
    assert_eq!(client.metrics().attempts, 0);
}
