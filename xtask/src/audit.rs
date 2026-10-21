// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Compare the partner reference, bindings and anonymous discovery separately.

use crate::{Result, http_client, root};
use scraper::{ElementRef, Html, Selector};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

const DOCS: &str = "https://partner.steamgames.com/doc/webapi";
const DISCOVERY: &str = "https://api.steampowered.com/ISteamWebAPIUtil/GetSupportedAPIList/v1/";

fn text<'a>(value: &'a Value, field: &str) -> Result<&'a str> {
    value[field]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("missing {field}").into())
}
fn array<'a>(value: &'a Value, field: &str) -> Result<&'a [Value]> {
    value[field]
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| format!("missing array {field}").into())
}
fn legal() -> Value {
    json!({"copyright": "Copyright 2026 steam-api-web contributors.", "license": "Apache-2.0", "SPDX-License-Identifier": "Apache-2.0", "license_url": "https://www.apache.org/licenses/LICENSE-2.0"})
}
fn digest(bytes: &[u8]) -> String {
    let value = aws_lc_rs::digest::digest(&aws_lc_rs::digest::SHA256, bytes);
    value
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn selector(value: &str) -> Result<Selector> {
    Selector::parse(value).map_err(|_| "invalid audit selector".into())
}

/// Parse as HTML5: Valve tables can omit closing tr tags. Splitting source text
/// silently loses rows such as AddItem's trade_restriction parameter.
fn article(document: &Html) -> Result<ElementRef<'_>> {
    let selected = selector(".documentation_bbcode")?;
    let mut articles = document.select(&selected);
    let article = articles.next().ok_or("missing documentation article")?;
    if articles.next().is_some() {
        return Err("ambiguous documentation article".into());
    }
    Ok(article)
}

fn content(element: ElementRef<'_>) -> String {
    element
        .text()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn interfaces(html: &str) -> Result<BTreeSet<String>> {
    let document = Html::parse_document(html);
    let article = article(&document)?;
    let links = selector("a[href]")?;
    let mut names = BTreeSet::new();
    for link in article.select(&links) {
        let href = link.value().attr("href").ok_or("missing link")?;
        let name = href
            .strip_prefix("/doc/webapi/")
            .or_else(|| href.strip_prefix("https://partner.steamgames.com/doc/webapi/"));
        if let Some(name) = name
            && !name.is_empty()
            && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            names.insert(name.to_owned());
        }
    }
    if names.is_empty() {
        return Err("reference index has no interfaces".into());
    }
    Ok(names)
}

fn parameters(table: ElementRef<'_>) -> Result<Vec<Value>> {
    let rows = selector("tr")?;
    let mut header = false;
    let mut result = Vec::new();
    let mut names = BTreeSet::new();
    for row in table.select(&rows) {
        // A nested example table inside a description is not a parameter list.
        if row
            .ancestors()
            .filter_map(ElementRef::wrap)
            .find(|node| node.value().name() == "table")
            .is_none_or(|ancestor| ancestor.id() != table.id())
        {
            continue;
        }
        let cells: Vec<_> = row
            .children()
            .filter_map(ElementRef::wrap)
            .filter(|cell| matches!(cell.value().name(), "td" | "th"))
            .collect();
        if !header {
            let titles: Vec<_> = cells.iter().copied().map(content).collect();
            if titles != ["Name", "Type", "Required", "Description"] {
                return Err("unrecognized parameter table header".into());
            }
            header = true;
            continue;
        }
        if cells.len() < 4 || cells[4..].iter().any(|cell| !content(*cell).is_empty()) {
            return Err("invalid parameter row".into());
        }
        let name = content(cells[0]);
        let kind = content(cells[1]);
        let required = content(cells[2]);
        if name.is_empty() || kind.is_empty() || !names.insert(name.clone()) {
            return Err("invalid or duplicate parameter".into());
        }
        let requirement = match required.as_str() {
            "✔" => "required",
            "" => "optional",
            "*" => "conditional",
            _ => return Err("unrecognized required marker".into()),
        };
        result.push(json!({"name":name,"type":kind,"requirement":requirement}));
    }
    if !header {
        return Err("missing parameter header".into());
    }
    Ok(result)
}

fn parameter_table(code: ElementRef<'_>) -> Option<ElementRef<'_>> {
    for node in code.next_siblings().filter_map(ElementRef::wrap) {
        match node.value().name() {
            "table" => return Some(node),
            "h2" | "h3" => break,
            _ => (),
        }
    }
    None
}

fn deprecated(code: ElementRef<'_>) -> bool {
    code.next_siblings()
        .take_while(|node| {
            ElementRef::wrap(*node)
                .is_none_or(|element| !matches!(element.value().name(), "h2" | "h3"))
        })
        .any(|node| {
            if let Some(element) = ElementRef::wrap(node) {
                content(element).contains("Deprecated - this API")
            } else {
                node.value()
                    .as_text()
                    .is_some_and(|text| text.contains("Deprecated - this API"))
            }
        })
}

fn methods(name: &str, html: &str) -> Result<Vec<Value>> {
    let document = Html::parse_document(html);
    let article = article(&document)?;
    let codes = selector("div.bb_code")?;
    let mut result = Vec::new();
    let mut signatures = BTreeSet::new();
    for block in article.select(&codes) {
        let code = content(block);
        let mut tokens = code.split_whitespace();
        let Some(verb @ ("GET" | "POST")) = tokens.next() else {
            continue;
        };
        let url = reqwest::Url::parse(tokens.next().ok_or("missing method URL")?)?;
        if url.scheme() != "https"
            || !matches!(
                url.host_str(),
                Some("partner.steam-api.com" | "api.steampowered.com")
            )
        {
            return Err("unrecognized method host".into());
        }
        let path = url.path().trim_start_matches('/');
        let parts: Vec<_> = path.trim_end_matches('/').split('/').collect();
        if parts.len() != 3 || parts[0] != name || !parts[2].starts_with('v') {
            return Err("method path does not match interface".into());
        }
        let version: u64 = parts[2][1..].parse()?;
        if version == 0 || !signatures.insert((parts[1].to_owned(), version, verb.to_owned())) {
            return Err("invalid or duplicate reference signature".into());
        }
        let table = parameter_table(block);
        let has_table = table.is_some();
        let parameters = table
            .map(parameters)
            .transpose()
            .map_err(|error| format!("{name}/{}: {error}", parts[1]))?
            .unwrap_or_default();
        result.push(json!({"name":parts[1],"version":version,"http_method":verb,"path":path,"parameters":parameters,"parameter_table_published":has_table,"deprecated":deprecated(block)}));
    }
    if result.is_empty() && name != "ISteamMicroTxnSandbox" {
        return Err(format!("{name}: no HTTP signatures; manual review required").into());
    }
    if name == "ISteamMicroTxnSandbox"
        && (!result.is_empty() || !content(article).contains("ISteamMicroTxn"))
    {
        return Err("Sandbox inheritance needs manual review".into());
    }
    Ok(result)
}

fn signature(interface: &str, method: &Value) -> Result<String> {
    let verb = text(method, "http_method")?;
    let version = method["version"]
        .as_u64()
        .filter(|v| *v > 0)
        .ok_or("invalid method version")?;
    if !matches!(verb, "GET" | "POST") {
        return Err("invalid HTTP verb".into());
    }
    Ok(format!(
        "{interface}/{} / v{version} / {verb}",
        text(method, "name")?
    ))
}

fn discovery(value: &Value) -> Result<BTreeMap<String, Value>> {
    let mut signatures = BTreeMap::new();
    for interface in array(&value["apilist"], "interfaces")? {
        let name = text(interface, "name")?;
        for method in array(interface, "methods")? {
            let mut normalized = json!({"interface":name, "name":text(method,"name")?, "version":method["version"], "http_method":text(method,"httpmethod")?, "parameters": []});
            let mut params = Vec::new();
            for parameter in array(method, "parameters")? {
                let optional = parameter["optional"]
                    .as_bool()
                    .ok_or("missing discovery optional flag")?;
                params.push(json!({"name":text(parameter,"name")?, "type":text(parameter,"type")?, "optional":optional}));
            }
            normalized["parameters"] = params.into();
            if signatures
                .insert(signature(name, &normalized)?, normalized)
                .is_some()
            {
                return Err("duplicate discovery signature".into());
            }
        }
    }
    Ok(signatures)
}

fn return_type(source: &str, method: &str) -> Result<String> {
    let prefix = format!("pub async fn {method}(");
    let declaration = source
        .split_once(&prefix)
        .ok_or_else(|| format!("missing public method {method}"))?
        .1;
    let declaration = declaration
        .split_once(" {")
        .ok_or("unrecognized function body")?
        .0;
    let result = declaration
        .split_once("->")
        .ok_or("missing method return type")?
        .1
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if !result.starts_with("Result<") {
        return Err("unrecognized method result type".into());
    }
    Ok(result)
}

fn parameter_contract(value: &Value) -> Result<BTreeMap<String, (String, String)>> {
    let parameters = value.as_array().ok_or("missing parameter contract")?;
    let mut contract = BTreeMap::new();
    for parameter in parameters {
        let name = text(parameter, "name")?;
        let kind = text(parameter, "type")?;
        let requirement = text(parameter, "requirement")?;
        if !matches!(requirement, "required" | "optional" | "conditional") {
            return Err("unknown parameter requirement".into());
        }
        if contract
            .insert(name.to_owned(), (kind.to_owned(), requirement.to_owned()))
            .is_some()
        {
            return Err("duplicate parameter contract".into());
        }
    }
    Ok(contract)
}

fn snapshot(inventory: &Value) -> Result<Value> {
    let mut entries = Vec::new();
    for interface in array(inventory, "interfaces")? {
        let methods: Vec<_> = array(interface, "methods")?.iter().filter(|method| method.get("inherited_from").is_none_or(Value::is_null)).map(|method| json!({"name":method["name"], "version":method["version"], "http_method":method["http_method"], "path":method["path"], "parameters":method["reference_parameters"],"parameter_table_published":method["reference_parameter_table"],"deprecated":method["reference_deprecated"]})).collect();
        entries.push(json!({"interface":interface["interface"], "documentation_sha256":interface["documentation_sha256"], "methods":methods}));
    }
    Ok(json!({"interfaces":entries, "sdk_reference":inventory["_metadata"]["sdk_reference"]}))
}

fn fetch(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<u8>> {
    use std::io::Read;
    const LIMIT: usize = 4 * 1024 * 1024;
    let response = client.get(url).send()?.error_for_status()?;
    let mut bytes = Vec::new();
    response.take((LIMIT + 1) as u64).read_to_end(&mut bytes)?;
    if bytes.len() > LIMIT {
        return Err("reference response exceeds 4 MiB".into());
    }
    Ok(bytes)
}

fn captured_reference(directory: &Path, inventory: &Value) -> Result<Value> {
    let index = fs::read(directory.join("index.html"))?;
    let names = interfaces(std::str::from_utf8(&index)?)?;
    let mut entries = Vec::new();
    for name in names {
        let bytes = fs::read(directory.join(format!("{name}.html")))?;
        entries.push(json!({"interface":name,"documentation_sha256":digest(&bytes),"methods":methods(&name,std::str::from_utf8(&bytes)?)?}));
    }
    Ok(
        json!({"interfaces":entries,"index_sha256":digest(&index),"sdk_reference":inventory["_metadata"]["sdk_reference"]}),
    )
}

fn live_reference() -> Result<(Value, Value)> {
    let client = http_client()?;
    let index = fetch(&client, &format!("{DOCS}?l=english"))?;
    let names: Vec<_> = interfaces(std::str::from_utf8(&index)?)?
        .into_iter()
        .collect();
    let mut entries = Vec::new();
    // Four bounded workers fetch public documentation only; no method is invoked.
    for batch in names.chunks(4) {
        let results = std::thread::scope(|scope| {
            let workers: Vec<_> = batch.iter().map(|name| {
                let client = &client;
                scope.spawn(move || -> Result<Value> {
                    let bytes = fetch(client, &format!("{DOCS}/{name}?l=english"))?;
                    Ok(json!({"interface":name,"documentation_sha256":digest(&bytes),"methods":methods(name,std::str::from_utf8(&bytes)?)?}))
                })
            }).collect();
            workers
                .into_iter()
                .map(|worker| {
                    worker
                        .join()
                        .map_err(|_| "reference worker failed".into())
                        .and_then(|result| result)
                })
                .collect::<Result<Vec<_>>>()
        })?;
        entries.extend(results);
    }
    let upstream: Value = serde_json::from_slice(&fetch(
        &client,
        "https://api.github.com/repos/Noxime/steamworks-rs/commits/master",
    )?)?;
    let sha = text(&upstream, "sha")?;
    let tree: Value = serde_json::from_slice(&fetch(
        &client,
        &format!("https://api.github.com/repos/Noxime/steamworks-rs/git/trees/{sha}?recursive=1"),
    )?)?;
    if tree["truncated"] != false {
        return Err("truncated SDK source tree".into());
    }
    let modules: Vec<_> = array(&tree, "tree")?
        .iter()
        .filter(|entry| entry["type"] == "blob")
        .filter_map(|entry| entry["path"].as_str())
        .filter(|path| path.starts_with("src/") && path.ends_with(".rs"))
        .collect();
    let discovered: Value = serde_json::from_slice(&fetch(&client, DISCOVERY)?)?;
    Ok((
        json!({"interfaces":entries,"index_sha256":digest(&index), "sdk_reference":{"commit_sha":sha,"tree_sha":text(&tree,"sha")?,"source_modules":modules}}),
        discovered,
    ))
}

fn report(inventory: &Value, reference: &Value, discovered: &Value) -> Result<Value> {
    let mut documented = BTreeMap::new();
    let mut bindings = BTreeSet::new();
    let mut drift = Vec::new();
    let mut parameter_rows = 0;
    for interface in array(reference, "interfaces")? {
        let name = text(interface, "interface")?;
        for method in array(interface, "methods")? {
            parameter_rows += parameter_contract(&method["parameters"])?.len();
            if documented
                .insert(signature(name, method)?, method)
                .is_some()
            {
                return Err("duplicate documented signature".into());
            }
        }
    }
    let mut tests = BTreeMap::new();
    for entry in fs::read_dir(root().join("tests"))? {
        let path = entry?.path();
        if path.is_file() && path.extension().is_some_and(|extension| extension == "rs") {
            tests.insert(
                path.strip_prefix(root())?.to_string_lossy().to_string(),
                fs::read_to_string(path)?,
            );
        }
    }
    let mut interfaces_report = Vec::new();
    let mut counts = BTreeMap::from([
        ("typed", 0),
        ("validated_opaque", 0),
        ("empty_acknowledgement", 0),
    ]);
    let mut inherited = 0;
    let mut mapped = 0;
    let mut modules = BTreeMap::new();
    for interface in array(inventory, "interfaces")? {
        let name = text(interface, "interface")?;
        if !array(reference, "interfaces")?
            .iter()
            .any(|entry| entry["interface"] == name)
        {
            drift.push(format!("reference no longer lists {name}"));
        }
        match text(interface, "binding_status")? {
            "planned" => {
                drift.push(format!("unbound reference interface: {name}"));
                interfaces_report
                    .push(json!({"interface":name,"binding_status":"planned","methods":[]}));
                continue;
            }
            "bound_contract_tests_only" => (),
            _ => return Err("unknown binding status".into()),
        }
        let module = text(interface, "rust_module")?;

        let source = match modules.entry(module) {
            std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(fs::read_to_string(root().join(format!("src/{module}.rs")))?)
            }
        };
        let mut entries = Vec::new();
        for method in array(interface, "methods")? {
            let key = signature(name, method)?;
            bindings.insert(key.clone());
            let is_inherited = method
                .get("inherited_from")
                .is_some_and(|value| !value.is_null());
            inherited += usize::from(is_inherited);
            if is_inherited {
                let parent = text(method, "inherited_from")?;
                if !documented.contains_key(&signature(parent, method)?) {
                    drift.push(format!("missing inherited reference origin: {key}"));
                }
            } else {
                if let Some(actual) = documented.get(&key) {
                    if actual["path"] != method["path"]
                        || actual["parameter_table_published"]
                            != method["reference_parameter_table"]
                        || actual["deprecated"] != method["reference_deprecated"]
                        || parameter_contract(&actual["parameters"])?
                            != parameter_contract(&method["reference_parameters"])?
                    {
                        drift.push(format!("reference contract changed: {key}"));
                    }
                } else {
                    drift.push(format!("reference no longer documents: {key}"));
                }
            }
            let rust_method = text(method, "rust_method")?;
            let result_type = return_type(source, rust_method)?;
            let shape = if result_type == "Result<OpaqueResponse>" {
                "validated_opaque"
            } else if result_type == "Result<()>" {
                "empty_acknowledgement"
            } else {
                "typed"
            };
            *counts
                .get_mut(shape)
                .ok_or("invalid result classification")? += 1;
            let needle = format!(".{rust_method}(");
            let fixtures: Vec<_> = tests
                .iter()
                .filter(|(_, source)| source.contains(name) && source.contains(&needle))
                .map(|(name, _)| name)
                .collect();
            if fixtures.is_empty() {
                drift.push(format!("no static protocol-fixture reference: {key}"));
            } else {
                mapped += 1;
            }
            entries.push(json!({"name":method["name"],"path":method["path"],"http_method":method["http_method"],"rust_method":rust_method,"result_type":result_type,"response_shape":shape,"parameter_table_published":method["reference_parameter_table"],"deprecated":method["reference_deprecated"],"fixture_sources":fixtures,"inherited":is_inherited}));
        }
        interfaces_report.push(json!({"interface":name,"module":module,"methods":entries}));
    }
    for key in documented.keys() {
        if !bindings.contains(key) {
            drift.push(format!("unbound reference method: {key}"));
        }
    }
    if reference["sdk_reference"]["commit_sha"]
        != inventory["_metadata"]["sdk_reference"]["commit_sha"]
        || reference["sdk_reference"]["source_modules"]
            != inventory["_metadata"]["sdk_reference"]["source_modules"]
    {
        drift.push("SDK reference changed; re-review P5 design".to_owned());
    }
    if inventory["_metadata"]["reference_parameter_rows"].as_u64() != Some(parameter_rows as u64) {
        drift.push("reference parameter-row total changed".to_owned());
    }
    let unpublished_tables: Vec<_> = documented
        .iter()
        .filter(|(_, method)| method["parameter_table_published"] == false)
        .map(|(key, _)| key)
        .collect();
    let deprecated_routes: Vec<_> = documented
        .iter()
        .filter(|(_, method)| method["deprecated"] == true)
        .map(|(key, _)| key)
        .collect();
    let mut requirements = BTreeMap::from([("required", 0), ("optional", 0), ("conditional", 0)]);
    for method in documented.values() {
        if !method["parameter_table_published"].is_boolean() || !method["deprecated"].is_boolean() {
            return Err("missing reference publication/deprecation state".into());
        }
        let contract = parameter_contract(&method["parameters"])?;
        if method["parameter_table_published"] == false && !contract.is_empty() {
            return Err("unpublished table contains inferred parameters".into());
        }
        for (_, (_, requirement)) in contract {
            *requirements
                .get_mut(requirement.as_str())
                .ok_or("unknown requirement")? += 1;
        }
    }
    let discovered_map = discovery(discovered)?;
    let outside: Vec<_> = discovered_map
        .iter()
        .filter(|(key, _)| !bindings.contains(*key))
        .map(|(_, method)| method)
        .collect();
    let overlap = discovered_map.len() - outside.len();
    Ok(json!({
        "_metadata":legal(),"binding_version":inventory["_metadata"]["binding_version"],
        "snapshot_captured_on":inventory["_metadata"]["retrieved_on"],
        "scope":"Partner reference contracts and static source/fixture mappings; anonymous discovery is a separate non-exhaustive observation. No credentialed writes are executed.",
        "documented":{"interfaces":array(reference,"interfaces")?.len(),"explicit_signatures":documented.len(),"inherited_routes":inherited,"bound_routes":bindings.len(),"parameter_rows":parameter_rows,"parameter_requirements":requirements,"unpublished_parameter_tables":unpublished_tables,"deprecated_routes":deprecated_routes,"contract_drift":drift},
        "response_shapes":counts,"fixture_mapped_routes":mapped,
        "anonymous_discovery":{"source":DISCOVERY,"interfaces":array(&discovered["apilist"],"interfaces")?.len(),"signatures":discovered_map.len(),"overlap_with_bindings":overlap,"outside_reference_bindings":outside,"not_visible_anonymously":bindings.len()-overlap},
        "sdk_reference":reference["sdk_reference"],
        "completion":{"documented_signature_mapping":documented.keys().all(|key|bindings.contains(key)),"reviewed_contracts_current":drift.is_empty(),"all_anonymously_discovered_signatures_bound":outside.is_empty(),"credentialed_production_acceptance":false,"native_sdk_implementation":false},
        "validation_boundary":"Parameter rows retain documentation required markers, including known prose/example contradictions. Fixture references are static mappings, not per-method test execution or semantic proof. Opaque results preserve unpublished schemas; live permissions and business outcomes require separate acceptance.",
        "interfaces":interfaces_report,"reference":reference,
    }))
}

pub(crate) fn run(args: &[String]) -> Result<()> {
    let mut live = false;
    let mut json_output = false;
    let mut output = None;
    let mut reference_dir = None;
    let mut values = args.iter();
    while let Some(arg) = values.next() {
        match arg.as_str() {
            "--live" if !live => live = true,
            "--json" if !json_output => json_output = true,
            "--reference-dir" if reference_dir.is_none() => {
                reference_dir = Some(values.next().ok_or("missing --reference-dir path")?)
            }
            "--output" if output.is_none() => {
                output = Some(values.next().ok_or("missing --output path")?)
            }
            _ => return Err(format!("unknown audit-api argument {arg}").into()),
        }
    }
    let inventory: Value =
        serde_json::from_slice(&fs::read(root().join("docs/api-inventory.json"))?)?;
    crate::repository::validate_bindings(&inventory)?;
    if live && reference_dir.is_some() {
        return Err("choose --live or --reference-dir".into());
    }
    let (reference, discovered) = if live {
        live_reference()?
    } else {
        (
            if let Some(directory) = reference_dir {
                captured_reference(Path::new(directory), &inventory)?
            } else {
                snapshot(&inventory)?
            },
            serde_json::from_slice(&fs::read(root().join("docs/api-discovery.json"))?)?,
        )
    };
    let mut result = report(&inventory, &reference, &discovered)?;
    result["mode"] = if live {
        "live_public_reference"
    } else {
        if reference_dir.is_some() {
            "captured_documentation"
        } else {
            "offline_snapshot"
        }
    }
    .into();
    if live {
        result["captured_unix_seconds"] = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs()
            .into();
    }
    let encoded = serde_json::to_string_pretty(&result)?;
    if let Some(path) = output {
        fs::write(Path::new(path), format!("{encoded}\n"))?;
    }
    if json_output {
        println!("{encoded}");
    } else {
        println!(
            "API audit: {} interfaces, {} explicit + {} inherited routes, {} parameter rows; {} fixture mappings",
            result["documented"]["interfaces"],
            result["documented"]["explicit_signatures"],
            result["documented"]["inherited_routes"],
            result["documented"]["parameter_rows"],
            result["fixture_mapped_routes"]
        );
        println!(
            "Responses: {} typed, {} empty acknowledgements, {} validated opaque",
            result["response_shapes"]["typed"],
            result["response_shapes"]["empty_acknowledgement"],
            result["response_shapes"]["validated_opaque"]
        );
        println!(
            "Anonymous discovery: {} signatures; {} outside the reference bindings. Credentialed acceptance and native SDK implementation remain incomplete.",
            result["anonymous_discovery"]["signatures"],
            array(&result["anonymous_discovery"], "outside_reference_bindings")?.len()
        );
    }
    let drift = array(&result["documented"], "contract_drift")?;
    if !drift.is_empty() {
        return Err(format!(
            "{} API reference changes require review; inspect the JSON contract_drift field",
            drift.len()
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn page(body: &str) -> String {
        format!(
            "<nav><a href=\"/doc/webapi/INav\">ignore</a></nav><div class=\"documentation_bbcode\">{body}</div><div id=\"hashLocationHighlight\"></div>"
        )
    }
    const TABLE: &str = "<table><tr><th>Name</th><th>Type</th><th>Required</th><th>Description</th></tr><tr><td><strong>appid</strong></td><td>uint32</td><td>✔</td><td>App &amp; game</td></tr></table>";
    #[test]
    fn missing_tables_and_deprecation_do_not_borrow_the_next_methods_contract() -> Result<()> {
        let html = page(&format!(
            "<div class=\"bb_code http\">GET https://api.steampowered.com/ITest/Old/v1/</div>Deprecated - this API is replaced.<h2>Next</h2><div class=\"bb_code http\">GET https://api.steampowered.com/ITest/New/v2/</div>{TABLE}"
        ));
        let values = methods("ITest", &html)?;
        assert_eq!(values[0]["parameter_table_published"], false);
        assert_eq!(values[0]["parameters"], json!([]));
        assert_eq!(values[0]["deprecated"], true);
        assert_eq!(values[1]["deprecated"], false);
        Ok(())
    }

    #[test]
    fn conditional_requirements_and_empty_padding_are_preserved() -> Result<()> {
        let table = TABLE
            .replace("✔", "*")
            .replace("App &amp; game</td>", "App &amp; game</td><td></td>");
        let html = page(&format!(
            "<div class=\"bb_code http\">POST https://api.steampowered.com/ITest/Write/v1/</div>{table}"
        ));
        let values = methods("ITest", &html)?;
        assert_eq!(values[0]["parameters"][0]["requirement"], "conditional");
        let changed = table.replace("<td></td>", "<td>unexpected</td>");
        assert!(methods("ITest",&page(&format!("<div class=\"bb_code http\">POST https://api.steampowered.com/ITest/Write/v1/</div>{changed}"))).is_err());
        Ok(())
    }

    #[test]
    fn html5_recovers_implicit_row_closures_without_losing_parameters() -> Result<()> {
        let table = "<table><tr><th>Name</th><th>Type</th><th>Required</th><th>Description</th></tr><tr><td>requestid</td><td>uint64</td><td>✔</td><td>request<tr><td>restriction</td><td>bool</td><td></td><td>optional</td></tr></tr></table>";
        let values = methods(
            "ITest",
            &page(&format!(
                "<div class=\"bb_code http\">POST https://partner.steam-api.com/ITest/Write/v1/</div>{table}"
            )),
        )?;
        assert_eq!(
            values[0]["parameters"]
                .as_array()
                .ok_or("missing parameters")?
                .len(),
            2
        );
        assert_eq!(values[0]["parameters"][1]["name"], "restriction");
        Ok(())
    }

    #[test]
    fn parameter_contracts_ignore_order_but_reject_missing_types_or_duplicates() -> Result<()> {
        let left = json!([{"name":"a","type":"uint32","requirement":"required"},{"name":"b","type":"string","requirement":"optional"}]);
        let right = json!([left[1], left[0]]);
        assert_eq!(parameter_contract(&left)?, parameter_contract(&right)?);
        let duplicate = json!([left[0], left[0]]);
        assert!(parameter_contract(&duplicate).is_err());
        let mut missing = left;
        missing[0]["type"] = Value::Null;
        assert!(parameter_contract(&missing).is_err());
        Ok(())
    }

    #[test]
    fn audit_detects_unbound_methods_contract_drift_and_incomplete_acceptance() -> Result<()> {
        let inventory: Value =
            serde_json::from_slice(&fs::read(root().join("docs/api-inventory.json"))?)?;
        let reference = snapshot(&inventory)?;
        let discovered: Value =
            serde_json::from_slice(&fs::read(root().join("docs/api-discovery.json"))?)?;
        let checked = report(&inventory, &reference, &discovered)?;
        assert_eq!(checked["completion"]["documented_signature_mapping"], true);
        assert_eq!(
            checked["completion"]["credentialed_production_acceptance"],
            false
        );
        assert_eq!(checked["completion"]["native_sdk_implementation"], false);
        let mut changed = reference.clone();
        changed["interfaces"][0]["methods"][0]["parameters"][0]["type"] = "uint128".into();
        assert_eq!(
            report(&inventory, &changed, &discovered)?["completion"]["reviewed_contracts_current"],
            false
        );
        let mut planned = inventory.clone();
        planned["interfaces"][0]["binding_status"] = "planned".into();
        let pending = report(&planned, &reference, &discovered)?;
        assert_eq!(pending["completion"]["documented_signature_mapping"], false);
        assert_eq!(
            pending["documented"]["bound_routes"].as_u64(),
            checked["documented"]["bound_routes"]
                .as_u64()
                .map(|count| count - 1)
        );
        Ok(())
    }

    #[test]
    fn nested_description_examples_do_not_truncate_parameter_tables() -> Result<()> {
        let table = TABLE.replace(
            "App &amp; game",
            "<div class=\"bb_code \">{example}</div> &unreviewed_prose;",
        );
        let html = page(&format!(
            "<div class=\"bb_code http\">POST https://partner.steam-api.com/ITest/Write/v0001</div>{table}"
        ));
        let parsed = methods("ITest", &html)?;
        assert_eq!(parsed[0]["path"], "ITest/Write/v0001");
        assert_eq!(
            parsed[0]["parameters"]
                .as_array()
                .ok_or("missing parameters")?
                .len(),
            1
        );
        Ok(())
    }

    #[test]
    fn index_uses_only_article_links_and_deduplicates() -> Result<()> {
        let html = page(
            "<a href=\"/doc/webapi/ITest\">A</a><a href=\"https://partner.steamgames.com/doc/webapi/ITest\">A</a>",
        );
        assert_eq!(interfaces(&html)?, BTreeSet::from(["ITest".to_owned()]));
        assert!(interfaces(&page("no links")).is_err());
        Ok(())
    }
    #[test]
    fn standard_and_cloud_examples_preserve_verbatim_versions_and_parameters() -> Result<()> {
        for code in [
            "<div class=\"bb_code http\">GET https://api.steampowered.com/ITest/Read/v001/</div>",
            "<div class=\"bb_code \">GET https://api.steampowered.com/ITest/Read/v001/?access_token=&lt;token&gt; HTTP/1.1\nHost: api.steampowered.com</div>",
        ] {
            let values = methods("ITest", &page(&format!("{code}{TABLE}")))?;
            assert_eq!(values.len(), 1);
            assert_eq!(values[0]["path"], "ITest/Read/v001/");
            assert_eq!(
                values[0]["parameters"][0],
                json!({"name":"appid","type":"uint32","requirement":"required"})
            );
        }
        Ok(())
    }
    #[test]
    fn missing_malformed_or_duplicate_reference_contracts_fail_closed() {
        assert!(methods("ITest", &page("no methods")).is_err());
        let method = format!(
            "<div class=\"bb_code http\">POST https://partner.steam-api.com/ITest/Write/v1/</div>{TABLE}"
        );
        assert!(methods("ITest", &page(&method.replace("Required", "Optional"))).is_err());
        assert!(methods("ITest", &page(&format!("{method}{method}"))).is_err());
        assert!(
            methods(
                "ITest",
                &page(&method.replace("partner.steam-api.com", "example.com"))
            )
            .is_err()
        );
        assert!(methods("ITest", &page(&method.replace("✔", "yes"))).is_err());
        assert!(
            methods(
                "ITest",
                &page(&method.replace("<tr><td>", "<tr><td></td><td>"))
            )
            .is_err()
        );
    }
    #[test]
    fn sandbox_and_non_signature_examples_are_explicit_exceptions() -> Result<()> {
        assert!(methods("ISteamMicroTxnSandbox", &page("unrelated page")).is_err());
        assert!(
            methods(
                "ISteamMicroTxnSandbox",
                &page("ISteamMicroTxn dummy database")
            )?
            .is_empty()
        );
        let html = page(&format!(
            "<div class=\"bb_code http\">&amp;files[0]=save</div><div class=\"bb_code http\">GET https://api.steampowered.com/ITest/Read/v1/</div>{TABLE}"
        ));
        assert_eq!(methods("ITest", &html)?.len(), 1);
        Ok(())
    }
    #[test]
    fn discovery_does_not_infer_authentication_or_drop_parameter_shapes() -> Result<()> {
        let value = json!({"apilist":{"interfaces":[{"name":"IExtra","methods":[{"name":"Read","version":1,"httpmethod":"GET","parameters":[{"name":"id","type":"uint64","optional":false}]}]}]}});
        let parsed = discovery(&value)?;
        assert_eq!(parsed.len(), 1);
        assert_eq!(
            parsed.values().next().ok_or("missing method")?["parameters"][0]["type"],
            "uint64"
        );
        let mut bad = value.clone();
        bad["apilist"]["interfaces"][0]["methods"][0]["httpmethod"] = "PUT".into();
        assert!(discovery(&bad).is_err());
        let mut bad = value;
        bad["apilist"]["interfaces"][0]["methods"][0]["parameters"][0]["optional"] = Value::Null;
        assert!(discovery(&bad).is_err());
        Ok(())
    }
    #[test]
    fn dom_text_and_return_types_keep_only_supported_shapes() -> Result<()> {
        assert_eq!(
            content(Html::parse_fragment("<b>name</b> &amp; &#x2714; &nbsp; value").root_element()),
            "name & ✔ value"
        );

        assert_eq!(
            return_type(
                "pub async fn read(&self) -> Result<Vec<u64>> { body }",
                "read"
            )?,
            "Result<Vec<u64>>"
        );
        assert!(return_type("pub async fn read_extra(&self) -> Result<()> { }", "read").is_err());
        Ok(())
    }
}
