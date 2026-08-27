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

/// Fewest necklace hops between two positions of one ring.
///
/// A ring is a circulant graph: each satellite links to the `reach` on either
/// side, so the far side is `ring_size / 2` away and every hop closes `reach`
/// of it. Returns `None` if the ring cannot relay at all.
pub fn necklace_hops(
    from_slot: usize,
    to_slot: usize,
    ring_size: usize,
    reach: usize,
) -> Option<usize> {
    if reach == 0 || ring_size == 0 {
        return None;
    }
    let raw = from_slot.abs_diff(to_slot) % ring_size;
    let around = raw.min(ring_size - raw);
    Some(around.div_ceil(reach))
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
pub fn exit_gateway(
    serving_slot: usize,
    ring_size: usize,
    reach: usize,
    hop_range: f64,
    feeder_range: impl Fn(usize) -> Option<f64>,
) -> Option<Gateway> {
    let mut best: Option<Gateway> = None;
    for slot in 0..ring_size {
        let Some(feeder) = feeder_range(slot) else {
            continue;
        };
        let Some(hops) = necklace_hops(serving_slot, slot, ring_size, reach) else {
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
    const REACH: usize = 2;
    const HOP: f64 = 4_437e3;

    #[test]
    fn hops_count_by_reach_and_go_the_short_way_round() {
        assert_eq!(necklace_hops(0, 0, RING, REACH), Some(0));
        assert_eq!(necklace_hops(0, 2, RING, REACH), Some(1), "reach is two");
        assert_eq!(necklace_hops(0, 3, RING, REACH), Some(2));
        // Ten places clockwise is two places the other way.
        assert_eq!(necklace_hops(0, 10, RING, REACH), Some(1));
        // The far side of the ring: six places, three hops.
        assert_eq!(necklace_hops(0, 6, RING, REACH), Some(3));
    }

    /// The default is to climb from where you are. A ring mate has to be
    /// enough closer to the anchor to pay for the hop.
    #[test]
    fn a_detour_must_beat_the_hop_it_costs() {
        // Ring mate one hop away is only slightly closer: not worth it.
        let marginal = exit_gateway(0, RING, REACH, HOP, |slot| match slot {
            0 => Some(23_000e3),
            2 => Some(21_000e3),
            _ => None,
        })
        .expect("a route exists");
        assert_eq!(marginal.slot, 0);
        assert_eq!(marginal.hops, 0);

        // Now it is closer by more than the hop costs.
        let worth_it = exit_gateway(0, RING, REACH, HOP, |slot| match slot {
            0 => Some(23_000e3),
            2 => Some(17_000e3),
            _ => None,
        })
        .expect("a route exists");
        assert_eq!(worth_it.slot, 2);
        assert_eq!(worth_it.hops, 1);
        assert!((worth_it.path - (HOP + 17_000e3)).abs() < 1.0);
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

    #[test]
    fn plane_mates_hand_over_directly_and_everyone_else_goes_down() {
        assert_eq!(migration_path(3, 3), MigrationPath::PlaneLink);
        assert_eq!(migration_path(3, 4), MigrationPath::ThroughTheWheel);
    }
}
