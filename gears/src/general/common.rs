use std::fmt::{Debug, Display};
use std::io::stdin;
use std::num::{NonZeroU64, NonZeroUsize};
use std::str::{FromStr, SplitWhitespace};
use std::time::Duration;

use bitintr::Pdep;
use colored::Colorize;
use edit_distance::edit_distance;
use itertools::Itertools;
use num::{Float, PrimInt};

use crate::general::common::Description::WithDescription;

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
    as_str
        .parse::<T>()
        .map_err(|_err| format!("Couldn't parse {name} ('{as_str}')"))
}

pub fn parse_int_from_str<T: PrimInt + FromStr>(as_str: &str, name: &str) -> Res<T> {
    // for some weird Rust reason, parse::<T>() returns a completely unbounded Err on failure,
    // so we just write the error message ourselves
    as_str
        .parse::<T>()
        .map_err(|_err| format!("Couldn't parse {name} ('{as_str}')"))
}

pub fn parse_int<T: PrimInt + FromStr + Display>(
    words: &mut SplitWhitespace,
    name: &str,
) -> Res<T> {
    parse_int_from_str(words.next().ok_or_else(|| format!("Missing {name}"))?, name)
}

pub fn parse_int_from_stdin<T: PrimInt + FromStr>() -> Res<T> {
    let mut s = String::default();
    stdin().read_line(&mut s).map_err(|e| e.to_string())?;
    parse_int_from_str(s.trim(), "integer")
}

pub fn parse_duration_ms(words: &mut SplitWhitespace, name: &str) -> Res<Duration> {
    let num_ms: i64 = parse_int(words, name)?;
    // The UGI client can send negative remaining time.
    Ok(Duration::from_millis(num_ms.max(0) as u64))
}

/// The name is used to identify the entity throughout all UIs and command line arguments.
/// Examples are games ('chess', 'mnk', etc), engines ('caps', 'random', etc), and UIs ('fen', 'pretty', etc)
pub trait NamedEntity: Debug {
    /// The short name must consist of a single word in lowercase letters and is usually used for text-based UIs
    fn short_name(&self) -> &str;

    /// The long name can be prettier than the short name and consist of more than one word
    fn long_name(&self) -> String;

    /// The optional description.
    fn description(&self) -> Option<String>;

    fn matches(&self, name: &str) -> bool {
        self.short_name().eq_ignore_ascii_case(name)
    }
}

pub trait StaticallyNamedEntity: NamedEntity {
    fn static_short_name() -> &'static str
    where
        Self: Sized;

    fn static_long_name() -> String
    where
        Self: Sized;

    fn static_description() -> String
    where
        Self: Sized;
}

impl<T: StaticallyNamedEntity> NamedEntity for T {
    fn short_name(&self) -> &str {
        Self::static_short_name()
    }

    fn long_name(&self) -> String {
        Self::static_long_name().to_string()
    }

    fn description(&self) -> Option<String> {
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

    fn long_name(&self) -> String {
        self.name.to_string()
    }

    fn description(&self) -> Option<String> {
        None
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Description {
    WithDescription,
    NoDescription,
}

fn list_to_string<I: ExactSizeIterator + Clone, F: Fn(&I::Item) -> String>(
    iter: I,
    to_name: F,
) -> String {
    itertools::intersperse(iter.map(|x| to_name(&x)), ", ".to_string()).collect::<String>()
}

fn select_name_impl<
    'a,
    I: ExactSizeIterator + Clone,
    F: Fn(&I::Item) -> String,
    G: Fn(&I::Item, &str) -> bool,
>(
    name: &str,
    mut list: I,
    typ: &str,
    game_name: &str,
    to_name: F,
    compare: G,
) -> Res<I::Item> {
    let idx = list.clone().find(|entity| compare(entity, name));
    match idx {
        None => {
            let list_as_string = match list.len() {
                0 => format!("There are no valid {typ} names (presumably your program version was built with those features disabled)"),
                1 => format!("The only valid {typ} for this version of the program is {}", to_name(&list.next().unwrap())),
                _ => {
                    let near_matches = list.clone().filter(|x|
                        edit_distance(&to_name(x).to_ascii_lowercase(), &format!("'{}'", name.to_ascii_lowercase().bold())) <= 3
                    ).collect_vec();
                    if near_matches.is_empty() {
                        format!("Valid {typ} names are {}", list_to_string(list, to_name))
                    } else {
                        format!("Perhaps you meant: {}", list_to_string(near_matches.iter(), |x| to_name(x)))
                    }
                }
            };
            let game_name = game_name.bold();
            let name = name.red();
            Err(format!(
                "Couldn't find {typ} '{name}' for the current game ({game_name}). {list_as_string}."))
        }
        Some(res) => Ok(res),
    }
}

pub fn to_name_and_optional_description<T: NamedEntity + ?Sized>(
    x: &T,
    description: Description,
) -> String {
    if description == WithDescription {
        format!(
            "\n{name:<18} {descr}",
            name = format!("'{}':", x.short_name().bold()),
            descr = x
                .description()
                .unwrap_or_else(|| "<No description>".to_string())
        )
    } else {
        format!("'{}'", x.short_name().bold())
    }
}

pub fn select_name_dyn<'a, T: NamedEntity + ?Sized>(
    name: &str,
    list: &'a [Box<T>],
    typ: &str,
    game_name: &str,
    descr: Description,
) -> Res<&'a T> {
    select_name_impl(
        name,
        list.iter(),
        typ,
        game_name,
        |x| to_name_and_optional_description(x.as_ref(), descr),
        |e, s| e.matches(s),
    )
    .map(|val| &**val)
}

/// There's probably a way to avoid having the exact same 1 line implementation for `select_name_static` and `select_name_dyn`
/// (the only difference is that `select_name_dyn` uses `Box<dyn T>` instead of `T` for the element type,
/// and `Box<dyn T>` doesn't satisfy `NamedEntity`, even though it's possible to call all the trait methods on it.)
pub fn select_name_static<'a, T: NamedEntity, I: ExactSizeIterator<Item = &'a T> + Clone>(
    name: &str,
    list: I,
    typ: &str,
    game_name: &str,
    descr: Description,
) -> Res<&'a T> {
    select_name_impl(
        name,
        list,
        typ,
        game_name,
        |x| to_name_and_optional_description(*x, descr),
        |e, s| e.matches(s),
    )
}

pub fn nonzero_usize(val: usize, name: &str) -> Res<NonZeroUsize> {
    NonZeroUsize::new(val).ok_or_else(|| format!("{name} can't be zero"))
}

pub fn nonzero_u64(val: u64, name: &str) -> Res<NonZeroU64> {
    NonZeroU64::new(val).ok_or_else(|| format!("{name} can't be zero"))
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
