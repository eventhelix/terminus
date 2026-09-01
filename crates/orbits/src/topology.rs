// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! How many optical terminals the backbone actually costs.
//!
//! A laser link is not a shared medium. Each end needs its own terminal —
//! aperture, steering, laser, detector, and the acquisition and tracking that
//! keeps two moving telescopes locked on each other — so a satellite holding
//! `n` links at once carries `n` terminals, and terminal count drives mass,
//! power, and cost long before capacity does.
//!
//! That makes "which links exist" a hardware question rather than a routing
//! one, and the answer is not the obvious one. The simplest possible topology
//! — no ring-to-ring links, no anchor-to-anchor links, every session reaching
//! its own anchor directly — is also the most expensive, and the reason is
//! [`crate::handover`]'s retention rule. A session keeps its anchor across
//! roughly nineteen access handovers, so the sessions riding any one access
//! satellite were anchored at different moments from different places and are
//! scattered across the shell. A direct topology has to turn every one of
//! those (access satellite, anchor) pairs into hardware.
//!
//! Feeding one gateway anchor and letting the shell carry traffic onward
//! collapses the wheel's side to a single terminal. The cost moves rather than
//! vanishing: the shell then needs anchor-to-anchor links, which a four
//! satellite plane cannot fully close — see
//! [`crate::backbone::intra_plane_reach`].

use std::collections::{BTreeMap, BTreeSet};

/// One session: which access satellite carries it, which anchor holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Session {
    /// Flat index of the serving access satellite.
    pub access: usize,
    /// Index of the anchor holding the working memory.
    pub anchor: usize,
}

/// Simultaneous links each side must hold, at one instant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalDemand {
    /// Terminals needed on each access satellite carrying anything, ascending,
    /// so a caller can take a maximum or a quantile without re-deriving the
    /// distribution.
    pub per_access: Vec<usize>,
    /// The same for each anchor.
    pub per_anchor: Vec<usize>,
    /// Distinct (access, anchor) pairs: how many links are in the sky.
    pub links: usize,
}

impl TerminalDemand {
    /// Most terminals any one access satellite must hold. This is what the
    /// spacecraft has to be built for; the mean is what it will usually use,
    /// and building for the mean means dropping sessions at the peak.
    pub fn max_access(&self) -> usize {
        self.per_access.last().copied().unwrap_or(0)
    }

    /// Most feeder terminals any one anchor must hold.
    pub fn max_anchor(&self) -> usize {
        self.per_anchor.last().copied().unwrap_or(0)
    }

    /// Quantile of the per-access-satellite demand, `q` in [0, 1].
    pub fn access_quantile(&self, q: f64) -> usize {
        quantile(&self.per_access, q)
    }

    /// Quantile of the per-anchor demand, `q` in [0, 1].
    pub fn anchor_quantile(&self, q: f64) -> usize {
        quantile(&self.per_anchor, q)
    }

    /// Pool per-instant demands into one distribution over satellite-instants.
    ///
    /// Sizing wants the whole day, not a snapshot: a single instant holds only
    /// as many samples as there are satellites, so a quantile taken from one
    /// is indistinguishable from its maximum. `links` becomes the busiest
    /// instant's count, since links are held simultaneously and do not
    /// accumulate.
    pub fn over_time(instants: impl IntoIterator<Item = TerminalDemand>) -> TerminalDemand {
        let mut per_access = Vec::new();
        let mut per_anchor = Vec::new();
        let mut links = 0;
        for d in instants {
            per_access.extend(d.per_access);
            per_anchor.extend(d.per_anchor);
            links = links.max(d.links);
        }
        per_access.sort_unstable();
        per_anchor.sort_unstable();
        TerminalDemand {
            per_access,
            per_anchor,
            links,
        }
    }
}

fn quantile(sorted: &[usize], q: f64) -> usize {
    if sorted.is_empty() {
        return 0;
    }
    let i = (q.clamp(0.0, 1.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted[i]
}

/// Terminals required when every session reaches its anchor directly: no
/// ring-to-ring links, no anchor-to-anchor links.
///
/// One link per distinct (access satellite, anchor) pair. Two sessions sharing
/// both ends share a link — a terminal is a pipe, not a circuit — so this
/// counts pairs, never sessions.
pub fn direct_demand(sessions: &[Session]) -> TerminalDemand {
    let mut by_access: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    let mut by_anchor: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for s in sessions {
        by_access.entry(s.access).or_default().insert(s.anchor);
        by_anchor.entry(s.anchor).or_default().insert(s.access);
    }
    let links = by_access.values().map(|a| a.len()).sum();
    let mut per_access: Vec<usize> = by_access.values().map(|a| a.len()).collect();
    let mut per_anchor: Vec<usize> = by_anchor.values().map(|a| a.len()).collect();
    per_access.sort_unstable();
    per_anchor.sort_unstable();
    TerminalDemand {
        per_access,
        per_anchor,
        links,
    }
}

/// Terminals required when each access satellite feeds one gateway anchor and
/// the shell carries traffic onward to whichever anchor holds the session.
///
/// `gateway` names the anchor each access satellite feeds — normally its
/// nearest, which is a geometry question the caller answers. The wheel's side
/// collapses to one terminal per satellite whatever the sessions do, because
/// which anchor a session belongs to stops being the wheel's problem.
///
/// `per_anchor` counts feeder terminals only. The anchor-to-anchor links this
/// topology depends on are a separate cost, and this function does not pretend
/// to price them.
pub fn gateway_demand(sessions: &[Session], gateway: &BTreeMap<usize, usize>) -> TerminalDemand {
    let mut by_anchor: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    let mut access_seen: BTreeSet<usize> = BTreeSet::new();
    for s in sessions {
        let Some(&gw) = gateway.get(&s.access) else {
            continue;
        };
        access_seen.insert(s.access);
        by_anchor.entry(gw).or_default().insert(s.access);
    }
    let mut per_anchor: Vec<usize> = by_anchor.values().map(|a| a.len()).collect();
    per_anchor.sort_unstable();
    TerminalDemand {
        per_access: vec![1; access_seen.len()],
        per_anchor,
        links: access_seen.len(),
    }
}

/// Terminals required when a ring pools its traffic over intra-ring links and
/// only some of its satellites carry feeder terminals.
///
/// This is what the necklace is for. Anchor spread is a property of the
/// *sessions*, not of the satellite carrying them, so it does not shrink when
/// you move sessions around — but it stops being paid once per satellite. A
/// ring reaching `u` distinct anchors needs `u` feeder links however they are
/// arranged, and spreading them over `hosts` feeder-equipped satellites costs
/// each host `ceil(u / hosts)` rather than costing all twelve satellites their
/// own private view of the shell.
///
/// `group_of` names each access satellite's ring. `hosts` is how many
/// satellites in that ring carry feeder terminals, which is bounded below by
/// reach: every satellite must be within
/// [`crate::backbone::intra_plane_reach`] hops of a host, or its traffic
/// cannot get out.
///
/// The `per_access` distribution here counts hosts only. The satellites that
/// carry no feeder terminal still need two necklace terminals each, and those
/// are cheap — 4,437 km, frozen, zero Doppler — but they are not free, so the
/// caller adds them.
pub fn relay_demand(
    sessions: &[Session],
    group_of: impl Fn(usize) -> usize,
    hosts: usize,
) -> TerminalDemand {
    assert!(
        hosts > 0,
        "a ring with no feeder host cannot reach the shell"
    );
    let mut by_group: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    let mut by_anchor: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for s in sessions {
        let g = group_of(s.access);
        by_group.entry(g).or_default().insert(s.anchor);
        by_anchor.entry(s.anchor).or_default().insert(g);
    }
    let links: usize = by_group.values().map(|a| a.len()).sum();
    let mut per_access = Vec::new();
    for anchors in by_group.values() {
        // Split the ring's links as evenly as the hosts allow.
        let each = anchors.len().div_ceil(hosts);
        for _ in 0..hosts {
            per_access.push(each);
        }
    }
    let mut per_anchor: Vec<usize> = by_anchor.values().map(|a| a.len()).collect();
    per_access.sort_unstable();
    per_anchor.sort_unstable();
    TerminalDemand {
        per_access,
        per_anchor,
        links,
    }
}

/// Terminals required when every satellite is the same satellite.
///
/// A fleet is built to one drawing. [`relay_demand`] quietly assumed
/// otherwise: designating three feeder hosts in a ring of twelve is two
/// different spacecraft, one with feeder terminals and one without, and a
/// programme that builds two models pays for two of everything — two
/// qualification campaigns, two spares pools, two of every lesson learned in
/// orbit. So the real question is not how few terminals the fleet needs on
/// average but how many *each* satellite must carry, since every one of them
/// carries the worst case.
///
/// The necklace still helps, but differently. A session on satellite `i` can
/// use a feeder terminal on any satellite within `reach` hops, so a window of
/// `2·reach + 1` satellites pools its terminals. Sizing is therefore set by
/// the busiest window, not by the busiest satellite and not by the ring
/// average:
///
/// ```text
///     terminals per satellite = max over windows of ceil(anchors / window)
/// ```
///
/// `slot_of` gives each access satellite its (ring, position) so windows can
/// be formed. `ring_size` is how many satellites are in a ring, and the window
/// wraps, because a ring does.
///
/// `available` says whether the satellite at a (ring, position) can act as a
/// relay at all. This is not a formality. The activation plan lights the duty
/// ring as a block and then scatters single satellites through the other five
/// rings, so a lit satellite outside the duty ring usually has *no lit
/// neighbour to borrow from*: its window is itself, and it must carry its own
/// sessions' anchors alone. Pooling over the whole ring regardless would price
/// a relay network that is switched off.
///
/// This is a sizing floor rather than a solved assignment: it assumes a window
/// can divide its anchors evenly across its available members, which an
/// optimal allocation can always do and a real timetable might not quite.
pub fn uniform_relay_demand(
    sessions: &[Session],
    slot_of: impl Fn(usize) -> (usize, usize),
    available: impl Fn(usize, usize) -> bool,
    ring_size: usize,
    reach: usize,
) -> TerminalDemand {
    assert!(ring_size > 0 && reach > 0);
    let window = (2 * reach + 1).min(ring_size);

    // Anchors wanted by the sessions sitting on each (ring, position).
    let mut wanted: BTreeMap<(usize, usize), BTreeSet<usize>> = BTreeMap::new();
    // Which ring positions want each anchor, so the shell side can be counted
    // the same way the wheel side is: one terminal covers a whole window, so
    // an anchor does not see every access satellite that talks to it, only one
    // endpoint per window that needs it.
    let mut anchor_positions: BTreeMap<usize, BTreeMap<usize, BTreeSet<usize>>> = BTreeMap::new();
    for s in sessions {
        let (ring, pos) = slot_of(s.access);
        wanted.entry((ring, pos)).or_default().insert(s.anchor);
        anchor_positions
            .entry(s.anchor)
            .or_default()
            .entry(ring)
            .or_default()
            .insert(pos);
    }

    let rings: BTreeSet<usize> = wanted.keys().map(|&(r, _)| r).collect();
    let mut per_access = Vec::new();
    let mut links = 0usize;
    for ring in rings {
        let mut ring_union: BTreeSet<usize> = BTreeSet::new();
        let mut worst = 0usize;
        for start in 0..ring_size {
            let mut union: BTreeSet<usize> = BTreeSet::new();
            let mut relays = 0usize;
            for d in 0..window {
                let pos = (start + d) % ring_size;
                if available(ring, pos) {
                    relays += 1;
                }
                if let Some(a) = wanted.get(&(ring, pos)) {
                    union.extend(a.iter().copied());
                    ring_union.extend(a.iter().copied());
                }
            }
            // A window with nothing switched on cannot pool: whichever
            // satellite holds the sessions carries their anchors by itself.
            let share = union.len().div_ceil(relays.max(1));
            worst = worst.max(share);
        }
        links += ring_union.len();
        for _ in 0..ring_size {
            per_access.push(worst);
        }
    }
    // An anchor's feeder terminals: one per window that needs it, per ring.
    let mut per_anchor: Vec<usize> = anchor_positions
        .values()
        .map(|rings| {
            rings
                .values()
                .map(|positions| cover_ring(positions, ring_size, window))
                .sum()
        })
        .collect();
    per_access.sort_unstable();
    per_anchor.sort_unstable();
    TerminalDemand {
        per_access,
        per_anchor,
        links,
    }
}

/// Fewest windows of `window` consecutive positions needed to cover every
/// position in `positions`, around a ring of `ring_size`.
///
/// Greedy from each possible starting offset: a ring has no natural left end,
/// so the best cover depends on where you begin, and there are only
/// `ring_size` places to begin.
fn cover_ring(positions: &BTreeSet<usize>, ring_size: usize, window: usize) -> usize {
    if positions.is_empty() {
        return 0;
    }
    let mut best = usize::MAX;
    for offset in 0..ring_size {
        let mut ordered: Vec<usize> = positions
            .iter()
            .map(|&p| (p + ring_size - offset) % ring_size)
            .collect();
        ordered.sort_unstable();
        let mut count = 0;
        let mut i = 0;
        while i < ordered.len() {
            let reach_end = ordered[i] + window - 1;
            count += 1;
            while i < ordered.len() && ordered[i] <= reach_end {
                i += 1;
            }
        }
        best = best.min(count);
    }
    best
}

/// Sessions riding each (access ring, anchor) pair, ascending.
///
/// With the wheel pooling a whole ring, an anchor holds exactly one telescope
/// per ring that talks to it — which makes every such pair a single point of
/// failure. Lose that one telescope and the anchor is not degraded toward
/// that ring, it is unreachable from it, and every session in this bucket
/// must move at once.
///
/// The wheel has no equivalent exposure: a ring pools two telescopes on each
/// of twelve satellites, so a session that loses one path borrows another.
/// The asymmetry is worth measuring rather than assuming, because it decides
/// whether the shell needs links of its own after all.
pub fn pair_load(sessions: &[Session], ring_of: impl Fn(usize) -> usize) -> Vec<usize> {
    let mut counts: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    for s in sessions {
        *counts.entry((ring_of(s.access), s.anchor)).or_insert(0) += 1;
    }
    let mut out: Vec<usize> = counts.into_values().collect();
    out.sort_unstable();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sessions(pairs: &[(usize, usize)]) -> Vec<Session> {
        pairs
            .iter()
            .map(|&(access, anchor)| Session { access, anchor })
            .collect()
    }

    /// A terminal is a pipe: sessions sharing both ends share one link. Were
    /// this counting sessions instead of pairs, every number in this module
    /// would scale with population rather than with anchor spread, and anchor
    /// spread is the whole quantity of interest.
    #[test]
    fn sessions_sharing_both_ends_share_one_link() {
        let d = direct_demand(&sessions(&[(0, 5), (0, 5), (0, 5)]));
        assert_eq!(d.links, 1);
        assert_eq!(d.max_access(), 1);
        assert_eq!(d.max_anchor(), 1);
    }

    /// The price of retention, as a test. One access satellite carrying
    /// sessions anchored in four places needs four terminals, and no amount of
    /// traffic engineering below the anchor changes that.
    #[test]
    fn scattered_anchors_cost_the_access_satellite_a_terminal_each() {
        let d = direct_demand(&sessions(&[(0, 1), (0, 2), (0, 3), (0, 4)]));
        assert_eq!(d.max_access(), 4);
        assert_eq!(d.max_anchor(), 1);
    }

    /// A gateway makes the wheel's terminal count independent of how scattered
    /// the anchors are, which is the entire argument for putting links in the
    /// shell.
    #[test]
    fn a_gateway_holds_the_wheel_to_one_terminal_however_scattered_the_anchors() {
        let scattered = sessions(&[(0, 1), (0, 2), (0, 3), (0, 4), (0, 5), (0, 6)]);
        let gateway: BTreeMap<usize, usize> = [(0usize, 9usize)].into_iter().collect();

        let direct = direct_demand(&scattered);
        let gw = gateway_demand(&scattered, &gateway);

        assert_eq!(direct.max_access(), 6);
        assert_eq!(gw.max_access(), 1);
        assert!(gw.max_access() < direct.max_access());
    }

    /// A quantile over one instant is a quantile over a handful of samples,
    /// which is why sizing pools the day. `links` is the busiest instant, not
    /// the sum -- links are held at the same time, they do not accumulate.
    #[test]
    fn pooling_instants_keeps_the_distribution_and_peaks_the_links() {
        let a = direct_demand(&sessions(&[(0, 1), (0, 2), (1, 1)]));
        let b = direct_demand(&sessions(&[(0, 1), (1, 1), (1, 2), (1, 3)]));
        assert_eq!(a.links, 3);
        assert_eq!(b.links, 4);

        let pooled = TerminalDemand::over_time([a, b]);
        assert_eq!(pooled.per_access, vec![1, 1, 2, 3]);
        assert_eq!(pooled.max_access(), 3);
        assert_eq!(pooled.links, 4, "links peak, they do not add up");
    }

    /// The necklace does not reduce anchor spread -- that belongs to the
    /// sessions -- it stops the ring paying for it twelve times over. Twelve
    /// satellites each seeing a different anchor need twelve terminals between
    /// them, not twelve each.
    #[test]
    fn pooling_a_ring_pays_for_its_anchors_once_not_once_per_satellite() {
        // One ring, twelve satellites, each carrying a session to its own anchor.
        let s: Vec<Session> = (0..12)
            .map(|i| Session {
                access: i,
                anchor: i,
            })
            .collect();
        let direct = direct_demand(&s);
        assert_eq!(direct.max_access(), 1, "each satellite sees one anchor...");
        assert_eq!(direct.links, 12, "...but the ring still buys twelve links");

        // Three feeder hosts share the ring's twelve links, four each.
        let relay = relay_demand(&s, |_| 0, 3);
        assert_eq!(relay.links, 12);
        assert_eq!(relay.max_access(), 4);
    }

    /// The saving is in the satellites that carry no feeder terminal at all.
    #[test]
    fn only_the_hosts_carry_feeder_terminals() {
        let s: Vec<Session> = (0..12)
            .map(|i| Session {
                access: i,
                anchor: i % 6,
            })
            .collect();
        let relay = relay_demand(&s, |_| 0, 3);
        assert_eq!(relay.per_access.len(), 3, "nine satellites carry none");
        assert_eq!(relay.max_access(), 2, "six anchors over three hosts");
    }

    /// One drawing means every satellite carries the worst window, so a fleet
    /// of identical satellites is sized by its busiest neighbourhood and never
    /// by its average one.
    #[test]
    fn one_drawing_means_every_satellite_carries_the_worst_window() {
        // A ring of 12. All the anchor spread sits on positions 0..=4.
        let s: Vec<Session> = (0..5)
            .flat_map(|i| {
                (0..5).map(move |a| Session {
                    access: i,
                    anchor: a,
                })
            })
            .collect();

        // Window of 5 (reach 2) covering positions 0..=4 sees 5 anchors: one each.
        let d = uniform_relay_demand(&s, |a| (0, a), |_, _| true, 12, 2);
        assert_eq!(d.max_access(), 1);
        assert_eq!(d.per_access.len(), 12, "every satellite is built the same");
        assert!(
            d.per_access.iter().all(|&n| n == d.max_access()),
            "one drawing: no satellite is cheaper than another"
        );
    }

    /// Reach is what makes the necklace worth having: a longer window pools
    /// more terminals, so each satellite carries fewer.
    #[test]
    fn more_reach_means_fewer_terminals_each() {
        let s: Vec<Session> = (0..12)
            .flat_map(|i| {
                (0..6).map(move |a| Session {
                    access: i,
                    anchor: a,
                })
            })
            .collect();
        let one_hop = uniform_relay_demand(&s, |a| (0, a), |_, _| true, 12, 1);
        let two_hop = uniform_relay_demand(&s, |a| (0, a), |_, _| true, 12, 2);
        assert!(
            two_hop.max_access() <= one_hop.max_access(),
            "two hops {} should not cost more than one {}",
            two_hop.max_access(),
            one_hop.max_access()
        );
    }

    /// An isolated satellite cannot pool. The activation plan lights the duty
    /// ring as a block and scatters singles through the other rings, so a lit
    /// satellite outside the duty ring has no lit neighbour to lean on and
    /// must carry its own sessions' anchors alone -- which is the direct
    /// topology again, on that satellite. Since one drawing sizes the fleet by
    /// its worst case, a single isolated satellite sets the whole build.
    #[test]
    fn an_isolated_satellite_carries_its_own_anchors() {
        // One lit satellite at position 6, four anchors, neighbours all dark.
        let s: Vec<Session> = (0..4)
            .map(|a| Session {
                access: 6,
                anchor: a,
            })
            .collect();

        let pooled = uniform_relay_demand(&s, |a| (0, a), |_, p| p == 6, 12, 2);
        assert_eq!(pooled.max_access(), 4, "alone, it carries all four");

        let with_neighbours = uniform_relay_demand(&s, |a| (0, a), |_, _| true, 12, 2);
        assert_eq!(
            with_neighbours.max_access(),
            1,
            "with four to lean on, one each"
        );
    }

    /// Pooling has to be counted on both sides. A ring exits through one
    /// terminal per window, so an anchor sees a handful of endpoints rather
    /// than every access satellite whose sessions it holds -- counting the
    /// shell as if traffic never pooled would price the necklace as no better
    /// than the direct topology on that side.
    #[test]
    fn an_anchor_sees_windows_not_satellites() {
        // Every satellite of one ring of 12 carries a session to anchor 0.
        let s: Vec<Session> = (0..12)
            .map(|i| Session {
                access: i,
                anchor: 0,
            })
            .collect();

        let direct = direct_demand(&s);
        assert_eq!(direct.max_anchor(), 12, "direct: twelve endpoints");

        // Windows of 5 cover twelve positions three times over.
        let pooled = uniform_relay_demand(&s, |a| (0, a), |_, _| true, 12, 2);
        assert_eq!(pooled.max_anchor(), 3, "pooled: three endpoints");
    }

    /// And a ring that wants an anchor from one place needs exactly one link
    /// to it, however many of its satellites carry sessions there.
    #[test]
    fn neighbouring_positions_share_one_anchor_link() {
        let s: Vec<Session> = (0..3)
            .map(|i| Session {
                access: i,
                anchor: 7,
            })
            .collect();
        let pooled = uniform_relay_demand(&s, |a| (0, a), |_, _| true, 12, 2);
        assert_eq!(pooled.max_anchor(), 1);
    }

    /// The blast radius of one anchor telescope: every session on that ring
    /// anchored there, all at once. Not a degradation -- a stranding.
    #[test]
    fn one_anchor_telescope_carries_a_whole_rings_sessions_to_it() {
        // Ring 0 is satellites 0..12, ring 1 is 12..24.
        let mut s: Vec<Session> = (0..9)
            .map(|i| Session {
                access: i,
                anchor: 4,
            })
            .collect();
        s.push(Session {
            access: 13,
            anchor: 4,
        });
        let load = pair_load(&s, |a| a / 12);
        assert_eq!(load, vec![1, 9], "nine on ring 0's link, one on ring 1's");
    }

    #[test]
    fn quantiles_read_off_the_distribution() {
        let d = direct_demand(&sessions(&[
            (0, 1),
            (1, 1),
            (1, 2),
            (2, 1),
            (2, 2),
            (2, 3),
            (3, 1),
            (3, 2),
            (3, 3),
            (3, 4),
        ]));
        assert_eq!(d.per_access, vec![1, 2, 3, 4]);
        assert_eq!(d.access_quantile(0.0), 1);
        assert_eq!(d.access_quantile(1.0), 4);
        assert_eq!(d.max_access(), 4);
    }
}
