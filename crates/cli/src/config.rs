// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Scenario configuration. Philosophy (design §3.5): configuration
//! errors die HERE, at startup — dangling refs, kind/config mismatches,
//! bad values, malformed traces. Anything that survives `load()` is
//! simulatable; runtime network problems are counted, not errored.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use terminus_core::bler::{BlerCurve, BlerError};
use terminus_core::trace::{ChannelTrace, TraceError};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("cannot read {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("scenario TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("{0}")]
    Trace(#[from] TraceError),
    #[error("medium {medium}: {source}")]
    Bler { medium: String, source: BlerError },
    #[error("duplicate node id or name: {what}")]
    DuplicateNode { what: String },
    #[error("duplicate medium name: {name}")]
    DuplicateMedium { name: String },
    #[error("node {node}: interface {interface} references unknown medium {medium}")]
    UnknownMedium {
        node: String,
        interface: String,
        medium: String,
    },
    #[error("node {node} references unknown peer node id {peer}")]
    UnknownPeer { node: String, peer: u16 },
    #[error("node {node}: kind {kind} requires exactly the matching section (app/relay/echo) and {ifs} interface(s)")]
    KindConfig {
        node: String,
        kind: String,
        ifs: String,
    },
    #[error("node {node} attaches to medium {medium} more than once")]
    DoubleAttach { node: String, medium: String },
    #[error("trace for medium {medium} contains pair {tx}->{rx} not attached to it")]
    UnattachedPair { medium: String, tx: u16, rx: u16 },
    #[error("medium {medium} has no attached interfaces")]
    OrphanMedium { medium: String },
    #[error("invalid value: {what}")]
    BadValue { what: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioFile {
    pub scenario: ScenarioMeta,
    #[serde(default)]
    pub nodes: Vec<NodeCfg>,
    #[serde(default)]
    pub media: Vec<MediumCfg>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioMeta {
    pub name: String,
    pub duration_s: f64,
    pub epoch_unix_s: u64,
    pub master_seed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    Terminal,
    Satellite,
    Gateway,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCfg {
    pub id: u16,
    pub name: String,
    pub kind: NodeKind,
    pub compute: ComputeCfg,
    pub interfaces: Vec<IfCfg>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<AppCfg>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relay: Option<RelayCfg>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub echo: Option<EchoCfg>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeCfg {
    pub cores: u32,
    pub queue: usize,
    pub rx_service_us: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfCfg {
    pub name: String,
    pub medium: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppCfg {
    pub peer: u16,
    pub rate_pps: f64,
    pub payload_len: usize,
    pub start_s: f64,
    pub src_port: u16,
    pub dst_port: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub burst: Option<BurstCfg>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BurstCfg {
    pub start_s: f64,
    pub end_s: f64,
    pub rate_pps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayCfg {
    pub telemetry_peer: u16,
    pub telemetry_period_s: f64,
    pub telemetry_if: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoCfg {
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediumCfg {
    pub name: String,
    pub trace: String,
    pub bler: Vec<[f64; 2]>,
}

pub struct LoadedScenario {
    pub file: ScenarioFile,
    pub dir: PathBuf,
    pub name_to_id: BTreeMap<String, u16>,
    pub traces: BTreeMap<String, ChannelTrace>,
    pub blers: BTreeMap<String, BlerCurve>,
    pub trace_sha256: BTreeMap<String, String>,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

pub fn load(scenario_path: &Path) -> Result<LoadedScenario, ConfigError> {
    let raw = std::fs::read_to_string(scenario_path).map_err(|source| ConfigError::Io {
        path: scenario_path.to_path_buf(),
        source,
    })?;
    let file: ScenarioFile = toml::from_str(&raw)?;
    let dir = scenario_path
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();
    validate_shape(&file)?;

    let name_to_id: BTreeMap<String, u16> =
        file.nodes.iter().map(|n| (n.name.clone(), n.id)).collect();

    // Media attachment map for trace cross-validation.
    let mut attached: BTreeMap<&str, BTreeSet<u16>> = BTreeMap::new();
    for n in &file.nodes {
        for ifc in &n.interfaces {
            attached
                .entry(ifc.medium.as_str())
                .or_default()
                .insert(n.id);
        }
    }

    let mut traces = BTreeMap::new();
    let mut blers = BTreeMap::new();
    let mut trace_sha256 = BTreeMap::new();
    for m in &file.media {
        let path = dir.join(&m.trace);
        let bytes = std::fs::read(&path).map_err(|source| ConfigError::Io {
            path: path.clone(),
            source,
        })?;
        trace_sha256.insert(m.name.clone(), sha256_hex(&bytes));
        let trace = ChannelTrace::from_csv(bytes.as_slice(), &name_to_id)?;
        for (tx, rx) in trace.pairs() {
            let members = &attached[m.name.as_str()];
            if !members.contains(&tx) || !members.contains(&rx) {
                return Err(ConfigError::UnattachedPair {
                    medium: m.name.clone(),
                    tx,
                    rx,
                });
            }
        }
        traces.insert(m.name.clone(), trace);
        blers.insert(
            m.name.clone(),
            BlerCurve::new(m.bler.iter().map(|r| (r[0], r[1])).collect()).map_err(|source| {
                ConfigError::Bler {
                    medium: m.name.clone(),
                    source,
                }
            })?,
        );
    }
    Ok(LoadedScenario {
        file,
        dir,
        name_to_id,
        traces,
        blers,
        trace_sha256,
    })
}

fn validate_shape(file: &ScenarioFile) -> Result<(), ConfigError> {
    if file.scenario.duration_s <= 0.0 {
        return Err(ConfigError::BadValue {
            what: "scenario.duration_s must be > 0".into(),
        });
    }
    // Duplicate medium names must die before the `media` set collapses them
    // (a BTreeMap keyed by name would otherwise silently last-wins).
    let mut medium_names = BTreeSet::new();
    for m in &file.media {
        if !medium_names.insert(m.name.as_str()) {
            return Err(ConfigError::DuplicateMedium {
                name: m.name.clone(),
            });
        }
    }

    let media: BTreeSet<&str> = file.media.iter().map(|m| m.name.as_str()).collect();
    let node_ids: BTreeSet<u16> = file.nodes.iter().map(|n| n.id).collect();

    // Duplicate ids/names must be caught before any check that consults
    // `node_ids` (e.g. peer references): a duplicate id collapses that set,
    // which would otherwise surface an unrelated UnknownPeer error first.
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for n in &file.nodes {
        if !ids.insert(n.id) {
            return Err(ConfigError::DuplicateNode {
                what: format!("id {}", n.id),
            });
        }
        if !names.insert(n.name.as_str()) {
            return Err(ConfigError::DuplicateNode {
                what: format!("name {}", n.name),
            });
        }
    }

    for n in &file.nodes {
        if n.id == 0 || n.id > 250 {
            return Err(ConfigError::BadValue {
                what: format!(
                    "node {}: id must be 1..=250 (IP scheme 10.0.0.<id>)",
                    n.name
                ),
            });
        }
        if n.compute.cores < 1 || n.compute.queue < 1 || n.compute.rx_service_us < 1 {
            return Err(ConfigError::BadValue {
                what: format!(
                    "node {}: compute cores/queue/rx_service_us must all be >= 1",
                    n.name
                ),
            });
        }
        let mut seen_media = BTreeSet::new();
        for ifc in &n.interfaces {
            if !media.contains(ifc.medium.as_str()) {
                return Err(ConfigError::UnknownMedium {
                    node: n.name.clone(),
                    interface: ifc.name.clone(),
                    medium: ifc.medium.clone(),
                });
            }
            if !seen_media.insert(ifc.medium.as_str()) {
                return Err(ConfigError::DoubleAttach {
                    node: n.name.clone(),
                    medium: ifc.medium.clone(),
                });
            }
        }
        // Kind ↔ section ↔ interface-count contract.
        let kind_ok = match n.kind {
            NodeKind::Terminal => {
                n.app.is_some() && n.relay.is_none() && n.echo.is_none() && n.interfaces.len() == 1
            }
            NodeKind::Satellite => {
                n.relay.is_some() && n.app.is_none() && n.echo.is_none() && n.interfaces.len() == 2
            }
            NodeKind::Gateway => {
                n.echo.is_some() && n.app.is_none() && n.relay.is_none() && n.interfaces.len() == 1
            }
        };
        if !kind_ok {
            let (kind, ifs) = match n.kind {
                NodeKind::Terminal => ("terminal", "1"),
                NodeKind::Satellite => ("satellite", "2"),
                NodeKind::Gateway => ("gateway", "1"),
            };
            return Err(ConfigError::KindConfig {
                node: n.name.clone(),
                kind: kind.into(),
                ifs: ifs.into(),
            });
        }
        if let Some(app) = &n.app {
            if !node_ids.contains(&app.peer) {
                return Err(ConfigError::UnknownPeer {
                    node: n.name.clone(),
                    peer: app.peer,
                });
            }
            if !(4..=60_000).contains(&app.payload_len) || app.rate_pps <= 0.0 {
                return Err(ConfigError::BadValue {
                    what: format!(
                        "node {}: payload_len must be 4..=60000 and rate_pps > 0 required",
                        n.name
                    ),
                });
            }
            if app.start_s < 0.0 {
                return Err(ConfigError::BadValue {
                    what: format!("node {}: app.start_s must be >= 0", n.name),
                });
            }
            if let Some(b) = &app.burst {
                if b.start_s < 0.0 {
                    return Err(ConfigError::BadValue {
                        what: format!("node {}: burst.start_s must be >= 0", n.name),
                    });
                }
                if b.rate_pps <= 0.0 || b.end_s <= b.start_s {
                    return Err(ConfigError::BadValue {
                        what: format!("node {}: burst window/rate invalid", n.name),
                    });
                }
            }
        }
        if let Some(relay) = &n.relay {
            if !node_ids.contains(&relay.telemetry_peer) {
                return Err(ConfigError::UnknownPeer {
                    node: n.name.clone(),
                    peer: relay.telemetry_peer,
                });
            }
            if relay.telemetry_period_s <= 0.0 {
                return Err(ConfigError::BadValue {
                    what: format!("node {}: telemetry_period_s must be > 0", n.name),
                });
            }
            if !n.interfaces.iter().any(|i| i.name == relay.telemetry_if) {
                return Err(ConfigError::BadValue {
                    what: format!(
                        "node {}: telemetry_if {} is not an interface",
                        n.name, relay.telemetry_if
                    ),
                });
            }
        }
    }

    // Every declared medium must have at least one attaching interface.
    // This also makes the attachment-map indexing in `load()`'s trace
    // cross-validation infallible.
    let referenced: BTreeSet<&str> = file
        .nodes
        .iter()
        .flat_map(|n| n.interfaces.iter().map(|i| i.medium.as_str()))
        .collect();
    for m in &file.media {
        if !referenced.contains(m.name.as_str()) {
            return Err(ConfigError::OrphanMedium {
                medium: m.name.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal valid scenario written to a temp dir with its trace file.
    fn write_fixture(mutate: impl FnOnce(&mut String)) -> Result<LoadedScenario, ConfigError> {
        let dir = std::env::temp_dir().join(format!(
            "terminus-cfg-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("traces")).unwrap();
        std::fs::write(
            dir.join("traces/m.csv"),
            "t_s,tx,rx,delay_us,sinr_db\n0.0,a,b,3000,12.0\n0.0,b,a,3000,12.0\n",
        )
        .unwrap();
        let mut toml = String::from(
            r#"
[scenario]
name = "t"
duration_s = 10.0
epoch_unix_s = 1753228800
master_seed = 1

[[nodes]]
id = 1
name = "a"
kind = "terminal"
compute = { cores = 1, queue = 4, rx_service_us = 100 }
interfaces = [{ name = "if0", medium = "m" }]
app = { peer = 2, rate_pps = 10.0, payload_len = 32, start_s = 1.0, src_port = 4001, dst_port = 7 }

[[nodes]]
id = 2
name = "b"
kind = "gateway"
compute = { cores = 1, queue = 4, rx_service_us = 100 }
interfaces = [{ name = "if0", medium = "m" }]
echo = { port = 7 }

[[media]]
name = "m"
trace = "traces/m.csv"
bler = [[-5.0, 1.0], [0.0, 0.001]]
"#,
        );
        mutate(&mut toml);
        std::fs::write(dir.join("scenario.toml"), toml).unwrap();
        load(&dir.join("scenario.toml"))
    }

    #[test]
    fn valid_fixture_loads() {
        let s = write_fixture(|_| {}).unwrap();
        assert_eq!(s.name_to_id["a"], 1);
        assert!(s.traces["m"].query(1, 2, 0).is_some());
        assert_eq!(s.trace_sha256["m"].len(), 64);
    }

    #[test]
    fn dangling_medium_ref_dies() {
        let e = write_fixture(|t| {
            *t = t.replace("medium = \"m\" }]\napp", "medium = \"nope\" }]\napp")
        });
        assert!(matches!(e, Err(ConfigError::UnknownMedium { .. })));
    }

    #[test]
    fn dangling_peer_dies() {
        let e = write_fixture(|t| *t = t.replace("peer = 2", "peer = 99"));
        assert!(matches!(e, Err(ConfigError::UnknownPeer { .. })));
    }

    #[test]
    fn duplicate_node_id_dies() {
        let e = write_fixture(|t| *t = t.replace("id = 2", "id = 1"));
        assert!(matches!(e, Err(ConfigError::DuplicateNode { .. })));
    }

    #[test]
    fn kind_config_mismatch_dies() {
        let e = write_fixture(|t| *t = t.replace("echo = { port = 7 }", ""));
        assert!(matches!(e, Err(ConfigError::KindConfig { .. })));
    }

    #[test]
    fn unknown_trace_node_dies() {
        // Trace mentions c, which is not a node.
        let e = write_fixture(|t| {
            let _ = t; // toml untouched; break the trace instead
        });
        // Overwrite the trace with an unknown name and reload.
        let dir = std::env::temp_dir().join(format!(
            "terminus-cfg-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::write(
            dir.join("traces/m.csv"),
            "t_s,tx,rx,delay_us,sinr_db\n0.0,a,c,3000,12.0\n",
        )
        .unwrap();
        drop(e);
        let e = load(&dir.join("scenario.toml"));
        assert!(matches!(e, Err(ConfigError::Trace(_))));
    }

    #[test]
    fn trace_pair_not_attached_dies() {
        // Node c lives on its own medium m2, whose trace nevertheless
        // carries a (valid) pair a->b that is not attached to m2 at all.
        let e = write_fixture(|t| {
            let dir = std::env::temp_dir().join(format!(
                "terminus-cfg-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            std::fs::write(
                dir.join("traces/m2.csv"),
                "t_s,tx,rx,delay_us,sinr_db\n0.0,a,b,3000,12.0\n",
            )
            .unwrap();
            t.push_str(
                r#"
[[nodes]]
id = 3
name = "c"
kind = "terminal"
compute = { cores = 1, queue = 4, rx_service_us = 100 }
interfaces = [{ name = "if0", medium = "m2" }]
app = { peer = 2, rate_pps = 10.0, payload_len = 32, start_s = 1.0, src_port = 4002, dst_port = 7 }

[[media]]
name = "m2"
trace = "traces/m2.csv"
bler = [[-5.0, 1.0], [0.0, 0.001]]
"#,
            );
        });
        assert!(matches!(e, Err(ConfigError::UnattachedPair { .. })));
    }

    #[test]
    fn duplicate_medium_name_dies() {
        let e = write_fixture(|t| {
            t.push_str(
                "\n[[media]]\nname = \"m\"\ntrace = \"traces/m.csv\"\nbler = [[-5.0, 1.0], [0.0, 0.001]]\n",
            );
        });
        assert!(matches!(e, Err(ConfigError::DuplicateMedium { .. })));
    }

    #[test]
    fn oversized_payload_dies() {
        let e = write_fixture(|t| *t = t.replace("payload_len = 32", "payload_len = 70000"));
        assert!(matches!(e, Err(ConfigError::BadValue { .. })));
    }

    #[test]
    fn negative_app_start_dies() {
        let e = write_fixture(|t| *t = t.replace("start_s = 1.0", "start_s = -1.0"));
        assert!(matches!(e, Err(ConfigError::BadValue { .. })));
    }

    #[test]
    fn negative_burst_start_dies() {
        let e = write_fixture(|t| {
            *t = t.replace(
                "app = { peer = 2, rate_pps = 10.0, payload_len = 32, start_s = 1.0, src_port = 4001, dst_port = 7 }",
                "app = { peer = 2, rate_pps = 10.0, payload_len = 32, start_s = 1.0, src_port = 4001, dst_port = 7, burst = { start_s = -1.0, end_s = 5.0, rate_pps = 20.0 } }",
            );
        });
        assert!(matches!(e, Err(ConfigError::BadValue { .. })));
    }

    #[test]
    fn orphan_medium_dies() {
        // A declared medium no interface attaches to must die with a typed
        // error at startup, not panic during trace cross-validation.
        let e = write_fixture(|t| {
            t.push_str(
                "\n[[media]]\nname = \"unused\"\ntrace = \"traces/m.csv\"\nbler = [[-5.0, 1.0], [0.0, 0.001]]\n",
            );
        });
        assert!(matches!(e, Err(ConfigError::OrphanMedium { .. })));
    }

    #[test]
    fn zero_service_time_dies() {
        let e = write_fixture(|t| {
            *t = t.replace(
                "rx_service_us = 100 }\ninterfaces = [{ name = \"if0\", medium = \"m\" }]\napp",
                "rx_service_us = 0 }\ninterfaces = [{ name = \"if0\", medium = \"m\" }]\napp",
            )
        });
        assert!(matches!(e, Err(ConfigError::BadValue { .. })));
    }
}
