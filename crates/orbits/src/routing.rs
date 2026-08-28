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
//! The hop is not free — 14.8 ms of light time each way at 2,200 km, plus the
//! half a millisecond the ring mate spends turning the frame around — so a
//! detour only pays when its feeder link is shorter by more than that. It often
//! is: the serving satellite is chosen for its elevation over a *town*, which
//! has nothing to do with where the anchors happen to be.
//!
//! Because a hop costs time in two currencies, the comparison here is made in
//! seconds rather than in metres. The two orderings agree almost everywhere and
//! disagree in a band about 150 km wide, where a nearer gateway one hop further
//! along loses more to processing than it saves in light.

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

/// Processing delay (s) at each satellite that forwards a packet.
///
/// **A stated guess, and every latency downstream moves linearly in it.** The
/// payload is regenerative: a relay demodulates, decodes forward error
/// correction, switches, re-encodes and re-modulates, and half a millisecond
/// is a fair allowance for that on a spacecraft. It is not a constant of the
/// sky and it is not measured — it is a number the design has to be told, in
/// the same way the re-anchor margin is, and it is stated here so that
/// everything downstream quotes one figure rather than inventing its own.
///
/// The scale is worth holding on to: a necklace hop costs 14.8 ms of light and
/// 0.5 ms of processing, so relays are a thirtieth of what a hop costs. They
/// change no conclusion on their own. What they do is decide the near-ties,
/// and they put a floor under the latency of a path that no amount of good
/// geometry can dig through.
pub const RELAY_DELAY: f64 = 0.5e-3;

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
    /// One-way time (s) from the serving satellite to the anchor: light over
    /// [`Gateway::path`], plus a relay delay for each hop. It does not include
    /// the radio leg from the town, which every candidate shares.
    pub latency: f64,
}

/// Choose the ring position to climb from, trading necklace hops against
/// feeder range.
///
/// `feeder_range(slot)` gives the distance from that ring position to the
/// anchor, or `None` where the planet is in the way. `hop_range` is the
/// constant chord between necklace neighbours.
///
/// `relay_delay` is what each hop costs in processing on top of its light
/// time — see [`RELAY_DELAY`] — and the winner is the one with the shorter
/// *time*, not the shorter path. Pass zero to compare in pure geometry.
///
/// Ties go to the serving satellite: a detour has to *win*, not merely draw,
/// because every hop is another link to keep alive and another place to fail.
///
/// **There is no hop limit and deliberately so.** Every position in the ring is
/// considered, and what bounds the detour is arithmetic rather than a rule: a
/// hop costs 4,437 km, so a gateway two places away has to be 8,874 km closer
/// to the anchor to be worth reaching. Against feeder links of 18,000 to 34,000
/// km that ceiling binds quickly. In a day of following a thousand towns the
/// routing never chose more than three hops, and at the default re-anchor
/// margin it never chose even one: a session free to re-anchor holds an anchor
/// its own satellite can already see, and the necklace earns its keep on reach
/// and on failures rather than in steady state.
pub fn exit_gateway(
    serving_slot: usize,
    ring_size: usize,
    links: usize,
    hop_range: f64,
    relay_delay: f64,
    feeder_range: impl Fn(usize) -> Option<f64>,
) -> Option<Gateway> {
    // A nanosecond is a third of a metre: close enough to equal that the
    // tiebreak below should decide it rather than floating-point noise.
    const TIE: f64 = 1e-9;
    let mut best: Option<Gateway> = None;
    for slot in 0..ring_size {
        let Some(feeder) = feeder_range(slot) else {
            continue;
        };
        let Some(hops) = necklace_hops(serving_slot, slot, ring_size, links) else {
            continue;
        };
        let path = hops as f64 * hop_range + feeder;
        let latency = crate::placement::one_way_latency(path, hops, relay_delay);
        let better = match best {
            None => true,
            // Strictly sooner, or just as soon from where we already are.
            Some(b) => latency < b.latency - TIE || (latency < b.latency + TIE && hops < b.hops),
        };
        if better {
            best = Some(Gateway {
                slot,
                hops,
                path,
                latency,
            });
        }
    }
    best
}

/// How a ring reaches an anchor once telescopes start failing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Routed {
    /// Which ring position climbs, from [`exit_gateway`]. On a detour this is
    /// the gateway toward the RELAY, not toward `anchor` — the ring climbs to
    /// the plane mate and the frozen link carries it the rest of the way.
    pub gateway: Gateway,
    /// The plane mate relaying, or `None` when the anchor is reached directly.
    pub relay: Option<usize>,
    /// Total path (m), including the intra-plane link when one is used.
    pub path: f64,
    /// One-way time (s), including a relay at every satellite that forwards.
    pub latency: f64,
}

/// Whether a ring can still reach an anchor, and what it costs.
///
/// Pooling left each anchor with exactly ONE feeder telescope per ring
/// (ADR-0018), so that telescope is a single point of failure for the whole
/// (ring, anchor) pair: losing it does not degrade the link, it severs it.
/// What buys the pair back is the frozen intra-plane link — a crippled anchor
/// is reached through a plane mate that still has a telescope into the ring
/// (ADR-0019).
///
/// Called for ONE access ring at a time. `alive(a)` says whether anchor `a`'s
/// telescope *into that ring* is up; `exit_via(a)` is the best gateway the
/// ring can offer toward anchor `a`, as [`exit_gateway`] computes it.
/// `plane_mates` holds the two cycle neighbours — a plane of four closes into
/// a cycle with the opposite satellite permanently occulted.
///
/// Returns `None` when neither the anchor nor a usable plane mate can be
/// reached. That `None` is what forces a session to re-anchor, and a great
/// many of them doing it at once is the migration storm.
pub fn feeder_route(
    anchor: usize,
    plane_mates: &[usize],
    alive: impl Fn(usize) -> bool,
    exit_via: impl Fn(usize) -> Option<Gateway>,
    plane_link: f64,
    relay_delay: f64,
) -> Option<Routed> {
    // Direct needs two things: a live telescope AND a ring that can see the
    // anchor. Failing either, the plane mates are still worth trying -- the
    // detour climbs to a mate and crosses the frozen link, so it never uses
    // the anchor's own telescope into this ring at all.
    if alive(anchor) {
        if let Some(gateway) = exit_via(anchor) {
            return Some(Routed {
                relay: None,
                path: gateway.path,
                latency: gateway.latency,
                gateway,
            });
        }
    }
    let mut best: Option<Routed> = None;
    for &mate in plane_mates {
        if !alive(mate) {
            continue;
        }
        let Some(gateway) = exit_via(mate) else {
            continue;
        };
        // The mate forwards rather than terminates, so it costs a relay on top
        // of the light over the plane link.
        let latency = gateway.latency + plane_link / crate::placement::SPEED_OF_LIGHT + relay_delay;
        if best.map_or(true, |b| latency < b.latency) {
            best = Some(Routed {
                relay: Some(mate),
                path: gateway.path + plane_link,
                latency,
                gateway,
            });
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
    /// Most of these tests are about geometry, and say so by pricing the relay
    /// at nothing. The one that is about the relay uses [`RELAY_DELAY`].
    const FREE: f64 = 0.0;

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
        let marginal = exit_gateway(0, RING, REACH, HOP, FREE, |slot| match slot {
            0 => Some(23_000e3),
            1 => Some(21_000e3),
            _ => None,
        })
        .expect("a route exists");
        assert_eq!(marginal.slot, 0);
        assert_eq!(marginal.hops, 0);

        // Now it is closer by more than the hop costs.
        let worth_it = exit_gateway(0, RING, REACH, HOP, FREE, |slot| match slot {
            0 => Some(23_000e3),
            1 => Some(17_000e3),
            _ => None,
        })
        .expect("a route exists");
        assert_eq!(worth_it.slot, 1);
        assert_eq!(worth_it.hops, 1);
        assert!((worth_it.path - (HOP + 17_000e3)).abs() < 1.0);

        // Two places away costs two hops, so the same saving no longer pays.
        let two_away = exit_gateway(0, RING, REACH, HOP, FREE, |slot| match slot {
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

    /// The relay delay is small enough to change no conclusion and large
    /// enough to decide a near-tie, which is the whole reason the comparison
    /// is made in seconds. Here the ring mate is a hop away and 100 km closer:
    /// shorter in metres, later in time, because 100 km of light is 0.33 ms
    /// and turning the frame around costs 0.50 ms.
    #[test]
    fn a_relay_delay_settles_what_metres_cannot() {
        let candidates = |slot: usize| match slot {
            0 => Some(23_000e3),
            1 => Some(23_000e3 - HOP - 100e3),
            _ => None,
        };
        let by_geometry = exit_gateway(0, RING, REACH, HOP, FREE, candidates).expect("a route");
        assert_eq!(by_geometry.slot, 1, "100 km shorter, so metres say hop");
        let by_clock = exit_gateway(0, RING, REACH, HOP, RELAY_DELAY, candidates).expect("a route");
        assert_eq!(by_clock.slot, 0, "0.33 ms saved for 0.50 ms spent");
        assert_eq!(by_clock.hops, 0);
    }

    /// A ring mate the planet is hiding cannot be a gateway however good its
    /// geometry would otherwise be.
    #[test]
    fn an_occulted_ring_mate_is_not_a_gateway() {
        let g = exit_gateway(0, RING, REACH, HOP, FREE, |slot| match slot {
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
        assert_eq!(exit_gateway(0, RING, REACH, HOP, FREE, |_| None), None);
    }

    /// Nothing caps the hop count; the price of a hop does. Even a ring mate
    /// with a perfect view -- zero distance to the anchor -- stops being worth
    /// reaching once the walk there exceeds what climbing from here would cost.
    #[test]
    fn the_hop_price_bounds_the_detour_without_a_limit() {
        // Three places away is 13,311 km of walking, so a flawless gateway
        // there loses to anything closer than that from where we stand.
        let far = exit_gateway(0, RING, REACH, HOP, FREE, |slot| match slot {
            0 => Some(12_000e3),
            3 => Some(1.0),
            _ => None,
        })
        .expect("a route exists");
        assert_eq!(far.slot, 0, "12,000 km beats 13,311 km of walking");

        // Push the local feeder out and the same gateway becomes worth it,
        // which is the point: the bound is a price, not a rule.
        let now_worth_it = exit_gateway(0, RING, REACH, HOP, FREE, |slot| match slot {
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

    /// 37,294 km — the frozen intra-plane chord at 20,000 km, four to a plane.
    const PLANE_LINK: f64 = 37_294e3;

    /// A stand-in gateway at a stated feeder range, climbing from where the
    /// session already is. Latency is pure light so the tests can do the
    /// relay arithmetic themselves.
    fn stub_gw(path: f64) -> Gateway {
        Gateway {
            slot: 0,
            hops: 0,
            path,
            latency: path / crate::placement::SPEED_OF_LIGHT,
        }
    }

    /// Anchor 0 is the held one; 1 and 3 are its cycle neighbours in the
    /// plane. 3 has the better view of this ring, which is what makes the
    /// "cheaper mate wins" test unambiguous.
    fn stub_ranges(a: usize) -> Option<Gateway> {
        match a {
            0 => Some(stub_gw(23_000e3)),
            1 => Some(stub_gw(26_000e3)),
            3 => Some(stub_gw(21_000e3)),
            _ => None,
        }
    }

    /// While the anchor's own telescope into this ring is up, nothing else
    /// matters: the plane links are spare capacity, not a route.
    #[test]
    fn a_live_telescope_is_reached_directly() {
        let r = feeder_route(0, &[1, 3], |_| true, stub_ranges, PLANE_LINK, RELAY_DELAY)
            .expect("a route exists");
        assert_eq!(r.relay, None);
        assert!((r.path - 23_000e3).abs() < 1.0);
    }

    /// Pooling gives an anchor exactly one telescope per ring, so losing it
    /// removes the whole ring-to-anchor path rather than one of several. The
    /// plane mate with the better view of the ring carries the detour.
    #[test]
    fn a_dead_telescope_is_reached_through_the_cheaper_plane_mate() {
        let r = feeder_route(0, &[1, 3], |a| a != 0, stub_ranges, PLANE_LINK, RELAY_DELAY)
            .expect("a route exists");
        assert_eq!(r.relay, Some(3), "21,000 km beats 26,000 km");
        assert!((r.path - (21_000e3 + PLANE_LINK)).abs() < 1.0);
    }

    /// The detour costs the plane link and one more relay -- the mate has to
    /// turn the frame around (ADR-0023).
    #[test]
    fn the_detour_pays_the_plane_link_and_one_more_relay() {
        let direct = feeder_route(0, &[1, 3], |_| true, stub_ranges, PLANE_LINK, RELAY_DELAY)
            .expect("a route exists");
        let detour = feeder_route(0, &[1, 3], |a| a != 0, stub_ranges, PLANE_LINK, RELAY_DELAY)
            .expect("a route exists");
        let expected =
            (PLANE_LINK + 21_000e3 - 23_000e3) / crate::placement::SPEED_OF_LIGHT + RELAY_DELAY;
        assert!(
            (detour.latency - direct.latency - expected).abs() < 1e-9,
            "detour {} direct {}",
            detour.latency,
            direct.latency
        );
    }

    /// Both neighbours dark into this ring is the case the plane links cannot
    /// answer. `None` is what forces the session to re-anchor.
    #[test]
    fn no_route_when_the_whole_plane_is_dark_into_this_ring() {
        assert_eq!(
            feeder_route(0, &[1, 3], |_| false, stub_ranges, PLANE_LINK, RELAY_DELAY),
            None
        );
    }

    /// A live telescope the ring cannot SEE is not a route either, and the
    /// plane mates are still worth trying. Anchor 5 is alive but absent from
    /// `stub_ranges`, so the ring is blind to it; mate 3 is both alive and
    /// reachable, and the detour never touches anchor 5's own telescope.
    #[test]
    fn a_live_anchor_the_ring_cannot_see_still_tries_its_mates() {
        let r = feeder_route(5, &[1, 3], |_| true, stub_ranges, PLANE_LINK, RELAY_DELAY)
            .expect("a route exists");
        assert_eq!(r.relay, Some(3));
        assert!((r.path - (21_000e3 + PLANE_LINK)).abs() < 1.0);
    }

    /// A mate with a live telescope the ring still cannot see is not a relay.
    /// Both mates here are alive but neither is in `stub_ranges`, so the ring
    /// has no way up through either -- which is the case that strands a
    /// session even with the plane links fitted.
    #[test]
    fn a_mate_the_ring_cannot_reach_is_not_a_relay() {
        let r = feeder_route(0, &[2, 4], |a| a != 0, stub_ranges, PLANE_LINK, RELAY_DELAY);
        assert_eq!(r, None, "neither 2 nor 4 is reachable in stub_ranges");
    }
}
