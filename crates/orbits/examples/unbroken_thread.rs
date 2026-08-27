//! Why FEC and ARQ work in tandem on the terminal↔anchor path: the ARQ
//! round-trip price, the stall arithmetic of a token stream under loss, and
//! how proactive repair overhead stretches the interval between stalls —
//! in calm and in flare conditions.
//!
//! Run: cargo run -p terminus-orbits --example unbroken_thread

use terminus_orbits::reliability::{arq_stall_interval, fec_residual_rate, fec_stall_interval};

const PACKET_RATE: f64 = 20.0; // token stream, packets/s
const RTT_MS: f64 = 180.0; // worst-geometry terminal↔anchor round trip
const STALL_BUDGET_MS: f64 = 100.0; // TER-REQ-003 p99 token stall

fn human(seconds: f64) -> String {
    if seconds < 60.0 {
        format!("{seconds:.1} s")
    } else if seconds < 3_600.0 {
        format!("{:.1} min", seconds / 60.0)
    } else if seconds < 86_400.0 {
        format!("{:.1} h", seconds / 3_600.0)
    } else {
        format!("{:.1} days", seconds / 86_400.0)
    }
}

fn main() {
    println!(
        "Token stream {PACKET_RATE:.0} packets/s; ARQ retransmission costs one\n\
         round trip: {RTT_MS:.0} ms — {:.1}x the {STALL_BUDGET_MS:.0} ms stall budget.\n",
        RTT_MS / STALL_BUDGET_MS
    );

    for (label, p) in [
        ("calm sky (1% residual loss)", 0.01),
        ("flare / handover (5% residual loss)", 0.05),
    ] {
        println!("{label}:");
        println!(
            "  ARQ only:              a {RTT_MS:.0} ms stall every {}",
            human(arq_stall_interval(PACKET_RATE, p))
        );
        for (k, r) in [(16_u64, 2_u64), (16, 4), (16, 8)] {
            println!(
                "  FEC {k}+{r} ({:>4.1}% overhead): residual {:.1e} ⇒ a stall every {}",
                r as f64 / k as f64 * 100.0,
                fec_residual_rate(k, r, p),
                human(fec_stall_interval(PACKET_RATE, k, r, p))
            );
        }
        println!();
    }

    println!(
        "FEC is the reflex: repair packets ride along and heal losses in zero\n\
         extra round trips. ARQ is the guarantee: whatever exceeds the repair\n\
         budget — or vanishes with a failed node — is retransmitted end to\n\
         end. The timetable breathes the overhead: lean in calm, generous\n\
         ahead of scheduled handovers and during flares.\n"
    );

    let access_rtt_ms = 24.3; // 2 × 12.15 ms edge one-way (coverage.rs)
    let slot_ms = 1.0;
    println!(
        "PHY contract (delegated per ADR-0011): residual erasures ≤1% calm /\n\
         ≤5% flare after PHY FEC + HARQ. HARQ soft-combining runs per radio\n\
         hop, where soft symbols live: the {access_rtt_ms:.1} ms access feedback loop\n\
         at {slot_ms:.0} ms slots keeps ~{:.0} transmissions in flight ⇒ ≥32 parallel\n\
         HARQ processes.",
        access_rtt_ms / slot_ms
    );
}
