use std::fmt::Display;
use std::io::stdin;
use std::str::{FromStr, SplitWhitespace};

use bitintr::Pdep;
use num::PrimInt;

pub fn pop_lsb64(x: &mut u64) -> u32 {
    let shift = x.trailing_zeros();
    *x &= *x - 1;
    shift
}

pub fn pop_lsb128(x: &mut u128) -> u32 {
    let shift = x.trailing_zeros();
    *x &= *x - 1;
    shift
}

pub fn ith_one_u64(idx: usize, val: u64) -> usize {
    debug_assert!(idx < val.count_ones() as usize);
    (1 << idx).pdep(val).trailing_zeros() as usize
}

pub fn ith_one_u128(idx: usize, val: u128) -> usize {
    let lower_bits = (val & u64::MAX as u128) as u64;
    let num_lower_ones = lower_bits.count_ones() as usize;
    if idx < num_lower_ones {
        ith_one_u64(idx, lower_bits)
    } else {
        let upper_bits = (val >> 64) as u64;
        ith_one_u64(idx - num_lower_ones, upper_bits) + 64
    }
}

pub fn parse_int_from_str<T: PrimInt + FromStr>(as_str: &str, name: &str) -> Result<T, String> {
    // for some weird Rust reason, parse::<T>() returns a completely unbounded Err on failure,
    // so we just write the error message ourselves
    as_str
        .parse::<T>()
        .map_err(|_err| format!("couldn't parse {name}"))
}

pub fn parse_int<T: PrimInt + FromStr + Display>(
    words: &mut SplitWhitespace,
    name: &str,
) -> Result<T, String> {
    parse_int_from_str(words.next().ok_or_else(|| format!("missing {name}"))?, name)
}

pub fn parse_int_from_stdin<T: PrimInt + FromStr>() -> Result<T, String> {
    let mut s = String::default();
    stdin().read_line(&mut s).map_err(|e| e.to_string())?;
    parse_int_from_str(s.trim(), "integer")
}

#[cfg(test)]
mod tests {
    use rand::{thread_rng, Rng};

    use crate::general::common::{ith_one_u128, ith_one_u64, pop_lsb128, pop_lsb64};

    #[test]
    fn pop_lsb64_test() {
        let mut x = 1;
        assert_eq!(pop_lsb64(&mut x), 0);
        assert_eq!(x, 0);
        x = 2;
        assert_eq!(pop_lsb64(&mut x), 1);
        assert_eq!(x, 0);
        x = 3;
        assert_eq!(pop_lsb64(&mut x), 0);
        assert_eq!(x, 2);
        x = 0b110001;
        assert_eq!(pop_lsb64(&mut x), 0);
        assert_eq!(x, 0b110000);
        x = 0b1100101100111001_0000_0000_0000_0000_0000;
        assert_eq!(pop_lsb64(&mut x), 20);
        assert_eq!(x, 0b1100101100111000_0000_0000_0000_0000_0000);
    }

    #[test]
    fn pop_lsb128_test() {
        let mut rng = thread_rng();
        for _ in 0..10_000 {
            let mut val = rng.gen_range(0..=u64::MAX);
            let mut val_u128 = val as u128;
            assert_eq!(pop_lsb64(&mut val), pop_lsb128(&mut val_u128));
            assert_eq!(val, val_u128 as u64);
        }
        let mut val = u64::MAX as u128 + 1;
        assert_eq!(pop_lsb128(&mut val), 64);
        assert_eq!(val, 0);
        val = (0b10001010110100101011010 << 64) + 0b10010100011;
        let copy = val;
        assert_eq!(pop_lsb128(&mut val), 0);
        assert_eq!(val, copy - 1);
        val = 0b10001010110100101011010 << 64;
        let copy = val;
        assert_eq!(pop_lsb128(&mut val), 65);
        assert_eq!(val, copy - (1 << 65));
        val = u128::MAX;
        assert_eq!(pop_lsb128(&mut val), 0);
        assert_eq!(val, u128::MAX - 1);
    }

    #[test]
    fn ith_one_u64_test() {
        assert_eq!(ith_one_u64(0, 1), 0);
        assert_eq!(ith_one_u64(0, 2), 1);
        assert_eq!(ith_one_u64(0, 3), 0);
        assert_eq!(ith_one_u64(1, 3), 1);
        assert_eq!(ith_one_u64(5, 0b1010101101), 9);
        assert_eq!(ith_one_u64(63, u64::MAX), 63);
    }

    #[test]
    fn ith_one_u128_test() {
        let mut rng = thread_rng();
        for _ in 0..10_000 {
            let val = rng.gen_range(0..=u64::MAX);
            let val_u128 = val as u128;
            let idx = rng.gen_range(0..val.count_ones()) as usize;
            assert_eq!(ith_one_u64(idx, val), ith_one_u128(idx, val_u128));
        }
        for i in 0..128 {
            assert_eq!(ith_one_u128(i, u128::MAX), i);
        }
        let val = (0b10010110110101010 << 80) + 0b11101;
        assert_eq!(ith_one_u128(3, val), 4);
        assert_eq!(ith_one_u128(4, val), 81);
    }
}
