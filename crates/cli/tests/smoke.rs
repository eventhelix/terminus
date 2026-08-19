//! End-to-end demo assertions: the three scripted features (handover,
//! BLER window, compute burst) must be visible in the run output.

use std::path::{Path, PathBuf};

use helixsim_core::records::MetricRecord;

fn scenario() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../scenarios/leo-testbed/scenario.toml")
}

fn run_once(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("helixsim-smoke-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    helixsim_cli::assemble::run_scenario(&scenario(), &dir).expect("run failed");
    dir
}

fn metrics(dir: &Path) -> Vec<MetricRecord> {
    std::fs::read_to_string(dir.join("metrics.ndjson"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

const S: u64 = 1_000_000_000;

#[test]
fn demo_features_visible_in_metrics_and_pcaps() {
    let dir = run_once("demo");
    let ms = metrics(&dir);

    // Echo flow works at all: term-a completes round trips.
    let rtts: Vec<&MetricRecord> =
        ms.iter().filter(|m| m.source == "node:term-a" && m.event == "echo_rtt").collect();
    assert!(rtts.len() > 300, "expected most of ~590 echoes to complete, got {}", rtts.len());
    // RTT sanity: ~20 ms nominal (2 hops of ~3-4 ms + 2.5 ms relay + gw).
    let sample = rtts[0].value_ns.unwrap();
    assert!((10_000_000..60_000_000).contains(&sample), "rtt {sample} ns out of band");

    // (a) Handover: round trips still complete after t=30s via sat-2.
    assert!(rtts.iter().any(|m| m.t_ns > 31 * S), "no echo replies after handover");
    assert!(
        ms.iter().any(|m| m.source == "node:sat-2" && m.event == "forward" && m.t_ns > 30 * S),
        "sat-2 never forwarded after handover"
    );
    // (No negative assertions on which satellite forwards when: the
    // feeder is a broadcast domain, so EVERY satellite hears the
    // gateway's replies and emits `forward` toward its access side at
    // all times — non-serving relays land as `unreachable` in the
    // medium. The post-30s echo_rtt assertion above is the real
    // handover proof: after sat-1's uplink pair closes, requests can
    // only reach the gateway via sat-2.)

    // (b) Degraded-SINR window: BLER drops on access during 18-24s.
    assert!(
        ms.iter().any(|m| m.source == "medium:access"
            && m.event == "drop_bler"
            && (18 * S..24 * S).contains(&m.t_ns)),
        "no BLER losses in the degraded window"
    );

    // (c) Burst: sat-3's compute queue overflows during 40-45s(+drain).
    assert!(
        ms.iter().any(|m| m.source == "compute:sat-3"
            && m.event == "drop_overflow"
            && (40 * S..46 * S).contains(&m.t_ns)),
        "no compute overflow during the burst"
    );

    // Telemetry control plane flows.
    assert!(ms.iter().any(|m| m.source == "node:gw" && m.event == "telemetry_rcvd"));

    // Pcaps exist and are non-trivial.
    for node in ["term-a", "term-b", "sat-1", "sat-2", "sat-3", "gw"] {
        let path = dir.join("nodes").join(format!("{node}.pcapng"));
        let len = std::fs::metadata(&path).unwrap().len();
        assert!(len > 200, "{node}.pcapng suspiciously small ({len} bytes)");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
