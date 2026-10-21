// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Publisher Workshop payment, contributor and item-description methods.
//!
//! Valve's reference leaves nested payment and localization messages unspecified.
//! Callers supply reviewed JSON messages explicitly; this module validates their
//! container shape and preserves every field without inventing nested contracts.
//! Results remain opaque and never imply a payment or persistence success.

use crate::{
    Error, Result, SteamClient,
    published_file::validate_message,
    request::{Access, Endpoint, Params},
    types::nonzero,
    user_auth::OpaqueResponse,
};
use serde::Serialize;
use serde_json::Value;
use std::fmt;

/// Access to the four documented publisher Workshop methods.
#[derive(Debug, Clone, Copy)]
pub struct WorkshopService<'a>(pub(crate) &'a SteamClient);

/// Borrowed payment-rule messages whose nested schema must be reviewed by the caller.
#[derive(Clone, Copy, Serialize)]
pub struct ItemPaymentRulesRequest<'a> {
    /// Application owning the item.
    pub appid: u32,
    /// Game-specific item identifier.
    pub gameitemid: u32,
    /// Raw associated-file message, as an object or an array of objects.
    pub associated_workshop_files: &'a Value,
    /// Raw partner-account message, as an object or an array of objects.
    pub partner_accounts: &'a Value,
    /// When true, ask Steam to validate without persisting; omission uses its default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validate_only: Option<bool>,
    /// Whether users may subscribe to the referenced Workshop items.
    pub make_workshop_files_subscribable: bool,
}

impl fmt::Debug for ItemPaymentRulesRequest<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ItemPaymentRulesRequest")
            .field("appid", &self.appid)
            .field("gameitemid", &self.gameitemid)
            .field("associated_workshop_files", &"[REDACTED]")
            .field("partner_accounts", &"[REDACTED]")
            .field("validate_only", &self.validate_only)
            .field(
                "make_workshop_files_subscribable",
                &self.make_workshop_files_subscribable,
            )
            .finish()
    }
}

/// A revenue-query time range. Steam treats the end as exclusive.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct ItemDailyRevenueRequest {
    /// Application owning the item definition.
    pub appid: u32,
    /// Item definition whose revenue should be returned.
    pub item_id: u32,
    /// Inclusive Unix timestamp aligned to the service's expected date boundary.
    pub date_start: u32,
    /// Exclusive Unix timestamp aligned to the service's expected date boundary.
    /// The reference does not specify a timezone, so no UTC alignment is inferred.
    pub date_end: u32,
}

impl WorkshopService<'_> {
    /// Send payment rules once, including validation-only requests.
    /// The opaque result must be checked against the publisher's Steam contract.
    pub async fn set_item_payment_rules(
        &self,
        request: ItemPaymentRulesRequest<'_>,
    ) -> Result<OpaqueResponse> {
        validate_app_item(request.appid, request.gameitemid)?;
        validate_message(request.associated_workshop_files)?;
        validate_message(request.partner_accounts)?;
        self.0
            .call_opaque(
                Endpoint::post(
                    "IWorkshopService/SetItemPaymentRules/v1/",
                    Access::Publisher,
                ),
                Params::json(&request)?,
            )
            .await
    }

    /// Read the finalized contributor data without assuming an undocumented envelope.
    pub async fn get_finalized_contributors(
        &self,
        appid: u32,
        gameitemid: u32,
    ) -> Result<OpaqueResponse> {
        validate_app_item(appid, gameitemid)?;
        #[derive(Serialize)]
        struct Request {
            appid: u32,
            gameitemid: u32,
        }
        self.0
            .call_opaque(
                Endpoint::get(
                    "IWorkshopService/GetFinalizedContributors/v1/",
                    Access::Publisher,
                ),
                Params::json(&Request { appid, gameitemid })?,
            )
            .await
    }

    /// Read daily revenue over an ordered, half-open date interval.
    /// Callers select the date-boundary timezone required by their Steam agreement.
    pub async fn get_item_daily_revenue(
        &self,
        request: ItemDailyRevenueRequest,
    ) -> Result<OpaqueResponse> {
        validate_app_item(request.appid, request.item_id)?;
        if request.date_start >= request.date_end {
            return Err(Error::InvalidParameter("date_start must precede date_end"));
        }
        self.0
            .call_opaque(
                Endpoint::get(
                    "IWorkshopService/GetItemDailyRevenue/v1/",
                    Access::Publisher,
                ),
                Params::json(&request)?,
            )
            .await
    }

    /// Send a reviewed raw language message once. Its nested schema is not
    /// documented; object and object-array fields are retained unchanged.
    pub async fn populate_item_descriptions(
        &self,
        appid: u32,
        languages: &Value,
    ) -> Result<OpaqueResponse> {
        nonzero(appid.into(), "appid must not be zero")?;
        validate_message(languages)?;
        #[derive(Serialize)]
        struct Request<'a> {
            appid: u32,
            languages: &'a Value,
        }
        self.0
            .call_opaque(
                Endpoint::post(
                    "IWorkshopService/PopulateItemDescriptions/v1/",
                    Access::Publisher,
                ),
                Params::json(&Request { appid, languages })?,
            )
            .await
    }
}

fn validate_app_item(appid: u32, itemid: u32) -> Result<()> {
    nonzero(appid.into(), "appid must not be zero")?;
    nonzero(itemid.into(), "item identifier must not be zero")
}
