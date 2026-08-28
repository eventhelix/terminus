//! Which way a question actually goes.
//!
//! The backbone offers a session two choices and no more. Leaving the ring, it
//! may climb to the shell from the satellite it is already on, or travel a few
//! places along the necklace first and climb from a ring mate with better
//! geometry. Moving between anchors, it may cross a plane link if the two
//! anchors are plane mates, or descend through the wheel and climb again if
//! they are not — because [`crate::backbone`] gives the shell frozen links
//! within a plane and none at all between planes.
//!
//! Both choices are arithmetic, not search. A necklace hop is a known 4,437 km
//! and the feeder ranges are known to the metre, so the shorter path is the
//! one with the smaller sum. There is no discovery here and no routing
//! protocol: the timetable can compute all of it years ahead.
//!
//! The hop is not free — 14.8 ms of light time each way at 2,200 km — so a
//! detour only pays when the ring mate's feeder link is shorter by more than
//! that. It often is: the serving satellite is chosen for its elevation over a
//! *town*, which has nothing to do with where the anchors happen to be.

/// How many ring mates a satellite actually has a laser terminal pointed at,
/// on each side.
///
/// **One.** Each access satellite carries two necklace terminals, aimed at its
/// immediate neighbours and held there for years (ADR-0018). Do not confuse
/// this with [`crate::backbone::intra_plane_reach`], which says how many ring
/// mates are *visible* — two, at 2,200 km. Visibility is a geometric fact and
/// a comfortable margin; connectivity is a hardware fact and the thing routing
/// must obey. A satellite can see past its neighbour and cannot talk past it.
pub const NECKLACE_LINKS: usize = 1;

/// Fewest necklace hops between two positions of one ring.
///
/// A ring is a circulant graph: each satellite links to the `links` on either
/// side, so the far side is `ring_size / 2` away and every hop closes `links`
/// of it. With `links = 1` — the built fleet — that is simply the number of
/// places between them, and a ring of twelve takes six hops to cross.
/// Returns `None` if the ring cannot relay at all.
pub fn necklace_hops(
    from_slot: usize,
    to_slot: usize,
    ring_size: usize,
    links: usize,
) -> Option<usize> {
    if links == 0 || ring_size == 0 {
        return None;
    }
    let raw = from_slot.abs_diff(to_slot) % ring_size;
    let around = raw.min(ring_size - raw);
    Some(around.div_ceil(links))
}

/// The satellite a session leaves its ring through.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gateway {
    /// Ring position that carries the feeder link up to the anchor.
    pub slot: usize,
    /// Necklace hops from the serving satellite to that position. Zero when
    /// the session climbs from where it already is.
    pub hops: usize,
    /// Total path length (m): the hops plus the feeder link.
    pub path: f64,
}

/// Choose the ring position to climb from, trading necklace hops against
/// feeder range.
///
/// `feeder_range(slot)` gives the distance from that ring position to the
/// anchor, or `None` where the planet is in the way. `hop_range` is the
/// constant chord between necklace neighbours.
///
/// Ties go to the serving satellite: a detour has to *win*, not merely draw,
/// because every hop is another link to keep alive and another place to fail.
///
/// **There is no hop limit and deliberately so.** Every position in the ring is
/// considered, and what bounds the detour is arithmetic rather than a rule: a
/// hop costs 4,437 km, so a gateway two places away has to be 8,874 km closer
/// to the anchor to be worth reaching. Against feeder links of 18,000 to 34,000
/// km that ceiling binds quickly, and in a day of following one town the
/// routing never chose more than two.
pub fn exit_gateway(
    serving_slot: usize,
    ring_size: usize,
    links: usize,
    hop_range: f64,
    feeder_range: impl Fn(usize) -> Option<f64>,
) -> Option<Gateway> {
    let mut best: Option<Gateway> = None;
    for slot in 0..ring_size {
        let Some(feeder) = feeder_range(slot) else {
            continue;
        };
        let Some(hops) = necklace_hops(serving_slot, slot, ring_size, links) else {
            continue;
        };
        let path = hops as f64 * hop_range + feeder;
        let better = match best {
            None => true,
            // Strictly shorter, or the same length from where we already are.
            Some(b) => path < b.path - 1.0 || (path < b.path + 1.0 && hops < b.hops),
        };
        if better {
            best = Some(Gateway { slot, hops, path });
        }
    }
    best
}

/// How working memory travels when a session changes anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationPath {
    /// Straight across, over the frozen link inside one MEO plane.
    PlaneLink,
    /// Down to an access satellite and back up, two feeder hops. This is what
    /// the second feeder telescope on every access satellite is for.
    ThroughTheWheel,
}

/// Which of the two an anchor change uses.
///
/// The shell has frozen links inside each plane and none between planes, so
/// plane mates hand a session straight across and everything else goes back
/// down through the wheel. Measured against the anchor policy, about seven in
/// eight anchor changes are between plane mates.
pub fn migration_path(from_plane: usize, to_plane: usize) -> MigrationPath {
    if from_plane == to_plane {
        MigrationPath::PlaneLink
    } else {
        MigrationPath::ThroughTheWheel
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RING: usize = 12;
    const REACH: usize = NECKLACE_LINKS;
    const HOP: f64 = 4_437e3;

    /// One terminal each way means one hop per place. A satellite can *see* two
    /// places along its ring and can only *talk* one, so routing counts places,
    /// not sightlines.
    #[test]
    fn a_hop_moves_one_place_because_that_is_where_the_terminal_points() {
        assert_eq!(necklace_hops(0, 0, RING, NECKLACE_LINKS), Some(0));
        assert_eq!(necklace_hops(0, 1, RING, NECKLACE_LINKS), Some(1));
        assert_eq!(
            necklace_hops(0, 2, RING, NECKLACE_LINKS),
            Some(2),
            "no skipping"
        );
        // Ten places clockwise is two places the other way.
        assert_eq!(necklace_hops(0, 10, RING, NECKLACE_LINKS), Some(2));
        // The far side of the ring: six places, six hops.
        assert_eq!(necklace_hops(0, 6, RING, NECKLACE_LINKS), Some(6));
    }

    /// The link graph is a subgraph of the visibility graph, and the gap is
    /// margin rather than capability: a satellite could see past a dead
    /// neighbour but has no terminal aimed there.
    #[test]
    fn links_never_outrun_what_the_ring_can_see() {
        let p = crate::CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
        let visible = crate::backbone::intra_plane_reach(&p, 2_200e3, RING);
        assert_eq!(visible, 2, "84 deg of sight against 30 deg of spacing");
        assert!(
            NECKLACE_LINKS <= visible,
            "a terminal cannot point somewhere the planet hides"
        );
    }

    /// The default is to climb from where you are. A ring mate has to be
    /// enough closer to the anchor to pay for the hop.
    #[test]
    fn a_detour_must_beat_the_hop_it_costs() {
        // The neighbour -- one hop, one terminal away -- is only slightly
        // closer to the anchor: not worth the 4,437 km detour.
        let marginal = exit_gateway(0, RING, REACH, HOP, |slot| match slot {
            0 => Some(23_000e3),
            1 => Some(21_000e3),
            _ => None,
        })
        .expect("a route exists");
        assert_eq!(marginal.slot, 0);
        assert_eq!(marginal.hops, 0);

        // Now it is closer by more than the hop costs.
        let worth_it = exit_gateway(0, RING, REACH, HOP, |slot| match slot {
            0 => Some(23_000e3),
            1 => Some(17_000e3),
            _ => None,
        })
        .expect("a route exists");
        assert_eq!(worth_it.slot, 1);
        assert_eq!(worth_it.hops, 1);
        assert!((worth_it.path - (HOP + 17_000e3)).abs() < 1.0);

        // Two places away costs two hops, so the same saving no longer pays.
        let two_away = exit_gateway(0, RING, REACH, HOP, |slot| match slot {
            0 => Some(23_000e3),
            2 => Some(17_000e3),
            _ => None,
        })
        .expect("a route exists");
        assert_eq!(
            two_away.slot, 0,
            "8,874 km of detour for 6,000 km of saving"
        );
    }

    /// A ring mate the planet is hiding cannot be a gateway however good its
    /// geometry would otherwise be.
    #[test]
    fn an_occulted_ring_mate_is_not_a_gateway() {
        let g = exit_gateway(0, RING, REACH, HOP, |slot| match slot {
            0 => Some(30_000e3),
            2 => None, // behind the planet
            _ => None,
        })
        .expect("a route exists");
        assert_eq!(g.slot, 0);
    }

    /// If nowhere in the ring can see the anchor, there is no route, and the
    /// caller has to pick a different anchor rather than a different path.
    #[test]
    fn no_gateway_when_the_whole_ring_is_blind() {
        assert_eq!(exit_gateway(0, RING, REACH, HOP, |_| None), None);
    }

    /// Nothing caps the hop count; the price of a hop does. Even a ring mate
    /// with a perfect view -- zero distance to the anchor -- stops being worth
    /// reaching once the walk there exceeds what climbing from here would cost.
    #[test]
    fn the_hop_price_bounds_the_detour_without_a_limit() {
        // Three places away is 13,311 km of walking, so a flawless gateway
        // there loses to anything closer than that from where we stand.
        let far = exit_gateway(0, RING, REACH, HOP, |slot| match slot {
            0 => Some(12_000e3),
            3 => Some(1.0),
            _ => None,
        })
        .expect("a route exists");
        assert_eq!(far.slot, 0, "12,000 km beats 13,311 km of walking");

        // Push the local feeder out and the same gateway becomes worth it,
        // which is the point: the bound is a price, not a rule.
        let now_worth_it = exit_gateway(0, RING, REACH, HOP, |slot| match slot {
            0 => Some(40_000e3),
            3 => Some(1.0),
            _ => None,
        })
        .expect("a route exists");
        assert_eq!(now_worth_it.slot, 3);
        assert_eq!(now_worth_it.hops, 3);
    }

    #[test]
    fn plane_mates_hand_over_directly_and_everyone_else_goes_down() {
        assert_eq!(migration_path(3, 3), MigrationPath::PlaneLink);
        assert_eq!(migration_path(3, 4), MigrationPath::ThroughTheWheel);
    }
}
