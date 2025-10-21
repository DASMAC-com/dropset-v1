use std::fmt::Display;

use colored::{
    Color,
    Colorize,
};

#[derive(strum_macros::Display)]
#[strum(serialize_all = "UPPERCASE")]
enum Message {
    Info,
    Success,
    Warning,
    Error,
}

fn log(msg_ty: Message, label: impl Display, msg: impl Display) {
    let color = msg_ty.get_color();
    println!(
        "[{}] {} {}",
        msg_ty.to_string().color(color),
        label.to_string().color(LogColor::Debug),
        msg.to_string().bright_black()
    );
}

impl Message {
    fn get_color(&self) -> LogColor {
        match self {
            Self::Info => LogColor::Info,
            Self::Success => LogColor::Highlight,
            Self::Warning => LogColor::Warning,
            Self::Error => LogColor::Error,
        }
    }
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

    pub fn log_info(label: impl Display, msg: impl Display) { log(Message::Info, label, msg) }
    pub fn log_success(label: impl Display, msg: impl Display) { log(Message::Success, label, msg) }
    pub fn log_warning(label: impl Display, msg: impl Display) { log(Message::Warning, label, msg) }
    pub fn log_error(label: impl Display, msg: impl Display) { log(Message::Error, label, msg) }
    pub fn log_divider() { println!("--------------------------------------------------------------------------------"); }

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
