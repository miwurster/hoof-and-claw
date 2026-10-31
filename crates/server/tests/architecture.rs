//! The architecture rules of `ARCHITECTURE.md` that the compiler alone does not enforce.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use cargo_metadata::{CargoOpt, DependencyKind, Metadata, MetadataCommand, Package};

/// Crates outside the workspace that only the crates in the table may depend on, and only as the table says.
const INFRASTRUCTURE: &[&str] = &[
    "anyhow",
    "axum",
    "rustls",
    "serde",
    "sqlx",
    "tokio",
    "tokio-tungstenite",
    "tower-http",
    "ts-rs",
];

const MODULE_NAMES_THAT_SAY_NOTHING: &[&str] =
    &["models", "services", "repositories", "utils", "helpers"];

/// The "May depend on" column of the crate table in `ARCHITECTURE.md`: workspace crates, then infrastructure.
fn may_depend_on(krate: &str) -> Option<(&'static [&'static str], &'static [&'static str])> {
    let rule: (&[&str], &[&str]) = match krate {
        "prices" => (&[], &[]),
        "guessing" => (&["prices"], &[]),
        "identity" => (&["guessing"], &[]),
        "game" => (&["prices", "guessing", "identity"], &["tokio"]),
        "bitstamp" => (&["prices"], &["tokio", "tokio-tungstenite", "rustls"]),
        "sqlite" => (&["prices", "guessing", "identity"], &["sqlx"]),
        "web" => (
            &["game", "prices", "guessing", "identity"],
            &["axum", "tower-http", "serde", "ts-rs"],
        ),
        "test-controls" => (&["game", "prices"], &["axum", "serde", "ts-rs"]),
        "server" => (
            &[
                "prices",
                "guessing",
                "identity",
                "game",
                "bitstamp",
                "sqlite",
                "web",
                "test-controls",
            ],
            INFRASTRUCTURE,
        ),
        _ => return None,
    };
    Some(rule)
}

fn workspace_manifest() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml")
}

fn metadata(features: CargoOpt) -> cargo_metadata::Result<Metadata> {
    MetadataCommand::new()
        .manifest_path(workspace_manifest())
        .features(features)
        .exec()
}

fn workspace_packages(metadata: &Metadata) -> Vec<&Package> {
    metadata.workspace_packages()
}

#[test]
fn crate_graph_follows_the_table() {
    let metadata = metadata(CargoOpt::AllFeatures).expect("cargo metadata should run");
    let members: BTreeSet<&str> = workspace_packages(&metadata)
        .iter()
        .map(|package| package.name.as_str())
        .collect();
    let mut violations = Vec::new();

    for package in workspace_packages(&metadata) {
        let name = package.name.as_str();
        let Some((crates, infrastructure)) = may_depend_on(name) else {
            violations.push(format!(
                "crate `{name}` is not in the crate table of ARCHITECTURE.md; add it there and to `may_depend_on` in {}",
                file!()
            ));
            continue;
        };
        for dependency in &package.dependencies {
            if dependency.kind == DependencyKind::Development {
                continue;
            }
            let dependency = dependency.name.as_str();
            let allowed = if members.contains(dependency) {
                crates.contains(&dependency)
            } else if INFRASTRUCTURE.contains(&dependency) {
                infrastructure.contains(&dependency)
            } else {
                true
            };
            if !allowed {
                violations.push(format!(
                    "crate `{name}` depends on `{dependency}`, but ARCHITECTURE.md allows `{name}` only {crates:?} and {infrastructure:?}"
                ));
            }
        }
    }

    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

#[test]
fn modules_are_named_after_the_domain() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut violations = Vec::new();
    let mut pending = vec![crates];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            let stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default();
            let is_rust_module = path.is_dir() || path.extension().is_some_and(|ext| ext == "rs");
            if is_rust_module && MODULE_NAMES_THAT_SAY_NOTHING.contains(&stem) {
                violations.push(format!(
                    "module {} is named `{stem}`; name it after a glossary term instead",
                    path.display()
                ));
            }
            if path.is_dir() && !path.ends_with("target") {
                pending.push(path);
            }
        }
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

#[test]
fn release_build_has_no_test_controls() {
    let metadata = metadata(CargoOpt::NoDefaultFeatures).expect("cargo metadata should run");
    let resolve = metadata
        .resolve
        .as_ref()
        .expect("cargo metadata should resolve the dependency graph");
    let server = workspace_packages(&metadata)
        .into_iter()
        .find(|package| package.name.as_str() == "server")
        .expect("the workspace should hold `server`");
    let node = resolve
        .nodes
        .iter()
        .find(|node| node.id == server.id)
        .expect("the graph should hold `server`");

    let mut reachable = BTreeSet::new();
    let mut pending = vec![&node.id];
    while let Some(id) = pending.pop() {
        if !reachable.insert(id) {
            continue;
        }
        // Only what the binary links counts, not what its tests use.
        if let Some(node) = resolve.nodes.iter().find(|node| &node.id == id) {
            let normal = node.deps.iter().filter(|dep| {
                dep.dep_kinds
                    .iter()
                    .any(|kind| kind.kind != DependencyKind::Development)
            });
            pending.extend(normal.map(|dep| &dep.pkg));
        }
    }
    let test_controls: Vec<_> = metadata
        .packages
        .iter()
        .filter(|package| {
            package.name.as_str() == "test-controls" && reachable.contains(&package.id)
        })
        .collect();

    assert!(
        test_controls.is_empty(),
        "the release build of `server` should not contain `test-controls`; make it an optional dependency behind the feature `test-controls`"
    );
}
