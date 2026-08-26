//! Golden-run acceptance (design §3.5): committed digests of the
//! canonical scenario's captures. On intentional behavior change:
//!   UPDATE_GOLDEN=1 cargo test -p terminus --test golden
//! then commit the regenerated golden.sha256 alongside the change.

use std::path::{Path, PathBuf};

fn scenario_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../scenarios/leo-testbed")
}

#[test]
fn outputs_match_committed_digests() {
    let run = std::env::temp_dir().join(format!("terminus-golden-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&run);
    terminus_cli::assemble::run_scenario(&scenario_dir().join("scenario.toml"), &run).unwrap();

    let mut lines = Vec::new();
    let mut nodes: Vec<_> = std::fs::read_dir(run.join("nodes"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    nodes.sort();
    for p in &nodes {
        let digest = terminus_cli::config::sha256_hex(&std::fs::read(p).unwrap());
        lines.push(format!("{digest}  nodes/{}", p.file_name().unwrap().to_string_lossy()));
    }
    let digest = terminus_cli::config::sha256_hex(&std::fs::read(run.join("metrics.ndjson")).unwrap());
    lines.push(format!("{digest}  metrics.ndjson"));
    let actual = lines.join("\n") + "\n";

    let golden_path = scenario_dir().join("golden.sha256");
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::write(&golden_path, &actual).unwrap();
        eprintln!("golden.sha256 updated");
    } else {
        let expected = std::fs::read_to_string(&golden_path)
            .expect("golden.sha256 missing — run once with UPDATE_GOLDEN=1");
        assert_eq!(
            actual, expected,
            "outputs drifted from golden digests; if intentional, rerun with UPDATE_GOLDEN=1 and commit"
        );
    }
    let _ = std::fs::remove_dir_all(&run);
}
