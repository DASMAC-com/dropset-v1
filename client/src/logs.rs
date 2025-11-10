//! Utilities for colorized, human-readable logging and key/value formatting.
//!
//! Provides macros for printing colored key/value pairs, an enum for color usage across the client,
//! and simple helpers for log dividers.

use colored::Color;

/// Format a key/value pair with optional color overrides.
///
/// Examples, where `key_color` and `value_color` are `LogColor` values:
/// - fmt_kv!(key, value)
/// - fmt_kv!(key, value, key_color)
/// - fmt_kv!(key, value, key_color, value_color)
#[macro_export]
macro_rules! fmt_kv {
    ($key:expr, $value:expr $(,)?) => {
        $crate::fmt_kv!(
            $key,
            $value,
            $crate::LogColor::Highlight,
            $crate::LogColor::FadedGray
        )
    };
    ($key:expr, $value:expr, $key_color:expr $(,)?) => {
        $crate::fmt_kv!($key, $value, $key_color, $crate::LogColor::FadedGray)
    };
    ($key:expr, $value:expr, $key_color:expr, $value_color:expr $(,)?) => {{
        use ::colored::Colorize;

        let __k = ::std::string::ToString::to_string(&$key);
        let __v = ::std::string::ToString::to_string(&$value);
        ::std::format!("{}: {}", __k.color($key_color), __v.color($value_color))
    }};
}

/// Prints a key/value pair with optional color overrides.
///
/// Examples, where `key_color` and `value_color` are `LogColor` values:
/// - fmt_kv!(key, value)
/// - fmt_kv!(key, value, key_color)
/// - fmt_kv!(key, value, key_color, value_color)
#[macro_export]
macro_rules! print_kv {
    ($key:expr, $value:expr $(,)?) => {
        ::std::println!("{}", $crate::fmt_kv!($key, $value))
    };
    ($key:expr, $value:expr, $key_color:expr $(,)?) => {
        ::std::println!("{}", $crate::fmt_kv!($key, $value, $key_color))
    };
    ($key:expr, $value:expr, $key_color:expr, $value_color:expr $(,)?) => {
        ::std::println!(
            "{}",
            $crate::fmt_kv!($key, $value, $key_color, $value_color)
        )
    };
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum LogColor {
    Highlight,
    Debug,
    Error,
    Warning,
    Header,
    Info,
    Gray,
    FadedGray,
}

#[rustfmt::skip]
mod unformatted {
    use super::*;

    pub fn fmt_divider() -> String { "--------------------------------------------------------------------------------".into() }

    pub fn log_divider() { println!("{}", fmt_divider()); }

    impl From<LogColor> for Color {
        fn from(value: LogColor) -> Color {
            match value {
                LogColor::Highlight  => Color::TrueColor { r: 255, g: 215, b: 87  },
                LogColor::Debug      => Color::TrueColor { r: 40, g: 100,  b: 153 },
                LogColor::Error      => Color::TrueColor { r: 255, g: 0,   b: 45  },
                LogColor::Warning    => Color::TrueColor { r: 180, g: 105, b: 0   },
                LogColor::Header     => Color::TrueColor { r: 0,   g: 255, b: 0   },
                LogColor::Info       => Color::TrueColor { r: 0,   g: 95,  b: 255 },
                LogColor::Gray       => Color::TrueColor { r: 192, g: 192, b: 192 },
                LogColor::FadedGray  => Color::TrueColor { r: 95,  g: 95,  b: 95  },
            }
        }
    }
}

pub use unformatted::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fmt_and_print_kv() {
        let _ = fmt_kv!("hello", "world");
        let _ = fmt_kv!("hello", "world", LogColor::Info);
        let _ = fmt_kv!("hello", "world", LogColor::Info, LogColor::Highlight);
        print_kv!("hello", "world");
        print_kv!("hello", "world", LogColor::Info);
        print_kv!("hello", "world", LogColor::Info, LogColor::Highlight);
    }
}
