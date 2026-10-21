// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

use crate::{Result, http_client, metadata, release::validate_version};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn audit(client: &reqwest::blocking::Client, name: &str, requirements: &[String]) -> Result<Value> {
    let url = format!("https://crates.io/api/v1/crates/{name}");
    let response = client.get(&url).send()?.error_for_status()?;
    let data: Value = serde_json::from_reader(response)?;
    let mut latest = None;
    for version in data["versions"]
        .as_array()
        .ok_or("registry response has no versions")?
    {
        if version["yanked"] != false {
            continue;
        }
        let number = version["num"].as_str().ok_or("invalid registry version")?;
        let parsed = validate_version(number.split('+').next().unwrap_or(number))?;
        if latest.as_ref().is_none_or(|(current, _)| parsed > *current) {
            latest = Some((parsed, version));
        }
    }
    let (_, latest) = latest.ok_or("registry contains no unyanked versions")?;
    let latest_number = latest["num"].as_str().ok_or("invalid registry version")?;
    let selected: Vec<_> = requirements
        .iter()
        .map(|req| req.trim_start_matches(['^', '=']))
        .collect();
    // Reqwest downcasts a concrete Rustls configuration; a different preview cannot substitute for its Rustls dependency.
    let exception = name == "rustls"
        && latest_number == "0.24.0-dev.2"
        && selected.iter().all(|version| *version == "0.23.46");
    let status = if exception {
        "compatible_exception"
    } else if selected.iter().all(|version| *version == latest_number) {
        "current"
    } else {
        "update_required"
    };
    Ok(
        json!({"manifest_minimum":selected.first().copied().unwrap_or(""), "latest_including_prerelease":latest_number, "latest_rust_version":latest["rust_version"], "status":status, "source":url}),
    )
}

pub(crate) fn check(args: &[String]) -> Result<()> {
    let json_output = match args {
        [] => false,
        [flag] if flag == "--json" => true,
        _ => return Err("expected check-dependencies [--json]".into()),
    };
    let metadata = metadata()?;
    let members = metadata["workspace_members"]
        .as_array()
        .ok_or("missing workspace members")?;
    let mut dependencies: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for package in metadata["packages"].as_array().ok_or("missing packages")? {
        if !members.contains(&package["id"]) {
            continue;
        }
        for dependency in package["dependencies"]
            .as_array()
            .ok_or("missing dependencies")?
        {
            if dependency["source"].is_null() {
                continue;
            }
            let name = dependency["name"]
                .as_str()
                .ok_or("dependency has no name")?;
            let requirement = dependency["req"]
                .as_str()
                .ok_or("dependency has no requirement")?;
            let requirements = dependencies.entry(name.to_owned()).or_default();
            if !requirements.iter().any(|value| value == requirement) {
                requirements.push(requirement.to_owned());
            }
        }
    }
    let entries: Vec<_> = dependencies.into_iter().collect();
    let client = http_client()?;
    let mut results = BTreeMap::new();
    // Limit registry concurrency to four requests without introducing an async maintenance runtime.
    for chunk in entries.chunks(4) {
        let values = std::thread::scope(|scope| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|(name, requirements)| {
                    let client = &client;
                    scope.spawn(move || {
                        audit(client, name, requirements).map(|value| (name.clone(), value))
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| {
                    handle
                        .join()
                        .map_err(|_| "dependency audit worker panicked".into())
                        .and_then(|result| result)
                })
                .collect::<Result<Vec<_>>>()
        })?;
        results.extend(values);
    }
    let outdated = results
        .values()
        .any(|value| value["status"] == "update_required");
    if json_output {
        let mut document = serde_json::Map::new();
        document.insert("_metadata".to_owned(), json!({"copyright":"Copyright 2026 steam-api-web contributors.", "license":"Apache-2.0", "license_url":"https://www.apache.org/licenses/LICENSE-2.0", "SPDX-License-Identifier":"Apache-2.0"}));
        document.extend(results);
        println!("{}", serde_json::to_string_pretty(&document)?);
    } else {
        for (name, value) in &results {
            println!(
                "{name}: manifest {}, latest {} [{}]",
                value["manifest_minimum"], value["latest_including_prerelease"], value["status"]
            );
        }
    }
    if outdated {
        return Err("newer unaudited direct dependencies exist; review before updating".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_order_places_new_previews_ahead_of_older_releases() -> Result<()> {
        assert!(validate_version("0.24.0-dev.2")? > validate_version("0.23.46")?);
        assert!(validate_version("1.0.0")? > validate_version("1.0.0-rc.10")?);
        assert!(validate_version("1.0.0-rc.10")? > validate_version("1.0.0-rc.2")?);
        assert!(validate_version("1.0.0-rc.2")? > validate_version("1.0.0-beta.2")?);
        Ok(())
    }
}
