//! Shared types for the Top-K demo: Count-Min Sketch, Event schema, Redis key helpers.
//!
//! The CMS byte layout is fixed so that the Flink Java job and this Rust crate
//! can read/write the exact same bytes via Redis:
//!
//!   bytes[0..4]   = width  (u32 little-endian)
//!   bytes[4..8]   = depth  (u32 little-endian)
//!   bytes[8..]    = width * depth counters (u32 little-endian, row-major: row i cell j at 8 + (i*width + j)*4)
//!
//! Hash family (matching Java side): h_i(x) = ((A[i] * x64 + B[i]) mod P) mod width
//!   where x64 = splitmix64 digest of the item bytes, P = 2^61 - 1.

use serde::{Deserialize, Serialize};

pub const CMS_WIDTH: usize = 2719;
pub const CMS_DEPTH: usize = 7;
pub const MERSENNE_P: u64 = (1u64 << 61) - 1;

/// Hash family constants. MUST match the Java side exactly.
pub const HASH_A: [u64; CMS_DEPTH] = [
    0x9E3779B97F4A7C15,
    0xBF58476D1CE4E5B9,
    0x94D049BB133111EB,
    0xD6E8FEB86659FD93,
    0xA24BAED4963EE407,
    0x85EBCA6B2A6D3F27,
    0xC2B2AE3D27D4EB4F,
];

pub const HASH_B: [u64; CMS_DEPTH] = [
    0x165667B19E3779F9,
    0x3C6EF372FE94F82B,
    0xA54FF53A5F1D36F1,
    0x510E527FADE682D1,
    0x9B05688C2B3E6C1F,
    0x1F83D9ABFB41BD6B,
    0x5BE0CD19137E2179,
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub user_id: String,
    pub item_id: String,
    /// Epoch seconds.
    pub ts: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeavyHitter {
    pub item: String,
    pub est: u64,
}

#[inline]
fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E3779B97F4A7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
    x ^ (x >> 31)
}

/// Digest arbitrary bytes to a single u64 using FNV-1a + splitmix64 finalize.
/// Matches Java `itemHash64`.
pub fn item_hash64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xCBF29CE484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001B3);
    }
    splitmix64(h)
}

#[inline]
fn row_hash(row: usize, x64: u64) -> usize {
    // Mersenne modulus trick: compute (A*x + B) mod (2^61 - 1) without overflow.
    let a = HASH_A[row] % MERSENNE_P;
    let b = HASH_B[row] % MERSENNE_P;
    let x = x64 % MERSENNE_P;
    let prod = mulmod_p61(a, x);
    let sum = (prod + b) % MERSENNE_P;
    (sum as usize) % CMS_WIDTH
}

/// (a * b) mod (2^61 - 1) using u128 intermediate.
#[inline]
fn mulmod_p61(a: u64, b: u64) -> u64 {
    let p = MERSENNE_P as u128;
    ((a as u128 * b as u128) % p) as u64
}

#[derive(Clone, Debug)]
pub struct CountMinSketch {
    width: usize,
    depth: usize,
    counters: Vec<u32>,
}

impl Default for CountMinSketch {
    fn default() -> Self {
        Self::new()
    }
}

impl CountMinSketch {
    pub fn new() -> Self {
        Self {
            width: CMS_WIDTH,
            depth: CMS_DEPTH,
            counters: vec![0u32; CMS_WIDTH * CMS_DEPTH],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn depth(&self) -> usize {
        self.depth
    }

    pub fn add(&mut self, item: &str) -> u64 {
        self.add_n(item, 1)
    }

    pub fn add_n(&mut self, item: &str, count: u32) -> u64 {
        let x = item_hash64(item.as_bytes());
        let mut min_after = u32::MAX;
        for row in 0..self.depth {
            let col = row_hash(row, x);
            let idx = row * self.width + col;
            let v = self.counters[idx].saturating_add(count);
            self.counters[idx] = v;
            if v < min_after {
                min_after = v;
            }
        }
        min_after as u64
    }

    pub fn estimate(&self, item: &str) -> u64 {
        let x = item_hash64(item.as_bytes());
        let mut min = u32::MAX;
        for row in 0..self.depth {
            let col = row_hash(row, x);
            let v = self.counters[row * self.width + col];
            if v < min {
                min = v;
            }
        }
        min as u64
    }

    /// Cell-wise saturating add. Both sketches must share width/depth.
    pub fn merge(&mut self, other: &CountMinSketch) {
        assert_eq!(self.width, other.width);
        assert_eq!(self.depth, other.depth);
        for (a, b) in self.counters.iter_mut().zip(other.counters.iter()) {
            *a = a.saturating_add(*b);
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(8 + self.counters.len() * 4);
        out.extend_from_slice(&(self.width as u32).to_le_bytes());
        out.extend_from_slice(&(self.depth as u32).to_le_bytes());
        for c in &self.counters {
            out.extend_from_slice(&c.to_le_bytes());
        }
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 8 {
            return Err("CMS buffer too small".into());
        }
        let width = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
        let depth = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
        let expected = 8 + width * depth * 4;
        if bytes.len() != expected {
            return Err(format!(
                "CMS length mismatch: got {}, expected {}",
                bytes.len(),
                expected
            ));
        }
        let mut counters = Vec::with_capacity(width * depth);
        let mut off = 8;
        for _ in 0..width * depth {
            counters.push(u32::from_le_bytes(bytes[off..off + 4].try_into().unwrap()));
            off += 4;
        }
        Ok(Self { width, depth, counters })
    }
}

/// Redis key conventions. Keep identical to the Java side.
pub mod keys {
    pub const ALL_TIME_CMS: &str = "topk:cms:all_time";
    pub const ALL_TIME_HEAP: &str = "topk:heap:all_time";

    pub fn minute_cms(epoch_min: i64) -> String {
        format!("topk:cms:min:{}", epoch_min)
    }
    pub fn minute_heap(epoch_min: i64) -> String {
        format!("topk:heap:min:{}", epoch_min)
    }
}

#[inline]
pub fn epoch_minute(ts_seconds: i64) -> i64 {
    ts_seconds.div_euclid(60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_estimate_exact_for_few_items() {
        let mut c = CountMinSketch::new();
        for _ in 0..100 {
            c.add("alpha");
        }
        for _ in 0..50 {
            c.add("beta");
        }
        assert_eq!(c.estimate("alpha"), 100);
        assert_eq!(c.estimate("beta"), 50);
        assert_eq!(c.estimate("never_seen"), 0);
    }

    #[test]
    fn merge_is_additive() {
        let mut a = CountMinSketch::new();
        let mut b = CountMinSketch::new();
        a.add_n("x", 10);
        b.add_n("x", 25);
        a.merge(&b);
        assert_eq!(a.estimate("x"), 35);
    }

    #[test]
    fn roundtrip_bytes() {
        let mut c = CountMinSketch::new();
        c.add_n("foo", 7);
        let buf = c.to_bytes();
        let d = CountMinSketch::from_bytes(&buf).unwrap();
        assert_eq!(d.estimate("foo"), 7);
        assert_eq!(d.width(), CMS_WIDTH);
        assert_eq!(d.depth(), CMS_DEPTH);
    }

    #[test]
    fn error_bound_under_uniform_load() {
        // With w=2719, d=7 and total N inserts, expected over-count <= e*N/w with prob >= 1 - e^-d.
        let mut c = CountMinSketch::new();
        let n = 100_000u32;
        for i in 0..n {
            c.add(&format!("item_{}", i));
        }
        // For a single previously-unseen probe, estimate should be small compared to N.
        let est = c.estimate("absent_probe");
        let bound = (std::f64::consts::E * n as f64 / CMS_WIDTH as f64) as u64 * 3;
        assert!(est <= bound, "estimate {} exceeded loose bound {}", est, bound);
    }
}
