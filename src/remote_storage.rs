// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Workshop subscriptions and file metadata through `ISteamRemoteStorage`.
//!
//! The Web API interface is distinct from native SDK Cloud file operations.
//! Response schemas are not specified in the official reference, so responses
//! preserve their original JSON and redact Debug output. POST reads and all
//! mutations are sent once; callers control recovery after ambiguous failures.

use crate::{
    Error, Result, SteamClient, SteamId,
    request::{Access, Endpoint, Params},
    types::nonzero,
    user::validate_user_app,
    user_auth::OpaqueResponse,
};

/// Access to all seven documented ISteamRemoteStorage Web methods.
#[derive(Debug, Clone, Copy)]
pub struct RemoteStorage<'a>(pub(crate) &'a SteamClient);

/// Parameters for enumerating a user's subscribed Workshop files.
#[derive(Debug, Clone, Copy)]
pub struct EnumerateSubscribedFilesRequest {
    /// User whose subscriptions are requested.
    pub steamid: SteamId,
    /// Application ID.
    pub appid: u32,
    /// Optional `EUCMListType` value; omitted retains Steam's default.
    pub listtype: Option<u32>,
}

/// Borrowed IDs for a collection-details or published-file-details request.
#[derive(Debug, Clone, Copy)]
pub struct PublishedFileIdsRequest<'a> {
    /// Nonempty collection or published-file IDs, preserving the full uint64 range.
    pub publishedfileids: &'a [u64],
}

/// Parameters for an authenticated UGC metadata lookup.
#[derive(Debug, Clone, Copy)]
pub struct UgcFileDetailsRequest {
    /// UGC file ID.
    pub ugcid: u64,
    /// Application ID.
    pub appid: u32,
    /// Optional owner constraint; Steam returns details only for this owner.
    pub steamid: Option<SteamId>,
}

/// Parameters for changing the game coordinator's use flag on a UGC file.
#[derive(Debug, Clone, Copy)]
pub struct SetUgcUsedByGcRequest {
    /// File owner's Steam ID.
    pub steamid: SteamId,
    /// UGC file ID.
    pub ugcid: u64,
    /// Application ID.
    pub appid: u32,
    /// New game coordinator use flag.
    pub used: bool,
}

/// Parameters for subscribing or unsubscribing a user's Workshop file.
#[derive(Debug, Clone, Copy)]
pub struct PublishedFileSubscriptionRequest {
    /// User whose subscription is changed.
    pub steamid: SteamId,
    /// Application ID.
    pub appid: u32,
    /// Published-file ID.
    pub publishedfileid: u64,
}

impl RemoteStorage<'_> {
    /// Enumerate subscribed files (publisher POST v1), without automatic replay.
    pub async fn enumerate_user_subscribed_files(
        &self,
        request: &EnumerateSubscribedFilesRequest,
    ) -> Result<OpaqueResponse> {
        validate_user_app(request.steamid, request.appid)?;
        let mut params = user_app_params(request.steamid, request.appid);
        if let Some(listtype) = request.listtype {
            params.push("listtype", listtype);
        }
        self.0
            .call_opaque(
                Endpoint::post(
                    "ISteamRemoteStorage/EnumerateUserSubscribedFiles/v1/",
                    Access::Publisher,
                ),
                params,
            )
            .await
    }

    /// Read collections (anonymous POST v1) using indexed `publishedfileids[i]`.
    /// The count is derived from the slice, so the wire count cannot disagree.
    pub async fn get_collection_details(
        &self,
        request: PublishedFileIdsRequest<'_>,
    ) -> Result<OpaqueResponse> {
        self.0
            .call_opaque(
                Endpoint::post(
                    "ISteamRemoteStorage/GetCollectionDetails/v1/",
                    Access::Anonymous,
                ),
                file_ids_params("collectioncount", request.publishedfileids)?,
            )
            .await
    }

    /// Read published files (anonymous POST v1) using indexed IDs and itemcount.
    pub async fn get_published_file_details(
        &self,
        request: PublishedFileIdsRequest<'_>,
    ) -> Result<OpaqueResponse> {
        self.0
            .call_opaque(
                Endpoint::post(
                    "ISteamRemoteStorage/GetPublishedFileDetails/v1/",
                    Access::Anonymous,
                ),
                file_ids_params("itemcount", request.publishedfileids)?,
            )
            .await
    }

    /// Read UGC metadata (user or publisher GET v1), optionally requiring an owner.
    pub async fn get_ugc_file_details(
        &self,
        request: &UgcFileDetailsRequest,
    ) -> Result<OpaqueResponse> {
        nonzero(request.ugcid, "ugcid must not be zero")?;
        nonzero(request.appid.into(), "appid must not be zero")?;
        let mut params = Params::new()
            .with("ugcid", request.ugcid)
            .with("appid", request.appid);
        if let Some(steamid) = request.steamid {
            nonzero(steamid.raw(), "steamid must not be zero")?;
            params.push("steamid", steamid);
        }
        self.0
            .call_opaque(
                Endpoint::get("ISteamRemoteStorage/GetUGCFileDetails/v1/", Access::User),
                params,
            )
            .await
    }

    /// Change a UGC use flag (publisher POST v1), without automatic replay.
    pub async fn set_ugc_used_by_gc(
        &self,
        request: &SetUgcUsedByGcRequest,
    ) -> Result<OpaqueResponse> {
        validate_user_app(request.steamid, request.appid)?;
        nonzero(request.ugcid, "ugcid must not be zero")?;
        self.0
            .call_opaque(
                Endpoint::post("ISteamRemoteStorage/SetUGCUsedByGC/v1/", Access::Publisher),
                user_app_params(request.steamid, request.appid)
                    .with("ugcid", request.ugcid)
                    .with("used", request.used),
            )
            .await
    }

    /// Subscribe a user to a Workshop file (publisher POST v1), sent once.
    pub async fn subscribe_published_file(
        &self,
        request: &PublishedFileSubscriptionRequest,
    ) -> Result<OpaqueResponse> {
        self.change_subscription("ISteamRemoteStorage/SubscribePublishedFile/v1/", request)
            .await
    }

    /// Unsubscribe a user from a Workshop file (publisher POST v1), sent once.
    pub async fn unsubscribe_published_file(
        &self,
        request: &PublishedFileSubscriptionRequest,
    ) -> Result<OpaqueResponse> {
        self.change_subscription("ISteamRemoteStorage/UnsubscribePublishedFile/v1/", request)
            .await
    }

    async fn change_subscription(
        &self,
        path: &'static str,
        request: &PublishedFileSubscriptionRequest,
    ) -> Result<OpaqueResponse> {
        validate_user_app(request.steamid, request.appid)?;
        nonzero(request.publishedfileid, "publishedfileid must not be zero")?;
        self.0
            .call_opaque(
                Endpoint::post(path, Access::Publisher),
                user_app_params(request.steamid, request.appid)
                    .with("publishedfileid", request.publishedfileid),
            )
            .await
    }
}

fn user_app_params(steamid: SteamId, appid: u32) -> Params {
    Params::new().with("steamid", steamid).with("appid", appid)
}

fn file_ids_params(count_name: &str, ids: &[u64]) -> Result<Params> {
    if ids.is_empty() || ids.contains(&0) {
        return Err(Error::InvalidParameter(
            "publishedfileids must contain nonzero IDs",
        ));
    }
    let count = u32::try_from(ids.len())
        .map_err(|_| Error::InvalidParameter("publishedfileids count must fit uint32"))?;
    let mut params = Params::new().with(count_name, count);
    for (index, id) in ids.iter().enumerate() {
        params.push(&format!("publishedfileids[{index}]"), id);
    }
    Ok(params)
}
