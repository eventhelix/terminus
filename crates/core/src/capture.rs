// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Run recording: per-node PCAPNG (the vantage tcpdump would give on
//! real hardware) + metrics.ndjson. Determinism notes that MUST hold
//! (CI-enforced):
//! - PcapNgWriter::with_endianness(.., Little) — never `new()` (native
//!   endianness would break cross-platform byte-identity).
//! - Every IDB carries IfTsResol(9): pcap-file 2.0 hard-codes
//!   nanosecond EPB timestamps and ignores if_tsresol, while the pcapng
//!   default is 10^-6 — without the option Wireshark misreads times.
//! - Frames are recorded on LINKTYPE_USER0 (147); the generated Lua
//!   dissector registers on wtap.USER0 and chains onward to IP.
//!
//! nexosim API note: `Recorder`'s inputs use the plain (macro-less)
//! `impl Model` 4-arg `(self, arg, cx, env)` form below, and this
//! resolved cleanly through `EventSource::connect` in the round-trip
//! test (Step 3) — no fallback to the `#[Model(type Env = ...)]` macro
//! form was needed.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::Duration;

use nexosim::model::{BuildContext, Context, Model, ProtoModel};
use pcap_file::pcapng::blocks::enhanced_packet::{EnhancedPacketBlock, EnhancedPacketOption};
use pcap_file::pcapng::blocks::interface_description::{
    InterfaceDescriptionBlock, InterfaceDescriptionOption,
};
use pcap_file::pcapng::PcapNgWriter;
use pcap_file::{DataLink, Endianness};
use serde::{Deserialize, Serialize};

use crate::records::{CaptureRecord, Direction, MetricRecord};

const EPB_FLAG_INBOUND: u32 = 0b01;
const EPB_FLAG_OUTBOUND: u32 = 0b10;

#[derive(Debug, Clone)]
pub struct IfSpec {
    pub node: u16,
    pub if_index: u8,
    /// IDB if_name, e.g. "term-a:if0".
    pub name: String,
}

pub struct RecorderEnv {
    writers: BTreeMap<u16, PcapNgWriter<File>>,
    /// (node, if_index) -> pcapng interface_id within that node's file.
    if_ids: BTreeMap<(u16, u8), u32>,
    metrics: BufWriter<File>,
    epoch_ns: u64,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Recorder {}

impl Model for Recorder {
    type Env = RecorderEnv;
}

impl Recorder {
    pub fn capture(&mut self, rec: CaptureRecord, _cx: &Context<Self>, env: &mut RecorderEnv) {
        let flags = match rec.dir {
            Direction::Tx => EPB_FLAG_OUTBOUND,
            Direction::Rx => EPB_FLAG_INBOUND,
        };
        let interface_id = *env
            .if_ids
            .get(&(rec.node, rec.if_index))
            .expect("capture record for unknown interface — assembly bug");
        let writer = env.writers.get_mut(&rec.node).expect("unknown node — assembly bug");
        writer
            .write_pcapng_block(EnhancedPacketBlock {
                interface_id,
                timestamp: Duration::from_nanos(env.epoch_ns + rec.t_ns),
                original_len: rec.bytes.len() as u32,
                data: Cow::Owned(rec.bytes),
                options: vec![EnhancedPacketOption::Flags(flags)],
            })
            .expect("pcapng write failed");
    }

    pub fn metric(&mut self, rec: MetricRecord, _cx: &Context<Self>, env: &mut RecorderEnv) {
        serde_json::to_writer(&mut env.metrics, &rec).expect("metrics write failed");
        env.metrics.write_all(b"\n").expect("metrics write failed");
    }

    pub fn flush(&mut self, _arg: (), _cx: &Context<Self>, env: &mut RecorderEnv) {
        env.metrics.flush().expect("metrics flush failed");
    }
}

pub struct ProtoRecorder {
    pub nodes_dir: PathBuf,
    pub metrics_path: PathBuf,
    pub epoch_ns: u64,
    pub node_names: BTreeMap<u16, String>,
    /// Assembly order defines per-node interface ids (0, 1, ...).
    pub ifs: Vec<IfSpec>,
}

impl ProtoModel for ProtoRecorder {
    type Model = Recorder;

    fn build(self, _cx: &mut BuildContext<Self>) -> (Recorder, RecorderEnv) {
        std::fs::create_dir_all(&self.nodes_dir).expect("create nodes dir");
        let mut writers = BTreeMap::new();
        let mut if_ids = BTreeMap::new();
        let mut next_if_id: BTreeMap<u16, u32> = BTreeMap::new();
        for spec in &self.ifs {
            let writer = writers.entry(spec.node).or_insert_with(|| {
                let name = self.node_names.get(&spec.node).expect("unnamed node");
                let file = File::create(self.nodes_dir.join(format!("{name}.pcapng")))
                    .expect("create pcapng");
                PcapNgWriter::with_endianness(file, Endianness::Little).expect("pcapng SHB")
            });
            let id = next_if_id.entry(spec.node).or_insert(0);
            writer
                .write_pcapng_block(InterfaceDescriptionBlock {
                    linktype: DataLink::USER0, // LINKTYPE_USER0 = 147
                    snaplen: 0,                // no limit
                    options: vec![
                        InterfaceDescriptionOption::IfName(Cow::Owned(spec.name.clone())),
                        InterfaceDescriptionOption::IfTsResol(9), // REQUIRED, see module docs
                    ],
                })
                .expect("write IDB");
            if_ids.insert((spec.node, spec.if_index), *id);
            *id += 1;
        }
        let metrics =
            BufWriter::new(File::create(&self.metrics_path).expect("create metrics.ndjson"));
        let env = RecorderEnv { writers, if_ids, metrics, epoch_ns: self.epoch_ns };
        (Recorder::default(), env)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexosim::ports::EventSource;
    use nexosim::simulation::{Mailbox, SimInit};
    use nexosim::time::MonotonicTime;
    use pcap_file::pcapng::blocks::Block;
    use pcap_file::pcapng::PcapNgReader;

    #[test]
    fn writes_readable_pcapng_and_ndjson() {
        let dir = std::env::temp_dir().join(format!("terminus-cap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let nodes_dir = dir.join("nodes");
        let metrics_path = dir.join("metrics.ndjson");
        let proto = ProtoRecorder {
            nodes_dir: nodes_dir.clone(),
            metrics_path: metrics_path.clone(),
            epoch_ns: 1_000_000_000, // epoch offset visible in timestamps
            node_names: BTreeMap::from([(1, "n1".to_string())]),
            ifs: vec![
                IfSpec { node: 1, if_index: 0, name: "n1:if0".into() },
                IfSpec { node: 1, if_index: 1, name: "n1:if1".into() },
            ],
        };
        let mbox = Mailbox::with_capacity(64);
        let mut bench = SimInit::with_num_threads(1);
        let cap = EventSource::new().connect(Recorder::capture, &mbox).register(&mut bench);
        let met = EventSource::new().connect(Recorder::metric, &mbox).register(&mut bench);
        let flush = EventSource::new().connect(Recorder::flush, &mbox).register(&mut bench);
        let mut simu = bench.add_model(proto, mbox, "recorder").init(MonotonicTime::EPOCH).unwrap();

        simu.process_event(
            &cap,
            CaptureRecord { node: 1, if_index: 1, t_ns: 500, dir: Direction::Rx, bytes: vec![9, 8, 7] },
        )
        .unwrap();
        simu.process_event(&met, MetricRecord::new(500, "medium:x", "tx").packet(42)).unwrap();
        simu.process_event(&flush, ()).unwrap();
        drop(simu);

        // PCAPNG readback: 2 IDBs then 1 EPB on interface 1 with epoch-shifted ns timestamp.
        let mut reader =
            PcapNgReader::new(File::open(nodes_dir.join("n1.pcapng")).unwrap()).unwrap();
        let mut idbs = 0;
        let mut epbs = 0;
        while let Some(block) = reader.next_block() {
            match block.unwrap() {
                Block::InterfaceDescription(idb) => {
                    assert_eq!(u32::from(idb.linktype), 147);
                    idbs += 1;
                }
                Block::EnhancedPacket(epb) => {
                    assert_eq!(epb.interface_id, 1);
                    assert_eq!(epb.timestamp, Duration::from_nanos(1_000_000_500));
                    assert_eq!(epb.data.as_ref(), &[9, 8, 7]);
                    epbs += 1;
                }
                _ => {}
            }
        }
        assert_eq!((idbs, epbs), (2, 1));

        let ndjson = std::fs::read_to_string(&metrics_path).unwrap();
        let rec: MetricRecord = serde_json::from_str(ndjson.lines().next().unwrap()).unwrap();
        assert_eq!(rec.event, "tx");
        assert_eq!(rec.packet_id, Some(42));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
