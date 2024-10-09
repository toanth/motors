/*
 *  Motors, a collection of board game engines.
 *  Copyright (C) 2024 ToTheAnd
 *
 *  Motors is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Motors is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Motors. If not, see <https://www.gnu.org/licenses/>.
 */

// adapted from jw's akimbo engine
#[macro_export]
macro_rules! spsa_params {
    ($($name:ident: $typ:ty = $value:expr; $range:expr; step=$step:expr;)*) => {

        #[cfg(not(feature = "spsa"))]
        pub mod cc {
            use gears::ugi::{EngineOption, UgiSpin};
            use gears::score::ScoreT;
            use $crate::Res;

            $(
                pub const fn $name() -> $typ {
                    return $value;
                }
            )*

            pub fn set_value(_: &str, _: isize) -> Res<()> {
                gears::general::common::anyhow::bail!("The SPSA feature isn't enabled, so it's not possible to set an SPSA value")
            }

            #[allow(unused)]
            pub const fn params() -> Vec<(&'static str, UgiSpin, usize)> {
                vec![]
            }

            pub fn ugi_options() -> Vec<EngineOption> {
                vec![]
            }

            pub fn ob_param_string() -> Vec<String> {
                vec![]
            }
        }

        #[cfg(feature = "spsa")]
        pub mod cc {
            use gears::ugi::{EngineOption, EngineOptionName, EngineOptionType};
            use gears::score::ScoreT;
            use gears::ugi::UgiSpin;
            use $crate::Res;

            mod vals {
                use super::*;

                // The fact that these are static mut doesn't cause any data races because when SPSAing, there is only one
                // engine thread, and only that sets and reads the values.
                // Unfortunately, RangeInclusive can't be used in a const fn until Rust 1.83
                $(
                    #[allow(non_upper_case_globals)]
                    pub(super) static mut $name: $typ = $value;
                )*
            }
            $(
                pub fn $name() -> $typ {
                    // SAFETY: Mutable statics are unsafe because they are not thread safe. But we're never setting or reading
                    // spsa values from any thread but the (main) search thread, so this isn't a concern.
                    unsafe {
                        vals::$name
                    }
                }
            )*
            pub fn set_value(name: &str, value: isize) -> Res<()> {
                // SAFETY: Mutable statics are unsafe because they are not thread safe. But we're never setting or reading
                // spsa values from any thread but the (main) search thread, so this isn't a concern.
                unsafe {
                    match name {
                        $(
                            stringify!($name) => vals::$name = <$typ>::try_from(value).unwrap(),
                        )*
                        _ => { gears::general::common::anyhow::bail!("'{name}' is not a valid SPSA parameter name") }
                    }
                    Ok(())
                }
            }
            pub fn params() -> Vec<(&'static str, UgiSpin, usize)> {
                vec![
                    $(
                        (
                            stringify!($name),
                            UgiSpin {
                                val: $name() as i64,
                                default: Some($value as i64),
                                min: Some(*$range.start() as i64),
                                max: Some(*$range.end() as i64),
                            },
                            $step
                        ),
                    )*
                ]
            }

            pub fn ugi_options() -> Vec<EngineOption> {
               params().into_iter().map(|(name, spin, _step)|
                    EngineOption {
                        name: EngineOptionName::Other(name.to_string()),
                        value: EngineOptionType::Spin(spin)
                    }
               )
               .collect()
            }

            pub fn ob_param_string() -> Vec<String> {
                params().into_iter().map(|(name, spin, step)| format!("{name}, int, {0}, {1}, {2}, {step}, 0.002", spin.val, spin.min.unwrap(), spin.max.unwrap())).collect()
            }
        }
    };
}
