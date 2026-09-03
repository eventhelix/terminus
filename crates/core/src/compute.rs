// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Processing-latency model (design §3.1): N cores + bounded FIFO.
//! Free core → completion at now + service_ns; else queue; overflow →
//! drop + counter. This concrete model defines the compute port
//! contract (see Interfaces in the plan / trait docs below); a
//! software-architecture model or measured real-code timings replace it
//! per node later by exposing the same ports and passing the same
//! contract tests (parameterize the constructor in `tests`).
//!
//! nexosim 1.0 risk-retirement note (this is the first nexosim model in
//! the codebase): `#[nexosim(schedulable)]` methods taking `cx: &Context<Self>`
//! compile and work exactly as written below — no fallback to a self-loop
//! output was needed. `schedulable!(Self::method)` + `cx.schedule_event(
//! Duration, .., arg)` is the working pattern for self-rescheduling from
//! within a model method; `#[Model]` on the impl block is likewise
//! sufficient (no extra derives beyond Serialize+Deserialize on the
//! struct). One path correction vs. the brief: `EventId` lives in
//! `nexosim::simulation`, not `nexosim::ports` (see the test bench below).
//! Tasks 5 and 11 can use this same shape directly.

use std::collections::VecDeque;
use std::time::Duration;

use nexosim::model::{schedulable, Context, Model};
use nexosim::ports::Output;
use serde::{Deserialize, Serialize};

use crate::packet::RxFrame;
use crate::records::MetricRecord;
use crate::simtime::now_ns;

#[derive(Serialize, Deserialize)]
pub struct FifoCompute {
    label: String,
    cores: u32,
    busy: u32,
    queue: VecDeque<RxFrame>,
    capacity: usize,
    service_ns: u64,
    pub done: Output<RxFrame>,
    pub metrics: Output<MetricRecord>,
}

#[Model]
impl FifoCompute {
    pub fn new(label: String, cores: u32, capacity: usize, service_ns: u64) -> Self {
        assert!(
            cores >= 1 && capacity >= 1 && service_ns >= 1,
            "validated at config load"
        );
        Self {
            label,
            cores,
            busy: 0,
            queue: VecDeque::new(),
            capacity,
            service_ns,
            done: Output::default(),
            metrics: Output::default(),
        }
    }

    pub async fn submit(&mut self, item: RxFrame, cx: &Context<Self>) {
        let t = now_ns(cx);
        if self.busy < self.cores {
            self.busy += 1;
            cx.schedule_event(
                Duration::from_nanos(self.service_ns),
                schedulable!(Self::complete),
                item.clone(),
            )
            .expect("service_ns >= 1, deadline is in the future");
            self.metrics
                .send(MetricRecord::new(t, &self.label, "submit").queue(self.queue.len() as u32))
                .await;
        } else if self.queue.len() < self.capacity {
            self.queue.push_back(item);
            self.metrics
                .send(MetricRecord::new(t, &self.label, "submit").queue(self.queue.len() as u32))
                .await;
        } else {
            self.metrics
                .send(
                    MetricRecord::new(t, &self.label, "drop_overflow")
                        .packet(item.packet.meta.id)
                        .queue(self.queue.len() as u32),
                )
                .await;
        }
    }

    #[nexosim(schedulable)]
    async fn complete(&mut self, item: RxFrame, cx: &Context<Self>) {
        let t = now_ns(cx);
        self.done.send(item).await;
        if let Some(next) = self.queue.pop_front() {
            cx.schedule_event(
                Duration::from_nanos(self.service_ns),
                schedulable!(Self::complete),
                next,
            )
            .expect("service_ns >= 1, deadline is in the future");
        } else {
            self.busy -= 1;
        }
        self.metrics
            .send(MetricRecord::new(t, &self.label, "done").queue(self.queue.len() as u32))
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexosim::ports::{event_queue, EventSinkReader, EventSource, SinkState};
    use nexosim::simulation::{EventId, Mailbox, SimInit};
    use nexosim::time::MonotonicTime;

    use crate::packet::{Packet, PacketMeta};

    fn frame(id: u64) -> RxFrame {
        RxFrame {
            if_index: 0,
            packet: Packet {
                bytes: vec![0xAA; 4],
                meta: PacketMeta {
                    id,
                    birth_ns: 0,
                    origin: 1,
                },
            },
        }
    }

    /// Bench: EventSource → submit; done + metrics into queues.
    fn bench(
        mk: impl FnOnce() -> FifoCompute,
    ) -> (
        nexosim::simulation::Simulation,
        EventId<RxFrame>,
        impl EventSinkReader<RxFrame>,
        impl EventSinkReader<MetricRecord>,
    ) {
        let mut compute = mk();
        let mbox = Mailbox::with_capacity(64);
        let mut bench = SimInit::with_num_threads(1);
        let submit = EventSource::new()
            .connect(FifoCompute::submit, &mbox)
            .register(&mut bench);
        let (done_sink, done) = event_queue(SinkState::Enabled);
        compute.done.connect_sink(done_sink);
        let (m_sink, metrics) = event_queue(SinkState::Enabled);
        compute.metrics.connect_sink(m_sink);
        let simu = bench
            .add_model(compute, mbox, "compute")
            .init(MonotonicTime::EPOCH)
            .unwrap();
        (simu, submit, done, metrics)
    }

    #[test]
    fn service_latency_single_item() {
        let (mut simu, submit, mut done, _m) =
            bench(|| FifoCompute::new("compute:t".into(), 1, 4, 1_000_000));
        simu.process_event(&submit, frame(1)).unwrap();
        simu.step().unwrap();
        assert_eq!(simu.time(), MonotonicTime::EPOCH + Duration::from_millis(1));
        assert_eq!(done.try_read().map(|f| f.packet.meta.id), Some(1));
    }

    #[test]
    fn fifo_queueing_serializes_service() {
        // 1 core, 3 items at t=0 → completions at 1ms, 2ms, 3ms in order.
        let (mut simu, submit, mut done, _m) =
            bench(|| FifoCompute::new("compute:t".into(), 1, 4, 1_000_000));
        for id in 1..=3 {
            simu.process_event(&submit, frame(id)).unwrap();
        }
        simu.step_until(MonotonicTime::EPOCH + Duration::from_millis(10))
            .unwrap();
        let ids: Vec<u64> = std::iter::from_fn(|| done.try_read())
            .map(|f| f.packet.meta.id)
            .collect();
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn overflow_drops_and_counts() {
        // 1 core, capacity 1: third simultaneous item must drop.
        let (mut simu, submit, mut done, mut metrics) =
            bench(|| FifoCompute::new("compute:t".into(), 1, 1, 1_000_000));
        for id in 1..=3 {
            simu.process_event(&submit, frame(id)).unwrap();
        }
        simu.step_until(MonotonicTime::EPOCH + Duration::from_millis(10))
            .unwrap();
        let ids: Vec<u64> = std::iter::from_fn(|| done.try_read())
            .map(|f| f.packet.meta.id)
            .collect();
        assert_eq!(ids, vec![1, 2]);
        let dropped: Vec<MetricRecord> = std::iter::from_fn(|| metrics.try_read())
            .filter(|m| m.event == "drop_overflow")
            .collect();
        assert_eq!(dropped.len(), 1);
        assert_eq!(dropped[0].packet_id, Some(3));
    }

    #[test]
    fn multicore_runs_in_parallel() {
        // 2 cores: two items at t=0 both complete at 1ms.
        let (mut simu, submit, mut done, _m) =
            bench(|| FifoCompute::new("compute:t".into(), 2, 4, 1_000_000));
        simu.process_event(&submit, frame(1)).unwrap();
        simu.process_event(&submit, frame(2)).unwrap();
        simu.step().unwrap();
        assert_eq!(simu.time(), MonotonicTime::EPOCH + Duration::from_millis(1));
        let ids: Vec<u64> = std::iter::from_fn(|| done.try_read())
            .map(|f| f.packet.meta.id)
            .collect();
        assert_eq!(ids, vec![1, 2]);
    }
}
