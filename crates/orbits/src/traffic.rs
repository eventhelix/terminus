// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! What the backbone actually has to carry.
//!
//! Two flows share these links and they are not remotely the same size. The
//! conversation itself is a trickle: a token stream is a few dozen packets a
//! second of text, and a question weighs less than the answer. The working
//! memory behind it is a torrent — [`crate::placement::KvCacheModel`] puts a
//! 32,768-token context at eleven gigabytes — and every time a session changes
//! anchor, all of it moves.
//!
//! So the interesting number is not either flow on its own but their ratio,
//! and the ratio turns on one quantity the geometry already fixed: how long an
//! anchor holds a session. A conversation shorter than that dwell never
//! migrates at all and costs only its trickle. One much longer migrates
//! repeatedly, and its working memory dominates everything else on the link by
//! orders of magnitude.
//!
//! Rates here are averages over a session, not peaks. A migration is a burst —
//! eleven gigabytes as fast as the link will take it, which
//! [`crate::placement::transfer_time`] prices at under a second on a 100 Gbps
//! link — and this module spreads that burst over the interval between
//! migrations to get a sustained figure. Sizing a link for the average and
//! sizing it for the burst are different exercises; only the first is here.

use crate::placement::KvCacheModel;

/// One conversation's demand on the network.
#[derive(Debug, Clone, Copy)]
pub struct SessionProfile {
    /// Tokens per second while the model is answering.
    pub tokens_per_second: f64,
    /// Bytes on the wire per token, payload and headers together. A token is
    /// a few characters of text; the packet around it is most of this.
    pub wire_bytes_per_token: f64,
    /// Context length the conversation reaches, in tokens. This sets the
    /// working memory that has to move at every anchor change.
    pub context_tokens: u64,
    /// Fraction of the session's wall-clock time actually spent streaming
    /// tokens. A person reads, thinks, and types; the link is idle between.
    pub duty_cycle: f64,
}

impl SessionProfile {
    /// Sustained bits per second of conversation, one direction.
    ///
    /// The uplink — the question — is far smaller than the answer and is not
    /// modelled separately: at these ratios it disappears into the rounding.
    pub fn conversational_bps(&self) -> f64 {
        self.tokens_per_second * self.wire_bytes_per_token * 8.0 * self.duty_cycle
    }

    /// Working memory (bytes) that moves in one migration.
    pub fn kv_bytes(&self, model: &KvCacheModel) -> f64 {
        model.bytes(self.context_tokens)
    }

    /// Sustained bits per second of migration traffic, averaging one whole
    /// working-memory transfer over each `anchor_dwell` seconds.
    ///
    /// This is the flow the architecture exists to avoid paying twice: the
    /// mind is anchored precisely so that an *access* handover, every eleven
    /// minutes, does not move it. What remains is the anchor's own dwell,
    /// hours rather than minutes, and this is what that costs.
    pub fn migration_bps(&self, model: &KvCacheModel, anchor_dwell: f64) -> f64 {
        if anchor_dwell <= 0.0 {
            return 0.0;
        }
        self.kv_bytes(model) * 8.0 / anchor_dwell
    }

    /// How many times the conversation's working memory moves before it ends.
    ///
    /// Below one, most conversations finish on the anchor they started on and
    /// the migration flow is a tail risk rather than a steady load.
    pub fn migrations_per_session(&self, session_seconds: f64, anchor_dwell: f64) -> f64 {
        if anchor_dwell <= 0.0 {
            return 0.0;
        }
        session_seconds / anchor_dwell
    }

    /// Ratio of migration traffic to conversation traffic on the same link.
    ///
    /// The number this module exists to produce. If it is large, the backbone
    /// is a memory-moving network that happens to carry conversations, and
    /// sizing it from token rates would be off by that factor.
    pub fn migration_ratio(&self, model: &KvCacheModel, anchor_dwell: f64) -> f64 {
        let conv = self.conversational_bps();
        if conv <= 0.0 {
            return f64::INFINITY;
        }
        self.migration_bps(model, anchor_dwell) / conv
    }
}

/// Load on one link, in bits per second.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinkLoad {
    pub conversational: f64,
    pub migration: f64,
}

impl LinkLoad {
    pub fn total(&self) -> f64 {
        self.conversational + self.migration
    }
}

/// Load on a link carrying `sessions` conversations.
///
/// `migration_crossings` is how many times a migration traverses this class of
/// link. A feeder link sees two: with no links in the shell, working memory
/// travels down from the old anchor and back up to the new one, so it crosses
/// the wheel-to-shell boundary twice. A radio link sees none — the terminal is
/// never told its mind moved.
pub fn link_load(
    sessions: f64,
    profile: &SessionProfile,
    model: &KvCacheModel,
    anchor_dwell: f64,
    migration_crossings: f64,
) -> LinkLoad {
    LinkLoad {
        conversational: sessions * profile.conversational_bps(),
        migration: sessions * profile.migration_bps(model, anchor_dwell) * migration_crossings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> KvCacheModel {
        KvCacheModel {
            layers: 80,
            kv_heads: 8,
            head_dim: 128,
            bytes_per_value: 2,
        }
    }

    fn profile() -> SessionProfile {
        SessionProfile {
            tokens_per_second: 20.0,
            wire_bytes_per_token: 64.0,
            context_tokens: 32_768,
            duty_cycle: 0.3,
        }
    }

    /// The point of the whole module. A conversation is a trickle and its
    /// working memory is a torrent, and sizing the backbone from token rates
    /// would be wrong by orders of magnitude rather than by a margin.
    #[test]
    fn working_memory_dwarfs_the_conversation_it_belongs_to() {
        let p = profile();
        let dwell = 3.4 * 3_600.0; // one anchor's hold, from the shell geometry

        assert!(
            p.conversational_bps() < 10_000.0,
            "a token stream is kilobits: {} bps",
            p.conversational_bps()
        );
        assert!(
            (p.kv_bytes(&model()) - 10.7e9).abs() < 0.2e9,
            "32k context is about 11 GB, got {}",
            p.kv_bytes(&model())
        );
        assert!(
            p.migration_ratio(&model(), dwell) > 100.0,
            "migration should dominate by orders of magnitude, got {}x",
            p.migration_ratio(&model(), dwell)
        );
    }

    /// A conversation shorter than an anchor's dwell mostly never migrates,
    /// which is the whole reason the mind was anchored in the first place.
    #[test]
    fn a_short_conversation_never_moves_its_memory() {
        let p = profile();
        let dwell = 3.4 * 3_600.0;
        assert!(p.migrations_per_session(600.0, dwell) < 0.1, "ten minutes");
        assert!(
            p.migrations_per_session(6.0 * 3_600.0, dwell) > 1.0,
            "six hours"
        );
    }

    /// A feeder link carries a migration twice, because with no links in the
    /// shell the working memory goes down through the wheel and back up.
    #[test]
    fn a_migration_crosses_the_feeder_boundary_twice() {
        let p = profile();
        let dwell = 3.4 * 3_600.0;
        let once = link_load(100.0, &p, &model(), dwell, 1.0);
        let twice = link_load(100.0, &p, &model(), dwell, 2.0);
        assert_eq!(twice.migration, 2.0 * once.migration);
        assert_eq!(
            twice.conversational, once.conversational,
            "conversation does not double; only the memory does"
        );
    }

    #[test]
    fn a_radio_link_carries_no_migration_at_all() {
        let load = link_load(50.0, &profile(), &model(), 3.4 * 3_600.0, 0.0);
        assert_eq!(load.migration, 0.0);
        assert!(load.total() > 0.0);
    }
}
