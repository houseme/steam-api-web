// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

use crate::{Result, flags, http_client, metadata, output, package, required, root};
use serde_json::Value;
use std::{cmp::Ordering, fmt::Write as _, fs, io::Write as _, path::Path, time::Duration};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Version {
    core: [u64; 3],
    prerelease: Vec<Identifier>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Identifier {
    Numeric(u64),
    Text(String),
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.core.cmp(&other.core).then_with(|| {
            match (self.prerelease.is_empty(), other.prerelease.is_empty()) {
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                _ => self.prerelease.cmp(&other.prerelease),
            }
        })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub(crate) fn validate_version(value: &str) -> Result<Version> {
    let (core, prerelease) = value.split_once('-').unwrap_or((value, ""));
    let numbers: Vec<_> = core.split('.').collect();
    if numbers.len() != 3 {
        return Err("expected MAJOR.MINOR.PATCH with an optional prerelease".into());
    }
    let mut parsed = [0; 3];
    for (number, target) in numbers.into_iter().zip(&mut parsed) {
        if number.is_empty()
            || !number.bytes().all(|b| b.is_ascii_digit())
            || (number.len() > 1 && number.starts_with('0'))
        {
            return Err("release version components must be decimal without leading zeros".into());
        }
        *target = number.parse()?;
    }
    if value.contains('-') && prerelease.is_empty() {
        return Err("empty prerelease identifier".into());
    }
    let mut identifiers = Vec::new();
    if !prerelease.is_empty() {
        for part in prerelease.split('.') {
            if part.is_empty() || !part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
                return Err("invalid prerelease identifier".into());
            }
            identifiers.push(if part.bytes().all(|b| b.is_ascii_digit()) {
                if part.len() > 1 && part.starts_with('0') {
                    return Err("numeric prerelease identifiers must not have leading zeros".into());
                }
                Identifier::Numeric(part.parse()?)
            } else {
                Identifier::Text(part.to_owned())
            });
        }
    }
    Ok(Version {
        core: parsed,
        prerelease: identifiers,
    })
}

fn changelog_section<'a>(text: &'a str, version: &str) -> Result<&'a str> {
    let heading = format!("## [{version}]");
    let mut begin = None;
    let mut end = text.len();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        if line.starts_with("## [") {
            if begin.is_some() {
                end = offset;
                break;
            }
            if line
                .strip_prefix(&heading)
                .is_some_and(|suffix| suffix.is_empty() || suffix.starts_with([' ', '\r', '\n']))
            {
                begin = Some(offset + line.len());
            }
        }
        offset += line.len();
    }
    let notes = begin
        .map(|start| text[start..end].trim())
        .filter(|notes| !notes.is_empty())
        .ok_or_else(|| format!("missing nonempty changelog section for {version}"))?;
    if !notes.lines().any(|line| {
        line.strip_prefix("- ")
            .or_else(|| line.strip_prefix("* "))
            .is_some_and(|body| !body.trim().is_empty())
    }) {
        return Err("release changelog must contain concrete bullet entries".into());
    }
    Ok(notes)
}

fn verify_release_version<'a>(
    package: &Value,
    version: &str,
    english: &'a str,
    chinese: &str,
) -> Result<&'a str> {
    validate_version(version)?;
    if package["name"] != "steam-api-web" || package["version"] != version {
        return Err("release version does not match the steam-api-web manifest".into());
    }
    if package["license"] != "Apache-2.0" || package["edition"] != "2024" {
        return Err("expected Apache-2.0 and Rust edition 2024".into());
    }
    let notes = changelog_section(english, version)?;
    changelog_section(chinese, version)?;
    Ok(notes)
}

fn release_version<'a>(
    event: &str,
    reference: &'a str,
    requested: &'a str,
    manifest: &'a str,
    dry_run: bool,
) -> Result<&'a str> {
    let version = match event {
        "push" if !dry_run => reference
            .strip_prefix('v')
            .ok_or("release pushes must use v<VERSION> tags")?,
        "push" => return Err("tag releases cannot be silently changed into dry runs".into()),
        "workflow_dispatch" => {
            if requested.is_empty() {
                manifest
            } else {
                requested
            }
        }
        _ => return Err("expected push or workflow_dispatch event".into()),
    };
    validate_version(version)?;
    Ok(version)
}

fn check_revision(dry_run: bool, revision: &str, tagged: Option<&str>) -> Result<()> {
    if revision.len() != 40 || !revision.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("expected a full Git commit revision".into());
    }
    if !dry_run && tagged != Some(revision) {
        return Err("publishing requires checking out the exact version tag".into());
    }
    Ok(())
}

fn archive_name(path: &Path) -> Result<&str> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("invalid archive filename")?;
    let version = name
        .strip_prefix("steam-api-web-")
        .and_then(|s| s.strip_suffix(".crate"))
        .ok_or("unexpected crate archive name")?;
    validate_version(version)?;
    Ok(name)
}

fn sha256(bytes: &[u8]) -> Result<String> {
    let digest = aws_lc_rs::digest::digest(&aws_lc_rs::digest::SHA256, bytes);
    let mut result = String::with_capacity(64);
    for byte in digest.as_ref() {
        write!(&mut result, "{byte:02x}")?;
    }
    Ok(result)
}

fn verify_checksum(name: &str, bytes: &[u8], checksums: &str) -> Result<()> {
    let lines: Vec<_> = checksums.lines().collect();
    if lines.len() != 1 {
        return Err("expected exactly one archive checksum".into());
    }
    let (digest, recorded_name) = lines[0]
        .split_once("  ")
        .ok_or("invalid SHA256SUMS syntax")?;
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || recorded_name != name
    {
        return Err("checksum manifest does not name this archive".into());
    }
    if sha256(bytes)? != digest {
        return Err("crate archive checksum mismatch".into());
    }
    Ok(())
}

fn verify_registry_metadata(
    metadata: &Value,
    package: &str,
    version: &str,
    checksum: &str,
) -> Result<()> {
    let value = &metadata["version"];
    if value["crate"] != package || value["num"] != version {
        return Err("registry returned a different package or version".into());
    }
    if value["yanked"] != false || value["checksum"] != checksum {
        return Err(
            "published version is yanked or its checksum differs from the validated archive".into(),
        );
    }
    Ok(())
}

fn registry_body(reader: impl std::io::Read) -> Result<Option<Value>> {
    match serde_json::from_reader(reader) {
        Ok(metadata) => Ok(Some(metadata)),
        // A successful status does not guarantee that the metadata body arrived intact.
        // Keep resets, read timeouts and truncated JSON inside the same bounded retry budget.
        Err(error) if error.is_io() || error.is_eof() => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn poll_registry(
    mut fetch: impl FnMut() -> Result<Option<Value>>,
    mut pause: impl FnMut(),
    attempts: usize,
    version: &str,
    checksum: &str,
) -> Result<()> {
    for attempt in 0..attempts {
        if let Some(metadata) = fetch()? {
            return verify_registry_metadata(&metadata, "steam-api-web", version, checksum);
        }
        if attempt + 1 < attempts {
            pause();
        }
    }
    Err("crates.io did not expose the validated archive before the retry budget expired".into())
}

pub(crate) fn check_package_files<'a>(files: impl IntoIterator<Item = &'a str>) -> Result<usize> {
    let mut files: Vec<_> = files.into_iter().collect();
    files.sort_unstable();
    if files.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("package contains duplicate paths".into());
    }
    for required in [
        "Cargo.toml",
        "Cargo.lock",
        "LICENSE",
        "README.md",
        "README_ZH.md",
        "CHANGELOG.md",
        "CHANGELOG_ZH.md",
        "src/lib.rs",
    ] {
        if !files.contains(&required) {
            return Err(format!("package is missing {required}").into());
        }
    }
    for path in &files {
        let allowed = matches!(
            *path,
            ".cargo_vcs_info.json"
                | "Cargo.toml"
                | "Cargo.toml.orig"
                | "Cargo.lock"
                | "LICENSE"
                | "README.md"
                | "README_ZH.md"
                | "CHANGELOG.md"
                | "CHANGELOG_ZH.md"
        ) || ((path.starts_with("src/") || path.starts_with("examples/"))
            && path.ends_with(".rs"));
        if !allowed
            || path
                .split('/')
                .any(|part| part == ".." || part == "." || part.is_empty())
        {
            return Err(format!("repository-only or unexpected content in crate: {path}").into());
        }
    }
    Ok(files.len())
}

fn check_normalized_manifest(manifest: &str) -> Result<()> {
    // Cargo normalizes explicit targets into individual table headers in the distributable manifest.
    if manifest
        .lines()
        .any(|line| matches!(line.trim(), "[[test]]" | "[[bench]]" | "[workspace]"))
    {
        return Err("published manifest must not retain repository tests, benchmarks or workspace configuration".into());
    }
    Ok(())
}

pub(crate) fn run(args: &[String]) -> Result<()> {
    let (command, args) = args.split_first().ok_or("expected release subcommand")?;
    match command.as_str() {
        "check" | "prepare" => {
            let permitted = if command == "prepare" {
                &["version", "notes", "event", "ref", "dry-run", "output"][..]
            } else {
                &["version", "notes"][..]
            };
            let values = flags(args, permitted)?;
            let metadata = metadata()?;
            let package = package(&metadata)?;
            let manifest_version = package["version"]
                .as_str()
                .ok_or("manifest has no version")?;
            let requested = values.get("version").map_or("", String::as_str);
            let dry_run = if command == "prepare" {
                match required(&values, "dry-run")? {
                    "true" => true,
                    "false" => false,
                    _ => return Err("--dry-run must be true or false".into()),
                }
            } else {
                true
            };
            let version = if command == "prepare" {
                release_version(
                    required(&values, "event")?,
                    required(&values, "ref")?,
                    requested,
                    manifest_version,
                    dry_run,
                )?
            } else if requested.is_empty() {
                manifest_version
            } else {
                requested
            };
            let english = fs::read_to_string(root().join("CHANGELOG.md"))?;
            let chinese = fs::read_to_string(root().join("CHANGELOG_ZH.md"))?;
            let notes = verify_release_version(package, version, &english, &chinese)?;
            if command == "prepare" {
                let revision = output("git", &["rev-parse", "HEAD"])?;
                let tagged = if dry_run {
                    None
                } else {
                    Some(output(
                        "git",
                        &[
                            "rev-parse",
                            "--verify",
                            &format!("refs/tags/v{version}^{{commit}}"),
                        ],
                    )?)
                };
                check_revision(dry_run, revision.trim(), tagged.as_deref().map(str::trim))?;
                let mut file = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(required(&values, "output")?)?;
                writeln!(
                    file,
                    "version={version}\ntag=v{version}\nrevision={}\nis_dry_run={dry_run}\nprerelease={}",
                    revision.trim(),
                    version.contains('-')
                )?;
            }
            if let Some(path) = values.get("notes") {
                fs::write(path, format!("# steam-api-web v{version}\n\n{notes}\n"))?;
            }
            println!("Release metadata and bilingual changelogs verified: {version}");
        }
        "checksum" | "verify-archive" => {
            let values = flags(args, &["archive", "checksums", "output"])?;
            let path = Path::new(required(&values, "archive")?);
            let name = archive_name(path)?;
            let bytes = fs::read(path)?;
            if command == "checksum" {
                fs::write(
                    required(&values, "output")?,
                    format!("{}  {name}\n", sha256(&bytes)?),
                )?;
                println!("Recorded SHA-256 for {name}");
            } else {
                verify_checksum(
                    name,
                    &bytes,
                    &fs::read_to_string(required(&values, "checksums")?)?,
                )?;
                println!("Verified CI package checksum");
            }
        }
        "verify-package" => {
            let values = flags(args, &["archive"])?;
            let list = if let Some(archive) = values.get("archive") {
                let name = archive_name(Path::new(archive))?;
                let prefix = format!("{}/", name.trim_end_matches(".crate"));
                let listing = output("tar", &["-tzf", archive])?;
                let manifest = output("tar", &["-xOf", archive, &format!("{prefix}Cargo.toml")])?;
                check_normalized_manifest(&manifest)?;
                let mut relative = Vec::new();
                for path in listing.lines() {
                    if path.ends_with('/') {
                        continue;
                    }
                    relative.push(
                        path.strip_prefix(&prefix)
                            .ok_or("archive has an unexpected root directory")?
                            .to_owned(),
                    );
                }
                relative.join("\n")
            } else {
                output(
                    "cargo",
                    &[
                        "package",
                        "--locked",
                        "--package",
                        "steam-api-web",
                        "--list",
                        "--allow-dirty",
                    ],
                )?
            };
            let count = check_package_files(list.lines())?;
            println!(
                "Verified distributable allowlist: {count} package files; repository tooling, workflows, tests and reference datasets excluded"
            );
        }
        "verify-registry" => {
            let values = flags(args, &["version", "archive"])?;
            let version = required(&values, "version")?;
            validate_version(version)?;
            let archive = Path::new(required(&values, "archive")?);
            if archive_name(archive)? != format!("steam-api-web-{version}.crate") {
                return Err("archive version mismatch".into());
            }
            let checksum = sha256(&fs::read(archive)?)?;
            let client = http_client()?;
            let url = format!("https://crates.io/api/v1/crates/steam-api-web/{version}");
            poll_registry(
                || {
                    let response = match client.get(&url).send() {
                        Ok(response) => response,
                        Err(error)
                            if error.is_connect() || error.is_timeout() || error.is_request() =>
                        {
                            return Ok(None);
                        }
                        Err(error) => return Err(error.into()),
                    };
                    match response.status().as_u16() {
                        200 => registry_body(response),
                        404 | 429 | 500 | 502 | 503 | 504 => Ok(None),
                        status => {
                            Err(format!("registry verification failed with HTTP {status}").into())
                        }
                    }
                },
                || std::thread::sleep(Duration::from_secs(3)),
                20,
                version,
                &checksum,
            )?;
            println!("Verified crates.io version and archive checksum");
        }
        _ => return Err(format!("unknown release subcommand: {command}").into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn semver_accepts_stable_rc_beta_and_rejects_injection() {
        for version in ["0.2.0", "1.2.3-rc.1", "2.0.0-beta.2"] {
            assert!(validate_version(version).is_ok(), "{version}");
        }
        for version in [
            "v1.2.3",
            "1.2",
            "01.2.3",
            "1.2.3-01",
            "1.2.3\ninjected=true",
            "$(id)",
            "1.2.3-",
            "1.2.3+a",
        ] {
            assert!(validate_version(version).is_err(), "{version}");
        }
    }

    #[test]
    fn exact_nonempty_changelog_section_is_required() -> Result<()> {
        let text = "## [Unreleased]\n- Future work\n## [0.2.0] - Unreleased\n### Added\n- Current change\n## [0.1.0]\n- Older change\n";
        assert_eq!(
            changelog_section(text, "0.2.0")?,
            "### Added\n- Current change"
        );
        for version in ["0.3.0", "0.2"] {
            assert!(changelog_section(text, version).is_err());
        }
        assert!(changelog_section("## [0.2.0]\n", "0.2.0").is_err());
        assert!(changelog_section("## [0.2.0]suffix\n- Change\n", "0.2.0").is_err());
        Ok(())
    }

    #[test]
    fn manifest_and_both_languages_must_match() {
        let package = json!({"name":"steam-api-web", "version":"0.2.0", "edition":"2024", "license":"Apache-2.0"});
        let notes = "## [0.2.0]\n- Added bindings\n";
        assert!(verify_release_version(&package, "0.2.0", notes, notes).is_ok());
        assert!(verify_release_version(&package, "0.3.0", notes, notes).is_err());
        assert!(
            verify_release_version(&package, "0.2.0", notes, "## [Unreleased]\n- Work\n").is_err()
        );
    }

    #[test]
    fn archive_checksum_and_name_must_match() -> Result<()> {
        let name = "steam-api-web-0.2.0.crate";
        let bytes = b"reviewed archive";
        let manifest = format!("{}  {name}\n", sha256(bytes)?);
        verify_checksum(name, bytes, &manifest)?;
        assert!(verify_checksum(name, b"different archive", &manifest).is_err());
        assert!(verify_checksum("steam-api-web-0.1.0.crate", bytes, &manifest).is_err());
        assert!(verify_checksum(name, bytes, &format!("{manifest}{manifest}")).is_err());
        assert!(archive_name(Path::new("other-0.2.0.crate")).is_err());
        Ok(())
    }

    #[test]
    fn dry_run_records_revision_without_requiring_a_tag() -> Result<()> {
        check_revision(true, &"a".repeat(40), None)?;
        assert_eq!(
            release_version("workflow_dispatch", "main", "0.2.0", "0.1.0", true)?,
            "0.2.0"
        );
        Ok(())
    }

    #[test]
    fn publication_rejects_a_different_tagged_commit() {
        assert!(check_revision(false, &"a".repeat(40), Some(&"b".repeat(40))).is_err());
        assert!(check_revision(false, &"a".repeat(40), Some(&"a".repeat(40))).is_ok());
    }

    #[test]
    fn tag_push_uses_tag_version_and_cannot_become_dry_run() -> Result<()> {
        assert_eq!(
            release_version("push", "v0.2.0", "0.1.0", "0.1.0", false)?,
            "0.2.0"
        );
        assert!(release_version("push", "v0.2.0", "0.2.0", "0.2.0", true).is_err());
        assert!(release_version("push", "main", "0.2.0", "0.2.0", false).is_err());
        Ok(())
    }

    #[test]
    fn registry_requires_exact_checksum_not_just_version_existence() -> Result<()> {
        let checksum = sha256(b"reviewed archive")?;
        let mut metadata = json!({"version":{"crate":"steam-api-web", "num":"0.2.0", "yanked":false, "checksum":checksum}});
        verify_registry_metadata(&metadata, "steam-api-web", "0.2.0", &checksum)?;
        metadata["version"]["checksum"] = json!("0".repeat(64));
        assert!(verify_registry_metadata(&metadata, "steam-api-web", "0.2.0", &checksum).is_err());
        metadata["version"]["checksum"] = json!(checksum);
        metadata["version"]["yanked"] = json!(true);
        assert!(verify_registry_metadata(&metadata, "steam-api-web", "0.2.0", &checksum).is_err());
        Ok(())
    }

    #[test]
    fn registry_retries_are_bounded_and_permanent_errors_fail() {
        let mut calls = 0;
        assert!(
            poll_registry(
                || {
                    calls += 1;
                    Ok(None)
                },
                || {},
                2,
                "0.2.0",
                "unused"
            )
            .is_err()
        );
        assert_eq!(calls, 2);
        calls = 0;
        assert!(
            poll_registry(
                || {
                    calls += 1;
                    Err("HTTP 403".into())
                },
                || {},
                2,
                "0.2.0",
                "unused"
            )
            .is_err()
        );
        assert_eq!(calls, 1);
    }

    #[test]
    fn registry_body_failures_retry_within_budget_and_malformed_json_fails() -> Result<()> {
        struct ResetReader;
        impl std::io::Read for ResetReader {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::from(std::io::ErrorKind::ConnectionReset))
            }
        }

        let metadata = json!({"version":{"crate":"steam-api-web", "num":"0.2.0", "yanked":false, "checksum":"reviewed"}});
        let complete = serde_json::to_vec(&metadata)?;
        let mut calls = 0;
        poll_registry(
            || {
                calls += 1;
                if calls == 1 {
                    registry_body(ResetReader)
                } else {
                    registry_body(complete.as_slice())
                }
            },
            || {},
            2,
            "0.2.0",
            "reviewed",
        )?;
        assert_eq!(calls, 2);

        calls = 0;
        assert!(
            poll_registry(
                || {
                    calls += 1;
                    registry_body(&b"{\"version\":"[..])
                },
                || {},
                2,
                "0.2.0",
                "reviewed",
            )
            .is_err()
        );
        assert_eq!(calls, 2);

        calls = 0;
        assert!(
            poll_registry(
                || {
                    calls += 1;
                    registry_body(&b"{\"version\":!}"[..])
                },
                || {},
                2,
                "0.2.0",
                "reviewed",
            )
            .is_err()
        );
        assert_eq!(calls, 1);
        Ok(())
    }

    #[test]
    fn package_allowlist_rejects_repository_tooling_and_path_traversal() -> Result<()> {
        let files = [
            "Cargo.toml",
            "Cargo.lock",
            "LICENSE",
            "README.md",
            "README_ZH.md",
            "CHANGELOG.md",
            "CHANGELOG_ZH.md",
            "src/lib.rs",
        ];
        check_package_files(files)?;
        for extra in [
            "xtask/src/main.rs",
            "scripts/release.py",
            ".github/workflows/ci.yml",
            "tests/api.rs",
            "benches/codec.rs",
            "benches/support/baseline.rs",
            "docs/api-inventory.json",
            "src/../secret.rs",
        ] {
            assert!(
                check_package_files(files.into_iter().chain([extra])).is_err(),
                "{extra}"
            );
        }
        Ok(())
    }

    #[test]
    fn normalized_manifest_excludes_repository_targets_and_workspace() {
        assert!(
            check_normalized_manifest(
                "[package]\nname = \"steam-api-web\"\n[[example]]\nname = \"current_players\"\n"
            )
            .is_ok()
        );
        for table in ["[[test]]", "[[bench]]", "[workspace]"] {
            assert!(check_normalized_manifest(&format!("[package]\n{table}\n")).is_err());
        }
    }
}
