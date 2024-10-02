pub use anyhow;
#[cfg(all(target_arch = "x86_64", target_feature = "bmi2", feature = "unsafe"))]
use std::arch::x86_64::{_pdep_u64, _pext_u64};
use std::fmt::{Debug, Display};
use std::io::stdin;
use std::iter::Peekable;
use std::num::{NonZeroU64, NonZeroUsize};
use std::str::{FromStr, SplitWhitespace};
use std::time::Duration;

use colored::Colorize;
use edit_distance::edit_distance;
use itertools::{Intersperse, Itertools};
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

// The `bitintr` crate provides similar features, but unfortunately it is bugged and unmaintained.

#[allow(unused)]
fn pdep64_fallback(val: u64, mut mask: u64) -> u64 {
    let mut res = 0;
    let mut bb = 1;
    while mask != 0 {
        if (val & bb) != 0 {
            res |= mask & mask.wrapping_neg();
        }
        mask &= mask - 1;
        bb = bb.wrapping_add(bb);
    }
    res
}

#[allow(unused)]
fn pext64_fallback(val: u64, mut mask: u64) -> u64 {
    let mut res = 0;
    let mut bb: u64 = 1;
    while mask != 0 {
        if val & mask & (mask.wrapping_neg()) != 0 {
            res |= bb;
        }
        mask &= mask - 1;
        bb = bb.wrapping_add(bb);
    }
    res
}

#[inline]
#[cfg(all(target_feature = "bmi2", target_arch = "x86_64", feature = "unsafe"))]
fn pdep64(val: u64, mask: u64) -> u64 {
    // SAFETY: This is always safe, due to the `target_feature` check above.
    // No combination of arguments to pdep produce UB
    unsafe { _pdep_u64(val, mask) }
}

#[inline]
#[allow(unused)]
#[cfg(not(all(target_feature = "bmi2", feature = "unsafe")))]
fn pdep64(val: u64, mask: u64) -> u64 {
    pdep64_fallback(val, mask)
}

#[inline]
#[allow(unused)]
#[cfg(all(target_feature = "bmi2", target_arch = "x86_64", feature = "unsafe"))]
fn pext64(val: u64, mask: u64) -> u64 {
    // SAFETY: This is always safe, due to the `target_feature` check above.
    // No combination of arguments to pext produce UB
    unsafe { _pext_u64(val, mask) }
}

#[inline]
#[allow(unused)]
#[cfg(not(all(target_feature = "bmi2", target_arch = "x86_64", feature = "unsafe")))]
fn pext64(val: u64, mask: u64) -> u64 {
    pext64_fallback(val, mask)
}

#[must_use]
#[inline]
pub fn ith_one_u64(idx: usize, val: u64) -> usize {
    debug_assert!(idx < val.count_ones() as usize);
    pdep64(1 << idx, val).trailing_zeros() as usize
}

#[must_use]
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

pub type Res<T> = anyhow::Result<T>;

pub fn parse_fp_from_str<T: Float + FromStr>(as_str: &str, name: &str) -> Res<T> {
    as_str
        .parse::<T>()
        .map_err(|_err| anyhow::anyhow!("Couldn't parse {name} ('{as_str}')"))
}

pub fn parse_int_from_str<T: PrimInt + FromStr>(as_str: &str, name: &str) -> Res<T> {
    // for some weird Rust reason, parse::<T>() returns a completely unbounded Err on failure,
    // so we just write the error message ourselves
    as_str
        .parse::<T>()
        .map_err(|_err| anyhow::anyhow!("Couldn't parse {name} ('{as_str}')"))
}

pub fn parse_int<T: PrimInt + FromStr + Display>(
    words: &mut Peekable<SplitWhitespace>,
    name: &str,
) -> Res<T> {
    parse_int_from_str(
        words
            .next()
            .ok_or_else(|| anyhow::anyhow!("Missing {name}"))?,
        name,
    )
}

pub fn parse_int_from_stdin<T: PrimInt + FromStr>() -> Res<T> {
    let mut s = String::default();
    stdin().read_line(&mut s)?;
    parse_int_from_str(s.trim(), "integer")
}

pub fn parse_bool_from_str(input: &str, name: &str) -> Res<bool> {
    if input.eq_ignore_ascii_case("true") {
        Ok(true)
    } else if input.eq_ignore_ascii_case("false") {
        Ok(false)
    } else {
        Err(anyhow::anyhow!(
            "Incorrect value for '{0}': Expected either '{1}' or '{2}', not '{3}'",
            name.bold(),
            "true".bold(),
            "false".bold(),
            input.red(),
        ))
    }
}

pub fn parse_duration_ms(words: &mut Peekable<SplitWhitespace>, name: &str) -> Res<Duration> {
    let num_ms: i64 = parse_int(words, name)?;
    // The UGI client can send negative remaining time.
    Ok(Duration::from_millis(num_ms.max(0) as u64))
}

/// The name is used to identify the entity throughout all UIs and command line arguments.
/// Examples are games ('chess', 'mnk', etc), engines ('caps', 'random', etc), and UIs ('fen', 'pretty', etc)
pub trait NamedEntity: Debug {
    /// The short name must consist of a single word in lowercase letters and is usually used for text-based UIs
    fn short_name(&self) -> String;

    /// The long name can be prettier than the short name and consist of more than one word
    fn long_name(&self) -> String;

    /// The optional description.
    fn description(&self) -> Option<String>;

    /// Does an input match the name?
    /// This can be overwritten in an implementation to consider additional names
    fn matches(&self, name: &str) -> bool {
        self.short_name().eq_ignore_ascii_case(name)
    }

    /// Is `name` (close to) a prefix of this entity's name, as determined by `matcher`?
    /// This can be overwritten in an implementation to consider additional names.
    /// 0 means an exact match, higher values are worse matches
    fn autocomplete_badness(&self, input: &str, matcher: fn(&str, &str) -> usize) -> usize {
        matcher(input, &self.short_name())
    }

    /// Some named entities, mostly commands, have subcommands
    fn sub_entities_completion(&self) -> &[Box<dyn NamedEntity>] {
        &[]
    }

    /// Other short names for this entity
    fn secondary_names(&self) -> Vec<String> {
        vec![]
    }
}

pub trait StaticallyNamedEntity: NamedEntity {
    fn static_short_name() -> impl Display
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
    fn short_name(&self) -> String {
        Self::static_short_name().to_string()
    }

    fn long_name(&self) -> String {
        Self::static_long_name().to_string()
    }

    fn description(&self) -> Option<String> {
        Some(Self::static_description())
    }
}

#[derive(Debug, Clone, Default)]
#[must_use]
pub struct Name {
    pub short: String,
    pub long: String,
    pub description: Option<String>,
}

impl NamedEntity for Name {
    fn short_name(&self) -> String {
        self.short.clone()
    }

    fn long_name(&self) -> String {
        self.long.clone()
    }

    fn description(&self) -> Option<String> {
        self.description.clone()
    }
}

impl Name {
    pub fn new<T: NamedEntity + ?Sized>(t: &T) -> Self {
        Self {
            short: t.short_name(),
            long: t.long_name(),
            description: t.description(),
        }
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
    fn short_name(&self) -> String {
        self.name.to_string()
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
    iter.map(|x| to_name(&x))
        .intersperse_(", ".to_string())
        .collect::<String>()
}

fn select_name_impl<
    I: ExactSizeIterator + Clone,
    F: Fn(&I::Item) -> String,
    G: Fn(&I::Item, &str) -> bool,
>(
    mut name: Option<&str>,
    mut list: I,
    typ: &str,
    game_name: &str,
    to_name: F,
    compare: G,
) -> Res<I::Item> {
    if let Some("list") = name {
        name = None;
    }
    let idx = match name {
        None => None,
        Some(name) => list.clone().find(|entity| compare(entity, name)),
    };
    match idx {
        None => {
            let list_as_string = match list.len() {
                0 => format!("There are no valid {typ} names (presumably your program version was built with those features disabled)"),
                1 => format!("The only valid {typ} for this version of the program is {}", to_name(&list.next().unwrap())),
                _ => {
                    match name {
                        None => { format!("Valid {typ} names are {}", list_to_string(list, to_name)) }
                        Some(name) => {
                            let near_matches = list.clone().filter(|x|
                                edit_distance(&to_name(x).to_ascii_lowercase(), &format!("'{}'", name.to_ascii_lowercase().bold())) <= 3
                            ).collect_vec();
                            if near_matches.is_empty() {
                                format!("Valid {typ} names are {}", list_to_string(list, to_name))
                            } else {
                                format!("Perhaps you meant: {}", list_to_string(near_matches.iter(), |x| to_name(x)))
                            }
                        }
                    }
                }
            };
            let game_name = game_name.bold();
            if let Some(name) = name {
                let name = name.red();
                Err(anyhow::anyhow!(
                    "Couldn't find {typ} '{name}' for the current game ({game_name}). {list_as_string}."))
            } else {
                Err(anyhow::anyhow!(list_as_string))
            }
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
        Some(name),
        list.iter(),
        typ,
        game_name,
        |x| to_name_and_optional_description(x.as_ref(), descr),
        |e, s| e.matches(s),
    )
    .map(|val| &**val)
}

// There's probably a way to avoid having the exact same 1 line implementation for `select_name_static` and `select_name_dyn`
// (the only difference is that `select_name_dyn` uses `Box<dyn T>` instead of `T` for the element type,
// and `Box<dyn T>` doesn't satisfy `NamedEntity`, even though it's possible to call all the trait methods on it.)
/// Selects a NamedEntity based on its name from a supplied list and prints a helpful error message if the name doesn't exist.
pub fn select_name_static<'a, T: NamedEntity, I: ExactSizeIterator<Item = &'a T> + Clone>(
    name: &str,
    list: I,
    typ: &str,
    game_name: &str,
    descr: Description,
) -> Res<&'a T> {
    select_name_impl(
        Some(name),
        list,
        typ,
        game_name,
        |x| to_name_and_optional_description(*x, descr),
        |e, s| e.matches(s),
    )
}

pub fn nonzero_usize(val: usize, name: &str) -> Res<NonZeroUsize> {
    NonZeroUsize::new(val).ok_or_else(|| anyhow::anyhow!("{name} can't be zero"))
}

pub fn nonzero_u64(val: u64, name: &str) -> Res<NonZeroU64> {
    NonZeroU64::new(val).ok_or_else(|| anyhow::anyhow!("{name} can't be zero"))
}

/// Avoid the warning about [`Itertools::intersperse`] conflicting with a future [`Iter::intersperse`]
/// and keep using a nicer syntax than of UFCS
pub trait IterIntersperse: Itertools + Sized {
    fn intersperse_(self, element: Self::Item) -> Intersperse<Self>
    where
        Self::Item: Clone,
    {
        itertools::intersperse(self, element)
    }
}

impl<I: Itertools> IterIntersperse for I {}

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
        x = 0b110_001;
        assert_eq!(pop_lsb64(&mut x), 0);
        assert_eq!(x, 0b110_000);
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
        val = (0b100_0101_0110_1001_0101_1010 << 64) + 0b100_1010_0011;
        let copy = val;
        assert_eq!(pop_lsb128(&mut val), 0);
        assert_eq!(val, copy - 1);
        val = 0b100_0101_0110_1001_0101_1010 << 64;
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
        assert_eq!(ith_one_u64(5, 0b10_1010_1101), 9);
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
        let val = (0b1_0010_1101_1010_1010 << 80) + 0b1_1101;
        assert_eq!(ith_one_u128(3, val), 4);
        assert_eq!(ith_one_u128(4, val), 81);
    }
}
