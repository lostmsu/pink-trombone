#[cfg(test)]
pub mod xorshift {
    use crate::NoiseSource;

    // from https://gitlab.com/pomma89/troschuetz-random/-/blob/main/src/Troschuetz.Random/Generators/XorShift128Generator.cs
    struct BaseGenerator {
        bit_buffer: u32,
        bit_count: i32,
        seed: u32,
    }

    impl BaseGenerator {
        pub fn reset(&mut self, seed: u32) {
            self.bit_buffer = 0;
            self.bit_count = 0;
            self.seed = seed;
        }
    }

    pub struct XorShift128 {
        x: u64,
        y: u64,
        bytes_available: bool,
        base: BaseGenerator,
    }

    const SEED_X: u64 = 521288629 << 32;
    const SEED_Y: u64 = 362436069;

    impl XorShift128 {
        pub fn new(seed: u32) -> XorShift128 {
            let mut gen = XorShift128 {
                base: BaseGenerator {
                    bit_buffer: 0,
                    bit_count: 0,
                    seed: seed,
                },
                x: 0,
                y: 0,
                bytes_available: false,
            };

            // base code
            gen.reset(seed);

            // this code
            gen
        }

        pub fn next_f64(&mut self) -> f64 {
            let mut tx = self.x;
            let ty = self.y;
            self.x = ty;
            tx ^= tx << 23;
            tx ^= tx >> 17;
            tx ^= ty ^ (ty >> 26);
            self.y = tx;
            self.bytes_available = false;

            let result = to_f64(tx.overflowing_add(ty).0);
            assert!(result >= 0.0 && result <= 1.0);
            result
        }

        pub fn next_u64(&mut self) -> u64 {
            let mut tx = self.x;
            let ty = self.y;
            self.x = ty;
            tx ^= tx << 23;
            tx ^= tx >> 17;
            tx ^= ty ^ (ty >> 26);
            self.y = tx;
            self.bytes_available = false;
            tx.overflowing_add(ty).0
        }

        pub fn reset(&mut self, seed: u32) {
            // base
            self.base.reset(seed);

            // this
            self.x = SEED_X + seed as u64;
            self.y = SEED_Y.overflowing_mul((seed as u64) << 32).0;
            self.bytes_available = false;

            self.next_u64();
        }
    }

    impl NoiseSource<f64> for XorShift128 {
        fn noise(&mut self) -> f64 {
            self.next_f64()
        }
    }

    fn to_f64(mut value: u64) -> f64 {
        value = (value >> 12) | 0x3FF0000000000000;
        let res = unsafe { std::mem::transmute::<u64, f64>(value) };
        res - 1.0
    }

    mod tests {
        // Note this useful idiom: importing names from outer (for mod tests) scope.
        use super::*;

        #[test]
        fn reproducible() {
            let mut generator = XorShift128::new(9452);
            let vals: Vec<f64> = (0..461456).map(|_| generator.next_f64()).collect();
            assert_eq!(format!("{:.10}", vals.last().unwrap()), "0.5612585810");
        }
    }
}
