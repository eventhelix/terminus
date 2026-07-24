//! Precomputed channel traces: per directed (tx, rx) pair, a time
//! series of {delay, SINR}. Step-hold between samples (no
//! interpolation). A row with BOTH value fields empty is the
//! "unreachable from here on" sentinel; a pair with no rows is never
//! reachable (design §3.5: absent = physics, not an error).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

use crate::simtime::secs_to_ns;

#[derive(Debug, thiserror::Error)]
pub enum TraceError {
    #[error("i/o error reading trace: {0}")]
    Io(#[from] std::io::Error),
    #[error("csv error in trace: {0}")]
    Csv(#[from] csv::Error),
    #[error("unknown node name `{0}` in trace")]
    UnknownNode(String),
    #[error("trace rows for pair {tx}->{rx} out of time order at t={t_s}")]
    OutOfOrder { tx: String, rx: String, t_s: f64 },
    #[error("trace row {tx}->{rx} t={t_s}: delay_us and sinr_db must both be set or both empty")]
    HalfEmpty { tx: String, rx: String, t_s: f64 },
    #[error("trace row {tx}->{rx} t={t_s}: delay_us must be >= 1")]
    BadDelay { tx: String, rx: String, t_s: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    pub delay_ns: u64,
    pub sinr_db: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChannelTrace {
    /// (tx, rx) -> time-ascending samples; `None` sample = sentinel.
    pairs: BTreeMap<(u16, u16), Vec<(u64, Option<Sample>)>>,
}

#[derive(Debug, Deserialize)]
struct Row {
    t_s: f64,
    tx: String,
    rx: String,
    delay_us: Option<u64>,
    sinr_db: Option<f64>,
}

impl ChannelTrace {
    pub fn from_csv(reader: impl Read, name_to_id: &BTreeMap<String, u16>) -> Result<Self, TraceError> {
        let mut rdr = csv::Reader::from_reader(reader);
        let mut pairs: BTreeMap<(u16, u16), Vec<(u64, Option<Sample>)>> = BTreeMap::new();
        for row in rdr.deserialize::<Row>() {
            let row = row?;
            let tx = *name_to_id
                .get(&row.tx)
                .ok_or_else(|| TraceError::UnknownNode(row.tx.clone()))?;
            let rx = *name_to_id
                .get(&row.rx)
                .ok_or_else(|| TraceError::UnknownNode(row.rx.clone()))?;
            let sample = match (row.delay_us, row.sinr_db) {
                (Some(0), Some(_)) => {
                    return Err(TraceError::BadDelay { tx: row.tx, rx: row.rx, t_s: row.t_s })
                }
                (Some(d), Some(s)) => Some(Sample { delay_ns: d * 1_000, sinr_db: s }),
                (None, None) => None,
                _ => return Err(TraceError::HalfEmpty { tx: row.tx, rx: row.rx, t_s: row.t_s }),
            };
            let t_ns = secs_to_ns(row.t_s);
            let series = pairs.entry((tx, rx)).or_default();
            if let Some((last, _)) = series.last() {
                if t_ns <= *last {
                    return Err(TraceError::OutOfOrder { tx: row.tx, rx: row.rx, t_s: row.t_s });
                }
            }
            series.push((t_ns, sample));
        }
        Ok(Self { pairs })
    }

    pub fn load(path: &Path, name_to_id: &BTreeMap<String, u16>) -> Result<Self, TraceError> {
        Self::from_csv(std::fs::File::open(path)?, name_to_id)
    }

    /// Step-hold lookup: the last sample with t <= t_ns governs.
    pub fn query(&self, tx: u16, rx: u16, t_ns: u64) -> Option<Sample> {
        let series = self.pairs.get(&(tx, rx))?;
        let idx = series.partition_point(|(t, _)| *t <= t_ns);
        if idx == 0 {
            return None;
        }
        series[idx - 1].1
    }

    /// All directed pairs present in the trace (for startup validation).
    pub fn pairs(&self) -> impl Iterator<Item = (u16, u16)> + '_ {
        self.pairs.keys().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn ids() -> BTreeMap<String, u16> {
        BTreeMap::from([("a".into(), 1), ("b".into(), 2)])
    }

    const CSV: &str = "\
t_s,tx,rx,delay_us,sinr_db
0.0,a,b,3000,12.0
10.0,a,b,3500,8.0
30.0,a,b,,
";

    #[test]
    fn step_hold_lookup() {
        let tr = ChannelTrace::from_csv(CSV.as_bytes(), &ids()).unwrap();
        let s = tr.query(1, 2, 0).unwrap();
        assert_eq!(s.delay_ns, 3_000_000);
        assert_eq!(s.sinr_db, 12.0);
        // held until next sample
        let s = tr.query(1, 2, 9_999_999_999).unwrap();
        assert_eq!(s.delay_ns, 3_000_000);
        let s = tr.query(1, 2, 10_000_000_000).unwrap();
        assert_eq!(s.delay_ns, 3_500_000);
    }

    #[test]
    fn sentinel_makes_pair_unreachable() {
        let tr = ChannelTrace::from_csv(CSV.as_bytes(), &ids()).unwrap();
        assert!(tr.query(1, 2, 30_000_000_000).is_none());
        assert!(tr.query(1, 2, 59_000_000_000).is_none());
    }

    #[test]
    fn absent_pair_and_pre_first_sample_are_unreachable() {
        let tr = ChannelTrace::from_csv(CSV.as_bytes(), &ids()).unwrap();
        assert!(tr.query(2, 1, 5_000_000_000).is_none()); // reverse pair absent
        let late = "t_s,tx,rx,delay_us,sinr_db\n5.0,a,b,3000,10.0\n";
        let tr = ChannelTrace::from_csv(late.as_bytes(), &ids()).unwrap();
        assert!(tr.query(1, 2, 1_000_000_000).is_none()); // before first sample
    }

    #[test]
    fn unknown_node_name_fails() {
        let bad = "t_s,tx,rx,delay_us,sinr_db\n0.0,a,zz,3000,10.0\n";
        assert!(matches!(
            ChannelTrace::from_csv(bad.as_bytes(), &ids()),
            Err(TraceError::UnknownNode(_))
        ));
    }

    #[test]
    fn out_of_order_rows_fail() {
        let bad = "t_s,tx,rx,delay_us,sinr_db\n10.0,a,b,3000,10.0\n5.0,a,b,3000,10.0\n";
        assert!(matches!(
            ChannelTrace::from_csv(bad.as_bytes(), &ids()),
            Err(TraceError::OutOfOrder { .. })
        ));
    }

    #[test]
    fn half_empty_row_fails() {
        let bad = "t_s,tx,rx,delay_us,sinr_db\n0.0,a,b,3000,\n";
        assert!(matches!(
            ChannelTrace::from_csv(bad.as_bytes(), &ids()),
            Err(TraceError::HalfEmpty { .. })
        ));
    }

    #[test]
    fn zero_delay_fails() {
        let bad = "t_s,tx,rx,delay_us,sinr_db\n0.0,a,b,0,10.0\n";
        assert!(matches!(
            ChannelTrace::from_csv(bad.as_bytes(), &ids()),
            Err(TraceError::BadDelay { .. })
        ));
    }
}
