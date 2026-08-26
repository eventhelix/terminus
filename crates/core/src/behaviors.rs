//! Slice-1 behaviors: UDP-echo terminal (CBR with optional burst),
//! satellite relay (+ periodic link telemetry), gateway echo server.
//! All addressing derives from the LinkFrame bytes (dst/src node ids);
//! IPs are the fixed scheme 10.0.0.<node_id> (config validates id <= 250).
//! Undecodable received bytes → `decode_error` metric, dropped, never
//! panicked (design §3.5). Encode failures → expect() (model bug).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use terminus_protocols::link::{ControlFrame, DataFrame, LinkFrame, LinkFrameChild};
use terminus_protocols::udp::{build_udp_ipv4, parse_udp_ipv4};
use terminus_protocols::PdlPacket;

use crate::node::{BehaviorCtx, NodeBehavior, TIMER_SEND, TIMER_TELEMETRY};
use crate::packet::Packet;

pub const TELEMETRY_OPCODE: u8 = 1;
const TTL: u8 = 64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BehaviorKind {
    Terminal(TerminalApp),
    Relay(Relay),
    Gateway(GatewayEcho),
}

impl NodeBehavior for BehaviorKind {
    fn on_start(&mut self, ctx: &mut BehaviorCtx) {
        match self {
            BehaviorKind::Terminal(b) => b.on_start(ctx),
            BehaviorKind::Relay(b) => b.on_start(ctx),
            BehaviorKind::Gateway(b) => b.on_start(ctx),
        }
    }
    fn on_frame(&mut self, if_index: u8, packet: &Packet, ctx: &mut BehaviorCtx) {
        match self {
            BehaviorKind::Terminal(b) => b.on_frame(if_index, packet, ctx),
            BehaviorKind::Relay(b) => b.on_frame(if_index, packet, ctx),
            BehaviorKind::Gateway(b) => b.on_frame(if_index, packet, ctx),
        }
    }
    fn on_timer(&mut self, timer_id: u64, ctx: &mut BehaviorCtx) {
        match self {
            BehaviorKind::Terminal(b) => b.on_timer(timer_id, ctx),
            BehaviorKind::Relay(b) => b.on_timer(timer_id, ctx),
            BehaviorKind::Gateway(b) => b.on_timer(timer_id, ctx),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Burst {
    pub start_ns: u64,
    pub end_ns: u64,
    pub rate_pps: f64,
}

/// UDP echo client: CBR sender toward `peer`, matches replies by the
/// 4-byte big-endian seq prefix in the UDP payload, reports RTT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalApp {
    pub peer: u16,
    pub src_port: u16,
    pub dst_port: u16,
    pub payload_len: usize, // >= 4, validated at config load
    pub rate_pps: f64,
    pub burst: Option<Burst>,
    pub start_ns: u64,
    pub end_ns: u64,
    pub seq: u32,
    pub sent: BTreeMap<u32, u64>, // seq -> t_sent_ns
}

impl TerminalApp {
    fn rate_at(&self, t_ns: u64) -> f64 {
        match self.burst {
            Some(b) if t_ns >= b.start_ns && t_ns < b.end_ns => b.rate_pps,
            _ => self.rate_pps,
        }
    }

    fn send_echo(&mut self, ctx: &mut BehaviorCtx) {
        let seq = self.seq;
        self.seq += 1;
        let mut payload = vec![0u8; self.payload_len];
        payload[..4].copy_from_slice(&seq.to_be_bytes());
        let ip = build_udp_ipv4(
            [10, 0, 0, ctx.node_id as u8],
            [10, 0, 0, self.peer as u8],
            self.src_port,
            self.dst_port,
            TTL,
            &payload,
        );
        let frame = DataFrame {
            version: 1,
            src: ctx.node_id,
            dst: self.peer,
            seq,
            flow_id: 0,
            payload: ip,
        };
        let bytes = frame.encode_to_vec().expect("encode failure is a model bug");
        self.sent.insert(seq, ctx.now_ns);
        let id = ctx.transmit_new(0, bytes);
        ctx.metric("echo_sent").packet_id = Some(id);
    }
}

impl NodeBehavior for TerminalApp {
    fn on_start(&mut self, ctx: &mut BehaviorCtx) {
        ctx.timer_in(TIMER_SEND, self.start_ns.max(1));
    }

    fn on_timer(&mut self, timer_id: u64, ctx: &mut BehaviorCtx) {
        if timer_id != TIMER_SEND || ctx.now_ns >= self.end_ns {
            return;
        }
        self.send_echo(ctx);
        let interval_ns = (1e9 / self.rate_at(ctx.now_ns)) as u64;
        ctx.timer_in(TIMER_SEND, interval_ns);
    }

    fn on_frame(&mut self, _if_index: u8, packet: &Packet, ctx: &mut BehaviorCtx) {
        let Ok(frame) = LinkFrame::decode_full(&packet.bytes) else {
            ctx.metric("decode_error");
            return;
        };
        if frame.dst != ctx.node_id {
            return; // overheard broadcast — not for us
        }
        let Ok(LinkFrameChild::DataFrame(d)) = frame.specialize() else {
            return; // control or unknown child — terminals ignore
        };
        let Some(udp) = parse_udp_ipv4(&d.payload) else {
            ctx.metric("decode_error");
            return;
        };
        if udp.dst_port != self.src_port || udp.payload.len() < 4 {
            return;
        }
        let seq = u32::from_be_bytes(udp.payload[..4].try_into().unwrap());
        if let Some(t_sent) = self.sent.remove(&seq) {
            let rtt = ctx.now_ns - t_sent;
            let m = ctx.metric("echo_rtt");
            m.packet_id = Some(packet.meta.id);
            m.value_ns = Some(rtt);
        }
    }
}

/// Satellite: forwards frames not addressed to it out the "other"
/// interface (static slice-1 routing: if_map[rx_if] = tx_if), and
/// emits periodic link-telemetry ControlFrames toward the gateway.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relay {
    pub if_map: Vec<u8>,
    pub telemetry_peer: u16,
    pub telemetry_if: u8,
    pub telemetry_period_ns: u64,
    pub seq: u32,
}

impl NodeBehavior for Relay {
    fn on_start(&mut self, ctx: &mut BehaviorCtx) {
        ctx.timer_in(TIMER_TELEMETRY, self.telemetry_period_ns);
    }

    fn on_timer(&mut self, timer_id: u64, ctx: &mut BehaviorCtx) {
        if timer_id != TIMER_TELEMETRY {
            return;
        }
        let frame = ControlFrame {
            version: 1,
            src: ctx.node_id,
            dst: self.telemetry_peer,
            seq: self.seq,
            opcode: TELEMETRY_OPCODE,
            args: vec![],
        };
        self.seq += 1;
        let bytes = frame.encode_to_vec().expect("encode failure is a model bug");
        let id = ctx.transmit_new(self.telemetry_if, bytes);
        ctx.metric("telemetry_sent").packet_id = Some(id);
        ctx.timer_in(TIMER_TELEMETRY, self.telemetry_period_ns);
    }

    fn on_frame(&mut self, if_index: u8, packet: &Packet, ctx: &mut BehaviorCtx) {
        let Ok(frame) = LinkFrame::decode_full(&packet.bytes) else {
            ctx.metric("decode_error");
            return;
        };
        if frame.dst == ctx.node_id {
            ctx.metric("telemetry_rcvd");
            return;
        }
        let out_if = self.if_map[if_index as usize];
        ctx.forward(out_if, packet.clone());
        ctx.metric("forward").packet_id = Some(packet.meta.id);
    }
}

/// Gateway: UDP echo server on `port` + telemetry sink.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayEcho {
    pub port: u16,
    pub seq: u32,
}

impl NodeBehavior for GatewayEcho {
    fn on_start(&mut self, _ctx: &mut BehaviorCtx) {}
    fn on_timer(&mut self, _timer_id: u64, _ctx: &mut BehaviorCtx) {}

    fn on_frame(&mut self, _if_index: u8, packet: &Packet, ctx: &mut BehaviorCtx) {
        let Ok(frame) = LinkFrame::decode_full(&packet.bytes) else {
            ctx.metric("decode_error");
            return;
        };
        if frame.dst != ctx.node_id {
            return;
        }
        match frame.specialize() {
            Ok(LinkFrameChild::DataFrame(d)) => {
                let Some(udp) = parse_udp_ipv4(&d.payload) else {
                    ctx.metric("decode_error");
                    return;
                };
                if udp.dst_port != self.port {
                    return;
                }
                let reply_ip = build_udp_ipv4(
                    udp.dst_ip,
                    udp.src_ip,
                    udp.dst_port,
                    udp.src_port,
                    TTL,
                    &udp.payload,
                );
                let reply = DataFrame {
                    version: 1,
                    src: ctx.node_id,
                    dst: d.src,
                    seq: self.seq,
                    flow_id: 0,
                    payload: reply_ip,
                };
                self.seq += 1;
                let bytes = reply.encode_to_vec().expect("encode failure is a model bug");
                let id = ctx.transmit_new(0, bytes);
                ctx.metric("echo_reply").packet_id = Some(id);
            }
            Ok(LinkFrameChild::ControlFrame(_)) => {
                ctx.metric("telemetry_rcvd").packet_id = Some(packet.meta.id);
            }
            _ => {
                ctx.metric("decode_error");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conformance::assert_behavior_deterministic;
    use crate::node::{drive_behavior, TIMER_SEND};
    use crate::packet::PacketMeta;
    use rand_chacha::rand_core::SeedableRng;
    use rand_chacha::ChaCha12Rng;

    fn terminal() -> TerminalApp {
        TerminalApp {
            peer: 6,
            src_port: 4001,
            dst_port: 7,
            payload_len: 64,
            rate_pps: 10.0,
            burst: Some(Burst { start_ns: 40_000_000_000, end_ns: 45_000_000_000, rate_pps: 500.0 }),
            start_ns: 1_000_000_000,
            end_ns: 60_000_000_000,
            seq: 0,
            sent: BTreeMap::new(),
        }
    }

    fn drive<B: NodeBehavior>(
        b: &mut B,
        node_id: u16,
        now_ns: u64,
        f: impl FnOnce(&mut B, &mut crate::node::BehaviorCtx),
    ) -> crate::node::Actions {
        let mut rng = ChaCha12Rng::seed_from_u64(1);
        let mut next_id = 0;
        drive_behavior(b, node_id, "node:test", now_ns, &mut rng, &mut next_id, f)
    }

    #[test]
    fn terminal_emits_valid_data_frame_and_reschedules() {
        let mut t = terminal();
        let a = drive(&mut t, 1, 2_000_000_000, |b, c| b.on_timer(TIMER_SEND, c));
        assert_eq!(a.transmits.len(), 1);
        let (if_idx, pkt) = &a.transmits[0];
        assert_eq!(*if_idx, 0);
        let frame = LinkFrame::decode_full(&pkt.bytes).unwrap();
        assert_eq!((frame.src, frame.dst), (1, 6));
        // 10 pps → next send in 100 ms
        assert_eq!(a.timers, vec![(TIMER_SEND, 100_000_000)]);
        assert_eq!(t.sent.len(), 1);
    }

    #[test]
    fn terminal_burst_window_speeds_up() {
        let mut t = terminal();
        let a = drive(&mut t, 1, 41_000_000_000, |b, c| b.on_timer(TIMER_SEND, c));
        assert_eq!(a.timers, vec![(TIMER_SEND, 2_000_000)]); // 500 pps → 2 ms
    }

    #[test]
    fn terminal_matches_reply_and_reports_rtt() {
        let mut t = terminal();
        drive(&mut t, 1, 2_000_000_000, |b, c| b.on_timer(TIMER_SEND, c));
        // Build the reply the gateway would send for seq 0.
        let mut payload = vec![0u8; 64];
        payload[..4].copy_from_slice(&0u32.to_be_bytes());
        let ip = build_udp_ipv4([10, 0, 0, 6], [10, 0, 0, 1], 7, 4001, 64, &payload);
        let reply = DataFrame { version: 1, src: 6, dst: 1, seq: 0, flow_id: 0, payload: ip };
        let pkt = Packet {
            bytes: reply.encode_to_vec().unwrap(),
            meta: PacketMeta { id: 9, birth_ns: 0, origin: 6 },
        };
        let a = drive(&mut t, 1, 2_020_000_000, |b, c| b.on_frame(0, &pkt, c));
        let rtt = a.metrics.iter().find(|m| m.event == "echo_rtt").expect("rtt metric");
        assert_eq!(rtt.value_ns, Some(20_000_000));
        assert!(t.sent.is_empty());
    }

    #[test]
    fn relay_forwards_out_other_interface_preserving_meta() {
        let mut r = Relay {
            if_map: vec![1, 0],
            telemetry_peer: 6,
            telemetry_if: 1,
            telemetry_period_ns: 5_000_000_000,
            seq: 0,
        };
        let data = DataFrame { version: 1, src: 1, dst: 6, seq: 3, flow_id: 0, payload: vec![0x45] };
        let pkt = Packet {
            bytes: data.encode_to_vec().unwrap(),
            meta: PacketMeta { id: 77, birth_ns: 0, origin: 1 },
        };
        let a = drive(&mut r, 3, 5_000_000_000, |b, c| b.on_frame(0, &pkt, c));
        assert_eq!(a.transmits.len(), 1);
        assert_eq!(a.transmits[0].0, 1, "access rx → feeder tx");
        assert_eq!(a.transmits[0].1.meta.id, 77, "meta preserved across relay");
        assert_eq!(a.transmits[0].1.bytes, pkt.bytes, "bytes untouched");
    }

    #[test]
    fn relay_drops_undecodable_with_counter() {
        let mut r = Relay {
            if_map: vec![1, 0],
            telemetry_peer: 6,
            telemetry_if: 1,
            telemetry_period_ns: 5_000_000_000,
            seq: 0,
        };
        let pkt = Packet { bytes: vec![0xFF, 0xFF], meta: PacketMeta { id: 1, birth_ns: 0, origin: 1 } };
        let a = drive(&mut r, 3, 1_000_000_000, |b, c| b.on_frame(0, &pkt, c));
        assert!(a.transmits.is_empty());
        assert_eq!(a.metrics[0].event, "decode_error");
    }

    #[test]
    fn gateway_echoes_udp_swapped() {
        let mut g = GatewayEcho { port: 7, seq: 0 };
        let mut payload = vec![0u8; 16];
        payload[..4].copy_from_slice(&5u32.to_be_bytes());
        let ip = build_udp_ipv4([10, 0, 0, 1], [10, 0, 0, 6], 4001, 7, 64, &payload);
        let req = DataFrame { version: 1, src: 1, dst: 6, seq: 5, flow_id: 0, payload: ip };
        let pkt = Packet {
            bytes: req.encode_to_vec().unwrap(),
            meta: PacketMeta { id: 5, birth_ns: 0, origin: 1 },
        };
        let a = drive(&mut g, 6, 1_000_000_000, |b, c| b.on_frame(0, &pkt, c));
        assert_eq!(a.transmits.len(), 1);
        let frame = LinkFrame::decode_full(&a.transmits[0].1.bytes).unwrap();
        assert_eq!((frame.src, frame.dst), (6, 1));
        let LinkFrameChild::DataFrame(d) = frame.specialize().unwrap() else { panic!() };
        let udp = parse_udp_ipv4(&d.payload).unwrap();
        assert_eq!((udp.src_port, udp.dst_port), (7, 4001));
        assert_eq!(udp.src_ip, [10, 0, 0, 6]);
        assert_eq!(udp.payload, payload, "echo payload byte-identical");
    }

    #[test]
    fn behaviors_are_deterministic() {
        assert_behavior_deterministic(terminal(), 1);
        assert_behavior_deterministic(
            Relay { if_map: vec![1, 0], telemetry_peer: 6, telemetry_if: 1, telemetry_period_ns: 5_000_000_000, seq: 0 },
            3,
        );
        assert_behavior_deterministic(GatewayEcho { port: 7, seq: 0 }, 6);
    }
}
