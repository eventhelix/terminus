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
