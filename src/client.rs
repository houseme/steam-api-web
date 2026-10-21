// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

use crate::{
    Error, Result,
    apps::Apps,
    cloud::Cloud,
    econ::EconService,
    econ_market::EconMarketService,
    economy::Economy,
    game_inventory::GameInventory,
    game_server_stats::GameServerStats,
    game_servers::GameServers,
    inventory::Inventory,
    leaderboards::Leaderboards,
    news::News,
    player::PlayerService,
    published_file::PublishedFileService,
    published_file_moderation::PublishedFileModerationService,
    published_item_search::PublishedItemSearch,
    published_item_voting::PublishedItemVoting,
    remote_storage::RemoteStorage,
    request::{Access, Endpoint, Operation, Params},
    response::Response,
    reviews::UserReviewsService,
    store::StoreService,
    user::User,
    user_auth::{OpaqueResponse, UserAuth},
    user_stats::UserStats,
    web_api_util::WebApiUtil,
    workshop::WorkshopService,
};
use reqwest::{
    Method,
    header::{CONTENT_TYPE, HeaderMap, HeaderValue, RETRY_AFTER},
};
use rustls_platform_verifier::BuilderVerifierExt;
use serde::de::DeserializeOwned;
use std::{
    collections::HashMap,
    fmt,
    net::IpAddr,
    sync::{
        Arc, Mutex as SyncMutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime},
};
use tokio::{
    sync::{Mutex, Semaphore},
    time::Instant,
};
use url::Url;
use zeroize::Zeroizing;

/// The two official API hosts. Publisher keys select Partner by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Host {
    /// Public host; also permits anonymous public methods.
    Public,
    /// Higher availability server host; every request requires a publisher key.
    Partner,
}
impl Host {
    fn url(self) -> &'static str {
        match self {
            Self::Public => "https://api.steampowered.com/",
            Self::Partner => "https://partner.steam-api.com/",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyKind {
    User,
    Publisher,
}

/// A validated API key. Debug output is redacted; owned text is zeroized on drop.
/// Publisher keys belong only on trusted servers.
#[derive(Clone)]
pub struct ApiKey {
    kind: KeyKind,
    secret: Zeroizing<String>,
}
impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiKey")
            .field("kind", &self.kind)
            .field("secret", &"[REDACTED]")
            .finish()
    }
}
impl ApiKey {
    /// Create a user key. Empty or invalid HTTP header values are rejected.
    pub fn user(value: impl Into<String>) -> Result<Self> {
        Self::new(KeyKind::User, value.into())
    }
    /// Create a publisher key, which also authorizes user-key methods.
    pub fn publisher(value: impl Into<String>) -> Result<Self> {
        Self::new(KeyKind::Publisher, value.into())
    }
    fn new(kind: KeyKind, value: String) -> Result<Self> {
        let secret = Zeroizing::new(value);
        if secret.trim().is_empty() || HeaderValue::from_str(&secret).is_err() {
            return Err(Error::Configuration(
                "API key must be a nonempty HTTP header value",
            ));
        }
        Ok(Self { kind, secret })
    }
}

/// Retry policy for replay-safe GET methods only. Authentication is never replayed.
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    /// Additional attempts (0 disables retries; maximum 10).
    pub max_retries: u32,
    /// Initial exponential backoff, with jitter between 50% and 100%.
    pub initial_backoff: Duration,
    /// Maximum local backoff; does not shorten a server Retry-After.
    pub max_backoff: Duration,
}
impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 2,
            initial_backoff: Duration::from_millis(200),
            max_backoff: Duration::from_secs(2),
        }
    }
}

/// Configuration for a pooled Steam client. Debug never exposes custom URLs.
#[derive(Clone)]
pub struct ClientBuilder {
    key: Option<ApiKey>,
    host: Option<Host>,
    base_url: Option<String>,
    connect_timeout: Duration,
    request_timeout: Duration,
    operation_timeout: Duration,
    max_in_flight: usize,
    max_response_bytes: usize,
    min_request_interval: Duration,
    retry: RetryPolicy,
}
impl fmt::Debug for ClientBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientBuilder")
            .field("key", &self.key)
            .field("host", &self.host)
            .finish_non_exhaustive()
    }
}
impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            key: None,
            host: None,
            base_url: None,
            connect_timeout: Duration::from_secs(3),
            request_timeout: Duration::from_secs(10),
            operation_timeout: Duration::from_secs(30),
            max_in_flight: 32,
            max_response_bytes: 8 * 1024 * 1024,
            min_request_interval: Duration::ZERO,
            retry: RetryPolicy::default(),
        }
    }
}
impl ClientBuilder {
    /// Configure a key. Publisher credentials select Partner unless overridden.
    pub fn api_key(mut self, key: ApiKey) -> Self {
        self.key = Some(key);
        self
    }
    /// Override automatic host selection. No automatic cross-host failover occurs.
    pub fn host(mut self, host: Host) -> Self {
        self.host = Some(host);
        self
    }
    /// Override the base URL for a trusted gateway or mock server.
    /// Credentials are sent to this URL. HTTPS is required except for loopback
    /// HTTP, intended for tests. Userinfo, queries and fragments are rejected.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }
    /// Timeout for connection establishment.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }
    /// Timeout for each HTTP attempt, including reading its body.
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }
    /// Overall deadline, including queueing, pacing and retries.
    pub fn operation_timeout(mut self, timeout: Duration) -> Self {
        self.operation_timeout = timeout;
        self
    }
    /// Maximum concurrent attempts, shared by clones; must be nonzero.
    pub fn max_in_flight(mut self, limit: usize) -> Self {
        self.max_in_flight = limit;
        self
    }
    /// Maximum successful response body size; checked while streaming chunks.
    pub fn max_response_bytes(mut self, limit: usize) -> Self {
        self.max_response_bytes = limit;
        self
    }
    /// Minimum interval between attempt starts across all clones (zero disables).
    /// Set this to match your Steam quota; it is not a distributed rate limiter.
    pub fn min_request_interval(mut self, interval: Duration) -> Self {
        self.min_request_interval = interval;
        self
    }
    /// Set the bounded read retry policy.
    pub fn retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry = policy;
        self
    }
    /// Build a client. Reuse it to retain the connection pool and shared limits.
    pub fn build(self) -> Result<SteamClient> {
        const MAX_DURATION: Duration = Duration::from_secs(86400);
        for timeout in [
            self.connect_timeout,
            self.request_timeout,
            self.operation_timeout,
        ] {
            if timeout.is_zero() || timeout > MAX_DURATION {
                return Err(Error::Configuration("timeouts must be in (0, 24 hours]"));
            }
        }
        if self.max_in_flight == 0
            || self.max_in_flight > Semaphore::MAX_PERMITS
            || self.max_response_bytes == 0
        {
            return Err(Error::Configuration(
                "resource limits must be nonzero and concurrency must fit a semaphore",
            ));
        }
        if self.retry.max_retries > 10
            || self.retry.initial_backoff > self.retry.max_backoff
            || self.retry.max_backoff > MAX_DURATION
            || self.min_request_interval > MAX_DURATION
        {
            return Err(Error::Configuration(
                "invalid retry policy or pacing interval",
            ));
        }
        let kind = self.key.as_ref().map(|k| k.kind);
        let host = self.host.unwrap_or(if kind == Some(KeyKind::Publisher) {
            Host::Partner
        } else {
            Host::Public
        });
        if host == Host::Partner && kind != Some(KeyKind::Publisher) {
            return Err(Error::MissingApiKey("publisher"));
        }
        let mut base_url = Url::parse(self.base_url.as_deref().unwrap_or(host.url()))
            .map_err(|_| Error::Configuration("invalid base URL"))?;
        let loopback = base_url.host_str().is_some_and(|host| {
            host == "localhost"
                || host
                    .trim_matches(['[', ']'])
                    .parse::<IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
        });
        if (base_url.scheme() != "https" && !(base_url.scheme() == "http" && loopback))
            || base_url.host_str().is_none()
            || !base_url.username().is_empty()
            || base_url.password().is_some()
            || base_url.query().is_some()
            || base_url.fragment().is_some()
        {
            return Err(Error::Configuration(
                "base URL requires HTTPS (or loopback HTTP), no userinfo, query or fragment",
            ));
        }
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        // An explicit public-host override must not smuggle a user key to Partner.
        if base_url.host_str() == Some("partner.steam-api.com") && kind != Some(KeyKind::Publisher)
        {
            return Err(Error::MissingApiKey("publisher"));
        }
        let mut headers = HeaderMap::new();
        if let Some(key) = &self.key {
            let mut value = HeaderValue::from_str(&key.secret)
                .map_err(|_| Error::Configuration("invalid API key header"))?;
            value.set_sensitive(true);
            headers.insert("x-webapi-key", value);
        }
        // A per-client provider avoids dependence on any host-installed global
        // provider, including ring. Never modify the process-wide default.
        let tls = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|_| Error::Configuration("AWS-LC TLS initialization failed"))?
        .with_platform_verifier()
        .map_err(|_| Error::Configuration("platform certificate verifier initialization failed"))?
        .with_no_client_auth();
        let http = reqwest::Client::builder()
            .tls_backend_preconfigured(tls)
            .http1_only()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(self.connect_timeout)
            .timeout(self.request_timeout)
            .pool_idle_timeout(Duration::from_secs(60))
            .pool_max_idle_per_host(self.max_in_flight)
            .user_agent(concat!("steam-api-web/", env!("CARGO_PKG_VERSION")))
            .default_headers(headers)
            .build()
            .map_err(|_| Error::Configuration("HTTP client initialization failed"))?;
        Ok(SteamClient {
            inner: Arc::new(Inner {
                http,
                base_url,
                kind,
                semaphore: Semaphore::new(self.max_in_flight),
                operation_timeout: self.operation_timeout,
                max_response_bytes: self.max_response_bytes,
                retry: self.retry,
                min_request_interval: self.min_request_interval,
                pacing: Mutex::new(Pacing::default()),
                urls: SyncMutex::new(HashMap::new()),
                metrics: Metrics::default(),
            }),
        })
    }
}

#[derive(Default)]
struct Pacing {
    next_start: Option<Instant>,
    cooldown: Option<Instant>,
}
struct Inner {
    http: reqwest::Client,
    base_url: Url,
    kind: Option<KeyKind>,
    semaphore: Semaphore,
    operation_timeout: Duration,
    max_response_bytes: usize,
    retry: RetryPolicy,
    min_request_interval: Duration,
    pacing: Mutex<Pacing>,
    /// Endpoint paths are a fixed internal set, so each joined URL is parsed
    /// once and cloned per call. The guard is never held across an await.
    urls: SyncMutex<HashMap<&'static str, Url>>,
    metrics: Metrics,
}

/// A cheap-to-clone, Send + Sync client; all clones share operational limits.
#[derive(Clone)]
pub struct SteamClient {
    inner: Arc<Inner>,
}
impl fmt::Debug for SteamClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SteamClient")
            .field("key_kind", &self.inner.kind)
            .finish_non_exhaustive()
    }
}

/// Atomic counter snapshot. Concurrent snapshots are approximate.
/// Calls rejected by argument/key validation and canceled futures are not counted
/// as completed calls. Attempts count actual HTTP sends, including retries.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClientMetrics {
    /// HTTP attempts started.
    pub attempts: u64,
    /// Additional HTTP attempts started.
    pub retries: u64,
    /// Operations that completed successfully.
    pub succeeded: u64,
    /// Operations that returned an error, including overall deadline failures.
    pub failed: u64,
}
#[derive(Default)]
struct Metrics {
    attempts: AtomicU64,
    retries: AtomicU64,
    succeeded: AtomicU64,
    failed: AtomicU64,
}

impl SteamClient {
    /// Start configuring a client. The default permits anonymous public reads.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }
    /// Authentication methods.
    pub fn user_auth(&self) -> UserAuth<'_> {
        UserAuth(self)
    }
    /// Profile, bans, ownership, groups, prices and deletion methods.
    pub fn user(&self) -> User<'_> {
        User(self)
    }
    /// Statistics, achievements, schema and player count methods.
    pub fn user_stats(&self) -> UserStats<'_> {
        UserStats(self)
    }
    /// Steam application and build administration.
    pub fn apps(&self) -> Apps<'_> {
        Apps(self)
    }
    /// Public API discovery and server time.
    pub fn web_api_util(&self) -> WebApiUtil<'_> {
        WebApiUtil(self)
    }
    /// Player library, playtime, levels and badges.
    pub fn player(&self) -> PlayerService<'_> {
        PlayerService(self)
    }
    /// Paginated Steam Store catalog.
    pub fn store(&self) -> StoreService<'_> {
        StoreService(self)
    }
    /// User review queries and summary data.
    pub fn reviews(&self) -> UserReviewsService<'_> {
        UserReviewsService(self)
    }
    /// Public and publisher-authenticated application news.
    pub fn news(&self) -> News<'_> {
        News(self)
    }
    /// Persistent game server account administration.
    pub fn game_servers(&self) -> GameServers<'_> {
        GameServers(self)
    }
    /// Game server player statistics.
    pub fn game_server_stats(&self) -> GameServerStats<'_> {
        GameServerStats(self)
    }
    /// Publisher leaderboard administration.
    pub fn leaderboards(&self) -> Leaderboards<'_> {
        Leaderboards(self)
    }
    /// OAuth-authorized Steam Cloud files and upload stages.
    pub fn cloud(&self) -> Cloud<'_> {
        Cloud(self)
    }
    /// Publisher inventory grants, crafting and dynamic properties.
    pub fn inventory(&self) -> Inventory<'_> {
        Inventory(self)
    }
    /// Publisher inventory history and item definitions.
    pub fn game_inventory(&self) -> GameInventory<'_> {
        GameInventory(self)
    }
    /// Asset prices, transactions and trade initiation.
    pub fn economy(&self) -> Economy<'_> {
        Economy(self)
    }
    /// Trade history, offers and inventory cache operations.
    pub fn econ(&self) -> EconService<'_> {
        EconService(self)
    }
    /// Market eligibility, assets, popular items and listing cancellation.
    pub fn econ_market(&self) -> EconMarketService<'_> {
        EconMarketService(self)
    }
    /// Workshop file queries and publisher administration.
    pub fn published_file(&self) -> PublishedFileService<'_> {
        PublishedFileService(self)
    }
    /// Workshop approval requests and decisions.
    pub fn published_file_moderation(&self) -> PublishedFileModerationService<'_> {
        PublishedFileModerationService(self)
    }
    /// Workshop publication, trend and vote searches.
    pub fn published_item_search(&self) -> PublishedItemSearch<'_> {
        PublishedItemSearch(self)
    }
    /// Published-item voting summaries.
    pub fn published_item_voting(&self) -> PublishedItemVoting<'_> {
        PublishedItemVoting(self)
    }
    /// Remote UGC details, collections and subscriptions.
    pub fn remote_storage(&self) -> RemoteStorage<'_> {
        RemoteStorage(self)
    }
    /// Workshop payments, contributors, revenue and descriptions.
    pub fn workshop(&self) -> WorkshopService<'_> {
        WorkshopService(self)
    }
    /// Inspect counters without logging sensitive requests or responses.
    pub fn metrics(&self) -> ClientMetrics {
        let m = &self.inner.metrics;
        ClientMetrics {
            attempts: m.attempts.load(Ordering::Relaxed),
            retries: m.retries.load(Ordering::Relaxed),
            succeeded: m.succeeded.load(Ordering::Relaxed),
            failed: m.failed.load(Ordering::Relaxed),
        }
    }
    pub(crate) async fn call<T: DeserializeOwned>(
        &self,
        endpoint: Endpoint,
        params: Params,
        envelope: &[&str],
    ) -> Result<T> {
        self.call_with(endpoint, params, |response| {
            response
                .decode(endpoint.path, envelope)
                .map(|(_, payload)| payload)
        })
        .await
    }
    pub(crate) async fn call_opaque(
        &self,
        endpoint: Endpoint,
        params: Params,
    ) -> Result<OpaqueResponse> {
        self.call_opaque_with_status(endpoint, params)
            .await
            .map(|(_, response)| response)
    }
    pub(crate) async fn call_opaque_with_status(
        &self,
        endpoint: Endpoint,
        params: Params,
    ) -> Result<(u16, OpaqueResponse)> {
        self.call_with(endpoint, params, |response| response.opaque(endpoint.path))
            .await
    }
    async fn call_with<T>(
        &self,
        endpoint: Endpoint,
        mut params: Params,
        decode: impl FnOnce(Response) -> Result<T>,
    ) -> Result<T> {
        match (endpoint.access, self.inner.kind) {
            (Access::Publisher, Some(KeyKind::Publisher))
            | (Access::Anonymous, _)
            | (Access::User, Some(_)) => (),
            (Access::Publisher, _) => return Err(Error::MissingApiKey("publisher")),
            _ => return Err(Error::MissingApiKey("user or publisher")),
        }
        params.push("format", "json");
        let encoded = params.finish();
        let result = tokio::time::timeout(self.inner.operation_timeout, async {
            decode(self.execute(endpoint, encoded).await?)
        })
        .await
        .unwrap_or(Err(Error::Deadline {
            endpoint: endpoint.path,
        }));
        if result.is_ok() {
            self.inner.metrics.succeeded.fetch_add(1, Ordering::Relaxed);
        } else {
            self.inner.metrics.failed.fetch_add(1, Ordering::Relaxed);
        }
        result
    }
    async fn execute(&self, endpoint: Endpoint, encoded: String) -> Result<Response> {
        let mut url = self.endpoint_url(&endpoint)?;
        if endpoint.operation != Operation::Post {
            url.set_query(Some(&encoded));
        }
        let limit = if endpoint.operation == Operation::Read {
            self.inner.retry.max_retries
        } else {
            0
        };
        let mut body = (endpoint.operation == Operation::Post).then_some(encoded);
        for attempt in 0..=limit {
            let permit = self
                .inner
                .semaphore
                .acquire()
                .await
                .map_err(|_| Error::Configuration("client closed"))?;
            self.pace().await;
            let mut request = self.inner.http.request(
                if endpoint.operation == Operation::Post {
                    Method::POST
                } else {
                    Method::GET
                },
                url.clone(),
            );
            if endpoint.operation == Operation::Post {
                request = request
                    .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(
                        body.take()
                            .ok_or(Error::Configuration("POST cannot be replayed"))?,
                    );
            }
            self.inner.metrics.attempts.fetch_add(1, Ordering::Relaxed);
            if attempt > 0 {
                self.inner.metrics.retries.fetch_add(1, Ordering::Relaxed);
            }
            let result = self.attempt(endpoint, request).await;
            drop(permit);
            let retryable = match &result {
                Err(Error::Http { status, .. }) => retryable_status(*status),
                Err(Error::Transport { .. }) => true,
                _ => false,
            };
            if !retryable || attempt == limit {
                return result;
            }
            let delay = jitter(
                self.inner
                    .retry
                    .initial_backoff
                    .saturating_mul(1_u32 << attempt.min(30))
                    .min(self.inner.retry.max_backoff),
            )?;
            tokio::time::sleep(delay).await;
        }
        Err(Error::Configuration("invalid retry state"))
    }
    /// Resolve an endpoint's URL from the per-client cache, parsing it only on
    /// first use. Poisoning cannot occur because the guard never panics.
    fn endpoint_url(&self, endpoint: &Endpoint) -> Result<Url> {
        let mut urls = self
            .inner
            .urls
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(url) = urls.get(endpoint.path) {
            return Ok(url.clone());
        }
        let url = self
            .inner
            .base_url
            .join(endpoint.path)
            .map_err(|_| Error::Configuration("invalid endpoint URL"))?;
        urls.insert(endpoint.path, url.clone());
        Ok(url)
    }
    async fn pace(&self) {
        loop {
            let mut state = self.inner.pacing.lock().await;
            let now = Instant::now();
            let due = state
                .next_start
                .into_iter()
                .chain(state.cooldown)
                .max()
                .unwrap_or(now);
            if due > now {
                drop(state);
                tokio::time::sleep_until(due).await;
            } else {
                state.next_start = now.checked_add(self.inner.min_request_interval);
                return;
            }
        }
    }
    async fn attempt(
        &self,
        endpoint: Endpoint,
        request: reqwest::RequestBuilder,
    ) -> Result<Response> {
        let transport = |error: reqwest::Error| Error::Transport {
            endpoint: endpoint.path,
            timeout: error.is_timeout(),
            connect: error.is_connect(),
        };
        let mut response = request.send().await.map_err(transport)?;
        let status = response.status();
        if !status.is_success() {
            let retry_after = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(parse_retry_after);
            if let Some(delay) = retry_after.filter(|_| retryable_status(status.as_u16())) {
                // Keep a bounded Instant even for malicious u64::MAX headers.
                // A delay above one year remains a one-year shared cooldown;
                // each caller's much shorter operation deadline still applies.
                let due = Instant::now() + delay.min(Duration::from_secs(366 * 86400));
                let mut state = self.inner.pacing.lock().await;
                state.cooldown = Some(state.cooldown.map_or(due, |old| old.max(due)));
            }
            return Err(Error::Http {
                endpoint: endpoint.path,
                status: status.as_u16(),
                retry_after,
            });
        }
        endpoint
            .response
            .validate_headers(endpoint.path, response.headers())?;
        let limit = self.inner.max_response_bytes;
        if response
            .content_length()
            .is_some_and(|len| len > limit as u64)
        {
            return Err(Error::ResponseTooLarge {
                endpoint: endpoint.path,
                limit,
            });
        }
        let mut bytes =
            Vec::with_capacity(response.content_length().unwrap_or(0).min(64 * 1024) as usize);
        while let Some(chunk) = response.chunk().await.map_err(transport)? {
            if chunk.len() > limit.saturating_sub(bytes.len()) {
                return Err(Error::ResponseTooLarge {
                    endpoint: endpoint.path,
                    limit,
                });
            }
            bytes.extend_from_slice(&chunk);
        }
        endpoint
            .response
            .parse(endpoint.path, status.as_u16(), &bytes)
    }
}

fn retryable_status(status: u16) -> bool {
    matches!(status, 408 | 429 | 500 | 502 | 503 | 504)
}
fn parse_retry_after(value: &HeaderValue) -> Option<Duration> {
    let value = value.to_str().ok()?.trim();
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }
    httpdate::parse_http_date(value).ok().map(|date| {
        date.duration_since(SystemTime::now())
            .unwrap_or(Duration::ZERO)
    })
}
fn jitter(cap: Duration) -> Result<Duration> {
    if cap.is_zero() {
        return Ok(cap);
    }
    let mut random = [0_u8; 8];
    aws_lc_rs::rand::fill(&mut random).map_err(|_| Error::Random)?;
    Ok(cap.mul_f64(0.5 + (u64::from_le_bytes(random) as f64 / u64::MAX as f64) * 0.5))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    #[test]
    fn retry_after_accepts_seconds_dates_and_past_dates() {
        assert_eq!(
            parse_retry_after(&HeaderValue::from_static("17")),
            Some(Duration::from_secs(17))
        );
        let future = httpdate::fmt_http_date(SystemTime::now() + Duration::from_secs(120));
        assert!(
            parse_retry_after(&HeaderValue::from_str(&future).unwrap()).unwrap()
                > Duration::from_secs(118)
        );
        assert_eq!(
            parse_retry_after(&HeaderValue::from_static("Thu, 01 Jan 1970 00:00:00 GMT")),
            Some(Duration::ZERO)
        );
        assert_eq!(parse_retry_after(&HeaderValue::from_static("-1")), None);
    }
}
