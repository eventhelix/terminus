//! Observability records: capture taps and metrics. These flow over
//! ordinary nexosim ports into the Recorder model (Task 7).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Tx,
    Rx,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureRecord {
    pub node: u16,
    pub if_index: u8,
    pub t_ns: u64,
    pub dir: Direction,
    pub bytes: Vec<u8>,
}

/// One NDJSON line in `metrics.ndjson`.
///
/// Event vocabulary (source prefix → events):
/// - `medium:<name>`  — `tx`, `delivered`, `drop_bler`, `unreachable`
/// - `compute:<node>` — `submit` (queue_len), `done` (queue_len), `drop_overflow`
/// - `netif:<node>:<if>` — `tx_down`, `rx_down`
/// - `node:<name>`    — `echo_sent`, `echo_rtt` (value_ns), `echo_reply`,
///                      `forward`, `telemetry_sent`, `telemetry_rcvd`, `decode_error`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricRecord {
    pub t_ns: u64,
    pub source: String,
    pub event: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub packet_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue_len: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_ns: Option<u64>,
}

impl MetricRecord {
    pub fn new(t_ns: u64, source: &str, event: &str) -> Self {
        Self {
            t_ns,
            source: source.to_string(),
            event: event.to_string(),
            packet_id: None,
            queue_len: None,
            value_ns: None,
        }
    }
    pub fn packet(mut self, id: u64) -> Self {
        self.packet_id = Some(id);
        self
    }
    pub fn queue(mut self, q: u32) -> Self {
        self.queue_len = Some(q);
        self
    }
    pub fn value(mut self, ns: u64) -> Self {
        self.value_ns = Some(ns);
        self
    }
}
