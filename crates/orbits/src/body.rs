/// Standard gravitational parameter of Earth, m³/s².
pub const EARTH_MU: f64 = 3.986004418e14;

/// A spherically symmetric central body.
///
/// `rotation_period` is the sidereal rotation period. For a synchronously
/// rotating (tidally locked) planet it equals the orbital period around the
/// star.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CentralBody {
    /// Standard gravitational parameter μ = GM, m³/s².
    pub mu: f64,
    /// Mean radius, m.
    pub radius: f64,
    /// Sidereal rotation period, s.
    pub rotation_period: f64,
}

impl CentralBody {
    /// Build a body from a mass in Earth masses, radius in meters, and
    /// rotation period in seconds.
    pub fn from_earth_masses(earth_masses: f64, radius: f64, rotation_period: f64) -> Self {
        Self {
            mu: EARTH_MU * earth_masses,
            radius,
            rotation_period,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_earth_masses_scales_mu() {
        let body = CentralBody::from_earth_masses(1.0, 6.371e6, 11.2 * 86_400.0);
        assert_eq!(body.mu, EARTH_MU);
        assert_eq!(body.radius, 6.371e6);
        assert_eq!(body.rotation_period, 967_680.0);

        let half = CentralBody::from_earth_masses(0.5, 6.371e6, 967_680.0);
        assert_eq!(half.mu, EARTH_MU * 0.5);
    }
}
