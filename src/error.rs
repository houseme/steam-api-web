// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

use std::time::Duration;

/// A result returned by this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Failures contain no request URLs, credentials, response bodies, or server text.
/// Malformed JSON can echo tickets in serde errors, so only coordinates survive.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Invalid configuration; messages never contain supplied values.
    #[error("invalid configuration: {0}")]
    Configuration(&'static str),
    /// Invalid arguments, rejected before sending a request.
    #[error("invalid parameter: {0}")]
    InvalidParameter(&'static str),
    /// No key of the necessary kind is configured.
    #[error("{0} API key required")]
    MissingApiKey(&'static str),
    /// Deadline covering queueing, pacing, backoff, network I/O and decoding.
    #[error("operation deadline exceeded for {endpoint}")]
    Deadline {
        /// Interface, method and version, without a query string.
        endpoint: &'static str,
    },
    /// Network failure. Raw upstream error sources are deliberately omitted.
    #[error("transport failure for {endpoint} (timeout: {timeout}, connect: {connect})")]
    Transport {
        /// Interface, method and version.
        endpoint: &'static str,
        /// Whether reqwest identified a timeout.
        timeout: bool,
        /// Whether reqwest identified a connection failure.
        connect: bool,
    },
    /// Unsuccessful HTTP response; no body is retained.
    #[error("HTTP {status} for {endpoint}")]
    Http {
        /// Interface, method and version.
        endpoint: &'static str,
        /// HTTP status code.
        status: u16,
        /// Server-requested cooldown, if present and valid.
        retry_after: Option<Duration>,
    },
    /// Steam business failure, including HTTP 200 error envelopes.
    #[error("Steam API rejected {endpoint} (code: {code:?})")]
    Api {
        /// Interface, method and version.
        endpoint: &'static str,
        /// Numeric Steam error / result code when available.
        code: Option<i64>,
    },
    /// Malformed or unexpected response. No untrusted text is exposed.
    #[error("invalid response for {endpoint} at line {line}, column {column}")]
    Decode {
        /// Interface, method and version.
        endpoint: &'static str,
        /// JSON error line (zero for an absent envelope).
        line: usize,
        /// JSON error column (zero for an absent envelope).
        column: usize,
    },
    /// Server response exceeded the configured limit.
    #[error("response exceeds {limit} bytes for {endpoint}")]
    ResponseTooLarge {
        /// Interface, method and version.
        endpoint: &'static str,
        /// Maximum response body size.
        limit: usize,
    },
    /// Operating system secure random source failure.
    #[error("secure random source unavailable")]
    Random,
}

impl Error {
    pub(crate) fn decode(endpoint: &'static str, error: serde_json::Error) -> Self {
        Self::Decode {
            endpoint,
            line: error.line(),
            column: error.column(),
        }
    }
}
