// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

use crate::{Result, output, root};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

fn has_han(value: &str) -> bool {
    value
        .chars()
        .any(|c| ('\u{3400}'..='\u{9fff}').contains(&c))
}

fn local_link(target: &str) -> Result<Option<String>> {
    let target = target.trim_matches(['<', '>']);
    let path = target.split(['#', '?']).next().unwrap_or("");
    if path.is_empty() || path.contains(':') {
        return Ok(None);
    }
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = path
                .get(i + 1..i + 3)
                .ok_or("truncated percent escape in Markdown link")?;
            decoded.push(u8::from_str_radix(hex, 16)?);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    Ok(Some(String::from_utf8(decoded)?))
}

fn check_markdown(path: &Path, text: &str, errors: &mut Vec<String>) -> Result<()> {
    let relative = path.strip_prefix(root())?.display();
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or("invalid Markdown filename")?;
    let peer = if let Some(stem) = stem.strip_suffix("_ZH") {
        format!("{stem}.md")
    } else {
        format!("{stem}_ZH.md")
    };
    if !path.with_file_name(peer).is_file() {
        errors.push(format!("{relative}: missing bilingual peer"));
    }
    if !text.matches("```").count().is_multiple_of(2) {
        errors.push(format!("{relative}: unbalanced code fences"));
    }
    // Repository links use ordinary inline Markdown syntax; remote links and anchors are skipped.
    let mut rest = text;
    while let Some((_, after)) = rest.split_once("](") {
        let Some((target, tail)) = after.split_once(')') else {
            break;
        };
        if let Some(target) = local_link(target)?
            && !path.parent().unwrap_or(root()).join(&target).exists()
        {
            errors.push(format!("{relative}: missing link target {target}"));
        }
        rest = tail;
    }
    let mut in_code = false;
    for line in text.lines() {
        if line.starts_with("```") {
            in_code = !in_code;
        } else if in_code
            && has_han(line)
            && (line.trim_start().starts_with("//") || line.trim_start().starts_with('#'))
        {
            errors.push(format!("{relative}: code example comments must be English"));
        }
    }
    Ok(())
}

#[derive(Debug, Default, PartialEq, Eq)]
struct BindingCoverage {
    total_interfaces: u64,
    total_methods: u64,
    total_explicit_methods: u64,
    total_inherited_methods: u64,
    bound_interfaces: u64,
    bound_methods: u64,
    bound_explicit_methods: u64,
    bound_inherited_methods: u64,
    remaining: Vec<String>,
}

fn required_text<'a>(value: &'a Value, field: &str) -> Result<&'a str> {
    value[field]
        .as_str()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("missing inventory field {field}").into())
}

fn rust_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_lowercase() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn binding_coverage(inventory: &Value) -> Result<BindingCoverage> {
    let interfaces = inventory["interfaces"]
        .as_array()
        .ok_or("missing interfaces")?;
    let mut coverage = BindingCoverage::default();
    let mut interface_names = BTreeSet::new();
    for interface in interfaces {
        let name = required_text(interface, "interface")?;
        if !interface_names.insert(name) {
            return Err(format!("duplicate inventory interface {name}").into());
        }
        let bound = match required_text(interface, "binding_status")? {
            "bound_contract_tests_only" => true,
            "planned" => false,
            status => return Err(format!("unknown binding status {status} for {name}").into()),
        };
        coverage.total_interfaces += 1;
        coverage.bound_interfaces += u64::from(bound);
        if bound && !rust_identifier(required_text(interface, "rust_module")?) {
            return Err(format!("invalid bound Rust module for {name}").into());
        }
        let methods = interface["methods"]
            .as_array()
            .filter(|methods| !methods.is_empty())
            .ok_or_else(|| format!("{name} has no method signatures"))?;
        let mut signatures = BTreeSet::new();
        for method in methods {
            let method_name = required_text(method, "name")?;
            let verb = required_text(method, "http_method")?;
            if !matches!(verb, "GET" | "POST") {
                return Err(format!("unsupported HTTP method for {name}/{method_name}").into());
            }
            let version = method["version"]
                .as_u64()
                .filter(|version| *version > 0)
                .ok_or("method version must be a positive integer")?;
            if !signatures.insert((method_name, version, verb)) {
                return Err(format!("duplicate inventory signature {name}/{method_name}").into());
            }
            let path = required_text(method, "path")?;
            let prefix = format!("{name}/{method_name}/v");
            let path_version = path
                .strip_prefix(&prefix)
                .map(|suffix| suffix.strip_suffix('/').unwrap_or(suffix))
                .filter(|suffix| !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()))
                .and_then(|suffix| suffix.parse::<u64>().ok());
            if path_version != Some(version) {
                return Err(format!("path does not match signature {name}/{method_name}").into());
            }
            if bound && !rust_identifier(required_text(method, "rust_method")?) {
                return Err(format!("invalid bound Rust method for {name}/{method_name}").into());
            }
            let inherited = method
                .get("inherited_from")
                .is_some_and(|value| !value.is_null());
            if inherited {
                let parent_name = required_text(method, "inherited_from")?;
                let parent = interfaces
                    .iter()
                    .find(|candidate| candidate["interface"] == parent_name)
                    .filter(|_| parent_name != name)
                    .ok_or_else(|| {
                        format!("missing inheritance source for {name}/{method_name}")
                    })?;
                let original = parent["methods"]
                    .as_array()
                    .and_then(|methods| {
                        methods.iter().find(|original| {
                            original["name"] == method["name"]
                                && original["version"] == method["version"]
                                && original["http_method"] == method["http_method"]
                                && original.get("inherited_from").is_none_or(Value::is_null)
                        })
                    })
                    .ok_or_else(|| {
                        format!("inherited signature differs for {name}/{method_name}")
                    })?;
                if bound
                    && (parent["rust_module"] != interface["rust_module"]
                        || original["rust_method"] != method["rust_method"])
                {
                    return Err(
                        format!("inherited Rust mapping differs for {name}/{method_name}").into(),
                    );
                }
            }
            coverage.total_methods += 1;
            coverage.total_explicit_methods += u64::from(!inherited);
            coverage.total_inherited_methods += u64::from(inherited);
            if bound {
                coverage.bound_methods += 1;
                coverage.bound_explicit_methods += u64::from(!inherited);
                coverage.bound_inherited_methods += u64::from(inherited);
            } else {
                coverage.remaining.push(format!("{verb} {path}"));
            }
        }
    }
    for (field, count) in [
        ("explicit_signatures", coverage.total_explicit_methods),
        (
            "inherited_sandbox_signatures",
            coverage.total_inherited_methods,
        ),
        ("bound_interfaces", coverage.bound_interfaces),
        ("bound_methods", coverage.bound_methods),
        ("bound_explicit_methods", coverage.bound_explicit_methods),
        ("bound_inherited_methods", coverage.bound_inherited_methods),
    ] {
        if inventory["_metadata"][field].as_u64() != Some(count) {
            return Err(format!("inventory total {field} disagrees with its signatures").into());
        }
    }
    Ok(coverage)
}

fn check_binding_sources(
    inventory: &Value,
    exports: &str,
    mut source: impl FnMut(&str) -> Result<String>,
) -> Result<()> {
    let mut errors = Vec::new();
    let mut sources = BTreeMap::new();
    for interface in inventory["interfaces"]
        .as_array()
        .ok_or("missing interfaces")?
    {
        if interface["binding_status"] != "bound_contract_tests_only" {
            continue;
        }
        let module = required_text(interface, "rust_module")?;
        let export = format!("pub mod {module};");
        if !exports.lines().any(|line| line.trim() == export) {
            errors.push(format!("bound module {module} is not publicly exported"));
        }
        let source = match sources.entry(module) {
            std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::btree_map::Entry::Vacant(entry) => entry.insert(source(module)?),
        };
        for method in interface["methods"].as_array().ok_or("missing methods")? {
            let name = required_text(method, "rust_method")?;
            let declaration = format!("pub async fn {name}(");
            if !source
                .lines()
                .any(|line| line.trim_start().starts_with(&declaration))
            {
                errors.push(format!("bound method {module}::{name} is absent"));
            }
            let path = required_text(method, "path")?;
            if !source.contains(&format!("\"{path}\"")) {
                errors.push(format!("bound route {path} is absent from {module}"));
            }
        }
    }
    if !errors.is_empty() {
        return Err(errors.join("\n").into());
    }
    Ok(())
}

fn inventory() -> Result<Value> {
    Ok(serde_json::from_str(&fs::read_to_string(
        root().join("docs/api-inventory.json"),
    )?)?)
}

fn check_sources(inventory: &Value) -> Result<()> {
    check_binding_sources(
        inventory,
        &fs::read_to_string(root().join("src/lib.rs"))?,
        |module| Ok(fs::read_to_string(root().join(format!("src/{module}.rs")))?),
    )
}

impl BindingCoverage {
    fn json(&self, inventory: &Value) -> Value {
        serde_json::json!({
            "_metadata": {
                "copyright": "Copyright 2026 steam-api-web contributors.",
                "license": "Apache-2.0",
                "license_url": "https://www.apache.org/licenses/LICENSE-2.0",
                "SPDX-License-Identifier": "Apache-2.0",
            },
            "scope": "protocol_bindings",
            "binding_version": inventory["_metadata"]["binding_version"],
            "validation": "Static source mapping coverage only; contract tests, live permissions, and production behavior require separate validation.",
            "interfaces": {"total": self.total_interfaces, "bound": self.bound_interfaces},
            "method_routes": {"total": self.total_methods, "bound": self.bound_methods},
            "explicit_signatures": {"total": self.total_explicit_methods, "bound": self.bound_explicit_methods},
            "inherited_routes": {"total": self.total_inherited_methods, "bound": self.bound_inherited_methods},
            "remaining": self.remaining,
        })
    }
}

pub(crate) fn coverage(args: &[String]) -> Result<()> {
    if !args.is_empty() && args != ["--json"] {
        return Err("coverage accepts only --json".into());
    }
    let inventory = inventory()?;
    let coverage = binding_coverage(&inventory)?;
    check_sources(&inventory)?;
    if args == ["--json"] {
        println!(
            "{}",
            serde_json::to_string_pretty(&coverage.json(&inventory))?
        );
    } else {
        println!(
            "Protocol binding coverage {}: {}/{} interfaces, {}/{} method routes (explicit {}/{}, inherited {}/{})",
            inventory["_metadata"]["binding_version"]
                .as_str()
                .unwrap_or("unknown"),
            coverage.bound_interfaces,
            coverage.total_interfaces,
            coverage.bound_methods,
            coverage.total_methods,
            coverage.bound_explicit_methods,
            coverage.total_explicit_methods,
            coverage.bound_inherited_methods,
            coverage.total_inherited_methods,
        );
        if coverage.remaining.is_empty() {
            println!("Remaining documented method routes: none");
        } else {
            println!("Remaining documented method routes:");
            for method in &coverage.remaining {
                println!("  {method}");
            }
        }
        println!(
            "Static source mapping coverage only; contract tests, live permissions, and production behavior require separate validation."
        );
    }
    Ok(())
}

pub(crate) fn check() -> Result<()> {
    let mut errors = Vec::new();
    let names = output(
        "git",
        &[
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ],
    )?;
    let names: BTreeSet<_> = names.split('\0').filter(|name| !name.is_empty()).collect();
    for name in names {
        let path = root().join(name);
        if !path.is_file() {
            continue;
        }
        if path.extension().is_some_and(|ext| ext == "py") {
            errors.push(format!("{name}: migrate Python maintenance code to xtask"));
        }
        let text = fs::read_to_string(&path)?;
        let mut body = text.as_str();
        if body.starts_with("#!") {
            body = body.split_once('\n').map_or("", |(_, body)| body);
        }
        const CARGO_BANNER: &str = "# This file is automatically @generated by Cargo.\n# It is not intended for manual editing.\n";
        if path.file_name().is_some_and(|name| name == "Cargo.lock") {
            body = body
                .strip_prefix(CARGO_BANNER)
                .unwrap_or(body)
                .trim_start_matches('\n');
        }
        let prefix = body.lines().take(6).collect::<Vec<_>>().join("\n");
        let legal_start = [
            "// Copyright 2026 steam-api-web contributors.",
            "# Copyright 2026 steam-api-web contributors.",
            "<!--\nCopyright 2026 steam-api-web contributors.",
            "Copyright 2026 steam-api-web contributors.",
        ];
        if path.extension().is_some_and(|ext| ext == "json") {
            let data: Value = serde_json::from_str(&text)?;
            let first_key = text
                .trim_start()
                .strip_prefix('{')
                .unwrap_or("")
                .trim_start();
            if !first_key.starts_with("\"_metadata\"")
                || data["_metadata"]["copyright"]
                    .as_str()
                    .is_none_or(str::is_empty)
                || data["_metadata"]["license"] != "Apache-2.0"
            {
                errors.push(format!("{name}: JSON must start with legal _metadata"));
            }
        } else if !legal_start.iter().any(|prefix| body.starts_with(*prefix))
            || !prefix.contains("SPDX-License-Identifier: Apache-2.0")
        {
            errors.push(format!("{name}: missing leading copyright/SPDX header"));
        }
        if path.extension().is_some_and(|ext| ext == "rs")
            && text.lines().any(|line| {
                let line = line.trim_start();
                has_han(line)
                    && (line.starts_with("//") || line.starts_with("/*") || line.starts_with('*'))
            })
        {
            errors.push(format!("{name}: Rust comments must be English"));
        }
        if path.extension().is_some_and(|ext| ext == "md") {
            check_markdown(&path, &text, &mut errors)?;
        }
    }
    let inventory = inventory()?;
    let coverage = binding_coverage(&inventory)?;
    check_sources(&inventory)?;
    let package_metadata = crate::metadata()?;
    if inventory["_metadata"]["binding_version"] != crate::package(&package_metadata)?["version"] {
        errors.push("binding inventory version must match the public crate".to_owned());
    }
    let interfaces = inventory["interfaces"]
        .as_array()
        .ok_or("missing interface inventory")?;
    let unique: BTreeSet<_> = interfaces
        .iter()
        .filter_map(|i| i["interface"].as_str())
        .collect();
    if interfaces.len() != 33 || unique.len() != 33 {
        errors.push("reference inventory must cover all 33 reviewed interfaces".to_owned());
    }
    for language in ["", "_ZH"] {
        let roadmap = fs::read_to_string(root().join(format!("docs/roadmap{language}.md")))?;
        for interface in &unique {
            if !roadmap.contains(*interface) {
                errors.push(format!("roadmap{language} missing interface {interface}"));
            }
        }
        for module in inventory["_metadata"]["sdk_reference"]["source_modules"]
            .as_array()
            .ok_or("missing SDK module inventory")?
        {
            let module = module.as_str().ok_or("invalid SDK module")?;
            if !roadmap.contains(module.strip_prefix("src/").unwrap_or(module)) {
                errors.push(format!("roadmap{language} missing SDK module {module}"));
            }
        }
    }
    let graph = output(
        "cargo",
        &[
            "tree",
            "--locked",
            "--package",
            "steam-api-web",
            "--edges",
            "normal",
            "--prefix",
            "none",
        ],
    )?;
    if !graph.lines().any(|line| line.starts_with("aws-lc-rs v"))
        || graph
            .lines()
            .any(|line| line.starts_with("ring v") || line.starts_with("native-tls v"))
    {
        errors.push("normal TLS graph must use AWS-LC without ring/native-tls".to_owned());
    }
    if !errors.is_empty() {
        return Err(errors.join("\n").into());
    }
    println!(
        "Repository checks passed: legal headers, bilingual links, {} interfaces ({} bound, {} method routes), Rust mappings, SDK modules, no Python and AWS-LC graph",
        coverage.total_interfaces, coverage.bound_interfaces, coverage.bound_methods,
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coverage_fixture() -> Value {
        serde_json::json!({
            "_metadata": {
                "binding_version": "0.5.0",
                "explicit_signatures": 3,
                "inherited_sandbox_signatures": 1,
                "bound_interfaces": 2,
                "bound_methods": 3,
                "bound_explicit_methods": 2,
                "bound_inherited_methods": 1,
            },
            "interfaces": [
                {
                    "interface": "IPrimary", "binding_status": "bound_contract_tests_only", "rust_module": "primary",
                    "methods": [
                        {"name": "Read", "version": 1, "http_method": "GET", "rust_method": "read", "path": "IPrimary/Read/v1/"},
                        {"name": "Write", "version": 1, "http_method": "POST", "rust_method": "write", "path": "IPrimary/Write/v1/"},
                    ],
                },
                {
                    "interface": "ISandbox", "binding_status": "bound_contract_tests_only", "rust_module": "primary",
                    "methods": [
                        {"name": "Read", "version": 1, "http_method": "GET", "rust_method": "read", "path": "ISandbox/Read/v1/", "inherited_from": "IPrimary"},
                    ],
                },
                {
                    "interface": "IPlanned", "binding_status": "planned",
                    "methods": [{"name": "Poll", "version": 1, "http_method": "GET", "path": "IPlanned/Poll/v1/"}],
                },
            ],
        })
    }

    fn fixture_source() -> String {
        "impl Primary {\n    pub async fn read(&self) {}\n    pub async fn write(&self) {}\n}\nconst READ: &str = \"IPrimary/Read/v1/\";\nconst WRITE: &str = \"IPrimary/Write/v1/\";\nconst SANDBOX: &str = \"ISandbox/Read/v1/\";\n".to_owned()
    }

    #[test]
    fn coverage_separates_explicit_signatures_inherited_routes_and_remaining() -> Result<()> {
        let inventory = coverage_fixture();
        let coverage = binding_coverage(&inventory)?;
        assert_eq!(coverage.total_interfaces, 3);
        assert_eq!(coverage.total_methods, 4);
        assert_eq!(coverage.total_explicit_methods, 3);
        assert_eq!(coverage.total_inherited_methods, 1);
        assert_eq!(coverage.bound_interfaces, 2);
        assert_eq!(coverage.bound_methods, 3);
        assert_eq!(coverage.bound_explicit_methods, 2);
        assert_eq!(coverage.bound_inherited_methods, 1);
        assert_eq!(coverage.remaining, ["GET IPlanned/Poll/v1/"]);
        let report = coverage.json(&inventory);
        assert_eq!(
            report["_metadata"]["copyright"],
            "Copyright 2026 steam-api-web contributors."
        );
        assert_eq!(report["_metadata"]["license"], "Apache-2.0");
        assert_eq!(
            report["_metadata"]["license_url"],
            "https://www.apache.org/licenses/LICENSE-2.0"
        );
        assert_eq!(report["_metadata"]["SPDX-License-Identifier"], "Apache-2.0");
        assert!(serde_json::to_string_pretty(&report)?.starts_with("{\n  \"_metadata\": {"));
        assert_eq!(report["scope"], "protocol_bindings");
        assert_eq!(report["method_routes"]["total"], 4);
        assert_eq!(report["remaining"][0], "GET IPlanned/Poll/v1/");
        Ok(())
    }

    #[test]
    fn coverage_cannot_inflate_totals_or_omit_method_signatures() {
        for field in [
            "explicit_signatures",
            "inherited_sandbox_signatures",
            "bound_interfaces",
            "bound_methods",
            "bound_explicit_methods",
            "bound_inherited_methods",
        ] {
            let mut inventory = coverage_fixture();
            inventory["_metadata"][field] = serde_json::json!(99);
            assert!(binding_coverage(&inventory).is_err(), "{field}");
        }
        let mut inventory = coverage_fixture();
        inventory["interfaces"][0]["methods"] = Value::Null;
        assert!(binding_coverage(&inventory).is_err());
    }

    #[test]
    fn coverage_rejects_missing_mappings_duplicate_signatures_and_unknown_status() {
        let mut inventory = coverage_fixture();
        inventory["interfaces"][0]["methods"][0]["rust_method"] = Value::Null;
        assert!(binding_coverage(&inventory).is_err());
        let mut inventory = coverage_fixture();
        inventory["interfaces"][1]["interface"] = serde_json::json!("IPrimary");
        assert!(binding_coverage(&inventory).is_err());
        let mut inventory = coverage_fixture();
        inventory["interfaces"][0]["methods"][1] = inventory["interfaces"][0]["methods"][0].clone();
        assert!(binding_coverage(&inventory).is_err());
        let mut inventory = coverage_fixture();
        inventory["interfaces"][0]["binding_status"] = serde_json::json!("unknown");
        assert!(binding_coverage(&inventory).is_err());
    }

    #[test]
    fn inherited_routes_require_the_same_explicit_signature_and_rust_mapping() {
        let mut inventory = coverage_fixture();
        inventory["interfaces"][1]["methods"][0]["version"] = serde_json::json!(2);
        inventory["interfaces"][1]["methods"][0]["path"] = serde_json::json!("ISandbox/Read/v2/");
        assert!(binding_coverage(&inventory).is_err());
        let mut inventory = coverage_fixture();
        inventory["interfaces"][1]["methods"][0]["rust_method"] = serde_json::json!("write");
        assert!(binding_coverage(&inventory).is_err());
        let mut inventory = coverage_fixture();
        inventory["interfaces"][1]["methods"][0]["inherited_from"] = serde_json::json!("ISandbox");
        assert!(binding_coverage(&inventory).is_err());
    }

    #[test]
    fn source_checks_require_public_exports_async_methods_and_exact_paths() -> Result<()> {
        let inventory = coverage_fixture();
        check_binding_sources(&inventory, "pub mod primary;", |_| Ok(fixture_source()))?;
        assert!(
            check_binding_sources(&inventory, "mod primary;", |_| Ok(fixture_source())).is_err()
        );
        assert!(
            check_binding_sources(&inventory, "pub mod primary;", |_| {
                Ok(fixture_source().replace("pub async fn read(", "pub async fn read_extra("))
            })
            .is_err()
        );
        assert!(
            check_binding_sources(&inventory, "pub mod primary;", |_| {
                Ok(fixture_source().replace("ISandbox/Read/v1/", "IPrimary/Read/v1/"))
            })
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn verbatim_version_spelling_is_preserved_without_inflating_signatures() -> Result<()> {
        let mut inventory = coverage_fixture();
        inventory["interfaces"][0]["methods"][0]["path"] = serde_json::json!("IPrimary/Read/v001/");
        assert_eq!(binding_coverage(&inventory)?.bound_methods, 3);
        assert!(
            check_binding_sources(&inventory, "pub mod primary;", |_| Ok(fixture_source()))
                .is_err()
        );
        check_binding_sources(&inventory, "pub mod primary;", |_| {
            Ok(fixture_source().replace("IPrimary/Read/v1/", "IPrimary/Read/v001/"))
        })?;
        inventory["interfaces"][0]["methods"][0]["path"] = serde_json::json!("IPrimary/Read/v2/");
        assert!(binding_coverage(&inventory).is_err());
        Ok(())
    }

    #[test]
    fn inherited_routes_reuse_one_source_read_for_the_shared_module() -> Result<()> {
        let mut reads = 0;
        check_binding_sources(&coverage_fixture(), "pub mod primary;", |_| {
            reads += 1;
            Ok(fixture_source())
        })?;
        assert_eq!(reads, 1);
        Ok(())
    }

    #[test]
    fn local_links_decode_paths_and_ignore_remote_urls_and_fragments() -> Result<()> {
        assert_eq!(
            local_link("<docs/a%20b.md#heading>")?,
            Some("docs/a b.md".to_owned())
        );
        assert_eq!(local_link("https://example.com/a")?, None);
        assert_eq!(local_link("#heading")?, None);
        assert!(local_link("docs/invalid%").is_err());
        Ok(())
    }
}
