// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! SINR→BLER as a pluggable step table (one curve in slice 1; per-MCS
//! later). Rows sorted by ascending SINR; lookup takes the last row
//! with sinr <= x; below the first row BLER is 1.0 (no link).

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum BlerError {
    #[error("BLER curve must be non-empty, sorted by strictly ascending SINR, BLER within [0,1]")]
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlerCurve {
    rows: Vec<(f64, f64)>,
}

impl BlerCurve {
    pub fn new(rows: Vec<(f64, f64)>) -> Result<Self, BlerError> {
        if rows.is_empty()
            || rows.windows(2).any(|w| w[0].0 >= w[1].0)
            || rows.iter().any(|(_, b)| !(0.0..=1.0).contains(b))
        {
            return Err(BlerError::Invalid);
        }
        Ok(Self { rows })
    }

    pub fn bler(&self, sinr_db: f64) -> f64 {
        let idx = self.rows.partition_point(|(s, _)| *s <= sinr_db);
        if idx == 0 {
            1.0
        } else {
            self.rows[idx - 1].1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_lookup_and_floor() {
        let c = BlerCurve::new(vec![(-5.0, 1.0), (0.0, 0.3), (10.0, 0.001)]).unwrap();
        assert_eq!(c.bler(-10.0), 1.0); // below first row
        assert_eq!(c.bler(-5.0), 1.0);
        assert_eq!(c.bler(0.0), 0.3);
        assert_eq!(c.bler(5.0), 0.3);
        assert_eq!(c.bler(20.0), 0.001);
    }

    #[test]
    fn rejects_bad_curves() {
        assert!(BlerCurve::new(vec![]).is_err());
        assert!(BlerCurve::new(vec![(0.0, 0.5), (0.0, 0.4)]).is_err()); // not strictly ascending
        assert!(BlerCurve::new(vec![(0.0, 1.5)]).is_err()); // BLER out of range
    }
}
