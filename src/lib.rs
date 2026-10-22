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
//!
//! # Error handling
//!
//! Request methods return [`Error::InvalidParameter`] for invalid local input
//! and [`Error::MissingApiKey`] for an absent or incompatible key. Requests may
//! also fail with [`Error::Transport`], [`Error::Http`], [`Error::Deadline`],
//! [`Error::Api`], [`Error::Decode`], or [`Error::ResponseTooLarge`]. Retry jitter
//! can return [`Error::Random`] if the operating system's random source fails.
//! Public errors omit credentials, URLs, response bodies, and upstream text.
//!
//! Only replay-safe GET reads use the configured retry policy. Authentication,
//! POSTs, and documented GET mutations are sent once. A timeout after a write
//! does not prove that Steam rejected it; reconcile its outcome before retrying.
//! Opaque responses preserve business fields without inventing a success state.
//! See each request type's field documentation for parameter units and limits.
//!
//! Interface accessors borrow this client and reuse its existing connection
//! pool and shared limits; selecting an accessor performs no network operation.

pub mod apps;
mod body;
pub mod broadcast;
pub mod cheat_reporting;
mod client;
pub mod cloud;
pub mod community;
pub mod econ;
pub mod econ_market;
pub mod economy;
mod error;
pub mod game_inventory;
pub mod game_notifications;
pub mod game_server_stats;
pub mod game_servers;
pub mod inventory;
pub mod leaderboards;
pub mod lobby_matchmaking;
pub mod micro_txn;
pub mod news;
pub mod partner_financials;
pub mod player;
pub mod published_file;
pub mod published_file_moderation;
pub mod published_item_search;
pub mod published_item_voting;
pub mod remote_storage;
mod request;
mod response;
pub mod reviews;
pub mod site_license;
pub mod store;
pub mod types;
pub mod user;
pub mod user_auth;
pub mod user_stats;
pub mod web_api_util;
pub mod workshop;

pub use client::{ApiKey, ClientBuilder, ClientMetrics, Host, RetryPolicy, SteamClient};
pub use error::{Error, Result};
pub use types::SteamId;
