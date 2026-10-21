// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Async Steam Web API bindings with bounded resource use and conservative retries.
//!
//! Clone and reuse one [`SteamClient`] to share its connection pool, concurrency
//! limit, pacing, rate-limit cooldown, and metrics. Requires a Tokio runtime.
//! TLS uses reqwest's Rustls / AWS-LC backend; native TLS is disabled.
//!
//! ```no_run
//! use steam_api_web::{ApiKey, SteamClient};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let key = ApiKey::publisher(std::env::var("STEAM_PUBLISHER_KEY")?)?;
//! let steam = SteamClient::builder().api_key(key).build()?;
//! let ownership = steam.user().check_app_ownership(76561198000000000, 480).await?;
//! println!("owns app: {}", ownership.ownsapp);
//! # Ok(())
//! # }
//! ```

pub mod apps;
mod client;
mod error;
pub mod game_server_stats;
pub mod game_servers;
pub mod leaderboards;
pub mod news;
pub mod player;
pub mod reviews;
pub mod store;
pub mod types;
pub mod user;
pub mod user_auth;
pub mod user_stats;
pub mod web_api_util;

pub use client::{ApiKey, ClientBuilder, ClientMetrics, Host, RetryPolicy, SteamClient};
pub use error::{Error, Result};
pub use types::SteamId;
