//! One medium per link domain (design §3.1). Wiring is static
//! (assembly-time `attach`); connectivity is data (the channel trace).
//! Per transmission at time t: which receivers are reachable, each
//! one's propagation delay, each one's SINR → BLER → seeded-RNG drop
//! decision. Handover is simply reachability changing between packets.
//!
//! nexosim 1.0 risk-retirement note (task 5): `Vec<Output<Packet>>` as a
//! serde-derived model field compiles and works exactly as written below
//! — no fallback newtype was needed. `#[derive(Serialize, Deserialize)]`
//! on `Medium` (which contains `pub deliveries: Vec<Output<Packet>>` and
//! `pub metrics: Output<MetricRecord>`) works because `nexosim::ports::
//! Output` itself implements Serialize/Deserialize, and serde's derive
//! for `Vec<T>` needs only `T: Serialize + Deserialize` — no special
//! casing was required.

use std::time::Duration;

use nexosim::model::{schedulable, Context, Model};
use nexosim::ports::Output;
use rand::Rng;
use rand_chacha::ChaCha12Rng;
use serde::{Deserialize, Serialize};

use crate::bler::BlerCurve;
use crate::packet::{Packet, Transmission};
use crate::records::MetricRecord;
use crate::simtime::now_ns;
use crate::trace::ChannelTrace;

#[derive(Serialize, Deserialize)]
pub struct Medium {
    label: String,
    trace: ChannelTrace,
    bler: BlerCurve,
    rng: ChaCha12Rng,
    /// node id per attachment; index-aligned with `deliveries`.
    attached: Vec<u16>,
    pub deliveries: Vec<Output<Packet>>,
    pub metrics: Output<MetricRecord>,
}

#[Model]
impl Medium {
    pub fn new(label: String, trace: ChannelTrace, bler: BlerCurve, rng: ChaCha12Rng) -> Self {
        Self {
            label,
            trace,
            bler,
            rng,
            attached: Vec::new(),
            deliveries: Vec::new(),
            metrics: Output::default(),
        }
    }

    /// Assembly-time attachment. `delivery` must already be connected
    /// to the receiving interface's `NetIf::rx` input.
    pub fn attach(&mut self, node_id: u16, delivery: Output<Packet>) {
        self.attached.push(node_id);
        self.deliveries.push(delivery);
    }

    pub async fn transmit(&mut self, tx: Transmission, cx: &Context<Self>) {
        let t = now_ns(cx);
        self.metrics
            .send(MetricRecord::new(t, &self.label, "tx").packet(tx.packet.meta.id))
            .await;
        let mut reached = 0u32;
        for (i, rx_node) in self.attached.iter().enumerate() {
            if *rx_node == tx.tx_node {
                continue; // no self-delivery
            }
            let Some(sample) = self.trace.query(tx.tx_node, *rx_node, t) else {
                continue; // pair unreachable at t — physics, counted below
            };
            reached += 1;
            let bler = self.bler.bler(sample.sinr_db);
            if self.rng.random::<f64>() < bler {
                self.metrics
                    .send(MetricRecord::new(t, &self.label, "drop_bler").packet(tx.packet.meta.id))
                    .await;
                continue;
            }
            cx.schedule_event(
                Duration::from_nanos(sample.delay_ns),
                schedulable!(Self::deliver),
                (i as u32, tx.packet.clone()),
            )
            .expect("trace validation guarantees delay_ns >= 1");
        }
        if reached == 0 {
            self.metrics
                .send(MetricRecord::new(t, &self.label, "unreachable").packet(tx.packet.meta.id))
                .await;
        }
    }

    #[nexosim(schedulable)]
    async fn deliver(&mut self, arg: (u32, Packet), cx: &Context<Self>) {
        let (idx, packet) = arg;
        let t = now_ns(cx);
        self.metrics
            .send(MetricRecord::new(t, &self.label, "delivered").packet(packet.meta.id))
            .await;
        self.deliveries[idx as usize].send(packet).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexosim::ports::{event_queue, EventSinkReader, EventSource, SinkState};
    use nexosim::simulation::{EventId, Mailbox, SimInit};
    use nexosim::time::MonotonicTime;
    use std::collections::BTreeMap;

    use crate::packet::PacketMeta;
    use crate::rng::model_rng;

    const CSV: &str = "\
t_s,tx,rx,delay_us,sinr_db
0.0,a,b,3000,12.0
30.0,a,b,,
30.0,a,c,2000,12.0
";

    fn ids() -> BTreeMap<String, u16> {
        BTreeMap::from([("a".into(), 1), ("b".into(), 2), ("c".into(), 3)])
    }

    fn pkt(id: u64) -> Transmission {
        Transmission {
            tx_node: 1,
            packet: Packet { bytes: vec![1, 2, 3], meta: PacketMeta { id, birth_ns: 0, origin: 1 } },
        }
    }

    /// Medium with nodes b (idx 0) and c (idx 1) attached via sinks.
    /// good=1.0 curve rows keep BLER at 0 for sinr >= 0.
    fn bench(seed_path: &str) -> (
        nexosim::simulation::Simulation,
        EventId<Transmission>,
        impl EventSinkReader<Packet>,
        impl EventSinkReader<Packet>,
        impl EventSinkReader<MetricRecord>,
    ) {
        let trace = ChannelTrace::from_csv(CSV.as_bytes(), &ids()).unwrap();
        let bler = BlerCurve::new(vec![(-100.0, 1.0), (0.0, 0.0)]).unwrap();
        let mut medium = Medium::new("medium:t".into(), trace, bler, model_rng(1, seed_path));
        let (b_sink, b_rx) = event_queue(SinkState::Enabled);
        let mut out_b = Output::default();
        out_b.connect_sink(b_sink);
        medium.attach(2, out_b);
        let (c_sink, c_rx) = event_queue(SinkState::Enabled);
        let mut out_c = Output::default();
        out_c.connect_sink(c_sink);
        medium.attach(3, out_c);
        let (m_sink, metrics) = event_queue(SinkState::Enabled);
        medium.metrics.connect_sink(m_sink);
        let mbox = Mailbox::with_capacity(64);
        let mut bench = SimInit::with_num_threads(1);
        let tx = EventSource::new().connect(Medium::transmit, &mbox).register(&mut bench);
        let simu = bench.add_model(medium, mbox, "medium").init(MonotonicTime::EPOCH).unwrap();
        (simu, tx, b_rx, c_rx, metrics)
    }

    #[test]
    fn delivers_with_trace_delay_to_reachable_receiver_only() {
        let (mut simu, tx, mut b_rx, mut c_rx, _m) = bench("medium:t");
        simu.process_event(&tx, pkt(1)).unwrap();
        simu.step().unwrap();
        assert_eq!(simu.time(), MonotonicTime::EPOCH + Duration::from_micros(3000));
        assert_eq!(b_rx.try_read().map(|p| p.meta.id), Some(1));
        assert_eq!(c_rx.try_read().map(|p| p.meta.id), None); // a->c unreachable before 30s
    }

    #[test]
    fn reachability_switches_with_time_handover() {
        let (mut simu, tx, mut b_rx, mut c_rx, _m) = bench("medium:t");
        // Advance past 30s, then transmit: now a->b is sentinel-closed, a->c open.
        simu.step_until(MonotonicTime::EPOCH + Duration::from_secs(31)).unwrap();
        simu.process_event(&tx, pkt(2)).unwrap();
        simu.step().unwrap();
        assert_eq!(b_rx.try_read().map(|p| p.meta.id), None);
        assert_eq!(c_rx.try_read().map(|p| p.meta.id), Some(2));
    }

    #[test]
    fn unreachable_is_counted_not_errored() {
        let (mut simu, tx, _b, _c, mut metrics) = bench("medium:t");
        // tx from node 2: no (2, *) pairs in trace at all.
        let t = Transmission { tx_node: 2, packet: pkt(3).packet };
        simu.process_event(&tx, t).unwrap();
        let events: Vec<String> =
            std::iter::from_fn(|| metrics.try_read()).map(|m| m.event).collect();
        assert!(events.contains(&"unreachable".to_string()));
    }

    #[test]
    fn bler_one_drops_everything_deterministically() {
        // Curve pinned at 1.0 regardless of SINR → every frame drops.
        let trace = ChannelTrace::from_csv(CSV.as_bytes(), &ids()).unwrap();
        let bler = BlerCurve::new(vec![(-100.0, 1.0)]).unwrap();
        let mut medium = Medium::new("medium:t".into(), trace, bler, model_rng(1, "medium:t"));
        let (b_sink, mut b_rx) = event_queue(SinkState::Enabled);
        let mut out_b = Output::default();
        out_b.connect_sink(b_sink);
        medium.attach(2, out_b);
        let (m_sink, mut metrics) = event_queue(SinkState::Enabled);
        medium.metrics.connect_sink(m_sink);
        let mbox = Mailbox::with_capacity(64);
        let mut bench = SimInit::with_num_threads(1);
        let tx = EventSource::new().connect(Medium::transmit, &mbox).register(&mut bench);
        let mut simu = bench.add_model(medium, mbox, "medium").init(MonotonicTime::EPOCH).unwrap();
        simu.process_event(&tx, pkt(9)).unwrap();
        simu.run().unwrap();
        assert_eq!(b_rx.try_read(), None);
        let events: Vec<String> =
            std::iter::from_fn(|| metrics.try_read()).map(|m| m.event).collect();
        assert!(events.contains(&"drop_bler".to_string()));
    }
}
