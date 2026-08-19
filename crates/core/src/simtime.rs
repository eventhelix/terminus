//! Sim-time helpers. The simulation always starts at
//! `MonotonicTime::EPOCH`; scenario wall-clock epoch is applied only in
//! the Recorder when stamping PCAPNG timestamps.

use nexosim::model::{Context, Model};
use nexosim::time::MonotonicTime;

pub const NS_PER_SEC: u64 = 1_000_000_000;

/// Current sim time as ns since sim t0.
pub fn now_ns<M: Model>(cx: &Context<M>) -> u64 {
    cx.time().duration_since(MonotonicTime::EPOCH).as_nanos() as u64
}

/// Config seconds (f64) → ns. Single rounding point for all config
/// time conversions, so f64 quirks cannot differ between call sites.
pub fn secs_to_ns(s: f64) -> u64 {
    (s * 1e9).round() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secs_to_ns_rounds() {
        assert_eq!(secs_to_ns(1.0), 1_000_000_000);
        assert_eq!(secs_to_ns(0.0000005), 500);
        assert_eq!(secs_to_ns(60.0), 60 * NS_PER_SEC);
    }
}
