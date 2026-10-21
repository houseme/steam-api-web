// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Publisher-only review of Workshop content requiring approval.
//! The documented item fields do not define the complete response envelope,
//! so results remain opaque rather than assuming an array name or success flag.

use crate::{
    Error, Result, SteamClient,
    published_file::validate_item,
    request::{Access, Endpoint, Params},
    types::nonzero,
    user_auth::OpaqueResponse,
};
use serde::{Serialize, Serializer};

/// Access to the two documented published-file moderation methods.
#[derive(Debug, Clone, Copy)]
pub struct PublishedFileModerationService<'a>(pub(crate) &'a SteamClient);

/// The public revision identifiers listed by Valve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PublishedFileRevision {
    /// Service-selected revision.
    Default = 0,
    /// Most recent content, regardless of its review state.
    Latest = 1,
    /// Snapshot last approved by a moderator.
    ApprovedSnapshot = 2,
    /// Snapshot last rejected by a moderator.
    RejectedSnapshot = 4,
}

impl Serialize for PublishedFileRevision {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_u32(*self as u32)
    }
}

/// Optional approval-query controls; omission preserves Steam's defaults.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct ApprovalRequestsOptions {
    /// Revision to inspect; Steam defaults to the approved snapshot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<PublishedFileRevision>,
    /// Zero-based result offset; Steam defaults to zero.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<u32>,
    /// Number to retrieve, from one through 1000; Steam defaults to 100.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
}

/// An approval decision tied to the exact revision the moderator reviewed.
#[derive(Debug, Clone, Copy)]
pub struct ProcessApprovalRequest {
    /// Application owning the item.
    pub appid: u32,
    /// Item being approved or rejected.
    pub publishedfileid: u64,
    /// Exact `revision_change_number` reviewed by the moderator, without truncation.
    pub change_number: u64,
    /// Approve when true, reject when false.
    pub approved: bool,
}

impl PublishedFileModerationService<'_> {
    /// Read pending requests, including requests already claimed by moderators.
    /// Items are ordered from oldest to newest by Steam.
    pub async fn get_all_approval_requests(
        &self,
        appid: u32,
        options: ApprovalRequestsOptions,
    ) -> Result<OpaqueResponse> {
        nonzero(appid.into(), "appid must not be zero")?;
        if options
            .count
            .is_some_and(|count| !(1..=1000).contains(&count))
        {
            return Err(Error::InvalidParameter("count must be between 1 and 1000"));
        }
        #[derive(Serialize)]
        struct Request {
            appid: u32,
            #[serde(flatten)]
            options: ApprovalRequestsOptions,
        }
        self.0
            .call_opaque(
                Endpoint::get(
                    "IPublishedFileModerationService/GetAllApprovalRequests/v1/",
                    Access::Publisher,
                ),
                Params::json(&Request { appid, options })?,
            )
            .await
    }

    /// Apply a decision using revision 2, the only revision permitted by Valve.
    /// A stale change number fails remotely and requires reviewing the new content.
    /// The mutation is never automatically replayed.
    pub async fn process_approval_request(
        &self,
        request: ProcessApprovalRequest,
    ) -> Result<OpaqueResponse> {
        validate_item(request.appid, request.publishedfileid)?;
        #[derive(Serialize)]
        struct Request {
            publishedfileid: u64,
            appid: u32,
            revision: PublishedFileRevision,
            change_number: u64,
            approved: bool,
        }
        self.0
            .call_opaque(
                Endpoint::post(
                    "IPublishedFileModerationService/ProcessApprovalRequest/v1/",
                    Access::Publisher,
                ),
                Params::json(&Request {
                    publishedfileid: request.publishedfileid,
                    appid: request.appid,
                    revision: PublishedFileRevision::ApprovedSnapshot,
                    change_number: request.change_number,
                    approved: request.approved,
                })?,
            )
            .await
    }
}
