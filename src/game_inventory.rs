// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! IGameInventory: publisher asset history and definition administration.
//!
//! The reference omits most response schemas. UpdateItemDefs additionally has
//! inconsistent response/result examples, so all responses remain opaque. Its
//! failed/updated arrays require caller inspection even after transport success.

use crate::{
    Error, Result, SteamClient, SteamId,
    request::{Access, Endpoint, Params},
    types::{nonzero, text},
    user::validate_user_app,
    user_auth::OpaqueResponse,
};
use serde_json::Value;

/// Access to all five documented IGameInventory methods.
#[derive(Debug, Clone, Copy)]
pub struct GameInventory<'a>(pub(crate) &'a SteamClient);

/// Original command and arguments for looking up a history action's details.
#[derive(Debug, Clone, Copy)]
pub struct HistoryCommandDetailsRequest<'a> {
    /// Application owning the asset.
    pub appid: u32,
    /// Account whose history is being inspected.
    pub steamid: SteamId,
    /// Original asset command.
    pub command: &'a str,
    /// Inventory context ID.
    pub contextid: u64,
    /// Arguments originally supplied; empty arguments are preserved.
    pub arguments: &'a str,
}

/// A bounded user inventory-history interval.
#[derive(Debug, Clone, Copy)]
pub struct UserHistoryRequest {
    /// Application owning the inventory.
    pub appid: u32,
    /// Account whose history is being inspected.
    pub steamid: SteamId,
    /// Inventory context ID.
    pub contextid: u64,
    /// Beginning of the history interval, as Unix seconds.
    pub starttime: u32,
    /// End of the history interval, as Unix seconds.
    pub endtime: u32,
}

/// Execute a support history action. The official parameter name is steamid,
/// although its description says asset ID; the binding preserves the wire name.
#[derive(Debug, Clone, Copy)]
pub struct HistoryExecuteCommandsRequest {
    /// Application owning the inventory.
    pub appid: u32,
    /// Original 64-bit steamid parameter from the reference.
    pub steamid: SteamId,
    /// Inventory context ID.
    pub contextid: u64,
    /// Unique 32-bit identifier of the support operator.
    pub actorid: u32,
}

/// Asset history lookup for publisher support tooling.
#[derive(Debug, Clone, Copy)]
pub struct AssetHistoryRequest {
    /// Application owning the asset.
    pub appid: u32,
    /// Inventory asset ID.
    pub assetid: u64,
    /// Inventory context ID.
    pub contextid: u64,
}

/// Borrowed definitions for one update. Definitions remain arbitrary JSON
/// objects because their fields belong to the application's economy schema.
#[derive(Debug, Clone, Copy)]
pub struct UpdateItemDefsRequest<'a> {
    /// Application owning these definitions.
    pub appid: u32,
    /// One or more definition objects; serialized into the itemdefs form field.
    pub itemdefs: &'a [Value],
}

impl GameInventory<'_> {
    /// Read details for an original inventory-history command.
    pub async fn get_history_command_details(
        &self,
        request: &HistoryCommandDetailsRequest<'_>,
    ) -> Result<OpaqueResponse> {
        validate_user_app(request.steamid, request.appid)?;
        text(request.command, "command must not be empty or contain NUL")?;
        if request.arguments.contains('\0') {
            return Err(Error::InvalidParameter("arguments must not contain NUL"));
        }
        self.0
            .call_opaque(
                Endpoint::get(
                    "IGameInventory/GetHistoryCommandDetails/v1/",
                    Access::Publisher,
                ),
                target(request.appid, request.steamid, request.contextid)
                    .with("command", request.command)
                    .with("arguments", request.arguments),
            )
            .await
    }

    /// Read user history for the supplied interval without fabricating a cursor.
    pub async fn get_user_history(&self, request: &UserHistoryRequest) -> Result<OpaqueResponse> {
        validate_user_app(request.steamid, request.appid)?;
        if request.starttime > request.endtime {
            return Err(Error::InvalidParameter("starttime must not exceed endtime"));
        }
        self.0
            .call_opaque(
                Endpoint::get("IGameInventory/GetUserHistory/v1/", Access::Publisher),
                target(request.appid, request.steamid, request.contextid)
                    .with("starttime", request.starttime)
                    .with("endtime", request.endtime),
            )
            .await
    }

    /// Execute a support action once. No automatic retries or command parameters
    /// beyond those specified in the current reference are introduced.
    pub async fn history_execute_commands(
        &self,
        request: &HistoryExecuteCommandsRequest,
    ) -> Result<OpaqueResponse> {
        validate_user_app(request.steamid, request.appid)?;
        self.0
            .call_opaque(
                Endpoint::post(
                    "IGameInventory/HistoryExecuteCommands/v1/",
                    Access::Publisher,
                ),
                target(request.appid, request.steamid, request.contextid)
                    .with("actorid", request.actorid),
            )
            .await
    }

    /// Read one asset's history in the supplied context.
    pub async fn support_get_asset_history(
        &self,
        request: &AssetHistoryRequest,
    ) -> Result<OpaqueResponse> {
        nonzero(request.appid.into(), "appid must not be zero")?;
        nonzero(request.assetid, "assetid must not be zero")?;
        self.0
            .call_opaque(
                Endpoint::get(
                    "IGameInventory/SupportGetAssetHistory/v1/",
                    Access::Publisher,
                ),
                Params::new()
                    .with("appid", request.appid)
                    .with("assetid", request.assetid)
                    .with("contextid", request.contextid),
            )
            .await
    }

    /// Update or create definitions without replay. The documented v0001 path is
    /// retained verbatim. Inspect failed IDs in the response before concluding
    /// that every definition changed. Cache flushing is a separate caller action.
    pub async fn update_item_defs(
        &self,
        request: &UpdateItemDefsRequest<'_>,
    ) -> Result<OpaqueResponse> {
        nonzero(request.appid.into(), "appid must not be zero")?;
        if request.itemdefs.is_empty() || request.itemdefs.iter().any(|value| !value.is_object()) {
            return Err(Error::InvalidParameter(
                "itemdefs must be a nonempty array of JSON objects",
            ));
        }
        for definition in request.itemdefs {
            if let Some(appid) = definition.get("appid") {
                let parsed = appid
                    .as_u64()
                    .or_else(|| appid.as_str()?.parse::<u64>().ok());
                if parsed != Some(u64::from(request.appid)) {
                    return Err(Error::InvalidParameter(
                        "item definition appid must match the request appid",
                    ));
                }
            }
        }
        let mut params = Params::new().with("appid", request.appid);
        params.push_json("itemdefs", request.itemdefs)?;
        self.0
            .call_opaque(
                Endpoint::post("IGameInventory/UpdateItemDefs/v0001", Access::Publisher),
                params,
            )
            .await
    }
}

fn target(appid: u32, steamid: SteamId, contextid: u64) -> Params {
    Params::new()
        .with("appid", appid)
        .with("steamid", steamid)
        .with("contextid", contextid)
}
