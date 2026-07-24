//! Success criterion 2 (design §3.4): Wireshark opens any node capture
//! with a clean dissection chain. Requires tshark on PATH — run with
//! `cargo test -- --ignored` (CI does).

use std::path::{Path, PathBuf};
use std::process::Command;

fn scenario() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../scenarios/leo-testbed/scenario.toml")
}

#[test]
#[ignore = "requires tshark on PATH; CI runs it"]
fn tshark_dissects_all_captures_cleanly() {
    let run = std::env::temp_dir().join(format!("helixsim-tshark-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&run);
    helixsim_cli::assemble::run_scenario(&scenario(), &run).unwrap();
    let lua = run.join("dissectors/link.lua");

    for node in ["term-a", "term-b", "sat-1", "sat-2", "sat-3", "gw"] {
        let pcap = run.join("nodes").join(format!("{node}.pcapng"));

        // Zero malformed / expert-error frames.
        let out = Command::new("tshark")
            .arg("-r").arg(&pcap)
            .arg("-X").arg(format!("lua_script:{}", lua.display()))
            .arg("-Y").arg("_ws.malformed || _ws.expert.severity == \"Error\"")
            .output()
            .expect("tshark not runnable");
        assert!(out.status.success(), "tshark failed on {node}: {}", String::from_utf8_lossy(&out.stderr));
        let bad = String::from_utf8_lossy(&out.stdout);
        assert!(bad.trim().is_empty(), "{node}: malformed frames:\n{bad}");

        // Positive check: the chain reaches UDP on data frames.
        let out = Command::new("tshark")
            .arg("-r").arg(&pcap)
            .arg("-X").arg(format!("lua_script:{}", lua.display()))
            .arg("-T").arg("fields").arg("-e").arg("frame.protocols")
            .output()
            .unwrap();
        let protos = String::from_utf8_lossy(&out.stdout);
        assert!(protos.lines().any(|l| l.contains("udp")), "{node}: no frame chained through to UDP");
        assert!(protos.lines().all(|l| !l.is_empty()), "{node}: frames with no protocol at all");
    }
    let _ = std::fs::remove_dir_all(&run);
}
