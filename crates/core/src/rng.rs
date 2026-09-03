// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Master-seed → per-model deterministic RNG derivation.
//!
//! Every stochastic draw in the simulator flows through a `ChaCha12Rng`
//! seeded from the scenario's master seed and the owning model's stable
//! path string (e.g. `"medium:access"`). The hash is hand-rolled
//! (FNV-1a + splitmix64) so seeds are stable across Rust releases and
//! platforms — `DefaultHasher` guarantees neither.

use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha12Rng;

/// Stable seed derivation: FNV-1a 64 over `path`, mixed with `master`
/// through splitmix64.
pub fn derive_seed(master: u64, path: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in path.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let mut z = master ^ h;
    z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// The per-model RNG every model must use for its stochastic draws.
pub fn model_rng(master: u64, path: &str) -> ChaCha12Rng {
    ChaCha12Rng::seed_from_u64(derive_seed(master, path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn same_inputs_same_seed() {
        assert_eq!(
            derive_seed(42, "medium:access"),
            derive_seed(42, "medium:access")
        );
    }

    #[test]
    fn different_paths_different_seeds() {
        assert_ne!(
            derive_seed(42, "medium:access"),
            derive_seed(42, "medium:feeder")
        );
    }

    #[test]
    fn different_masters_different_seeds() {
        assert_ne!(
            derive_seed(42, "medium:access"),
            derive_seed(43, "medium:access")
        );
    }

    #[test]
    fn rng_streams_are_reproducible() {
        let mut a = model_rng(7, "node:term-a");
        let mut b = model_rng(7, "node:term-a");
        let xs: Vec<f64> = (0..8).map(|_| a.random::<f64>()).collect();
        let ys: Vec<f64> = (0..8).map(|_| b.random::<f64>()).collect();
        assert_eq!(xs, ys);
    }
}
