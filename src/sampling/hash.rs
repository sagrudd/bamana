#[derive(Debug, Clone)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

pub fn fnv1a64(bytes: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut hash = OFFSET_BASIS;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

pub fn should_keep_fraction(sample: u64, fraction: f64) -> bool {
    if fraction >= 1.0 {
        return true;
    }
    if fraction <= 0.0 {
        return false;
    }

    let threshold = (fraction * (u64::MAX as f64)).floor() as u64;
    sample <= threshold
}

#[cfg(test)]
mod tests {
    use super::{SplitMix64, fnv1a64, should_keep_fraction};

    #[test]
    fn splitmix64_is_reproducible() {
        let mut left = SplitMix64::new(42);
        let mut right = SplitMix64::new(42);
        for _ in 0..16 {
            assert_eq!(left.next_u64(), right.next_u64());
        }
    }

    #[test]
    fn fnv1a64_is_stable() {
        assert_eq!(fnv1a64(b"bamana"), 0xa6c6_f056_33a5_f9f9);
    }

    #[test]
    fn full_fraction_keeps_everything() {
        assert!(should_keep_fraction(0, 1.0));
        assert!(should_keep_fraction(u64::MAX, 1.0));
    }
}
