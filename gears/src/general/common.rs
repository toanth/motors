use std::fmt::{Debug, Display};
use std::io::stdin;
use std::num::{NonZeroU64, NonZeroUsize};
use std::str::{FromStr, SplitWhitespace};

use bitintr::Pdep;
use colored::Colorize;
use itertools::Itertools;
use num::{Float, PrimInt};

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

pub type Res<T> = Result<T, String>;

pub fn parse_fp_from_str<T: Float + FromStr>(as_str: &str, name: &str) -> Res<T> {
    as_str.parse::<T>().map_err(|_err| format!("couldn't parse {name} ('{as_str}')"))
}

pub fn parse_int_from_str<T: PrimInt + FromStr>(as_str: &str, name: &str) -> Res<T> {
    // for some weird Rust reason, parse::<T>() returns a completely unbounded Err on failure,
    // so we just write the error message ourselves
    as_str
        .parse::<T>()
        .map_err(|_err| format!("couldn't parse {name} ('{as_str}')"))
}

pub fn parse_int<T: PrimInt + FromStr + Display>(
    words: &mut SplitWhitespace,
    name: &str,
) -> Res<T> {
    parse_int_from_str(words.next().ok_or_else(|| format!("missing {name}"))?, name)
}

pub fn parse_int_from_stdin<T: PrimInt + FromStr>() -> Res<T> {
    let mut s = String::default();
    stdin().read_line(&mut s).map_err(|e| e.to_string())?;
    parse_int_from_str(s.trim(), "integer")
}


/// The name is used to identify the entity throughout all UIs and command line arguments.
/// Examples are games ('chess', 'mnk', etc), engines ('caps', 'random', etc), and UIs ('fen', 'pretty', etc)
pub trait NamedEntity : Debug {
    /// The short name must consist of a single word in lowercase letters and is usually used for text-based UIs
    fn short_name(&self) -> &str;

    /// The long name can be prettier than the short name and consist of more than one word
    fn long_name(&self) -> &str;

    /// The optional description.
    fn description(&self) -> Option<&str>;

    fn matches(&self, name: &str) -> bool {
        self.short_name().eq_ignore_ascii_case(name)
    }
}

pub trait StaticallyNamedEntity: NamedEntity {
    fn static_short_name() -> &'static str where Self: Sized;

    fn static_long_name() -> &'static str where Self: Sized;

    fn static_description() -> &'static str where Self: Sized;
}

impl<T: StaticallyNamedEntity> NamedEntity for T {
    fn short_name(&self) -> &str {
        Self::static_short_name()
    }

    fn long_name(&self) -> &str {
        Self::static_long_name()
    }

    fn description(&self) -> Option<&str> {
        Some(Self::static_description())
    }
}

pub type EntityList<T> = Vec<T>;
// T is usually of a dyn trait
pub type DynEntityList<T> = Vec<Box<T>>;

#[derive(Debug)]
pub struct GenericSelect<T: Debug> {
    pub name: &'static str,
    pub val: T, // can be a factory function / object in many cases
}

impl<T: Debug> NamedEntity for GenericSelect<T> {
    fn short_name(&self) -> &str {
        self.name
    }

    fn long_name(&self) -> &str {
        self.name
    }

    fn description(&self) -> Option<&str> {
        None
    }
}

fn select_name_impl<'a, T, F: Fn(&T) -> &str, G: Fn(&T, &str) -> bool>(
    name: &str,
    list: &'a [T],
    typ: &str,
    game_name: &str,
    to_name: F,
    compare: G,
) -> Res<&'a T> {
    let idx = list.iter().find_position(|entity| compare(entity, name));
    match idx {
        None => {
            let list_as_string = match list.len() {
                0 => format!("There are no valid {typ} names (presumably your program version was built with those features disabled)"),
                1 => format!("The only valid {typ} for this version of the program is {}", to_name(list.iter().next().unwrap()).bold()),
                _ => format!("Valid {typ} names are {}", itertools::intersperse(
                    list.iter().map(|entity| to_name(entity).bold().to_string()),
                    ", ".to_string(),
                ).collect::<String>())
            };
            let game_name = game_name.bold();
            let name = name.red();
            Err(format!(
                "Couldn't find {typ} '{name}' for the current game ({game_name}). {list_as_string}."))
        }
        Some((_, res)) => Ok(res),
    }
}

pub fn select_name_dyn<'a, T: NamedEntity>(
    name: &str,
    list: &'a [Box<T>],
    typ: &str,
    game_name: &str,
) -> Res<&'a T>
where
    T: ?Sized,
{
    select_name_impl(name, list, typ, game_name, |e| e.short_name(), |e, s| e.matches(s)).map(|val| &**val)
}

/// There's probably a way to avoid having the exact same 1 line implementation for select_name_static and select_name_dyn
/// (the only difference is that select_name_dyn uses Box<dyn T> instead of T for the element type, and Box<dyn T> doesn't satisfy
/// NamedEntity, even though it's possible to call all the trait methods on it.)
pub fn select_name_static<'a, T: NamedEntity>(
    name: &str,
    list: &'a [T],
    typ: &str,
    game_name: &str,
) -> Res<&'a T> {
    select_name_impl(name, list, typ, game_name, |e| e.short_name(), |e, s| e.matches(s))
}


pub fn nonzero_usize(val: usize, name: &str) -> Res<NonZeroUsize> {
    NonZeroUsize::new(val).ok_or_else(|| format!("{name} can't be zero"))
}

pub fn nonzero_u64(val: u64, name: &str) -> Res<NonZeroU64> {
    NonZeroU64::new(val).ok_or_else(|| format!("{name} can't be zero"))
}

#[cfg(test)]
mod tests {
    use rand::{Rng, thread_rng};

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
        x = 0b1100_1011_0011_1001_0000_0000_0000_0000_0000;
        assert_eq!(pop_lsb64(&mut x), 20);
        assert_eq!(x, 0b1100_1011_0011_1000_0000_0000_0000_0000_0000);
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