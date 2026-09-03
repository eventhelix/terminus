// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Node = assembly (design §3.1): behavior model(s) + one NetIf per
//! interface + a ComputeModel, composed at bench time. `NodeBehavior`
//! is the slot where real production components go later; behaviors
//! act only through `BehaviorCtx` (no direct port/clock/file access),
//! which is what makes model-in-the-loop → software-in-the-loop honest.

use std::time::Duration;

use nexosim::model::{schedulable, Context, Model};
use nexosim::ports::Output;
use rand_chacha::ChaCha12Rng;
use serde::{Deserialize, Serialize};

use crate::behaviors::BehaviorKind;
use crate::packet::{Packet, PacketMeta, RxFrame};
use crate::records::MetricRecord;
use crate::simtime::now_ns;

pub const TIMER_SEND: u64 = 1;
pub const TIMER_TELEMETRY: u64 = 2;

pub trait NodeBehavior: Send + 'static {
    fn on_start(&mut self, ctx: &mut BehaviorCtx);
    fn on_frame(&mut self, if_index: u8, packet: &Packet, ctx: &mut BehaviorCtx);
    fn on_timer(&mut self, timer_id: u64, ctx: &mut BehaviorCtx);
}

/// Everything a behavior may do, buffered for the NodeModel to apply.
#[derive(Debug, Default, PartialEq)]
pub struct Actions {
    /// (if_index, packet) to transmit.
    pub transmits: Vec<(u8, Packet)>,
    /// (timer_id, delay_ns) to schedule.
    pub timers: Vec<(u64, u64)>,
    pub metrics: Vec<MetricRecord>,
}

pub struct BehaviorCtx<'a> {
    pub node_id: u16,
    pub source: &'a str,
    pub now_ns: u64,
    /// Per-node seeded RNG — the ONLY randomness a behavior may use.
    pub rng: &'a mut ChaCha12Rng,
    next_packet_id: &'a mut u64,
    actions: &'a mut Actions,
}

impl<'a> BehaviorCtx<'a> {
    /// Mint a new packet from freshly encoded bytes; returns its id.
    pub fn transmit_new(&mut self, if_index: u8, bytes: Vec<u8>) -> u64 {
        let id = (u64::from(self.node_id) << 48) | *self.next_packet_id;
        *self.next_packet_id += 1;
        self.actions.transmits.push((
            if_index,
            Packet {
                bytes,
                meta: PacketMeta {
                    id,
                    birth_ns: self.now_ns,
                    origin: self.node_id,
                },
            },
        ));
        id
    }

    /// Relay an existing packet unchanged (meta preserved — the id
    /// stays traceable end-to-end across hops).
    pub fn forward(&mut self, if_index: u8, packet: Packet) {
        self.actions.transmits.push((if_index, packet));
    }

    pub fn timer_in(&mut self, timer_id: u64, delay_ns: u64) {
        self.actions.timers.push((timer_id, delay_ns));
    }

    /// Emit a metric; mutate the returned record to attach fields.
    pub fn metric(&mut self, event: &str) -> &mut MetricRecord {
        self.actions
            .metrics
            .push(MetricRecord::new(self.now_ns, self.source, event));
        self.actions.metrics.last_mut().unwrap()
    }
}

/// Runs a behavior against BehaviorCtx outside any simulation — used by
/// NodeModel below, by behavior unit tests, and by the conformance suite.
pub fn drive_behavior<B: NodeBehavior>(
    behavior: &mut B,
    node_id: u16,
    source: &str,
    now_ns: u64,
    rng: &mut ChaCha12Rng,
    next_packet_id: &mut u64,
    f: impl FnOnce(&mut B, &mut BehaviorCtx),
) -> Actions {
    let mut actions = Actions::default();
    let mut ctx = BehaviorCtx {
        node_id,
        source,
        now_ns,
        rng,
        next_packet_id,
        actions: &mut actions,
    };
    f(behavior, &mut ctx);
    actions
}

#[derive(Serialize, Deserialize)]
pub struct NodeModel {
    node_id: u16,
    source: String,
    behavior: BehaviorKind,
    rng: ChaCha12Rng,
    next_packet_id: u64,
    /// One per interface, index-aligned with the node's NetIfs.
    pub to_ifs: Vec<Output<Packet>>,
    pub metrics: Output<MetricRecord>,
}

#[Model]
impl NodeModel {
    pub fn new(node_id: u16, source: String, behavior: BehaviorKind, rng: ChaCha12Rng) -> Self {
        Self {
            node_id,
            source,
            behavior,
            rng,
            next_packet_id: 0,
            to_ifs: Vec::new(),
            metrics: Output::default(),
        }
    }

    fn drive(&mut self, now: u64, f: impl FnOnce(&mut BehaviorKind, &mut BehaviorCtx)) -> Actions {
        let mut actions = Actions::default();
        let mut ctx = BehaviorCtx {
            node_id: self.node_id,
            source: &self.source,
            now_ns: now,
            rng: &mut self.rng,
            next_packet_id: &mut self.next_packet_id,
            actions: &mut actions,
        };
        f(&mut self.behavior, &mut ctx);
        actions
    }

    async fn apply(&mut self, actions: Actions, cx: &Context<Self>) {
        for (if_index, packet) in actions.transmits {
            self.to_ifs[if_index as usize].send(packet).await;
        }
        for (timer_id, delay_ns) in actions.timers {
            // Zero-delay scheduling errors in nexosim; clamp to 1 ns.
            cx.schedule_event(
                Duration::from_nanos(delay_ns.max(1)),
                schedulable!(Self::timer),
                timer_id,
            )
            .expect("timer deadline in the future");
        }
        for m in actions.metrics {
            self.metrics.send(m).await;
        }
    }

    #[nexosim(init)]
    async fn init(&mut self, cx: &Context<Self>) {
        let now = now_ns(cx);
        let actions = self.drive(now, |b, ctx| b.on_start(ctx));
        self.apply(actions, cx).await;
    }

    pub async fn frame_in(&mut self, rx: RxFrame, cx: &Context<Self>) {
        let now = now_ns(cx);
        let actions = self.drive(now, |b, ctx| b.on_frame(rx.if_index, &rx.packet, ctx));
        self.apply(actions, cx).await;
    }

    #[nexosim(schedulable)]
    async fn timer(&mut self, timer_id: u64, cx: &Context<Self>) {
        let now = now_ns(cx);
        let actions = self.drive(now, |b, ctx| b.on_timer(timer_id, ctx));
        self.apply(actions, cx).await;
    }
}
