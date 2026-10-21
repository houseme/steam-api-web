// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! IInventoryService: publisher inventory grants, crafting, and dynamic properties.
//!
//! Every method requires a publisher key with Economy permissions. Mutations
//! never replay automatically, including calls carrying a caller-owned request
//! ID. Steam validates ownership, recipes, request deduplication, and concurrency.
//! Results preserve the original JSON, including JSON encoded inside strings.
//!
//! The reference contains inconsistent required markers: AddItem's notification,
//! request ID, and restriction flags are explicitly optional in the prose, and
//! its example omits itempropsjson. GetItemDefs filters are listed as scalar
//! uint64 values despite their plural names; these bindings preserve that shape.

use crate::{
    Error, Result, SteamClient, SteamId,
    request::{Access, Endpoint, Params},
    types::{nonzero, text},
    user::validate_user_app,
    user_auth::OpaqueResponse,
};
use serde::{Serialize, Serializer, ser::SerializeMap};
use std::collections::HashSet;

/// Access to all ten documented IInventoryService methods.
#[derive(Debug, Clone, Copy)]
pub struct Inventory<'a>(pub(crate) &'a SteamClient);

/// Item grant and optional caller-managed deduplication identifier.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct AddItemRequest<'a> {
    /// Application owning the item definitions.
    pub appid: u32,
    /// Recipient's Steam ID.
    pub steamid: SteamId,
    /// Definitions to grant. Repeated definitions intentionally grant instances.
    pub itemdefid: &'a [u64],
    /// Optional encoded item-properties JSON. Steam defines its schema; this
    /// client validates JSON syntax and transmits its original text unchanged.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub itempropsjson: Option<&'a str>,
    /// Request a Steam notification; omitted defaults to false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify: Option<bool>,
    /// Persist a unique, nonzero ID before sending to use Steam's at-most-once
    /// execution support. Reuse the same ID when reconciling an ambiguous call.
    /// Omitted or zero uses the server default; this crate never generates IDs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requestid: Option<u64>,
    /// Apply purchase-like market and trade cooldowns; omitted defaults to false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trade_restriction: Option<bool>,
}

/// Promo grant; omitting itemdefid checks and grants all eligible promo items.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct AddPromoItemRequest {
    /// Application owning the promo definitions.
    pub appid: u32,
    /// Recipient's Steam ID.
    pub steamid: SteamId,
    /// Specific promo definition, or all eligible promos when omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub itemdefid: Option<u64>,
    /// Whether Steam should notify the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify: Option<bool>,
    /// Caller-owned execution identifier; never generated or changed here.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requestid: Option<u64>,
}

/// Irreversible consumption of part or all of an inventory stack.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct ConsumeItemRequest {
    /// Application owning the item.
    pub appid: u32,
    /// Item owner's Steam ID.
    pub steamid: SteamId,
    /// Unique inventory item ID.
    pub itemid: u64,
    /// Positive quantity, encoded as the documented decimal string.
    #[serde(serialize_with = "serialize_decimal")]
    pub quantity: u64,
    /// Required caller-owned execution identifier. Use a persisted unique,
    /// nonzero value for deduplication; zero is transmitted unchanged.
    pub requestid: u64,
}

/// Materials and target for one crafting recipe. The two material slices must
/// have equal nonzero length; quantities correspond to item IDs by index.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct ExchangeItemRequest<'a> {
    /// Application owning the crafting definitions.
    pub appid: u32,
    /// Item owner's Steam ID.
    pub steamid: SteamId,
    /// Distinct inventory item IDs to consume.
    pub materialsitemid: &'a [u64],
    /// Positive quantity for each material item ID.
    pub materialsquantity: &'a [u32],
    /// Definition of the item to create.
    pub outputitemdefid: u64,
}

/// Filters for reading item definitions. Empty filters are omitted, following
/// the unfiltered query in Valve's example despite its required-marker column.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct ItemDefsOptions<'a> {
    /// UTC modification boundary in YYYYMMDDTHHMMSSZ form, or no time filter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modifiedsince: Option<&'a str>,
    /// Specific item-definition ID; the reference specifies scalar uint64.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub itemdefids: Option<u64>,
    /// Specific Workshop ID; the reference specifies scalar uint64.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workshopids: Option<u64>,
    /// Maximum acceptable cached-data age in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_max_age_seconds: Option<u32>,
}

/// Shared item-type selection for consolidation and quantity queries.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct ItemTypesRequest<'a> {
    /// Application owning the items.
    pub appid: u32,
    /// Item owner's Steam ID.
    pub steamid: SteamId,
    /// Nonempty list of item-definition IDs.
    pub itemdefid: &'a [u64],
    /// For Consolidate, include stacks with active trade or market restrictions.
    /// GetQuantity also accepts this flag, but its meaning is undocumented.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force: Option<bool>,
}

/// Exactly one dynamic-property operation. Float values use decimal strings
/// on the wire because the Web API declares property_value_float as a string.
#[derive(Debug, Clone, Copy)]
pub enum DynamicPropertyValue<'a> {
    /// Set a string, including an empty string.
    String(&'a str),
    /// Set a boolean.
    Boolean(bool),
    /// Set a signed 64-bit integer without floating-point conversion.
    Integer(i64),
    /// Set a finite 32-bit float.
    Float(f32),
    /// Remove this property from the item.
    Remove,
}

impl Serialize for DynamicPropertyValue<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            Self::String(value) => map.serialize_entry("property_value_string", value)?,
            Self::Boolean(value) => map.serialize_entry("property_value_bool", value)?,
            Self::Integer(value) => map.serialize_entry("property_value_int", value)?,
            Self::Float(value) => {
                if !value.is_finite() {
                    return Err(serde::ser::Error::custom("property float must be finite"));
                }
                map.serialize_entry("property_value_float", &value.to_string())?;
            }
            Self::Remove => map.serialize_entry("remove_property", &true)?,
        }
        map.end()
    }
}

/// One property mutation for an inventory item.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct DynamicPropertyUpdate<'a> {
    /// Unique inventory item ID.
    pub itemid: u64,
    /// Dynamic-property name.
    pub property_name: &'a str,
    /// Property operation; cannot contain competing value types.
    #[serde(flatten)]
    pub value: DynamicPropertyValue<'a>,
}

/// Dynamic-property batch with Steam's optimistic concurrency timestamp.
#[derive(Debug, Clone, Copy)]
pub struct ModifyItemsRequest<'a> {
    /// Application owning the inventory; sent outside input_json.
    pub appid: u32,
    /// Item owner's Steam ID.
    pub steamid: SteamId,
    /// Original Unix timestamp. Steam rejects items modified after this time;
    /// the client neither refreshes it nor retries a rejected mutation.
    pub timestamp: u32,
    /// Property operations affecting at most 100 distinct items. Several
    /// properties of the same item may be updated in one call.
    pub updates: &'a [DynamicPropertyUpdate<'a>],
}

impl Inventory<'_> {
    /// Grant items without automatic replay. Existing request IDs return current
    /// item state after a prior success; a previously failed request may run again.
    pub async fn add_item(&self, request: &AddItemRequest<'_>) -> Result<OpaqueResponse> {
        validate_user_app(request.steamid, request.appid)?;
        validate_ids(request.itemdefid)?;
        if let Some(json) = request.itempropsjson {
            serde_json::from_str::<serde::de::IgnoredAny>(json)
                .map_err(|_| Error::InvalidParameter("itempropsjson must contain valid JSON"))?;
        }
        self.service(
            Endpoint::post("IInventoryService/AddItem/v1/", Access::Publisher),
            request,
        )
        .await
    }

    /// Grant a specific promo or all eligible promos without automatic replay.
    pub async fn add_promo_item(&self, request: &AddPromoItemRequest) -> Result<OpaqueResponse> {
        validate_user_app(request.steamid, request.appid)?;
        if let Some(itemdefid) = request.itemdefid {
            nonzero(itemdefid, "itemdefid must not be zero")?;
        }
        self.service(
            Endpoint::post("IInventoryService/AddPromoItem/v1/", Access::Publisher),
            request,
        )
        .await
    }

    /// Irreversibly consume inventory without automatic replay, even with a
    /// request ID. Reconcile ambiguous outcomes before manually resubmitting.
    pub async fn consume_item(&self, request: &ConsumeItemRequest) -> Result<OpaqueResponse> {
        validate_user_app(request.steamid, request.appid)?;
        nonzero(request.itemid, "itemid must not be zero")?;
        nonzero(request.quantity, "quantity must not be zero")?;
        self.service(
            Endpoint::post("IInventoryService/ConsumeItem/v1/", Access::Publisher),
            request,
        )
        .await
    }

    /// Exchange material items under a server-defined crafting recipe. An empty
    /// returned item_json array can mean an unsuccessful exchange; inspect it.
    pub async fn exchange_item(&self, request: &ExchangeItemRequest<'_>) -> Result<OpaqueResponse> {
        validate_user_app(request.steamid, request.appid)?;
        validate_ids(request.materialsitemid)?;
        nonzero(request.outputitemdefid, "outputitemdefid must not be zero")?;
        if request.materialsitemid.len() != request.materialsquantity.len()
            || request.materialsquantity.contains(&0)
        {
            return Err(Error::InvalidParameter(
                "material IDs and positive quantities must pair by index",
            ));
        }
        let distinct: HashSet<_> = request.materialsitemid.iter().collect();
        if distinct.len() != request.materialsitemid.len() {
            return Err(Error::InvalidParameter(
                "material item IDs must be distinct",
            ));
        }
        self.service(
            Endpoint::post("IInventoryService/ExchangeItem/v1/", Access::Publisher),
            request,
        )
        .await
    }

    /// Read an inventory, retaining encoded item_json and any unknown metadata.
    pub async fn get_inventory(&self, appid: u32, steamid: SteamId) -> Result<OpaqueResponse> {
        validate_user_app(steamid, appid)?;
        self.service(
            Endpoint::get("IInventoryService/GetInventory/v1/", Access::Publisher),
            &Target { appid, steamid },
        )
        .await
    }

    /// Read item definitions with scalar filters and a UTC modification boundary.
    pub async fn get_item_defs(
        &self,
        appid: u32,
        options: ItemDefsOptions<'_>,
    ) -> Result<OpaqueResponse> {
        nonzero(appid.into(), "appid must not be zero")?;
        if let Some(value) = options.modifiedsince
            && !value.is_empty()
            && !valid_compact_timestamp(value)
        {
            return Err(Error::InvalidParameter(
                "modifiedsince must be empty or a valid YYYYMMDDTHHMMSSZ timestamp",
            ));
        }
        #[derive(Serialize)]
        struct Request<'a> {
            appid: u32,
            #[serde(flatten)]
            options: ItemDefsOptions<'a>,
        }
        self.service(
            Endpoint::get("IInventoryService/GetItemDefs/v1/", Access::Publisher),
            &Request { appid, options },
        )
        .await
    }

    /// Read the price sheet for a numeric ECurrency value. The reference declares
    /// no appid parameter; the key must have Economy permissions.
    pub async fn get_price_sheet(&self, ecurrency: i32) -> Result<OpaqueResponse> {
        #[derive(Serialize)]
        struct Request {
            ecurrency: i32,
        }
        self.service(
            Endpoint::get("IInventoryService/GetPriceSheet/v1/", Access::Publisher),
            &Request { ecurrency },
        )
        .await
    }

    /// Consolidate item stacks without automatic replay. Steam applies the most
    /// restrictive market and trade restrictions when stacks are combined.
    pub async fn consolidate(&self, request: &ItemTypesRequest<'_>) -> Result<OpaqueResponse> {
        validate_user_app(request.steamid, request.appid)?;
        validate_ids(request.itemdefid)?;
        self.service(
            Endpoint::post("IInventoryService/Consolidate/v1/", Access::Publisher),
            request,
        )
        .await
    }

    /// Query available totals for item definitions. This read does not reserve
    /// inventory or establish atomicity with a later mutation.
    pub async fn get_quantity(&self, request: &ItemTypesRequest<'_>) -> Result<OpaqueResponse> {
        validate_user_app(request.steamid, request.appid)?;
        validate_ids(request.itemdefid)?;
        self.service(
            Endpoint::get("IInventoryService/GetQuantity/v1/", Access::Publisher),
            request,
        )
        .await
    }

    /// Modify dynamic properties without automatic replay. Steam enforces the
    /// supplied timestamp and the final 1024-byte property limit for each item.
    pub async fn modify_items(&self, request: &ModifyItemsRequest<'_>) -> Result<OpaqueResponse> {
        validate_user_app(request.steamid, request.appid)?;
        if request.updates.is_empty() {
            return Err(Error::InvalidParameter("updates must not be empty"));
        }
        let mut items = HashSet::new();
        let mut properties = HashSet::new();
        for update in request.updates {
            nonzero(update.itemid, "itemid must not be zero")?;
            text(
                update.property_name,
                "property_name must not be empty or contain NUL",
            )?;
            items.insert(update.itemid);
            if !properties.insert((update.itemid, update.property_name)) {
                return Err(Error::InvalidParameter(
                    "each item property may occur only once per update batch",
                ));
            }
            if matches!(update.value, DynamicPropertyValue::Float(value) if !value.is_finite()) {
                return Err(Error::InvalidParameter("property float must be finite"));
            }
        }
        if items.len() > 100 {
            return Err(Error::InvalidParameter(
                "updates may affect at most 100 distinct items",
            ));
        }
        #[derive(Serialize)]
        struct Request<'a> {
            steamid: SteamId,
            timestamp: u32,
            updates: &'a [DynamicPropertyUpdate<'a>],
        }
        let params = Params::json(&Request {
            steamid: request.steamid,
            timestamp: request.timestamp,
            updates: request.updates,
        })?
        .with("appid", request.appid);
        self.0
            .call_opaque(
                Endpoint::post("IInventoryService/ModifyItems/v1/", Access::Publisher),
                params,
            )
            .await
    }

    async fn service(
        &self,
        endpoint: Endpoint,
        request: &impl Serialize,
    ) -> Result<OpaqueResponse> {
        self.0.call_opaque(endpoint, Params::json(request)?).await
    }
}

#[derive(Serialize)]
struct Target {
    appid: u32,
    steamid: SteamId,
}

fn validate_ids(ids: &[u64]) -> Result<()> {
    if ids.is_empty() || ids.contains(&0) {
        Err(Error::InvalidParameter(
            "item ID arrays must contain nonzero IDs",
        ))
    } else {
        Ok(())
    }
}

fn serialize_decimal<S: Serializer>(
    value: &u64,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    serializer.collect_str(value)
}

fn valid_compact_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 16
        || bytes[8] != b'T'
        || bytes[15] != b'Z'
        || bytes
            .iter()
            .enumerate()
            .any(|(index, byte)| index != 8 && index != 15 && !byte.is_ascii_digit())
    {
        return false;
    }
    let number = |start: usize, end: usize| {
        bytes[start..end]
            .iter()
            .fold(0_u32, |value, byte| value * 10 + u32::from(byte - b'0'))
    };
    let year = number(0, 4);
    let days = match number(4, 6) {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => {
            29
        }
        2 => 28,
        _ => return false,
    };
    year != 0
        && (1..=days).contains(&number(6, 8))
        && number(9, 11) <= 23
        && number(11, 13) <= 59
        && number(13, 15) <= 59
}
