//! PopCap `MTRand`: MT19937 with 31-bit output. See `docs/re/framework/rng.md`.

const N: usize = 624;
const M: usize = 397;
const MATRIX_A: u32 = 0x9908_B0DF;
const UPPER_MASK: u32 = 0x8000_0000;
const LOWER_MASK: u32 = 0x7FFF_FFFF;
/// Seed substituted for 0, matching the original.
const DEFAULT_SEED: u32 = 4357;

#[derive(Clone, PartialEq, Eq)]
pub struct MtRand {
    mt: [u32; N],
    mti: usize,
}

impl MtRand {
    pub fn new(seed: u32) -> Self {
        let mut r = Self { mt: [0; N], mti: N };
        r.seed(seed);
        r
    }

    pub fn seed(&mut self, seed: u32) {
        self.mt[0] = if seed == 0 { DEFAULT_SEED } else { seed };
        for i in 1..N {
            let prev = self.mt[i - 1];
            self.mt[i] = 1_812_433_253u32
                .wrapping_mul(prev ^ (prev >> 30))
                .wrapping_add(i as u32);
        }
        self.mti = N;
    }

    /// Restore exact generator state, e.g. captured from the original's memory.
    /// `mti` must be in `0..=624`.
    pub fn from_state(mt: [u32; N], mti: usize) -> Self {
        assert!(mti <= N, "mti out of range: {mti}");
        Self { mt, mti }
    }

    pub fn state(&self) -> (&[u32; N], usize) {
        (&self.mt, self.mti)
    }

    /// Next value in `0..=0x7FFF_FFFF`.
    pub fn next_u31(&mut self) -> u32 {
        if self.mti >= N {
            self.regenerate();
        }
        let mut y = self.mt[self.mti];
        self.mti += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9D2C_5680;
        y ^= (y << 15) & 0xEFC6_0000;
        y ^= y >> 18;
        y & LOWER_MASK
    }

    /// `next_u31() % range`, the form inlined at the original's call sites.
    pub fn next_range(&mut self, range: u32) -> u32 {
        self.next_u31() % range
    }

    fn regenerate(&mut self) {
        let mag01 = |y: u32| if y & 1 == 0 { 0 } else { MATRIX_A };
        for k in 0..N {
            let y = (self.mt[k] & UPPER_MASK) | (self.mt[(k + 1) % N] & LOWER_MASK);
            self.mt[k] = self.mt[(k + M) % N] ^ (y >> 1) ^ mag01(y);
        }
        self.mti = 0;
    }
}

impl std::fmt::Debug for MtRand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MtRand")
            .field("mti", &self.mti)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference MT19937 `genrand_int32` outputs for seed 5489, masked to 31 bits.
    #[test]
    fn matches_reference_mt19937() {
        let mut r = MtRand::new(5489);
        let expected = [
            3_499_211_612u32,
            581_869_302,
            3_890_346_734,
            3_586_334_585,
            545_404_204,
        ];
        for e in expected {
            assert_eq!(r.next_u31(), e & LOWER_MASK);
        }
    }

    /// State words captured from the original's app RNG right after seeding
    /// (seed 0x7FEF1344, mti = 624). First and last few words.
    #[test]
    fn matches_original_seeding() {
        let r = MtRand::new(0x7FEF_1344);
        let (mt, mti) = r.state();
        assert_eq!(mti, 624);
        assert_eq!(
            &mt[..4],
            &[0x7fef_1344, 0x0d85_873a, 0xf5a2_63e4, 0xffd9_0926]
        );
        assert_eq!(
            &mt[620..],
            &[0xa609_465c, 0x0ce3_1383, 0x679c_d01d, 0x4d01_197b]
        );
    }

    #[test]
    fn zero_seed_uses_default() {
        assert_eq!(MtRand::new(0), MtRand::new(DEFAULT_SEED));
    }

    #[test]
    fn state_roundtrip() {
        let mut a = MtRand::new(1234);
        for _ in 0..700 {
            a.next_u31();
        }
        let (mt, mti) = a.state();
        let mut b = MtRand::from_state(*mt, mti);
        for _ in 0..1000 {
            assert_eq!(a.next_u31(), b.next_u31());
        }
    }
}
