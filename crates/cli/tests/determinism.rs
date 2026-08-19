//! Determinism invariant (CI, always on): running the same scenario
//! with the same seed twice produces byte-identical artifacts.

use std::path::{Path, PathBuf};

fn scenario() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../scenarios/leo-testbed/scenario.toml")
}

/// Every deterministic artifact in a run dir, relative path → bytes.
/// visualether.toml is excluded: it embeds the absolute run-dir path
/// by design (see output.rs) and differs between output directories.
fn artifacts(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for name in ["metrics.ndjson", "scenario.snapshot.toml", "dissectors/link.lua"] {
        out.push((name.to_string(), std::fs::read(dir.join(name)).unwrap()));
    }
    let mut nodes: Vec<_> = std::fs::read_dir(dir.join("nodes"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    nodes.sort();
    for p in nodes {
        out.push((
            format!("nodes/{}", p.file_name().unwrap().to_string_lossy()),
            std::fs::read(&p).unwrap(),
        ));
    }
    out
}

#[test]
fn same_seed_byte_identical_outputs() {
    let base = std::env::temp_dir().join(format!("helixsim-det-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let (a, b) = (base.join("a"), base.join("b"));
    helixsim_cli::assemble::run_scenario(&scenario(), &a).unwrap();
    helixsim_cli::assemble::run_scenario(&scenario(), &b).unwrap();
    let (fa, fb) = (artifacts(&a), artifacts(&b));
    assert_eq!(
        fa.iter().map(|(n, _)| n).collect::<Vec<_>>(),
        fb.iter().map(|(n, _)| n).collect::<Vec<_>>(),
        "file sets differ"
    );
    for ((name, bytes_a), (_, bytes_b)) in fa.iter().zip(fb.iter()) {
        assert_eq!(bytes_a, bytes_b, "{name} differs between identical runs");
    }
    let _ = std::fs::remove_dir_all(&base);
}
