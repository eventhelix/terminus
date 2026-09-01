// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! What it costs to put a ring where you want it, and to keep it there.
//!
//! Two questions decide whether a constellation design may lean on a phase
//! relationship between its rings.
//!
//! **Can you launch into it?** A launch reaches a given inertial plane only
//! while the launch site rotates under that plane. On a tidally locked world
//! the site comes back around once per *rotation* - here once per 11.2 days -
//! so plane windows are rare, and hitting a particular along-orbit slot inside
//! one is rarer still.
//!
//! **Can you hold it?** Nothing holds a phase for free. Two satellites whose
//! semi-major axes differ by even a hundred metres keep different periods and
//! walk apart forever; see [`slot_walk_time`]. The walk is cheap to stop
//! ([`phase_hold_dv`]) but never finished, and it must be paid on every
//! spacecraft for the life of the fleet.
//!
//! A design whose coverage does not depend on inter-ring phase escapes both
//! questions: rings may be launched in any order, a slipped window costs
//! nothing, and no propellant is spent holding rings against each other.

use crate::CentralBody;

use std::f64::consts::PI;

/// How often a launch site returns beneath a given inertial orbital plane:
/// once per rotation of the planet.
pub fn plane_launch_window_interval(body: &CentralBody) -> f64 {
    body.rotation_period
}

/// Time between successive planes' launch windows for a wheel of `planes`
/// evenly spread over 180 degrees.
///
/// The same beat as the duty-ring shift change: the terminator and the launch
/// site sweep the ring nodes at the same rate.
pub fn plane_window_spacing(body: &CentralBody, planes: usize) -> f64 {
    body.rotation_period / (2.0 * planes as f64)
}

/// Along-orbit drift rate (rad/s) of a satellite injected `delta_a` (m) away
/// from the ring's nominal semi-major axis.
///
/// Mean motion goes as `a^(-3/2)`, so `dn/n = -(3/2) da/a`: a satellite
/// injected high falls behind, one injected low runs ahead, and neither stops
/// on its own.
pub fn along_track_drift_rate(body: &CentralBody, altitude: f64, delta_a: f64) -> f64 {
    let a = body.radius + altitude;
    let n = 2.0 * PI / crate::circular::orbital_period(body, altitude);
    1.5 * n * delta_a / a
}

/// Time (s) for that drift to carry a satellite through one whole in-ring slot
/// of a `sats_per_plane` ring - the point at which an unmaintained phase plan
/// has become meaningless.
pub fn slot_walk_time(
    body: &CentralBody,
    altitude: f64,
    delta_a: f64,
    sats_per_plane: usize,
) -> f64 {
    let slot = 2.0 * PI / sats_per_plane as f64;
    slot / along_track_drift_rate(body, altitude, delta_a).abs()
}

/// Velocity change (m/s) needed to null an along-track drift caused by a
/// semi-major-axis error of `delta_a` (m), from the vis-viva relation
/// `dv = n da / 2`.
///
/// Small per correction; the cost is that it recurs forever, on every
/// spacecraft whose phase the design depends on.
pub fn phase_hold_dv(body: &CentralBody, altitude: f64, delta_a: f64) -> f64 {
    let n = 2.0 * PI / crate::circular::orbital_period(body, altitude);
    0.5 * n * delta_a.abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference_planet() -> CentralBody {
        CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0)
    }

    fn assert_close(actual: f64, expected: f64, rel_tol: f64) {
        let rel = ((actual - expected) / expected).abs();
        assert!(
            rel < rel_tol,
            "actual {actual}, expected {expected}, rel err {rel}"
        );
    }

    #[test]
    fn plane_windows_are_rare_and_evenly_spaced() {
        let p = reference_planet();
        // One window per plane per 11.2-day rotation...
        assert_close(plane_launch_window_interval(&p) / 86_400.0, 11.2, 1e-12);
        // ...and six planes share that rotation, 22.4 h apart: the same beat
        // as the duty-ring shift change.
        assert_close(plane_window_spacing(&p, 6) / 3600.0, 22.4, 1e-12);
    }

    #[test]
    fn a_hundred_metre_injection_error_eats_a_slot_within_two_years() {
        let p = reference_planet();
        let walk = slot_walk_time(&p, 2_200e3, 100.0, 12) / 86_400.0;
        assert!((400.0..470.0).contains(&walk), "slot walk {walk} days");
        // Stopping it is nearly free per burn - the cost is that it never ends.
        let dv = phase_hold_dv(&p, 2_200e3, 100.0);
        assert!(dv < 0.05, "dv {dv} m/s");
    }

    #[test]
    fn drift_scales_linearly_with_injection_error() {
        let p = reference_planet();
        let one = along_track_drift_rate(&p, 2_200e3, 100.0);
        let ten = along_track_drift_rate(&p, 2_200e3, 1_000.0);
        assert_close(ten / one, 10.0, 1e-12);
        // A kilometre off walks a 30 deg slot in well under two months.
        let walk = slot_walk_time(&p, 2_200e3, 1_000.0, 12) / 86_400.0;
        assert!(walk < 60.0, "slot walk {walk} days");
    }
}
