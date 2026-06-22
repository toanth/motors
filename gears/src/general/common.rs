use crate::games::PosHash;
use crate::general::common::Description::WithDescription;
use crate::score::Score;
pub use anyhow;
use anyhow::bail;
use colored::Colorize;
use edit_distance::edit_distance;
use itertools::Itertools;
pub use linkme::distributed_slice;
use num::{Float, PrimInt};
#[cfg(all(target_arch = "x86_64", target_feature = "bmi2", feature = "unsafe"))]
use std::arch::x86_64::{_pdep_u64, _pext_u64};
use std::fmt::{Debug, Display};
use std::io::stdin;
use std::iter::Peekable;
use std::num::{NonZeroU64, NonZeroUsize};
use std::str::{FromStr, SplitAsciiWhitespace};
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicI64, AtomicU64};
use std::time::Duration;

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

pub trait TokensToString {
    fn string(&mut self) -> String;
}

impl TokensToString for Tokens<'_> {
    fn string(&mut self) -> String {
        self.join(" ")
    }
}

pub type Tokens<'a> = Peekable<SplitAsciiWhitespace<'a>>;

pub fn tokens(input: &str) -> Tokens<'_> {
    input.split_ascii_whitespace().peekable()
}

pub fn tokens_to_string(first: &str, mut rest: Tokens) -> String {
    first.to_string() + " " + &rest.join(" ")
}

pub type Res<T> = anyhow::Result<T>;

pub fn sigmoid(score: Score, scale: f64) -> f64 {
    let x = f64::from(score.0);
    1.0 / (1.0 + (-x / scale).exp())
}

pub fn parse_fp_from_str<T: Float + FromStr>(as_str: &str, name: &str) -> Res<T> {
    as_str.parse::<T>().map_err(|_err| anyhow::anyhow!("Couldn't parse {name} ('{as_str}')"))
}

pub fn parse_int_from_str<T: PrimInt + FromStr>(as_str: &str, name: &str) -> Res<T> {
    // for some weird Rust reason, parse::<T>() returns a completely unbounded Err on failure,
    // so we just write the error message ourselves
    let parse = |s: &str| s.parse::<T>().map_err(|_| anyhow::anyhow!("Couldn't parse {name} ('{as_str}')"));
    match parse(as_str) {
        Ok(res) => Ok(res),
        Err(_err) => {
            if let Some(s) = as_str.strip_suffix("k") {
                Ok(parse(s)? * T::from(1000).ok_or(anyhow::anyhow!("'k' too large"))?)
            } else if let Some(s) = as_str.strip_suffix("m") {
                Ok(parse(s)? * T::from(1000 * 1000).ok_or(anyhow::anyhow!("'m' too large"))?)
            } else {
                bail!("Couldn't parse {name} ('{as_str}')")
            }
        }
    }
}

pub fn parse_int<T: PrimInt + FromStr + Display>(words: &mut Tokens, name: &str) -> Res<T> {
    parse_int_from_str(words.next().ok_or_else(|| anyhow::anyhow!("Missing {name}"))?, name)
}

pub fn parse_int_from_stdin<T: PrimInt + FromStr>() -> Res<T> {
    let mut s = String::default();
    _ = stdin().read_line(&mut s)?;
    parse_int_from_str(s.trim(), "integer")
}

pub fn parse_bool_from_str(input: &str, name: &str) -> Res<bool> {
    if input.eq_ignore_ascii_case("true") || input.eq_ignore_ascii_case("on") || input == "1" {
        Ok(true)
    } else if input.eq_ignore_ascii_case("false") || input.eq_ignore_ascii_case("off") || input == "0" {
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

pub fn parse_duration_ms(words: &mut Tokens, name: &str) -> Res<Duration> {
    let time = words.next().ok_or_else(|| anyhow::anyhow!("Missing {name}"))?;
    let ms: i64 = match parse_int_from_str(time, name) {
        Ok(res) => res,
        Err(err) => {
            if let Some(t) = time.strip_suffix("ms") {
                parse_int_from_str(t, name)?
            } else if let Some(t) = time.strip_suffix("s") {
                1000 * parse_int_from_str::<i64>(t, name)?
            } else if let Some(t) = time.strip_suffix("min") {
                60 * 1000 * parse_int_from_str::<i64>(t, name)?
            } else if let Some(t) = time.strip_suffix("h") {
                60 * 60 * 1000 * parse_int_from_str::<i64>(t, name)?
            } else {
                return Err(err);
            }
        }
    };
    // The UGI client can send negative remaining time.
    Ok(Duration::from_millis(ms.max(0) as u64))
}

/// Apparently, this will soon be unnecessary. Remove once stable Rust implements trait upcasting
// TODO: Remove
pub trait AsNamedEntity {
    fn upcast(&self) -> &dyn NamedEntity;
}
impl<T: NamedEntity> AsNamedEntity for T {
    fn upcast(&self) -> &dyn NamedEntity {
        self
    }
}

/// The name is used to identify the entity throughout all UIs and command line arguments.
/// Examples are games ('chess', 'mnk', etc), engines ('caps', 'random', etc), and UIs ('fen', 'pretty', etc)
pub trait NamedEntity: Debug + AsNamedEntity {
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
    fn autocomplete_badness(&self, input: &str, matcher: fn(&str, &str) -> isize) -> isize {
        matcher(input, &self.short_name())
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
        Self { short: t.short_name(), long: t.long_name(), description: t.description() }
    }
    pub fn from_name(string: &str) -> Self {
        Self { short: string.to_string(), long: string.to_string(), description: None }
    }
    pub fn from_name_descr(name: &str, descr: &str) -> Self {
        Self { short: name.to_string(), long: name.to_string(), description: Some(descr.to_string()) }
    }
}

pub type EntityList<T> = Vec<T>;

// TODO: Rework, description should be required
pub struct SimpleSelect<T> {
    pub name: &'static str,
    pub val: T, // can be a factory function / object in many cases
}

impl<T> Debug for SimpleSelect<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GenericSelect({})", self.name)
    }
}

impl<T> NamedEntity for SimpleSelect<T> {
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

#[derive(Debug, Clone)]
pub struct GenericSelect<T: Debug> {
    pub name: String,
    pub alternative_name: Option<String>,
    pub description: Option<String>,
    pub val: T,
}

impl<T: Debug> GenericSelect<T> {
    pub fn simple(name: &str, val: T) -> Self {
        Self { name: name.to_string(), alternative_name: None, description: None, val }
    }

    pub fn full(name: &str, alternative: Option<&str>, description: &str, val: T) -> Self {
        Self {
            name: name.to_string(),
            alternative_name: alternative.map(|s| s.to_string()),
            description: Some(description.to_string()),
            val,
        }
    }
}

impl<T: Debug> NamedEntity for GenericSelect<T> {
    fn short_name(&self) -> String {
        self.name.clone()
    }

    fn long_name(&self) -> String {
        if let Some(name) = self.alternative_name.clone() { name } else { self.name.clone() }
    }

    fn description(&self) -> Option<String> {
        self.description.clone()
    }

    fn matches(&self, name: &str) -> bool {
        name.eq_ignore_ascii_case(&self.name)
            || self.alternative_name.as_ref().is_some_and(|n| name.eq_ignore_ascii_case(n))
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Description {
    WithDescription,
    NoDescription,
}

fn list_to_string<I: Iterator + Clone, F: Fn(&I::Item) -> String>(iter: I, to_name: F) -> String {
    iter.map(|x| to_name(&x)).join(", ")
}

pub fn red_name(word: &str) -> String {
    let len = word.chars().count();
    if len > 50 {
        format!("{0}{1}", word.chars().take(50).collect::<String>().red(), "...(rest omitted for brevity)".dimmed())
    } else {
        word.red().to_string()
    }
}

#[cold]
fn error_msg<I: Iterator + Clone, F: Fn(&I::Item) -> String>(
    name: &str,
    mut list: I,
    typ: &str,
    game_name: &str,
    to_name: F,
) -> Res<I::Item> {
    let list_as_string = match list.clone().count() {
        0 => format!(
            "There are no valid {typ} names (presumably your program version was built with those features disabled)"
        ),
        1 => format!("The only valid {typ} for this version of the program is {}", to_name(&list.next().unwrap())),
        _ => {
            let near_matches = list
                .clone()
                .filter(|x| {
                    edit_distance(to_name(x).to_ascii_lowercase(), format!("'{}'", name.to_ascii_lowercase().bold()))
                        <= 3
                })
                .collect_vec();
            if near_matches.is_empty() {
                format!("Valid {typ} names are {}", list_to_string(list, to_name))
            } else {
                format!("Perhaps you meant: {}", list_to_string(near_matches.iter(), |x| to_name(x)))
            }
        }
    };
    let game_name = game_name.bold();
    bail!("Couldn't find {typ} '{}' for the current game ({game_name}). {list_as_string}.", red_name(name))
}

fn select_name_impl<I: Iterator + Clone, F: Fn(&I::Item) -> String, G: Fn(&I::Item, &str) -> bool>(
    name: &str,
    list: I,
    typ: &str,
    game_name: &str,
    to_name: F,
    matches: G,
) -> Res<I::Item> {
    let idx = list.clone().find(|entity| matches(entity, name));
    match idx {
        Some(res) => Ok(res),
        None => error_msg(name, list, typ, game_name, to_name),
    }
}

pub fn to_name_and_optional_description<T: NamedEntity + ?Sized>(x: &T, description: Description) -> String {
    if description == WithDescription {
        format!(
            "\n{name:<18} {descr}",
            name = format!("'{}':", x.short_name().bold()),
            descr = x.description().unwrap_or_else(|| "<No description>".to_string())
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

// There's probably a way to avoid having the exact same 1 line implementation for `select_name_static` and `select_name_dyn`
// (the only difference is that `select_name_dyn` uses `Box<dyn T>` instead of `T` for the element type,
// and `Box<dyn T>` doesn't satisfy `NamedEntity`, even though it's possible to call all the trait methods on it.)
/// Selects a NamedEntity based on its name from a supplied list and prints a helpful error message if the name doesn't exist.
pub fn select_name_static<'a, T: NamedEntity, I: Iterator<Item = &'a T> + Clone>(
    name: &str,
    list: I,
    typ: &str,
    game_name: &str,
    descr: Description,
) -> Res<&'a T> {
    select_name_impl(name, list, typ, game_name, |x| to_name_and_optional_description(*x, descr), |e, s| e.matches(s))
}

pub fn nonzero_usize(val: usize, name: &str) -> Res<NonZeroUsize> {
    NonZeroUsize::new(val).ok_or_else(|| anyhow::anyhow!("{name} can't be zero"))
}

pub fn nonzero_u64(val: u64, name: &str) -> Res<NonZeroU64> {
    NonZeroU64::new(val).ok_or_else(|| anyhow::anyhow!("{name} can't be zero"))
}

/// A simple `const` random number generator adapted from my C++ algebra implementation,
/// originally from here: <https://www.pcg-random.org/> (I hate that website)
pub struct PcgXslRr128_64Oneseq(u128);

const MUTLIPLIER: u128 = (2_549_297_995_355_413_924 << 64) + 4_865_540_595_714_422_341;
const INCREMENT: u128 = (6_364_136_223_846_793_005 << 64) + 1_442_695_040_888_963_407;

// the pcg xsl rr 128 64 oneseq generator, aka pcg64_oneseq (most other pcg generators have additional problems)
impl PcgXslRr128_64Oneseq {
    pub const fn new(seed: u128) -> Self {
        Self(seed.wrapping_add(INCREMENT).wrapping_mul(MUTLIPLIER).wrapping_add(INCREMENT))
    }

    pub const fn generate(&mut self) -> PosHash {
        self.0 = self.0.wrapping_mul(MUTLIPLIER);
        self.0 = self.0.wrapping_add(INCREMENT);
        let upper = (self.0 >> 64) as u64;
        let xored = upper ^ (self.0 as u64);
        PosHash(xored.rotate_right((upper >> (122 - 64)) as u32))
    }
}

pub struct Entry {
    sum: AtomicI64,
    calls: AtomicU64,
    name: &'static str,
}

impl Entry {
    pub const fn new(name: &'static str) -> Self {
        Self { sum: AtomicI64::new(0), calls: AtomicU64::new(0), name }
    }
    pub fn record(&self, val: i64) {
        _ = self.sum.fetch_add(val, Relaxed);
        _ = self.calls.fetch_add(1, Relaxed);
    }
    pub fn record_sum(&self, val: i64) {
        _ = self.sum.fetch_add(val, Relaxed);
    }
    pub fn reset(&self) {
        self.sum.store(0, Relaxed);
        self.calls.store(0, Relaxed);
    }

    pub fn print(&self, percent: bool) {
        let sum = self.sum.load(Relaxed);
        let calls = self.calls.load(Relaxed);
        let avg = sum as f64 / calls as f64;
        let name = self.name;
        if percent {
            let percent = avg * 100.;
            println!("{name}: {sum} of {calls} nodes ({percent:.2}%)");
        } else {
            println!("{name}: Sum {sum} of {calls} calls (average {avg:.3})");
        }
    }
}

#[distributed_slice]
pub static TRACKED_VALUES: [Entry];

#[distributed_slice]
pub static TRACKED_NODE_RELATIVE: [Entry];

#[macro_export]
macro_rules! track {
    ($name: expr, $val: expr) => {{
        #[linkme::distributed_slice($crate::general::common::TRACKED_VALUES)]
        static ENTRY: $crate::general::common::Entry = $crate::general::common::Entry::new(const { $name });
        let value = $val;
        ENTRY.record(value as i64);
        value
    }};
    ($val: expr) => {{ track!(stringify!($val), $val) }};
}

#[macro_export]
macro_rules! track_rel {
    ($name: expr, $val: expr) => {{
        #[linkme::distributed_slice($crate::general::common::TRACKED_NODE_RELATIVE)]
        static ENTRY: $crate::general::common::Entry = $crate::general::common::Entry::new(const { $name });
        let value = $val;
        ENTRY.record_sum(value as i64);
        value
    }};
    ($name: expr) => {{ track_rel!($name, 1) }};
}

pub fn dbg_print() {
    for e in TRACKED_VALUES {
        e.print(false);
    }
    for e in TRACKED_NODE_RELATIVE {
        e.print(true);
    }
}

pub fn dbg_reset() {
    for e in TRACKED_VALUES {
        e.reset();
    }
    for e in TRACKED_NODE_RELATIVE {
        e.reset();
    }
}

pub fn dbg_end_search(nodes: u64) {
    for e in TRACKED_NODE_RELATIVE {
        _ = e.calls.fetch_add(nodes, Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rng, RngExt};

    use crate::general::common::{ith_one_u128, ith_one_u64};
    // TODO: Test this on bitboards instead
    // #[test]
    // fn pop_lsb64_test() {
    //     let mut x = 1;
    //     assert_eq!(pop_lsb64(&mut x), 0);
    //     assert_eq!(x, 0);
    //     x = 2;
    //     assert_eq!(pop_lsb64(&mut x), 1);
    //     assert_eq!(x, 0);
    //     x = 3;
    //     assert_eq!(pop_lsb64(&mut x), 0);
    //     assert_eq!(x, 2);
    //     x = 0b110_001;
    //     assert_eq!(pop_lsb64(&mut x), 0);
    //     assert_eq!(x, 0b110_000);
    //     x = 0b1100_1011_0011_1001_0000_0000_0000_0000_0000;
    //     assert_eq!(pop_lsb64(&mut x), 20);
    //     assert_eq!(x, 0b1100_1011_0011_1000_0000_0000_0000_0000_0000);
    // }
    //
    // #[test]
    // fn pop_lsb128_test() {
    //     let mut rng = rng();
    //     for _ in 0..10_000 {
    //         let mut val = rng.random_range(0..=u64::MAX);
    //         let mut val_u128 = val as u128;
    //         assert_eq!(pop_lsb64(&mut val), pop_lsb128(&mut val_u128));
    //         assert_eq!(val, val_u128 as u64);
    //     }
    //     let mut val = u64::MAX as u128 + 1;
    //     assert_eq!(pop_lsb128(&mut val), 64);
    //     assert_eq!(val, 0);
    //     val = (0b100_0101_0110_1001_0101_1010 << 64) + 0b100_1010_0011;
    //     let copy = val;
    //     assert_eq!(pop_lsb128(&mut val), 0);
    //     assert_eq!(val, copy - 1);
    //     val = 0b100_0101_0110_1001_0101_1010 << 64;
    //     let copy = val;
    //     assert_eq!(pop_lsb128(&mut val), 65);
    //     assert_eq!(val, copy - (1 << 65));
    //     val = u128::MAX;
    //     assert_eq!(pop_lsb128(&mut val), 0);
    //     assert_eq!(val, u128::MAX - 1);
    // }

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
        let mut rng = rng();
        for _ in 0..10_000 {
            let val = rng.random_range(0..=u64::MAX);
            let val_u128 = val as u128;
            let idx = rng.random_range(0..val.count_ones()) as usize;
            assert_eq!(ith_one_u64(idx, val), ith_one_u128(idx, val_u128));
        }
        for i in 0..128 {
            assert_eq!(ith_one_u128(i, u128::MAX), i);
        }
        let val = (0b1_0010_1101_1010_1010 << 80) + 0b1_1101;
        assert_eq!(ith_one_u128(3, val), 4);
        assert_eq!(ith_one_u128(4, val), 81);
    }

    #[test]
    fn pcg_test() {
        let mut generator = PcgXslRr128_64Oneseq::new(42);
        assert_eq!(generator.0 >> 64, 1_610_214_578_838_163_691);
        assert_eq!(generator.0 & ((1 << 64) - 1), 13_841_303_961_814_150_380);
        let rand = generator.generate();
        assert_eq!(rand.0, 2_915_081_201_720_324_186);
        let rand = generator.generate();
        assert_eq!(rand.0, 13_533_757_442_135_995_717);
        let rand = generator.generate();
        assert_eq!(rand.0, 13_172_715_927_431_628_928);
    }
}
