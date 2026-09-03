// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Bench assembly: scenario config → wired nexosim models → run →
//! flushed output directory. Wiring per node (see the plan's diagram):
//! NodeModel.to_ifs[i] → NetIf.tx → Medium.transmit → deliveries →
//! NetIf.rx → FifoCompute.submit → done → NodeModel.frame_in; all
//! capture/metrics → Recorder.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use nexosim::ports::{EventSource, Output};
use nexosim::simulation::{Mailbox, SimInit};
use nexosim::time::MonotonicTime;

use terminus_core::behaviors::{BehaviorKind, Burst, GatewayEcho, Relay, TerminalApp};
use terminus_core::capture::{IfSpec, ProtoRecorder, Recorder};
use terminus_core::compute::FifoCompute;
use terminus_core::medium::Medium;
use terminus_core::netif::NetIf;
use terminus_core::node::NodeModel;
use terminus_core::rng::model_rng;
use terminus_core::simtime::{secs_to_ns, NS_PER_SEC};

use crate::config::{self, NodeCfg, NodeKind};
use crate::output;

// Generous capacities: the model graph is cyclic and bounded mailboxes
// + cycles can deadlock under bursts.
const MBOX: usize = 4096;

pub fn run_scenario(scenario_path: &Path, run_dir: &Path) -> anyhow::Result<()> {
    let loaded = config::load(scenario_path)?;
    output::write_static_outputs(run_dir, &loaded)?;

    let seed = loaded.file.scenario.master_seed;
    let duration = Duration::from_nanos(secs_to_ns(loaded.file.scenario.duration_s));
    let epoch_ns = loaded.file.scenario.epoch_unix_s * NS_PER_SEC;

    let mut bench = SimInit::with_num_threads(1); // determinism invariant

    // --- Recorder ---------------------------------------------------
    let recorder_mbox: Mailbox<Recorder> = Mailbox::with_capacity(MBOX);
    let mut ifspecs = Vec::new();
    for n in &loaded.file.nodes {
        for (i, ifc) in n.interfaces.iter().enumerate() {
            ifspecs.push(IfSpec {
                node: n.id,
                if_index: i as u8,
                name: format!("{}:{}", n.name, ifc.name),
            });
        }
    }
    let proto_recorder = ProtoRecorder {
        nodes_dir: run_dir.join("nodes"),
        metrics_path: run_dir.join("metrics.ndjson"),
        epoch_ns,
        node_names: loaded
            .file
            .nodes
            .iter()
            .map(|n| (n.id, n.name.clone()))
            .collect(),
        ifs: ifspecs,
    };
    let flush = EventSource::new()
        .connect(Recorder::flush, &recorder_mbox)
        .register(&mut bench);

    // --- Media (constructed now, attached during the node loop) -----
    let mut media: BTreeMap<String, (Medium, Mailbox<Medium>)> = loaded
        .file
        .media
        .iter()
        .map(|m| {
            let label = format!("medium:{}", m.name);
            let mut model = Medium::new(
                label.clone(),
                loaded.traces[&m.name].clone(),
                loaded.blers[&m.name].clone(),
                model_rng(seed, &label),
            );
            model.metrics.connect(Recorder::metric, &recorder_mbox);
            (m.name.clone(), (model, Mailbox::with_capacity(MBOX)))
        })
        .collect();

    // --- Nodes -------------------------------------------------------
    struct NodeParts {
        model: NodeModel,
        mbox: Mailbox<NodeModel>,
        compute: FifoCompute,
        compute_mbox: Mailbox<FifoCompute>,
        ifs: Vec<(NetIf, Mailbox<NetIf>, String)>,
        name: String,
    }
    let mut parts: Vec<NodeParts> = Vec::new();
    let duration_s = loaded.file.scenario.duration_s;
    for n in &loaded.file.nodes {
        let mbox = Mailbox::with_capacity(MBOX);
        let compute_mbox = Mailbox::with_capacity(MBOX);
        let mut model = NodeModel::new(
            n.id,
            format!("node:{}", n.name),
            build_behavior(n, duration_s),
            model_rng(seed, &format!("node:{}", n.name)),
        );
        model.metrics.connect(Recorder::metric, &recorder_mbox);
        let mut compute = FifoCompute::new(
            format!("compute:{}", n.name),
            n.compute.cores,
            n.compute.queue,
            n.compute.rx_service_us * 1_000,
        );
        compute.done.connect(NodeModel::frame_in, &mbox);
        compute.metrics.connect(Recorder::metric, &recorder_mbox);

        let mut ifs = Vec::new();
        for (i, ifc) in n.interfaces.iter().enumerate() {
            let if_mbox: Mailbox<NetIf> = Mailbox::with_capacity(MBOX);
            let mut netif = NetIf::new(n.id, i as u8, format!("netif:{}:{}", n.name, ifc.name));
            let (medium, medium_mbox) = media.get_mut(&ifc.medium).expect("validated medium ref");
            netif.to_medium.connect(Medium::transmit, &*medium_mbox);
            netif.to_compute.connect(FifoCompute::submit, &compute_mbox);
            netif.capture.connect(Recorder::capture, &recorder_mbox);
            netif.metrics.connect(Recorder::metric, &recorder_mbox);
            // NodeModel tx port for this interface.
            let mut tx_port = Output::default();
            tx_port.connect(NetIf::tx, &if_mbox);
            model.to_ifs.push(tx_port);
            // Medium delivery port back to this interface.
            let mut delivery = Output::default();
            delivery.connect(NetIf::rx, &if_mbox);
            medium.attach(n.id, delivery);
            ifs.push((netif, if_mbox, ifc.name.clone()));
        }
        parts.push(NodeParts {
            model,
            mbox,
            compute,
            compute_mbox,
            ifs,
            name: n.name.clone(),
        });
    }

    // --- Move everything into the bench ------------------------------
    let mut bench = bench.add_model(proto_recorder, recorder_mbox, "recorder");
    for (name, (model, mbox)) in media {
        bench = bench.add_model(model, mbox, &format!("medium:{name}"));
    }
    for p in parts {
        bench = bench.add_model(p.model, p.mbox, &format!("node:{}", p.name));
        bench = bench.add_model(p.compute, p.compute_mbox, &format!("compute:{}", p.name));
        for (netif, if_mbox, if_name) in p.ifs {
            bench = bench.add_model(netif, if_mbox, &format!("netif:{}:{}", p.name, if_name));
        }
    }

    // --- Run ----------------------------------------------------------
    let t0 = MonotonicTime::EPOCH;
    // nexosim's error types carry a `Box<dyn Any + Send>` panic payload,
    // so they are not `Sync` and don't satisfy anyhow::Context's bound —
    // format via Display instead.
    let mut simu = bench
        .init(t0)
        .map_err(|e| anyhow::anyhow!("simulation init: {e}"))?;
    simu.step_until(t0 + duration)
        .map_err(|e| anyhow::anyhow!("simulation run: {e}"))?;
    simu.process_event(&flush, ())
        .map_err(|e| anyhow::anyhow!("flush recorder: {e}"))?;
    drop(simu); // drops models + RecorderEnv → files closed
    Ok(())
}

fn build_behavior(n: &NodeCfg, duration_s: f64) -> BehaviorKind {
    match n.kind {
        NodeKind::Terminal => {
            let a = n.app.as_ref().expect("validated: terminal has app");
            BehaviorKind::Terminal(TerminalApp {
                peer: a.peer,
                src_port: a.src_port,
                dst_port: a.dst_port,
                payload_len: a.payload_len,
                rate_pps: a.rate_pps,
                burst: a.burst.map(|b| Burst {
                    start_ns: secs_to_ns(b.start_s),
                    end_ns: secs_to_ns(b.end_s),
                    rate_pps: b.rate_pps,
                }),
                start_ns: secs_to_ns(a.start_s),
                end_ns: secs_to_ns(duration_s),
                seq: 0,
                sent: std::collections::BTreeMap::new(),
            })
        }
        NodeKind::Satellite => {
            let r = n.relay.as_ref().expect("validated: satellite has relay");
            let telemetry_if = n
                .interfaces
                .iter()
                .position(|i| i.name == r.telemetry_if)
                .expect("validated: telemetry_if exists") as u8;
            BehaviorKind::Relay(Relay {
                if_map: vec![1, 0], // validated: satellites have exactly 2 interfaces
                telemetry_peer: r.telemetry_peer,
                telemetry_if,
                telemetry_period_ns: secs_to_ns(r.telemetry_period_s),
                seq: 0,
            })
        }
        NodeKind::Gateway => {
            let e = n.echo.as_ref().expect("validated: gateway has echo");
            BehaviorKind::Gateway(GatewayEcho {
                port: e.port,
                seq: 0,
            })
        }
    }
}
