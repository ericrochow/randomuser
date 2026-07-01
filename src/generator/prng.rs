use md5::{Digest, Md5};
use rand_mt::Mt19937GenRand32;

pub struct Prng {
    mt: Mt19937GenRand32,
}

impl Default for Prng {
    fn default() -> Self {
        Self {
            mt: Mt19937GenRand32::new(0),
        }
    }
}

impl Prng {
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed from a user-supplied string + page number, mirroring the original seedRNG():
    ///
    ///   seed = seed.len() == 18 ? &seed[..16] : seed
    ///   seed = if page != 1 { seed + page.to_string() } else { seed }
    ///   integer = parseInt(md5(seed).hex[..8], 16)
    ///   mersenne.seed(integer)
    pub fn seed_from_str(&mut self, seed: &str, page: u32) {
        let s = if seed.len() == 18 { seed.get(..16).unwrap_or(seed) } else { seed };
        let with_page = if page != 1 {
            format!("{}{}", s, page)
        } else {
            s.to_string()
        };

        let hash = Md5::digest(with_page.as_bytes());
        let hex = format!("{:x}", hash);
        let int = u32::from_str_radix(&hex[..8], 16).unwrap_or(0);

        self.mt = Mt19937GenRand32::new(int);
    }

    /// Generates a random integer in [min, max] inclusive, matching JS:
    ///   mersenne.rand(max - min + 1) + min
    /// which is:  (mt.next_u32() % span) + min  (modulo reduction, same bias as JS impl)
    pub fn range(&mut self, min: i64, max: i64) -> i64 {
        assert!(max >= min, "Prng::range: max ({max}) must be >= min ({min})");
        if max == min {
            return min;
        }
        let span = (max - min + 1) as u64;
        let r = (self.mt.next_u32() as u64) % span;
        min + r as i64
    }

    /// Pick a random element from a slice.
    ///
    /// Panics if `items` is empty — call sites are expected to validate their
    /// data at startup (`Generator::init`) rather than at request time.
    pub fn random_item<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        assert!(
            !items.is_empty(),
            "random_item: slice is empty — ensure all required data files are present"
        );
        let idx = self.range(0, (items.len() - 1) as i64) as usize;
        &items[idx]
    }

    /// random(mode, length) — generates a string of `length` random chars from the mode charset:
    ///   1  → hex lowercase      "abcdef1234567890"
    ///   2  → alphanumeric mixed "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890"
    ///   3  → digits             "0123456789"
    ///   4  → uppercase          "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
    ///   5  → digits excl. 0,1   "23456789"
    ///   6  → lowercase          "abcdefghijklmnopqrstuvwxyz"
    pub fn random_chars(&mut self, mode: u8, length: usize) -> String {
        let chars: &[u8] = match mode {
            1 => b"abcdef1234567890",
            2 => b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890",
            3 => b"0123456789",
            4 => b"ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            5 => b"23456789",
            6 => b"abcdefghijklmnopqrstuvwxyz",
            _ => unreachable!("random_chars called with invalid mode {mode}"),
        };
        (0..length)
            .map(|_| {
                let idx = self.range(0, (chars.len() - 1) as i64) as usize;
                chars[idx] as char
            })
            .collect()
    }

    /// Generate a v4 UUID string seeded from the MT (replaces faker.random.uuid()).
    pub fn gen_uuid(&mut self) -> String {
        let mut bytes = [0u8; 16];
        for chunk in bytes.chunks_mut(4) {
            let n = self.mt.next_u32();
            let b = n.to_le_bytes();
            chunk.copy_from_slice(&b[..chunk.len()]);
        }
        // Set version 4 and RFC 4122 variant bits
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5],
            bytes[6], bytes[7],
            bytes[8], bytes[9],
            bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        )
    }

    /// Generate a latitude string with 4 decimal places (replaces faker.address.latitude()).
    pub fn gen_latitude(&mut self) -> String {
        let raw = self.range(-900_000, 900_000);
        format!("{:.4}", raw as f64 / 10_000.0)
    }

    /// Generate a longitude string with 4 decimal places (replaces faker.address.longitude()).
    pub fn gen_longitude(&mut self) -> String {
        let raw = self.range(-1_800_000, 1_800_000);
        format!("{:.4}", raw as f64 / 10_000.0)
    }
}

/// Left-pad `n` to `width` with '0' (matches JS pad(n, width)).
pub fn pad(n: impl std::fmt::Display, width: usize) -> String {
    format!("{:0>width$}", n, width = width)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_is_deterministic() {
        let mut p1 = Prng::new();
        p1.seed_from_str("abc123", 1);
        let v1: Vec<i64> = (0..10).map(|_| p1.range(0, 100)).collect();

        let mut p2 = Prng::new();
        p2.seed_from_str("abc123", 1);
        let v2: Vec<i64> = (0..10).map(|_| p2.range(0, 100)).collect();

        assert_eq!(v1, v2);
    }

    #[test]
    fn different_seeds_differ() {
        let mut p1 = Prng::new();
        p1.seed_from_str("seed_a", 1);
        let v1: i64 = p1.range(0, 1_000_000);

        let mut p2 = Prng::new();
        p2.seed_from_str("seed_b", 1);
        let v2: i64 = p2.range(0, 1_000_000);

        assert_ne!(v1, v2);
    }

    #[test]
    fn page_changes_sequence() {
        let mut p1 = Prng::new();
        p1.seed_from_str("testseed", 1);
        let v1: i64 = p1.range(0, 1_000_000);

        let mut p2 = Prng::new();
        p2.seed_from_str("testseed", 2);
        let v2: i64 = p2.range(0, 1_000_000);

        assert_ne!(v1, v2);
    }

    #[test]
    fn random_chars_lengths() {
        let mut p = Prng::new();
        p.seed_from_str("test", 1);
        for mode in 1u8..=6 {
            let s = p.random_chars(mode, 8);
            assert_eq!(s.len(), 8, "mode {mode} produced wrong length");
        }
    }

    #[test]
    fn uuid_is_v4_format() {
        let mut p = Prng::new();
        p.seed_from_str("uuid_test", 1);
        let u = p.gen_uuid();
        let parts: Vec<&str> = u.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 4);
        assert_eq!(parts[4].len(), 12);
        assert_eq!(&parts[2][..1], "4", "version nibble must be 4");
        assert!(
            matches!(&parts[3][..1], "8" | "9" | "a" | "b"),
            "variant nibble must be 8/9/a/b"
        );
    }

    #[test]
    fn pad_zero_fills() {
        assert_eq!(pad(5u32, 2), "05");
        assert_eq!(pad(12u32, 2), "12");
        assert_eq!(pad(1u32, 4), "0001");
    }
}
