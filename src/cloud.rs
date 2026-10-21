// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! OAuth-scoped Steam Cloud metadata and staged file modifications.
//!
//! Use a client configured with [`crate::Host::Public`] and a user's token with
//! `read_cloud` or `write_cloud` scope for the application. API keys do not grant
//! Cloud access. Tokens are form fields outside `input_json`; every reply must
//! include a successful `x-eresult` header, including bodyless acknowledgements.
//!
//! An upload requires BeginAppUploadBatch, BeginHTTPUpload, a separate HTTP PUT,
//! CommitHTTPUpload, and CompleteAppUploadBatch. This module returns upload
//! instructions but never follows returned URLs or forwards credentials to them.
//! Callers must report the actual PUT result and complete the batch even when an
//! upload, commit, or deletion fails. No asynchronous cleanup runs from Drop.

use crate::{
    Error, Result, SteamClient, SteamId,
    request::{Access, Endpoint, Params},
    types::{integer, nonzero, optional_integer, text},
};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashSet},
    fmt,
};
use zeroize::Zeroizing;

/// Access to all six documented ICloudService methods.
#[derive(Debug, Clone, Copy)]
pub struct Cloud<'a>(pub(crate) &'a SteamClient);

/// A user's OAuth access token. Debug is redacted and owned text is zeroized.
/// The token format is intentionally opaque; Steam can change it over time.
pub struct OAuthAccessToken(Zeroizing<String>);

impl OAuthAccessToken {
    /// Validate and own a nonempty token without imposing a hexadecimal format.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = Zeroizing::new(value.into());
        text(&value, "access_token must not be empty or contain NUL")?;
        Ok(Self(value))
    }

    /// Explicitly expose the OAuth secret for transmission. Never log it.
    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for OAuthAccessToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OAuthAccessToken([REDACTED])")
    }
}

/// A page request for a user's application files; requires `read_cloud` scope.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct EnumerateUserFilesRequest {
    /// Application ID within the OAuth token's authorized scope.
    pub appid: u32,
    /// Include extended metadata such as download URLs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extended_details: Option<bool>,
    /// Maximum files on this page; omitted uses Steam's default of 500.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    /// Starting file index; omitted uses zero.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_index: Option<u32>,
}

/// A page of Cloud files. Debug hides private file names and download URLs.
#[derive(Deserialize)]
pub struct CloudFiles {
    /// Files on this page; an omitted empty list is preserved as an empty vector.
    #[serde(default)]
    pub files: Vec<CloudFile>,
    /// Total files available for the authorized user and application.
    pub total_files: u32,
    /// Additional server fields without an inferred meaning.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Cloud file metadata; extended fields remain absent unless Steam sends them.
#[derive(Deserialize)]
pub struct CloudFile {
    /// File name relative to Steam's configured Cloud roots.
    pub filename: String,
    /// Application ID, when included.
    pub appid: Option<u32>,
    /// File ID, accepting a number or decimal string without rounding.
    #[serde(default, deserialize_with = "optional_integer")]
    pub ugcid: Option<u64>,
    /// Modification time in Unix seconds, when included.
    #[serde(default, deserialize_with = "optional_integer")]
    pub timestamp: Option<u64>,
    /// Byte length, when included.
    pub file_size: Option<u32>,
    /// Download URL, when extended details include it. This client never fetches it.
    pub url: Option<String>,
    /// File owner's Steam ID, when included.
    pub steamid_creator: Option<SteamId>,
    /// Internal Steam flags, retained without interpretation.
    pub flags: Option<u32>,
    /// Platform list exactly as returned by Steam.
    pub platforms_to_sync: Option<String>,
    /// SHA-1 digest in hexadecimal, when included.
    pub file_sha: Option<String>,
    /// Additional server metadata.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Intended changes for a Cloud batch; requires `write_cloud` scope.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct BeginAppUploadBatchRequest<'a> {
    /// Application ID.
    pub appid: u32,
    /// User-facing name of the source machine.
    pub machine_name: &'a str,
    /// All files intended for upload; an empty list is allowed for delete-only batches.
    pub files_to_upload: &'a [&'a str],
    /// All files intended for deletion; an empty list is allowed for upload-only batches.
    pub files_to_delete: &'a [&'a str],
}

/// An opened batch, which must be completed after all changes have been attempted.
#[derive(Debug, Clone, Deserialize)]
pub struct CloudUploadBatch {
    /// Batch ID to retain for completion, preserving the full uint64 range.
    #[serde(deserialize_with = "integer")]
    pub batch_id: u64,
    /// Steam's currently unused application change counter.
    #[serde(deserialize_with = "integer")]
    pub app_change_number: u64,
}

/// Supported platform selections for a Cloud file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CloudPlatform {
    /// Synchronize on every supported platform.
    All,
    /// Windows.
    Windows,
    /// macOS.
    MacOS,
    /// Linux.
    Linux,
    /// Android.
    Android,
    /// iOS.
    IPhoneOS,
    /// Nintendo Switch.
    Switch,
}

impl Serialize for CloudPlatform {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Self::All => "all",
            Self::Windows => "windows",
            Self::MacOS => "macos",
            Self::Linux => "linux",
            Self::Android => "android",
            Self::IPhoneOS => "iphoneos",
            Self::Switch => "switch",
        })
    }
}

/// Metadata for staging one file upload. This request contains no file bytes.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct BeginHttpUploadRequest<'a> {
    /// Application ID.
    pub appid: u32,
    /// File byte length; zero is valid for an empty file.
    pub file_size: u32,
    /// Cloud file name, transmitted without URL decoding or path normalization.
    pub filename: &'a str,
    /// Exactly 40 hexadecimal digits containing the file's SHA-1 digest.
    pub file_sha: &'a str,
    /// Platforms on which this file should synchronize; must be nonempty and distinct.
    pub platforms_to_sync: &'a [CloudPlatform],
    /// Batch ID returned by BeginAppUploadBatch.
    pub upload_batch_id: u64,
    /// Optional flag shown in Steam's request example but not its parameter table.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_public: Option<bool>,
}

/// One header required by the independent storage provider's PUT request.
/// Header values can contain signatures; Debug is redacted.
#[derive(Deserialize)]
pub struct CloudUploadHeader {
    /// Exact header name returned by Steam.
    pub name: String,
    /// Exact header value returned by Steam; never log it.
    pub value: String,
}

/// Instructions for a caller-managed HTTP PUT; obtaining these is not an upload.
/// This type does not assemble or request a URL, preventing implicit forwarding
/// of the Steam client's API key or OAuth token to a storage host.
#[derive(Deserialize)]
pub struct CloudHttpUpload {
    /// Staged UGC ID, accepting a number or decimal string.
    #[serde(deserialize_with = "integer")]
    pub ugcid: u64,
    /// Timestamp assigned to the staged change.
    pub timestamp: u32,
    /// Storage provider's destination host.
    pub url_host: String,
    /// Exact path and signed query, including its leading slash.
    pub url_path: String,
    /// Whether Steam requires HTTPS for the independent PUT.
    pub use_https: bool,
    /// Headers the caller must include in the PUT; values can contain secrets.
    #[serde(default)]
    pub request_headers: Vec<CloudUploadHeader>,
    /// Additional metadata, retained without issuing any request.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Report the actual outcome of an independent file PUT, including failed PUTs.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct CommitHttpUploadRequest<'a> {
    /// Application ID.
    pub appid: u32,
    /// Whether the storage provider's HTTP PUT succeeded (200, 201, or 204).
    pub transfer_succeeded: bool,
    /// File name used when staging the upload.
    pub filename: &'a str,
    /// SHA-1 digest used when staging the upload.
    pub file_sha: &'a str,
}

/// Steam's outcome for committing one staged file.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct CloudFileCommit {
    /// Whether Steam committed the file. False is a failed upload, even when
    /// the API's HTTP response and x-eresult header both indicate success.
    pub file_committed: bool,
}

/// Overall result reported when ending a Cloud batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudBatchResult {
    /// All intended operations succeeded (EResult 1).
    Success,
    /// One or more intended operations failed (EResult 2).
    Failure,
}

impl Serialize for CloudBatchResult {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_u32(match self {
            Self::Success => 1,
            Self::Failure => 2,
        })
    }
}

/// End an opened batch even if an upload, commit, or deletion failed.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct CompleteAppUploadBatchRequest {
    /// Application ID.
    pub appid: u32,
    /// Batch ID returned when the batch was opened.
    pub batch_id: u64,
    /// Actual aggregate result of the attempted file operations.
    pub batch_eresult: CloudBatchResult,
}

/// Permanently delete one Cloud file within the intended batch changes.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct DeleteCloudFileRequest<'a> {
    /// Application ID.
    pub appid: u32,
    /// Exact Cloud file name. Steam retains no previous version after deletion.
    pub filename: &'a str,
}

impl Cloud<'_> {
    /// Enumerate one page of files using a `read_cloud` token (GET v1).
    pub async fn enumerate_user_files(
        &self,
        token: &OAuthAccessToken,
        request: &EnumerateUserFilesRequest,
    ) -> Result<CloudFiles> {
        nonzero(request.appid.into(), "appid must not be zero")?;
        if request.count == Some(0) {
            return Err(Error::InvalidParameter("count must not be zero"));
        }
        self.0
            .call(
                Endpoint::get("ICloudService/EnumerateUserFiles/v1/", Access::Anonymous)
                    .cloud_json(),
                oauth_params(token, request)?,
                &["response"],
            )
            .await
    }

    /// Begin a batch (POST v1), without automatic replay. Keep its ID until
    /// CompleteAppUploadBatch succeeds, including when a later operation fails.
    pub async fn begin_app_upload_batch(
        &self,
        token: &OAuthAccessToken,
        request: &BeginAppUploadBatchRequest<'_>,
    ) -> Result<CloudUploadBatch> {
        nonzero(request.appid.into(), "appid must not be zero")?;
        text(
            request.machine_name,
            "machine_name must not be empty or contain NUL",
        )?;
        let mut files = HashSet::new();
        for &filename in request
            .files_to_upload
            .iter()
            .chain(request.files_to_delete)
        {
            text(filename, "file names must not be empty or contain NUL")?;
            if !files.insert(filename) {
                return Err(Error::InvalidParameter("batch file names must be distinct"));
            }
        }
        self.0
            .call(
                Endpoint::post("ICloudService/BeginAppUploadBatch/v1/", Access::Anonymous)
                    .cloud_json(),
                oauth_params(token, request)?,
                &["response"],
            )
            .await
    }

    /// Stage an upload (POST v1), returning instructions for a separate PUT.
    /// This method never transfers bytes or follows the returned destination.
    pub async fn begin_http_upload(
        &self,
        token: &OAuthAccessToken,
        request: &BeginHttpUploadRequest<'_>,
    ) -> Result<CloudHttpUpload> {
        validate_file(request.appid, request.filename)?;
        validate_sha1(request.file_sha)?;
        nonzero(request.upload_batch_id, "upload_batch_id must not be zero")?;
        if request.platforms_to_sync.is_empty()
            || request.platforms_to_sync.len() > 7
            || request
                .platforms_to_sync
                .iter()
                .enumerate()
                .any(|(index, value)| request.platforms_to_sync[..index].contains(value))
        {
            return Err(Error::InvalidParameter(
                "platforms_to_sync must contain distinct platforms",
            ));
        }
        self.0
            .call(
                Endpoint::post("ICloudService/BeginHTTPUpload/v1/", Access::Anonymous).cloud_json(),
                oauth_params(token, request)?,
                &["response"],
            )
            .await
    }

    /// Report a successful or failed PUT (POST v1), without replay. Inspect
    /// `file_committed`; HTTP success alone does not establish file persistence.
    pub async fn commit_http_upload(
        &self,
        token: &OAuthAccessToken,
        request: &CommitHttpUploadRequest<'_>,
    ) -> Result<CloudFileCommit> {
        validate_file(request.appid, request.filename)?;
        validate_sha1(request.file_sha)?;
        self.0
            .call(
                Endpoint::post("ICloudService/CommitHTTPUpload/v1/", Access::Anonymous)
                    .cloud_json(),
                oauth_params(token, request)?,
                &["response"],
            )
            .await
    }

    /// Complete a batch (POST v1), reporting its actual aggregate outcome.
    /// Steam requires completion even after individual file operations fail.
    /// A successful bodyless acknowledgement is authenticated by x-eresult=1.
    pub async fn complete_app_upload_batch(
        &self,
        token: &OAuthAccessToken,
        request: &CompleteAppUploadBatchRequest,
    ) -> Result<()> {
        nonzero(request.appid.into(), "appid must not be zero")?;
        nonzero(request.batch_id, "batch_id must not be zero")?;
        self.0
            .call(
                Endpoint::post(
                    "ICloudService/CompleteAppUploadBatch/v1/",
                    Access::Anonymous,
                )
                .cloud_acknowledgement(),
                oauth_params(token, request)?,
                &[],
            )
            .await
    }

    /// Permanently delete one Cloud file (POST v1), without automatic replay.
    /// A successful deletion has no body and is authenticated by x-eresult=1.
    pub async fn delete(
        &self,
        token: &OAuthAccessToken,
        request: &DeleteCloudFileRequest<'_>,
    ) -> Result<()> {
        validate_file(request.appid, request.filename)?;
        self.0
            .call(
                Endpoint::post("ICloudService/Delete/v1/", Access::Anonymous)
                    .cloud_acknowledgement(),
                oauth_params(token, request)?,
                &[],
            )
            .await
    }
}

fn oauth_params(token: &OAuthAccessToken, request: &impl Serialize) -> Result<Params> {
    Ok(Params::json(request)?.with("access_token", token.expose_secret()))
}

fn validate_file(appid: u32, filename: &str) -> Result<()> {
    nonzero(appid.into(), "appid must not be zero")?;
    text(filename, "filename must not be empty or contain NUL")
}

fn validate_sha1(value: &str) -> Result<()> {
    if value.len() != 40 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::InvalidParameter(
            "file_sha must contain 40 hexadecimal digits",
        ));
    }
    Ok(())
}

macro_rules! redacted_debug {
    ($($name:ident),+ $(,)?) => {$(
        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($name), "([REDACTED])"))
            }
        }
    )+};
}

redacted_debug!(CloudFiles, CloudFile, CloudUploadHeader, CloudHttpUpload);
