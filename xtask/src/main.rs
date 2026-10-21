// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

mod dependencies;
mod release;
mod repository;

use rustls_platform_verifier::BuilderVerifierExt;
use serde_json::Value;
use std::{collections::BTreeMap, path::Path, process::Command, sync::Arc, time::Duration};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() {
    if let Err(error) = run() {
        eprintln!("xtask: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    std::env::set_current_dir(root())?;
    let mut args = std::env::args().skip(1);
    let command = args
        .next()
        .ok_or("expected check-repository, coverage, check-dependencies, or release")?;
    let rest: Vec<_> = args.collect();
    match command.as_str() {
        "check-repository" if rest.is_empty() => repository::check(),
        "coverage" => repository::coverage(&rest),
        "check-dependencies" => dependencies::check(&rest),
        "release" => release::run(&rest),
        _ => Err(format!("unknown command or arguments: {command}").into()),
    }
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or(Path::new("."))
}

fn output(program: &str, args: &[&str]) -> Result<String> {
    let result = Command::new(program)
        .args(args)
        .current_dir(root())
        .output()?;
    if !result.status.success() {
        return Err(format!(
            "{program} {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&result.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(result.stdout)?)
}

fn metadata() -> Result<Value> {
    Ok(serde_json::from_str(&output(
        "cargo",
        &["metadata", "--locked", "--no-deps", "--format-version", "1"],
    )?)?)
}

fn package(metadata: &Value) -> Result<&Value> {
    metadata["packages"]
        .as_array()
        .and_then(|packages| packages.iter().find(|p| p["name"] == "steam-api-web"))
        .ok_or_else(|| "steam-api-web package metadata is missing".into())
}

fn flags(args: &[String], permitted: &[&str]) -> Result<BTreeMap<String, String>> {
    let mut values = BTreeMap::new();
    let mut args = args.iter();
    while let Some(name) = args.next() {
        let key = name
            .strip_prefix("--")
            .filter(|key| permitted.contains(key))
            .ok_or_else(|| format!("unknown option {name}"))?;
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {name}"))?;
        if values.insert(key.to_owned(), value.to_owned()).is_some() {
            return Err(format!("duplicate option {name}").into());
        }
    }
    Ok(values)
}

fn required<'a>(values: &'a BTreeMap<String, String>, name: &str) -> Result<&'a str> {
    values
        .get(name)
        .map(String::as_str)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| format!("missing --{name}").into())
}

fn http_client() -> Result<reqwest::blocking::Client> {
    let tls = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_safe_default_protocol_versions()?
    .with_platform_verifier()?
    .with_no_client_auth();
    Ok(reqwest::blocking::Client::builder()
        .tls_backend_preconfigured(tls)
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(45))
        .user_agent("steam-api-web-xtask/0.2")
        .build()?)
}
