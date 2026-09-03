// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! First-order orbital screening for constellation trade studies.
//!
//! Everything is parameterized by a [`CentralBody`]; no planet is hard-coded.
//! Units are SI throughout: meters, seconds, kilograms, radians.

pub mod acquisition;
pub mod activation;
pub mod atmosphere;
pub mod backbone;
pub mod beams;
mod body;
pub mod circular;
pub mod climate;
pub mod constellation;
pub mod coverage;
pub mod duty;
pub mod handover;
pub mod hill;
pub mod oblateness;
pub mod placement;
pub mod plane_tracking;
pub mod radio;
pub mod relativity;
pub mod reliability;
pub mod routing;
pub mod spin_orbit;
pub mod station_keeping;
pub mod topology;
pub mod traffic;
pub mod walker;

pub use body::{CentralBody, EARTH_MU};
