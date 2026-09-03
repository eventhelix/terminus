// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Conformance checks for NodeBehavior implementations (design §3.5).
//! Any implementation — hand-written model today, wrapped real code
//! later — must be a pure function of (its state, BehaviorCtx inputs):
//! identical state + identical ctx inputs ⇒ identical Actions.

use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha12Rng;

use crate::node::{drive_behavior, NodeBehavior};

/// Panics if two clones of `behavior` diverge on an identical
/// on_start → on_timer(SEND) → on_timer(TELEMETRY) drive.
pub fn assert_behavior_deterministic<B: NodeBehavior + Clone>(behavior: B, node_id: u16) {
    let run = |mut b: B| {
        let mut rng = ChaCha12Rng::seed_from_u64(99);
        let mut next_id = 0u64;
        let mut all = Vec::new();
        all.push(drive_behavior(
            &mut b,
            node_id,
            "node:conf",
            0,
            &mut rng,
            &mut next_id,
            |b, c| b.on_start(c),
        ));
        for timer_id in [crate::node::TIMER_SEND, crate::node::TIMER_TELEMETRY] {
            all.push(drive_behavior(
                &mut b,
                node_id,
                "node:conf",
                2_000_000_000,
                &mut rng,
                &mut next_id,
                |b, c| b.on_timer(timer_id, c),
            ));
        }
        all
    };
    let a = run(behavior.clone());
    let b = run(behavior);
    assert_eq!(
        a, b,
        "NodeBehavior must be deterministic given identical state and ctx inputs"
    );
}
