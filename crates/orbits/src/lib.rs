//! First-order orbital screening for constellation trade studies.
//!
//! Everything is parameterized by a [`CentralBody`]; no planet is hard-coded.
//! Units are SI throughout: meters, seconds, kilograms, radians.

mod body;
pub mod acquisition;
pub mod backbone;
pub mod beams;
pub mod circular;
pub mod constellation;
pub mod coverage;
pub mod hill;
pub mod placement;
pub mod plane_tracking;
pub mod radio;
pub mod relativity;
pub mod spin_orbit;

pub use body::{CentralBody, EARTH_MU};
