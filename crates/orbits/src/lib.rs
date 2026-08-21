//! First-order orbital screening for constellation trade studies.
//!
//! Everything is parameterized by a [`CentralBody`]; no planet is hard-coded.
//! Units are SI throughout: meters, seconds, kilograms, radians.

mod body;
pub mod circular;

pub use body::{CentralBody, EARTH_MU};
