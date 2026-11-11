//! Deterministic randomness. Everything generated from a seed goes through here,
//! so the same seed always yields the same world.

/// SplitMix64 step: also used as a general-purpose 64-bit hash mixer.
#[inline]
pub fn mix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Stable hash of a seed and integer coordinates (e.g. a chunk position).
pub fn hash_coords(seed: u64, x: i32, y: i32, z: i32) -> u64 {
    let mut h = mix64(seed);
    h = mix64(h ^ x as u32 as u64);
    h = mix64(h ^ ((y as u32 as u64) << 21));
    mix64(h ^ ((z as u32 as u64) << 42))
}

pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: mix64(seed) }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix64(self.state)
    }

    /// Uniform in [0, 1).
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }

    /// Uniform integer in [lo, hi).
    pub fn range_i(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.next_u64() % (hi - lo).max(1) as u64) as i32
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.f32() < p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every city is built from these numbers: if this fails, every seed now gives another city.
    /// Only change the values when that is the intent.
    #[test]
    fn the_sequences_never_change() {
        // SplitMix64's published first output for state 0.
        assert_eq!(mix64(0), 0xE220_A839_7B1D_CDAF);
        assert_eq!(mix64(1), 0x910A_2DEC_8902_5CC1);
        assert_eq!(hash_coords(1, 0, 0, 0), 0xE281_95DD_D9EE_4956);
        assert_eq!(hash_coords(42, -3, 7, -11), 0x1F76_C14E_3C59_213B);
        let mut r = Rng::new(42);
        assert_eq!(
            [r.next_u64(), r.next_u64(), r.next_u64(), r.next_u64()],
            [
                0xF4AB_D143_FEB2_4055,
                0x7C81_6738_C129_03B2,
                0x113E_5DEC_6F8F_D8A8,
                0xAD4A_5990_62FD_1739,
            ]
        );
        let mut r = Rng::new(42);
        assert_eq!(r.f32(), 16_034_769.0 / (1 << 24) as f32);
        assert_eq!(r.range_i(-5, 17), 13);
    }
}
