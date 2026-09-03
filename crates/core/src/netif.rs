// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Deliberately dumb network interface (design §3.1): capture tap
//! attachment point (like tcpdump on a NIC) and up/down state. Tx-side
//! tap records every send (including frames the medium later drops);
//! rx-side tap records only deliveries — losses are visible by diffing
//! vantages, as on real hardware.

use nexosim::model::{Context, Model};
use nexosim::ports::Output;
use serde::{Deserialize, Serialize};

use crate::packet::{Packet, RxFrame, Transmission};
use crate::records::{CaptureRecord, Direction, MetricRecord};
use crate::simtime::now_ns;

#[derive(Serialize, Deserialize)]
pub struct NetIf {
    node_id: u16,
    if_index: u8,
    label: String,
    up: bool,
    pub to_medium: Output<Transmission>,
    pub to_compute: Output<RxFrame>,
    pub capture: Output<CaptureRecord>,
    pub metrics: Output<MetricRecord>,
}

#[Model]
impl NetIf {
    pub fn new(node_id: u16, if_index: u8, label: String) -> Self {
        Self {
            node_id,
            if_index,
            label,
            up: true,
            to_medium: Output::default(),
            to_compute: Output::default(),
            capture: Output::default(),
            metrics: Output::default(),
        }
    }

    pub async fn tx(&mut self, packet: Packet, cx: &Context<Self>) {
        let t = now_ns(cx);
        if !self.up {
            self.metrics
                .send(MetricRecord::new(t, &self.label, "tx_down"))
                .await;
            return;
        }
        self.capture
            .send(CaptureRecord {
                node: self.node_id,
                if_index: self.if_index,
                t_ns: t,
                dir: Direction::Tx,
                bytes: packet.bytes.clone(),
            })
            .await;
        self.to_medium
            .send(Transmission {
                tx_node: self.node_id,
                packet,
            })
            .await;
    }

    pub async fn rx(&mut self, packet: Packet, cx: &Context<Self>) {
        let t = now_ns(cx);
        if !self.up {
            self.metrics
                .send(MetricRecord::new(t, &self.label, "rx_down"))
                .await;
            return;
        }
        self.capture
            .send(CaptureRecord {
                node: self.node_id,
                if_index: self.if_index,
                t_ns: t,
                dir: Direction::Rx,
                bytes: packet.bytes.clone(),
            })
            .await;
        self.to_compute
            .send(RxFrame {
                if_index: self.if_index,
                packet,
            })
            .await;
    }

    pub fn set_up(&mut self, up: bool) {
        self.up = up;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexosim::ports::{event_queue, EventSinkReader, EventSource, SinkState};
    use nexosim::simulation::{EventId, Mailbox, SimInit};
    use nexosim::time::MonotonicTime;

    use crate::packet::PacketMeta;

    fn pkt(id: u64) -> Packet {
        Packet {
            bytes: vec![0xDE, 0xAD],
            meta: PacketMeta {
                id,
                birth_ns: 0,
                origin: 7,
            },
        }
    }

    // Four `impl EventSinkReader<...>` members, which cannot be named in a
    // type alias on stable Rust — there is no factoring this lint would
    // accept short of boxing the sinks in test code.
    #[allow(clippy::type_complexity)]
    fn bench() -> (
        nexosim::simulation::Simulation,
        EventId<Packet>,
        EventId<Packet>,
        EventId<bool>,
        impl EventSinkReader<Transmission>,
        impl EventSinkReader<RxFrame>,
        impl EventSinkReader<CaptureRecord>,
        impl EventSinkReader<MetricRecord>,
    ) {
        let mut netif = NetIf::new(7, 0, "netif:n7:if0".into());
        let (med_sink, to_medium) = event_queue(SinkState::Enabled);
        netif.to_medium.connect_sink(med_sink);
        let (cmp_sink, to_compute) = event_queue(SinkState::Enabled);
        netif.to_compute.connect_sink(cmp_sink);
        let (cap_sink, capture) = event_queue(SinkState::Enabled);
        netif.capture.connect_sink(cap_sink);
        let (m_sink, metrics) = event_queue(SinkState::Enabled);
        netif.metrics.connect_sink(m_sink);
        let mbox = Mailbox::with_capacity(64);
        let mut bench = SimInit::with_num_threads(1);
        let tx = EventSource::new()
            .connect(NetIf::tx, &mbox)
            .register(&mut bench);
        let rx = EventSource::new()
            .connect(NetIf::rx, &mbox)
            .register(&mut bench);
        let up = EventSource::new()
            .connect(NetIf::set_up, &mbox)
            .register(&mut bench);
        let simu = bench
            .add_model(netif, mbox, "netif")
            .init(MonotonicTime::EPOCH)
            .unwrap();
        (simu, tx, rx, up, to_medium, to_compute, capture, metrics)
    }

    #[test]
    fn tx_captures_then_forwards_to_medium() {
        let (mut simu, tx, _rx, _up, mut to_medium, _tc, mut capture, _m) = bench();
        simu.process_event(&tx, pkt(1)).unwrap();
        let cap = capture.try_read().unwrap();
        assert_eq!(cap.dir, Direction::Tx);
        assert_eq!(cap.bytes, vec![0xDE, 0xAD]);
        let t = to_medium.try_read().unwrap();
        assert_eq!(t.tx_node, 7);
        assert_eq!(t.packet.meta.id, 1);
    }

    #[test]
    fn rx_captures_then_forwards_to_compute() {
        let (mut simu, _tx, rx, _up, _tm, mut to_compute, mut capture, _m) = bench();
        simu.process_event(&rx, pkt(2)).unwrap();
        assert_eq!(capture.try_read().unwrap().dir, Direction::Rx);
        let f = to_compute.try_read().unwrap();
        assert_eq!(f.if_index, 0);
        assert_eq!(f.packet.meta.id, 2);
    }

    #[test]
    fn down_interface_swallows_traffic() {
        let (mut simu, tx, rx, up, mut to_medium, mut to_compute, mut capture, mut metrics) =
            bench();
        simu.process_event(&up, false).unwrap();
        simu.process_event(&tx, pkt(3)).unwrap();
        simu.process_event(&rx, pkt(4)).unwrap();
        assert!(to_medium.try_read().is_none());
        assert!(to_compute.try_read().is_none());
        assert!(capture.try_read().is_none());
        assert_eq!(
            metrics.try_read().map(|m| m.event),
            Some("tx_down".to_string())
        );
        assert_eq!(
            metrics.try_read().map(|m| m.event),
            Some("rx_down".to_string())
        );
        assert!(metrics.try_read().is_none());
    }
}
