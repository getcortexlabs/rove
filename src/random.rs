//! xoshiro256++ (Blackman and Vigna, 2021), seeded through SplitMix64.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Random {
    state: [u64; 4],
}

impl Random {
    pub fn new(seed: u64) -> Self {
        let mut mix = seed;
        let mut next = || {
            mix = mix.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = mix;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        };
        Self {
            state: [next(), next(), next(), next()],
        }
    }

    pub(crate) fn state(&self) -> [u64; 4] {
        self.state
    }

    pub(crate) fn from_state(state: [u64; 4]) -> Option<Self> {
        (state != [0; 4]).then_some(Self { state })
    }

    pub fn next_u64(&mut self) -> u64 {
        let s = &mut self.state;
        let result = s[0].wrapping_add(s[3]).rotate_left(23).wrapping_add(s[0]);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        result
    }

    /// Uniform on `[0, 1)`, from the top 53 bits.
    pub fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform on `0..n`, without modulo bias (Lemire, 2019).
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0, "cannot draw from an empty range");
        let n = n as u64;
        let mut product = u128::from(self.next_u64()) * u128::from(n);
        if (product as u64) < n {
            let threshold = n.wrapping_neg() % n;
            while (product as u64) < threshold {
                product = u128::from(self.next_u64()) * u128::from(n);
            }
        }
        (product >> 64) as usize
    }

    pub fn coin(&mut self) -> bool {
        self.next_u64() >> 63 == 1
    }

    /// Standard normal, by the Box–Muller transform.
    pub fn normal(&mut self) -> f64 {
        let radius = (-2.0 * (1.0 - self.uniform()).ln()).sqrt();
        radius * (std::f64::consts::TAU * self.uniform()).cos()
    }

    /// Exponential with rate one.
    pub fn exponential(&mut self) -> f64 {
        -(1.0 - self.uniform()).ln()
    }
}

#[cfg(test)]
mod tests {
    use super::Random;

    #[test]
    fn matches_the_reference_sequence() {
        let mut random = Random {
            state: [1, 2, 3, 4],
        };
        let outputs: Vec<u64> = (0..6).map(|_| random.next_u64()).collect();
        assert_eq!(
            outputs,
            [
                41943041,
                58720359,
                3588806011781223,
                3591011842654386,
                9228616714210784205,
                9973669472204895162
            ]
        );
    }

    #[test]
    fn seeding_runs_splitmix64() {
        let mut random = Random::new(20261028);
        assert_eq!(
            random.state,
            [
                0x94df_6083_30a3_24cb,
                0xf86c_1768_6e39_0f05,
                0x625a_c457_9f45_cda9,
                0x1330_a455_fa5a_79a6
            ]
        );
        assert_eq!(random.next_u64(), 104954236386094285);
    }

    #[test]
    fn draws_have_the_expected_moments() {
        let mut random = Random::new(7);
        let n = 200_000;
        let mut counts = [0usize; 6];
        let (mut uniform, mut normal, mut square, mut exponential) = (0.0, 0.0, 0.0, 0.0);
        for _ in 0..n {
            let u = random.uniform();
            assert!((0.0..1.0).contains(&u));
            uniform += u;
            let z = random.normal();
            normal += z;
            square += z * z;
            exponential += random.exponential();
            counts[random.below(6)] += 1;
        }
        let n = n as f64;
        assert!((uniform / n - 0.5).abs() < 0.005);
        assert!((normal / n).abs() < 0.01);
        assert!((square / n - 1.0).abs() < 0.015);
        assert!((exponential / n - 1.0).abs() < 0.01);
        for count in counts {
            assert!((count as f64 / n - 1.0 / 6.0).abs() < 0.005, "{counts:?}");
        }
    }
}
